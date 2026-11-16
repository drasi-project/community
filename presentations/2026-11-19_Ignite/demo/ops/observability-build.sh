#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ "$(uname -m)" != arm64 && "$(uname -m)" != aarch64 ]]; then
  echo "This qualification targets ARM64; MinIO and Python wheel selection are ARM64-pinned." >&2
  exit 1
fi
python3 -m pip download --only-binary=:all: --platform manylinux2014_aarch64 \
  --implementation cp --python-version 3.12 --abi cp312 \
  -r "$ROOT/observability/probe/requirements.txt" \
  --dest "$ROOT/observability/probe/wheels" --quiet
podman build -q -t localhost/ignite-observability-probe:phase1 \
  -f "$ROOT/observability/probe/Containerfile" "$ROOT/observability/probe"
