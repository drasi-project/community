#!/usr/bin/env python3
"""Foreground workload runner and explicit version-checked scenario controls."""
import importlib.util
import json
import os
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import settings  # noqa: E402
import urllib.request

DEMO = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("runtime", DEMO / "ops/native-runtime.py")
runtime = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runtime)


def api(path, body=None):
    request = urllib.request.Request(settings.url("WORKLOAD") + path,
        data=None if body is None else json.dumps(body).encode(), headers={"Content-Type":"application/json"})
    with urllib.request.urlopen(request,timeout=10) as response:
        return json.load(response)


def main():
    action = sys.argv[1] if len(sys.argv)>1 else ""
    if action in ("run","client"):
        runtime.verify_bundle()
        binary = runtime.WORKLOADS
        args = ["serve"] if action=="run" else sys.argv[1:]
        os.execve(binary,[str(binary),*args],os.environ.copy())
    elif action in ("status","context","decisions"):
        print(json.dumps(api("/health" if action=="status" else "/"+action),indent=2))
    elif action=="approve" and len(sys.argv)==5:
        print(json.dumps(api("/approvals",{"command_id":sys.argv[2],"candidate_id":sys.argv[3],"basis":sys.argv[4]}),indent=2))
    elif action=="configure":
        print(json.dumps(api("/configuration",json.load(sys.stdin)),indent=2))
    elif action=="control" and len(sys.argv)==4:
        state = api("/health")
        print(json.dumps(api("/control",{
            "run_epoch":state["run_epoch"],"revision":state["revision"],
            "endpoint":sys.argv[2],"action":sys.argv[3],
        }),indent=2))
    elif action=="reset":
        state=api("/health")
        print(json.dumps(api("/reset",{k:state[k] for k in ["run_epoch","revision"]}),indent=2))
    else:
        raise RuntimeError("usage: workload.py run|status|context|decisions|approve COMMAND_ID CANDIDATE_ID BASIS|configure < request.json|client support|batch [count]|control EU-A|EU-B|US-C fail|throttle|pause-reports|resume-reports|restore|reset")


if __name__=="__main__":
    main()
