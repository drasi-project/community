#!/usr/bin/env python3
"""Local release gates. Mutating commands require exclusive use of this demo."""
import argparse
import datetime
import hashlib
import importlib.util
import json
from pathlib import Path
import platform
import shutil
import subprocess
import time

DEMO = Path(__file__).resolve().parents[1]
ROOT = DEMO.parents[3]
OUT = DEMO / ".native/evidence/release"


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, DEMO / path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


recovery = load("release_recovery", "tests/recovery-integration.py")
work = recovery.work


def utc():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def save(name, value):
    OUT.mkdir(parents=True, exist_ok=True)
    target = OUT / name
    temporary = target.with_suffix(".tmp")
    temporary.write_text(json.dumps(value, indent=2) + "\n")
    temporary.replace(target)


def command(args, cwd=DEMO, timeout=60):
    return subprocess.check_output(args, cwd=cwd, text=True, timeout=timeout).strip()


def baseline():
    before = work.api("/health")
    started = time.monotonic()
    recovery.reset()
    work.wait(lambda: work.api("/health")["continuous_context"] == "ready")
    state = work.api("/health")
    assert state["run_epoch"] != before["run_epoch"]
    assert state["revision"] == 1 and state["pending"] == 0
    assert all(e["mode"] == "ok" and e["reporting"] for e in state["model"]["endpoints"])
    assert {w["id"]: w["destination"] for w in state["model"]["workloads"]} == {
        "support": "EU-A", "batch": "EU-B"}
    return {"at": utc(), "reset_seconds": time.monotonic() - started, "health": state}


def incident():
    started = time.monotonic()
    epoch = work.api("/health")["run_epoch"]
    work.control("fail")
    candidate = work.wait(recovery.candidate)
    assessment_seconds = time.monotonic() - started
    body, trace = work.submit()
    before = work.api("/tasks/" + body["id"])["task"]
    work.wait(lambda: any(a["status"] == 503 for a in work.api("/tasks/" + body["id"])["attempts"]))
    work.wait(lambda: body["id"] in work.api("/context")["queries"]["overdue-tasks"], 25)
    after = work.api("/tasks/" + body["id"])["task"]
    assert before == after, "Deadline must change result without changing task input"
    overdue = work.api("/context")["queries"]["overdue-tasks"][body["id"]]
    approved = recovery.approve(candidate)
    ident = approved["command"]["id"]
    work.wait(lambda: recovery.command(ident)["status"] == "recovery_confirmed", 30)
    result = work.api("/tasks/" + body["id"])
    applied = recovery.command(ident)
    assert result["task"]["endpoint"] == "EU-B"
    assert result["task"]["completed_revision"] == applied["applied_revision"]
    assert applied["run_epoch"] == epoch
    assert applied["recovered_ms"] >= applied["applied_ms"] + 5000
    decisions = work.api("/decisions")
    assert decisions["recovery"][ident]["state"] == "confirmed"
    assert {w["id"]: w["destination"] for w in work.api("/health")["model"]["workloads"]} == {
        "support": "EU-B", "batch": "US-C"}
    return {"at": utc(), "epoch": epoch, "input_to_assessment_seconds": assessment_seconds,
            "applied_to_recovery_seconds": (applied["recovered_ms"] - applied["applied_ms"]) / 1000,
            "elapsed_seconds": time.monotonic() - started, "candidate": candidate,
            "trace": trace, "unchanged_task": before, "overdue": overdue, "task": result,
            "command": applied, "recovery": decisions["recovery"][ident]}


def cycles():
    preserve("cycles.json")
    evidence = {"started_at": utc(), "passed": False, "cycles": [],
                "native_build_sha256": sha(DEMO / ".native/build.json"),
                "qualification_helper_sha256": sha(Path(__file__))}
    try:
        evidence["initial"] = baseline()
        for index in range(5):
            item = incident()
            item["reset"] = baseline()
            evidence["cycles"].append(item)
            save("cycles.json", evidence)
            print(f"cycle {index + 1}/5: real deadline, approval, recovery, reset passed", flush=True)
        assert len({c["epoch"] for c in evidence["cycles"]}) == 5
        evidence["passed"] = True
    except Exception as error:
        evidence["error"] = str(error)
        raise
    finally:
        evidence["ended_at"] = utc()
        save("cycles.json", evidence)


def containers():
    names = command(["podman", "ps", "--filter",
                     "label=com.docker.compose.project=ignite-observability",
                     "--format", "{{.Names}}"]).splitlines()
    assert len(names) == 10 and all(n.startswith("ignite-observability-") for n in names)
    return names


