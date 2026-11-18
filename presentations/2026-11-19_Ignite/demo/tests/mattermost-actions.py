#!/usr/bin/env python3
"""Live test of the Mattermost platform (plan/contracts.md sections 13–14).

Needs this copy's Mattermost (bootstrapped), the AI service and PostgreSQL, with demo/.env
loaded. It posts proposal cards as the `drasi` bot (the card JSON of contracts § 15), presses
their buttons as `oncall`, and checks the approval rows and the updated cards. Its approvals
use candidates Drasi has never proposed, so Drasi rejects them and no route changes; they do
become the run's latest approval. Its cards are deleted at the end.

    set -a; . ./.env; set +a; python3 tests/mattermost-actions.py
"""
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import urllib.error
import urllib.request
import uuid

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "ops"))
import settings  # noqa: E402

DEMO = Path(__file__).resolve().parents[1]
IDS = json.loads((DEMO / ".native/mattermost.json").read_text())
MM, AI = settings.url("MATTERMOST"), settings.url("WORKLOAD")
ACTIONS = f"http://host.containers.internal:{settings.port('WORKLOAD')}/mattermost/actions"


def call(base, method, route, body=None, token=None, headers=None):
    request = urllib.request.Request(base + route, method=method,
                                     data=None if body is None else json.dumps(body).encode())
    request.add_header("content-type", "application/json")
    if token:
        request.add_header("authorization", f"Bearer {token}")
    for key, value in (headers or {}).items():
        request.add_header(key, value)
    try:
        with urllib.request.urlopen(request, timeout=10) as response:
            return response.status, json.loads(response.read() or b"null"), response.headers
    except urllib.error.HTTPError as error:
        return error.code, json.loads(error.read() or b"null"), error.headers


def sql(text):
    return subprocess.run(
        ["podman", "exec", "-i", settings.container("postgres"), "psql", "-X", "-q", "-t", "-A",
         "-U", os.environ["POSTGRES_USER"], "-d", os.environ["POSTGRES_DB"]],
        input=text, text=True, check=True, capture_output=True).stdout.strip()


def card(run_epoch, candidate):
    context = {"candidate_id": candidate, "basis": "test-basis", "revision": 1, "run_epoch": run_epoch,
               "change": "Move summaries to US-C and support to EU-B"}
    actions = [{"id": "approve", "name": "Approve", "type": "button", "style": "primary",
                "integration": {"url": ACTIONS, "context": {"action": "approve", **context}}},
               {"id": "decline", "name": "Reject", "type": "button", "style": "danger",
                "integration": {"url": ACTIONS, "context": {"action": "decline", **context}}}]
    body = {"channel_id": IDS["channel_id"], "root_id": "", "message": "",
            "props": {"ignite_card": "proposal", "ignite_id": f"proposal-{candidate}", "ignite_run_epoch": run_epoch,
                      "attachments": [{"fallback": "Proposed change", "color": "#f2a33a",
                                       "title": "Proposed change: Move summaries to US-C and support to EU-B",
                                       "text": "Test card (tests/mattermost-actions.py).",
                                       "fields": [{"title": "Why", "value": "EU-A is down", "short": True}],
                                       "footer": "Drasi · incident-proposal", "actions": actions}]}}
    status, post, _ = call(MM, "POST", "/api/v4/posts", body, os.environ["IGNITE_MATTERMOST_BOT_TOKEN"])
    assert status == 201, (status, post)
    return post["id"]


def approval(approval_id):
    row = sql(f"SELECT to_jsonb(a) FROM ignite_approvals a WHERE id='{approval_id}'")
    return json.loads(row) if row else None


def post(post_id):
    return call(MM, "GET", f"/api/v4/posts/{post_id}", token=os.environ["MATTERMOST_ADMIN_TOKEN"])[1]


