#!/usr/bin/env python3
"""Foreground stock Server runner and scoped database controls; stdlib only."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

DEMO = Path(__file__).resolve().parents[1]
ROOT = DEMO.parents[3]
SERVER = ROOT / "drasi-server"
CORE = ROOT / "drasi-core"
BINARY = SERVER / "target/debug/drasi-server"


def required(name):
    value = os.environ.get(name)
    if not value:
        raise RuntimeError(f"{name} must be exported (load demo/.env first)")
    return value


def sql(text, tuples=False):
    # psql runs in this demo's known container; no host DB client is required.
    command = [
        "podman", "exec", "-i", "ignite-observability-postgres-1",
        "psql", "-X", "-v", "ON_ERROR_STOP=1",
        "-U", required("POSTGRES_USER"), "-d", required("POSTGRES_DB"),
    ]
    if tuples:
        command.extend(["-q", "-t", "-A"])
    return subprocess.run(command, input=text, text=True, check=True, capture_output=True).stdout


def digest(path):
    with path.open("rb") as file:
        return hashlib.file_digest(file, "sha256").hexdigest()


def stamp():
    ui = DEMO / ".native/ui/dist"
    if not (ui / "index.html").is_file():
        raise RuntimeError("Server UI assets missing; run ops/native-build.sh")
    files = [BINARY, DEMO / ".native/target/debug/ignite-workloads", DEMO / ".native/server.json",
             *sorted((DEMO / ".native/plugins").glob("*")),
             *sorted(path for path in ui.rglob("*") if path.is_file())]
    value = {
        "revisions": {
            repo.name: {
                "head": subprocess.check_output(["git", "-C", str(repo), "rev-parse", "HEAD"], text=True).strip(),
                "tracked_diff_sha256": hashlib.sha256(
                    subprocess.check_output(["git", "-C", str(repo), "diff", "--binary", "HEAD"])
                ).hexdigest(),
            }
            for repo in [CORE, SERVER, ROOT / "community"]
        },
        "files": {str(path.relative_to(ROOT)): digest(path) for path in files},
        "demo_sources": {
            str(path.relative_to(DEMO)): digest(path)
            for directory in ["domain", "plugins", "ops/config", "drasi", "workloads", "database", "policies"]
            for path in sorted((DEMO / directory).rglob("*"))
            if path.is_file() and path.suffix in {".rs", ".toml", ".cypher", ".sql", ".rego"}
        },
        "lockfiles": {
            str(path.relative_to(ROOT)): digest(path)
            for path in [CORE / "Cargo.lock", SERVER / "Cargo.lock", DEMO / "Cargo.lock",
                         SERVER / "ui/package-lock.json"]
        },
        "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
    }
    (DEMO / ".native/build.json").write_text(json.dumps(value, indent=2) + "\n")


def verify_bundle():
    value = json.loads((DEMO / ".native/build.json").read_text())
    for path, expected in value["files"].items():
        if digest(ROOT / path) != expected:
            raise RuntimeError(f"Native bundle changed; run ops/native-build.sh: {path}")


def prune():
    # Every query snapshots these tables on start but only reads the current
    # run, so earlier runs' rows only slow startup. Delete them while Server is
    # stopped, in batches below the source's per-transaction change limit.
    for table in ["ignite_attempts", "ignite_breaches", "ignite_receipts", "ignite_tasks"]:
        removed = 0
        while True:
            output = sql(f"""DELETE FROM {table} WHERE ctid IN (
                SELECT ctid FROM {table}
                WHERE run_epoch <> (SELECT run_epoch FROM ignite_scenario WHERE singleton)
                LIMIT 2000);""")
            count = int(output.split()[-1])
            removed += count
            if count == 0:
                break
        if removed:
            print(f"pruned {removed} earlier-run rows from {table}", file=sys.stderr)


def main():
    action = sys.argv[1] if len(sys.argv) > 1 else ""
    if action == "stamp":
        stamp()
    elif action == "init":
        print(sql((DEMO / "database/schema.sql").read_text()), end="")
        print(sql((DEMO / "database/workload.sql").read_text()), end="")
        # Initial fixture is explicit once; subsequent starts never seed or reset.
        if sql("SELECT run_epoch FROM ignite_scenario;").count("(0 rows)"):
            print(sql((DEMO / "database/reset.sql").read_text()), end="")
    elif action == "reset":
        print(sql((DEMO / "database/reset.sql").read_text()), end="")
    elif action == "migrate":
        print(sql((DEMO / "database/workload.sql").read_text()), end="")
    elif action == "sql":
        print(sql(sys.stdin.read()), end="")
    elif action == "run":
        verify_bundle()
        prune()
        env = os.environ.copy()
        env["IGNITE_POSTGRES_PASSWORD"] = required("POSTGRES_PASSWORD")
        os.chdir(DEMO / ".native")
        os.execve(BINARY, [
            str(BINARY), "--config", str(DEMO / ".native/server.json"),
            "--plugins-dir", str(DEMO / ".native/plugins"),
        ], env)
    else:
        raise RuntimeError("usage: native-runtime.py init|migrate|reset|run|sql|stamp")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, RuntimeError, subprocess.CalledProcessError) as error:
        print(f"Native integration failed: {error}", file=sys.stderr)
        if isinstance(error, subprocess.CalledProcessError) and error.stderr:
            print(error.stderr, file=sys.stderr)
        sys.exit(1)