def listeners():
    found = {}
    for port in [18421, 18500, 18580]:
        pid = command(["lsof", "-t", f"-iTCP:{port}", "-sTCP:LISTEN"])
        assert pid.isdecimal(), f"Expected exactly one listener on {port}"
        found[str(port)] = {"pid": int(pid), "process": command(["ps", "-p", pid, "-o", "command="])}
    assert "drasi-server" in found["18421"]["process"]
    assert "ignite-workloads" in found["18500"]["process"]
    assert "vite" in found["18580"]["process"]
    return found


def sample(names, processes):
    disk = shutil.disk_usage(DEMO)
    return {"at": utc(), "health": work.api("/health"),
            "reports": work.api("/context")["queries"]["endpoint-freshness"],
            "publications_pending": len(work.api("/decisions")["publications"]),
            "host_processes": command(["ps", "-p", ",".join(str(p["pid"]) for p in processes.values()),
                                       "-o", "pid=,pcpu=,rss=,etime=,command="]),
            "containers": json.loads(command(["podman", "stats", "--no-stream", "--format", "json", *names])),
            "database": json.loads(work.native.runtime.sql(
                "SELECT json_build_object('bytes',pg_database_size(current_database()),"
                "'tasks',(SELECT count(*) FROM ignite_tasks),"
                "'commands',(SELECT count(*) FROM ignite_commands),"
                "'reports',(SELECT json_agg(r) FROM ignite_reports r));", tuples=True)),
            "disk": {"total": disk.total, "used": disk.used, "free": disk.free}}


def soak():
    preserve("soak.json")
    evidence = {"passed": False, "samples": [], "tasks": [], "errors": [],
                "native_build_sha256": sha(DEMO / ".native/build.json"),
                "qualification_helper_sha256": sha(Path(__file__))}
    begin = time.monotonic()
    try:
        evidence["initial"] = baseline()
        names, processes = containers(), listeners()
        evidence["processes"] = processes
        begin = time.monotonic()
        evidence["started_at"] = utc()
        deadline = begin + 3600
        next_traffic = next_sample = begin
        while time.monotonic() < deadline:
            elapsed = time.monotonic() - begin
            if elapsed >= 120 and "incident" not in evidence:
                evidence["incident"] = incident()
                work.control("restore")
            if elapsed >= 180 and "browser_reconnect" not in evidence:
                command(["node", "scripts/release-browser.mjs", "reconnect"], cwd=DEMO / "ui", timeout=90)
                evidence["browser_reconnect"] = json.loads((OUT / "browser-reconnect.json").read_text())
            if time.monotonic() >= next_traffic:
                for workload in ["support", "batch"]:
                    body, trace = work.submit(workload)
                    work.wait(lambda: work.api("/tasks/" + body["id"])["task"]["completed_ms"], 20)
                    task = work.api("/tasks/" + body["id"])["task"]
                    assert task["run_epoch"] == evidence["initial"]["health"]["run_epoch"]
                    evidence["tasks"].append({"id": task["id"], "workload": workload,
                                              "trace_id": trace["trace_id"], "endpoint": task["endpoint"],
                                              "completed_ms": task["completed_ms"]})
                next_traffic = max(next_traffic + 10, time.monotonic() + 1)
            if time.monotonic() >= next_sample:
                point = sample(names, processes)
                evidence["samples"].append(point)
                assert point["health"]["continuous_context"] == "ready", "Unexpected unknown context"
                assert point["health"]["pending"] == 0, "Queue did not drain"
                assert point["disk"]["free"] > 1024 ** 3, "Less than 1 GiB disk headroom"
                save("soak.json", evidence)
                next_sample = time.monotonic() + 60
            time.sleep(min(1, max(0, deadline - time.monotonic())))
        evidence["ended_at"] = utc()
        evidence["duration_seconds"] = time.monotonic() - begin
        assert evidence["duration_seconds"] >= 3600
        assert len(evidence["tasks"]) >= 600 and len(evidence["samples"]) >= 55
        assert evidence["browser_reconnect"]["passed"]
        work.wait(lambda: not work.api("/decisions")["publications"], 60)
        evidence["final"] = baseline()
        assert sha(DEMO / ".native/build.json") == evidence["native_build_sha256"], "Build changed during soak"
        evidence["passed"] = True
    except Exception as error:
        evidence["errors"].append({"at": utc(), "message": str(error)})
        evidence["ended_at"] = utc()
        evidence["duration_seconds"] = time.monotonic() - begin
        raise
    finally:
        save("soak.json", evidence)
        print(json.dumps({k: evidence.get(k) for k in
                          ["passed", "started_at", "ended_at", "duration_seconds", "errors"]}), flush=True)


