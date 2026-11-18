#!/usr/bin/env python3
"""Create a shell-compatible, private local environment without printing secrets.

`observability-env.py` creates demo/.env once. `observability-env.py grafana-token`
(run by `ops/observability-stack.sh up`) makes sure IGNITE_GRAFANA_TOKEN in .env is a
working token of Grafana's `drasi-annotations` service account (role Editor), which
Drasi's `grafana-annotations` HTTP reaction uses to post decision annotations.
Grafana stores only a hash of each token, so the token is created through Grafana's
API once Grafana runs, and again if Grafana's data volume was recreated.
"""
import base64
import json
import os
from pathlib import Path
import secrets
import sys
import time
import urllib.error
import urllib.request

sys.path.insert(0, str(Path(__file__).resolve().parent))
import settings  # noqa: E402

path = Path(__file__).resolve().parents[1] / ".env"
ACCOUNT = "drasi-annotations"


def create_env():
    if path.exists():
        if path.stat().st_mode & 0o077:
            raise SystemExit(f"{path} must have permissions 600")
        print("Existing private demo/.env retained.")
        return
    secret = lambda: secrets.token_hex(24)
    password, public_key, secret_key = secret(), "pk-lf-" + secret(), "sk-lf-" + secret()
    values = {
        "POSTGRES_USER": "ignite",
        "POSTGRES_DB": "ignite",
        "POSTGRES_PASSWORD": password,
        "DATABASE_URL": f"postgresql://ignite:{password}@127.0.0.1:{settings.port('POSTGRES')}/ignite",
        "LANGFUSE_DATABASE_URL": f"postgresql://ignite:{password}@postgres:5432/langfuse",
        "LANGFUSE_PUBLIC_KEY": public_key,
        "LANGFUSE_SECRET_KEY": secret_key,
        "LANGFUSE_AUTH_BASE64": base64.b64encode(f"{public_key}:{secret_key}".encode()).decode(),
        "LANGFUSE_SALT": secret(),
        "LANGFUSE_ENCRYPTION_KEY": secrets.token_hex(32),
        "LANGFUSE_NEXTAUTH_SECRET": secret(),
        "LANGFUSE_ADMIN_EMAIL": "operator@ignite.local",
        "LANGFUSE_ADMIN_PASSWORD": secret(),
        "CLICKHOUSE_PASSWORD": secret(),
        "REDIS_PASSWORD": secret(),
        "MINIO_PASSWORD": secret(),
        "GRAFANA_ADMIN_PASSWORD": secret(),
        # Filled by `observability-env.py grafana-token` once Grafana runs.
        "IGNITE_GRAFANA_TOKEN": "",
        # Mattermost (ops/mattermost.py bootstrap adds its tokens and webhook URL once it runs).
        "MATTERMOST_ADMIN_PASSWORD": secret(),
        "MATTERMOST_ONCALL_PASSWORD": secret(),
    }
    with os.fdopen(os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), "w") as stream:
        stream.writelines(f"{key}='{value}'\n" for key, value in values.items())
    print("Created private demo/.env; no credentials printed.")


def read_env():
    values = {}
    for line in path.read_text().splitlines():
        key, separator, value = line.partition("=")
        if separator:
            values[key.strip()] = value.strip().strip("'")
    return values


def write_env(key, value):
    lines = [line for line in path.read_text().splitlines() if not line.startswith(f"{key}=")]
    lines.append(f"{key}='{value}'")
    temporary = path.with_name(".env.tmp")
    with os.fdopen(os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600), "w") as stream:
        stream.write("\n".join(lines) + "\n")
    os.replace(temporary, path)


def grafana(method, route, body=None, password=None, token=None):
    request = urllib.request.Request(settings.url("GRAFANA") + route, method=method,
                                     data=None if body is None else json.dumps(body).encode())
    request.add_header("content-type", "application/json")
    if token:
        request.add_header("authorization", f"Bearer {token}")
    else:
        basic = base64.b64encode(f"admin:{password}".encode()).decode()
        request.add_header("authorization", f"Basic {basic}")
    try:
        with urllib.request.urlopen(request, timeout=5) as response:
            return response.status, json.loads(response.read() or b"null")
    except urllib.error.HTTPError as error:
        return error.code, None


def grafana_token():
    if not path.exists():
        raise SystemExit("Run ops/observability-stack.sh env first.")
    env = read_env()
    for attempt in range(60):
        try:
            status, _ = grafana("GET", "/api/health", password="")
            if status == 200:
                break
        except OSError:
            pass
        time.sleep(1)
    else:
        raise SystemExit(f"Grafana is not answering on {settings.url('GRAFANA')}")
    token = env.get("IGNITE_GRAFANA_TOKEN")
    # Grafana falls back to anonymous Viewer for an unknown token, so check for the permission the reaction needs.
    if token:
        status, permissions = grafana("GET", "/api/access-control/user/permissions", token=token)
        if status == 200 and "annotations:write" in (permissions or {}):
            print("Grafana service-account token in demo/.env is valid.")
            return
    password = env["GRAFANA_ADMIN_PASSWORD"]
    status, found = grafana("GET", f"/api/serviceaccounts/search?query={ACCOUNT}", password=password)
    if status != 200:
        raise SystemExit(f"Grafana service-account search failed ({status})")
    account = next((a for a in found["serviceAccounts"] if a["name"] == ACCOUNT), None)
    if account is None:
        status, account = grafana("POST", "/api/serviceaccounts", {"name": ACCOUNT, "role": "Editor"}, password=password)
        if status not in (200, 201):
            raise SystemExit(f"Creating Grafana service account failed ({status})")
    status, created = grafana("POST", f"/api/serviceaccounts/{account['id']}/tokens",
                              {"name": f"drasi-{int(time.time())}"}, password=password)
    if status != 200:
        raise SystemExit(f"Creating Grafana service-account token failed ({status})")
    write_env("IGNITE_GRAFANA_TOKEN", created["key"])
    print("Wrote a new Grafana service-account token to demo/.env; not printed.")


if __name__ == "__main__":
    grafana_token() if sys.argv[1:] == ["grafana-token"] else create_env()
