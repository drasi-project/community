#!/usr/bin/env python3
"""Bootstrap and maintain this copy's Mattermost, the incident channel (plan/contracts.md § 13).

`mattermost.py database` creates the database `mattermost` in this copy's PostgreSQL
container (before Mattermost starts). `mattermost.py bootstrap` (run by `ops/observability-stack.sh up` when it starts Mattermost)
makes sure everything the demo needs exists, creating only what is missing:

- users `admin` (system admin) and `oncall` ("On-call engineer", the presenter), team
  `ai-platform`, public channel `incident-ai-support`, bot `drasi`, all members of both;
- personal access tokens for `admin`, `oncall` and the bot, and an incoming webhook for
  Grafana's alerts.

Secrets go to demo/.env (mode 600) and are never printed. Non-secret IDs go to
demo/.native/mattermost.json, which the Drasi config generator and the AI service read.
The bootstrap talks to Mattermost through `mmctl --local` inside its container, so it needs
no stored admin session. Exit status 3 means the Grafana webhook URL changed (the stack
script then recreates Grafana so its contact point picks it up).
"""
import importlib.util
import json
import secrets
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import settings  # noqa: E402

DEMO = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("observability_env", DEMO / "ops/observability-env.py")
envfile = importlib.util.module_from_spec(spec)
spec.loader.exec_module(envfile)

TEAM, CHANNEL = "ai-platform", "incident-ai-support"
IDS = DEMO / ".native/mattermost.json"
URL = settings.url("MATTERMOST")
CONTAINER = settings.container("mattermost")
WEBHOOK = "Grafana alerts"


def mmctl(*args, check=True):
    """mmctl in local mode inside the container; JSON output parsed. Secrets stay in memory."""
    result = subprocess.run(["podman", "exec", CONTAINER, "mmctl", "--local", "--json", *args],
                            capture_output=True, text=True)
    if check and result.returncode != 0:
        # mmctl echoes its arguments, never secrets we pass on stdin; passwords are passed as
        # arguments, so report only the command name.
        raise SystemExit(f"mmctl {args[0]} {args[1] if len(args) > 1 else ''} failed: "
                         f"{result.stderr.strip().splitlines()[-1] if result.stderr.strip() else result.returncode}")
    text = result.stdout.strip()
    if not text:
        return None
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        # Some commands print one JSON document per line.
        return [json.loads(line) for line in text.splitlines() if line.strip().startswith(("{", "["))]


def api(method, route, token, body=None):
    request = urllib.request.Request(URL + route, method=method,
                                     data=None if body is None else json.dumps(body).encode())
    request.add_header("authorization", f"Bearer {token}")
    request.add_header("content-type", "application/json")
    try:
        with urllib.request.urlopen(request, timeout=10) as response:
            return response.status, json.loads(response.read() or b"null")
    except urllib.error.HTTPError as error:
        return error.code, None


def database():
    env = envfile.read_env()
    psql = ["podman", "exec", "-i", settings.container("postgres"), "psql", "-X", "-q", "-t", "-A",
            "-U", env["POSTGRES_USER"], "-d", env["POSTGRES_DB"]]
    found = subprocess.run([*psql, "-c", "SELECT 1 FROM pg_database WHERE datname='mattermost'"],
                           capture_output=True, text=True, check=True).stdout.strip()
    if found != "1":
        subprocess.run([*psql, "-c", "CREATE DATABASE mattermost"], capture_output=True, text=True, check=True)
        print("Created database mattermost.")


def wait():
    for _ in range(120):
        try:
            with urllib.request.urlopen(URL + "/api/v4/system/ping", timeout=3) as response:
                if response.status == 200:
                    return
        except OSError:
            pass
        time.sleep(1)
    raise SystemExit(f"Mattermost is not answering on {URL}")


def first(items, **match):
    return next((i for i in (items or []) if all(i.get(k) == v for k, v in match.items())), None)


def user(name, email, password, nickname, admin=False):
    found = mmctl("user", "search", name, check=False)
    found = found if isinstance(found, list) else [found] if found else []
    existing = first(found, username=name)
    if existing:
        return existing
    args = ["user", "create", "--username", name, "--email", email, "--password", password,
            "--nickname", nickname, "--email-verified"]
    if admin:
        args.append("--system-admin")
    created = mmctl(*args)
    print(f"Created Mattermost user {name}.")
    return created[0] if isinstance(created, list) else created


def token(key, username, user_id, description):
    """Keep the token in .env while it still works for this user; otherwise make a new one."""
    current = envfile.read_env().get(key)
    if current:
        status, me = api("GET", "/api/v4/users/me", current)
        if status == 200 and me and me.get("id") == user_id:
            return False
    created = mmctl("token", "generate", username, description)
    created = created[0] if isinstance(created, list) else created
    envfile.write_env(key, created["token"])
    print(f"Wrote a new Mattermost token for {username} to demo/.env ({key}); not printed.")
    return True


def ensure_secret(key):
    if not envfile.read_env().get(key):
        envfile.write_env(key, secrets.token_urlsafe(24))
    return envfile.read_env()[key]