def preflight():
    recovery.work.native.runtime.verify_bundle()
    assert (DEMO / ".env").stat().st_mode & 0o077 == 0, ".env must be private"
    for file in ["ui/node_modules/.package-lock.json", "ui/dist/index.html",
                 ".native/build.json", "Cargo.lock", "ui/package-lock.json"]:
        assert (DEMO / file).is_file(), f"Missing prebuilt/locked prerequisite: {file}"
    state = work.api("/health")
    assert state["continuous_context"] == "ready" and state["streams_attached"]
    result = {"at": utc(), "health": state, "processes": listeners(), "containers": containers()}
    save("preflight.json", result)
    print(json.dumps(result, indent=2))


def sha(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def preserve(name):
    previous = OUT / name
    if previous.exists():
        previous.rename(OUT / f"{previous.stem}-{time.time_ns()}.json")


def source_fingerprints(repo):
    files = subprocess.check_output(["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
                                    cwd=repo).decode().split("\0")
    return {p: sha(repo / p) for p in sorted(set(files)) if p and (repo / p).is_file()}


def verify():
    release = DEMO / ".native/release"
    expected = (release / "manifest.sha256").read_text().split()[0]
    assert sha(release / "manifest.json") == expected, "Manifest checksum mismatch"
    manifest = json.loads((release / "manifest.json").read_text())
    for path, digest in manifest["artifacts"].items():
        assert sha(ROOT / path) == digest, f"Release artifact changed: {path}"
    for path, digest in manifest["external_libraries"].items():
        assert sha(Path(path)) == digest, f"Laptop runtime library changed: {path}"
    for repo, source in manifest["source"].items():
        assert command(["git", "rev-parse", "HEAD"], ROOT / repo) == source["head"]
        assert command(["git", "branch", "--show-current"], ROOT / repo) == source["branch"]
        assert command(["git", "status", "--short"], ROOT / repo) == source["status"], f"Source file set changed: {repo}"
        assert source_fingerprints(ROOT / repo) == source["source_files"], f"Release source changed: {repo}"
    for image in manifest["images"]:
        actual = command(["podman", "image", "inspect", "--format", "{{.Id}}", image["id"]])
        assert actual.removeprefix("sha256:") == image["id"].removeprefix("sha256:")
    print("Release source, locked dependencies, binaries, assets and cached images match manifest.")


def offline_images():
    images = []
    for name in containers():
        ident = command(["podman", "inspect", "--format", "{{.Image}}", name])
        image = json.loads(command(["podman", "image", "inspect", ident]))[0]
        images.append({"container": name, "id": ident, "digests": image.get("RepoDigests", [])})
    result = command(["podman", "run", "--rm", "--pull=never", "--network=none",
                      "--name", "ignite-release-offline-probe", "--entrypoint", "python",
                      "localhost/ignite-observability-probe:phase1",
                      "-c", "import opentelemetry.sdk,requests; print('cached probe dependencies loaded offline')"])
    save("offline-images.json", {"passed": True, "at": utc(), "images": images,
         "isolated_probe": result,
         "boundary": "all ten running image IDs present locally; new probe container starts with no pull/no network; no full offline stack restart"})
    print(result)


def stamp():
    release = DEMO / ".native/release"
    release.mkdir(exist_ok=True)
    gates = {name: json.loads((OUT / f"{name}.json").read_text())
             for name in ["cycles", "soak", "offline-browser", "offline-images", "recordings"]}
    assert all(gates[name]["passed"] for name in ["cycles", "soak", "offline-browser", "offline-images"])
    assert len(gates["cycles"]["cycles"]) == 5
    assert gates["soak"]["duration_seconds"] >= 3600 and not gates["soak"]["errors"]
    assert len(gates["recordings"]["clips"]) == 4
    for clip in gates["recordings"]["clips"]:
        recorded = DEMO / "ui/public" / clip["src"].lstrip("/")
        built = DEMO / "ui/dist" / clip["src"].lstrip("/")
        assert sha(recorded) == clip["sha256"] == sha(built), f"Replay changed: {clip['scene']}"
    frontend = json.loads((OUT / "frontend-tests.json").read_text())
    assert frontend["stats"]["unexpected"] == 0 and frontend["stats"]["expected"] >= 18
    build_hash = sha(DEMO / ".native/build.json")
    assert all(gates[name]["native_build_sha256"] == build_hash for name in ["cycles", "soak", "recordings"])
    source = {}
    for name in ["drasi-core", "drasi-server", "community"]:
        repo = ROOT / name
        # Fingerprint source in place; do not copy private ignored data or caches.
        fingerprints = source_fingerprints(repo)
        source[name] = {
            "head": command(["git", "rev-parse", "HEAD"], repo),
            "branch": command(["git", "branch", "--show-current"], repo),
            "tracked_diff_sha256": hashlib.sha256(subprocess.check_output(
                ["git", "diff", "--binary", "HEAD"], cwd=repo)).hexdigest(),
            "source_files": fingerprints,
            "status": command(["git", "status", "--short"], repo)}
    images = []
    for name in containers():
        ident = command(["podman", "inspect", "--format", "{{.Image}}", name])
        image = json.loads(command(["podman", "image", "inspect", ident]))[0]
        images.append({"container": name, "id": ident, "digests": image.get("RepoDigests", []),
                       "tags": image.get("RepoTags", []), "architecture": image["Architecture"]})
    native = json.loads((DEMO / ".native/build.json").read_text())
    for path, digest in native["demo_sources"].items():
        assert sha(DEMO / path) == digest, f"Rebuild native source before stamping: {path}"
    recovery.work.native.runtime.verify_bundle()
    paths = [ROOT / p for p in native["files"]]
    paths += [p for p in (DEMO / "ui/dist").rglob("*") if p.is_file()]
    paths += [ROOT / "drasi-core/Cargo.lock", ROOT / "drasi-server/Cargo.lock",
              DEMO / "Cargo.lock", DEMO / "ui/package-lock.json"]
    linkage, external = {}, {}
    if platform.system() == "Darwin":
        for binary in paths:
            if binary.name in ["drasi-server", "ignite-workloads"] or binary.suffix == ".dylib":
                output = command(["otool", "-L", str(binary)])
                linkage[str(binary.relative_to(ROOT))] = output.splitlines()[1:]
                for line in output.splitlines()[1:]:
                    library = line.strip().split(" (")[0]
                    if library.startswith("/opt/homebrew/"):
                        external[library] = sha(Path(library))
    licenses = [ROOT / name / "LICENSE" for name in ["community", "drasi-core", "drasi-server"]]
    lock = json.loads((DEMO / "ui/package-lock.json").read_text())
    for package in lock["packages"]:
        directory = DEMO / "ui" / package
        if package.startswith("node_modules/") and directory.is_dir():
            licenses.extend(p for p in directory.iterdir() if p.is_file()
                            and p.name.upper().startswith(("LICENSE", "LICENCE", "NOTICE", "COPYING")))
    manifest = {"schema": 1, "stamped_at": utc(), "scope": "in-place Apple ARM64 laptop release; not portable",
                "release_id": f"ignite-{datetime.datetime.now(datetime.timezone.utc):%Y%m%dT%H%M%SZ}-{build_hash[:12]}",
                "root": str(ROOT), "source": source, "native_build": native, "images": images,
                "host": {"platform": platform.platform(), "architecture": platform.machine(),
                         "python": platform.python_version(), "node": command(["node", "--version"]),
                         "npm": command(["npm", "--version"]), "podman": command(["podman", "--version"]),
                         "rustc": command(["rustc", "--version"])},
                "artifacts": {str(p.relative_to(ROOT)): sha(p) for p in paths},
                "external_libraries": external, "dynamic_linkage": linkage,
                "preserved_license_files": {str(p.relative_to(ROOT)): sha(p) for p in licenses if p.is_file()},
                "qualification": {p.name: {"sha256": sha(p), "path": str(p)}
                                  for p in OUT.glob("*.json")},
                "supporting_regressions": {p.name: {"sha256": sha(p), "path": str(p)}
                                          for p in (DEMO / ".native/evidence").glob("*.json")},
                "excluded": ["private .env", "database/product volumes", "node_modules", "Cargo caches",
                             "container image layers (remain in Podman)", "browser installation"],
                "human_gates": ["three timed 22-minute rehearsals including offline and deliberate fallback",
                               "actual laptop/display/clicker and projected privacy checks",
                               "physical venue/network-disconnected rehearsal"]}
    (release / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    (release / "manifest.sha256").write_text(sha(release / "manifest.json") + "  manifest.json\n")
    print(str(release / "manifest.json"))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["preflight", "cycles", "soak", "stamp", "verify", "offline-images"])
    args = parser.parse_args()
    {"preflight": preflight, "cycles": cycles, "soak": soak, "stamp": stamp,
     "verify": verify, "offline-images": offline_images}[args.action]()