def press(post_id, action):
    status, body, _ = call(MM, "POST", f"/api/v4/posts/{post_id}/actions/{action}", {},
                           os.environ["MATTERMOST_ONCALL_TOKEN"])
    assert status == 200, (status, body)


def main():
    epoch = call(AI, "GET", "/health")[1]["run_epoch"]
    created = []
    try:
        # Approve on the card: the approval tool's record, with who, which card and what change.
        first = card(epoch, f"test-{uuid.uuid4()}")
        created.append(first)
        press(first, "approve")
        row = approval(f"mm-{first}")
        assert row and row["requested_by"] == "@oncall" and row["post_id"] == first, row
        assert row["change"] == "Move summaries to US-C and support to EU-B" and row["revision"] == 1, row
        assert row["status"] in ("pending", "rejected"), row
        props = post(first)["props"]
        assert "actions" not in props["attachments"][0], props
        assert props["attachments"][0]["fields"][-1]["value"].startswith("Approved by @oncall"), props
        # A stale candidate: Drasi's approval-stale rejects it (no route changes).
        for _ in range(30):
            if approval(f"mm-{first}")["status"] == "rejected":
                break
            time.sleep(1)
        else:
            raise AssertionError("Drasi did not reject the test approval")
        print("approve on the card: ok (recorded, card updated, Drasi rejected the unknown plan)")

        # Reject on the card: recorded as declined; nothing else happens.
        # (Rejected first, so the run's latest approval ends as one Drasi decided.)
        older, newest = card(epoch, f"test-{uuid.uuid4()}"), card(epoch, f"test-{uuid.uuid4()}")
        created += [older, newest]
        press(older, "decline")
        row = approval(f"mm-{older}")
        assert row["status"] == "declined" and row["reason"] == "Rejected by @oncall", row
        assert post(older)["props"]["attachments"][0]["fields"][-1]["value"].startswith("Rejected by @oncall")
        print("reject: ok (declined, card updated)")

        # The desk's backup presses the newest open proposal card of this run.
        status, body, _ = call(AI, "POST", "/mattermost/approve-latest", {"run_epoch": epoch})
        assert status == 200 and body == {"post_id": newest, "approval": f"mm-{newest}"}, (status, body)
        assert approval(f"mm-{newest}")["requested_by"] == "@oncall"
        print("approve-latest: ok (newest card pressed as oncall)")

        status, body, _ = call(AI, "POST", "/mattermost/approve-latest", {"run_epoch": epoch})
        assert status == 404, (status, body)
        print("approve-latest with no open card: 404")

        # A card from an earlier run records nothing.
        stale = card("earlier-run", f"test-{uuid.uuid4()}")
        created.append(stale)
        press(stale, "approve")
        assert approval(f"mm-{stale}") is None
        assert "actions" in post(stale)["props"]["attachments"][0]
        print("earlier run: ok (nothing recorded)")

        # Sign-in hands the browser Mattermost's session cookies and goes to the channel.
        request = urllib.request.Request(f"http://localhost:{settings.port('WORKLOAD')}/mattermost/sign-in")
        opener = urllib.request.build_opener(type("NoRedirect", (urllib.request.HTTPRedirectHandler,),
                                                  {"redirect_request": lambda *a, **k: None}))
        try:
            opener.open(request, timeout=10)
            raise AssertionError("sign-in did not redirect")
        except urllib.error.HTTPError as error:
            assert error.code == 302, error.code
            assert error.headers["location"].endswith("/ai-platform/channels/incident-ai-support")
            cookies = " ".join(error.headers.get_all("set-cookie") or [])
            assert "MMAUTHTOKEN=" in cookies and "MMCSRF=" in cookies
        print("sign-in: ok")
    finally:
        for post_id in created:
            call(MM, "DELETE", f"/api/v4/posts/{post_id}", token=os.environ["MATTERMOST_ADMIN_TOKEN"])
    print("Mattermost actions: all checks passed")


if __name__ == "__main__":
    main()