def bootstrap():
    if not envfile.path.exists():
        raise SystemExit("Run ops/observability-stack.sh env first.")
    wait()
    admin_password = ensure_secret("MATTERMOST_ADMIN_PASSWORD")
    oncall_password = ensure_secret("MATTERMOST_ONCALL_PASSWORD")
    admin = user("admin", "admin@ai-platform.local", admin_password, "Administrator", admin=True)
    oncall = user("oncall", "oncall@ai-platform.local", oncall_password, "On-call engineer")

    team = first(mmctl("team", "list"), name=TEAM)
    if not team:
        team = mmctl("team", "create", "--name", TEAM, "--display-name", "AI Platform", "--email", "admin@ai-platform.local")
        team = team[0] if isinstance(team, list) else team
        print(f"Created Mattermost team {TEAM}.")

    # `bot list` needs a session; local mode finds the bot as a user.
    found = mmctl("user", "search", "drasi", check=False)
    bot = first(found if isinstance(found, list) else [found] if found else [], username="drasi")
    # Bots can't be created in local mode: use the admin's token.
    token("MATTERMOST_ADMIN_TOKEN", "admin", admin["id"], "ignite-ai-service")
    if not bot:
        status, bot = api("POST", "/api/v4/bots", envfile.read_env()["MATTERMOST_ADMIN_TOKEN"],
                          {"username": "drasi", "display_name": "Drasi",
                           "description": "Drasi's incident cards (HTTP reaction incident-channel)"})
        if status != 201:
            raise SystemExit(f"Creating the Mattermost bot failed ({status})")
        print("Created Mattermost bot drasi.")
    bot_id = bot.get("user_id") or bot.get("id")

    mmctl("team", "users", "add", TEAM, "admin", "oncall", "drasi")
    channel = mmctl("channel", "search", "--team", TEAM, CHANNEL, check=False)
    channel = channel[0] if isinstance(channel, list) and channel else channel
    if not channel or channel.get("name") != CHANNEL:
        channel = mmctl("channel", "create", "--team", TEAM, "--name", CHANNEL, "--display-name", CHANNEL,
                        "--purpose", "Incidents for the customer support assistant and the summaries job")
        channel = channel[0] if isinstance(channel, list) else channel
        print(f"Created Mattermost channel #{CHANNEL}.")
    mmctl("channel", "users", "add", f"{TEAM}:{CHANNEL}", "admin", "oncall", "drasi")

    # Names as the channel shows them (full-name display).
    for person, first_name, last_name in [(admin, "Platform", "admin"), (oncall, "On-call", "engineer")]:
        api("PUT", f"/api/v4/users/{person['id']}/patch", envfile.read_env()["MATTERMOST_ADMIN_TOKEN"],
            {"first_name": first_name, "last_name": last_name})
    token("MATTERMOST_ONCALL_TOKEN", "oncall", oncall["id"], "ignite-desk")
    token("IGNITE_MATTERMOST_BOT_TOKEN", "drasi", bot_id, "drasi-incident-channel")

    hooks = mmctl("webhook", "list", TEAM) or []
    hooks = hooks if isinstance(hooks, list) else [hooks]
    hook = first(hooks, display_name=WEBHOOK, channel_id=channel["id"])
    if not hook:
        hook = mmctl("webhook", "create-incoming", "--channel", f"{TEAM}:{CHANNEL}", "--user", "admin",
                     "--display-name", WEBHOOK, "--description", "Grafana alert notifications", "--lock-to-channel")
        hook = hook[0] if isinstance(hook, list) else hook
        print("Created the incoming webhook for Grafana's alerts.")
    webhook_url = f"http://mattermost:8065/hooks/{hook['id']}"
    changed = envfile.read_env().get("MATTERMOST_ALERT_WEBHOOK_URL") != webhook_url
    if changed:
        envfile.write_env("MATTERMOST_ALERT_WEBHOOK_URL", webhook_url)
        print("Wrote the Grafana alert webhook URL to demo/.env; not printed.")

    IDS.parent.mkdir(parents=True, exist_ok=True)
    ids = {"team": TEAM, "channel": CHANNEL, "team_id": team["id"], "channel_id": channel["id"],
           "bot_user_id": bot_id, "oncall_user_id": oncall["id"], "admin_user_id": admin["id"]}
    if not IDS.exists() or json.loads(IDS.read_text()) != ids:
        IDS.write_text(json.dumps(ids, indent=2) + "\n")
        print(f"Wrote {IDS.relative_to(DEMO)}.")
    print(f"Mattermost ready: {URL}/{TEAM}/channels/{CHANNEL}")
    return 3 if changed else 0


def clear():
    """Delete every post in the channel (what the AI service's reset does)."""
    ids = json.loads(IDS.read_text())
    token_ = envfile.read_env()["MATTERMOST_ADMIN_TOKEN"]
    removed = 0
    while True:
        status, page = api("GET", f"/api/v4/channels/{ids['channel_id']}/posts?per_page=200", token_)
        if status != 200 or not page["order"]:
            break
        for post_id in page["order"]:
            if api("DELETE", f"/api/v4/posts/{post_id}", token_)[0] == 200:
                removed += 1
    print(f"Deleted {removed} posts from #{CHANNEL}.")


if __name__ == "__main__":
    command = sys.argv[1] if len(sys.argv) > 1 else ""
    if command == "database":
        database()
    elif command == "bootstrap":
        sys.exit(bootstrap())
    elif command == "clear":
        clear()
    else:
        raise SystemExit("usage: mattermost.py database|bootstrap|clear")
