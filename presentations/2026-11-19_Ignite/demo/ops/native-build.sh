#!/usr/bin/env bash
set -euo pipefail
DEMO="$(cd "$(dirname "$0")/.." && pwd)"
ROOT="$(cd "$DEMO/../../../.." && pwd)"
SERVER="$ROOT/drasi-server"
CORE="$ROOT/drasi-core"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_TARGET_DIR="$DEMO/.native/target"
export JQ_LIB_DIR="${JQ_LIB_DIR:-/opt/homebrew/lib}"
mkdir -p "$DEMO/.native/plugins"
case "$(uname -s)" in
    Darwin) extension=dylib ;;
    Linux) extension=so ;;
    *) echo "Unsupported native platform" >&2; exit 1 ;;
esac
cd "$DEMO"
npm --prefix "$SERVER/ui" run build
mkdir -p "$DEMO/.native/ui/dist"
rsync -a --delete "$SERVER/ui/dist/" "$DEMO/.native/ui/dist/"
CARGO_TARGET_DIR="$SERVER/target" cargo build --locked --manifest-path "$SERVER/Cargo.toml" --bin drasi-server
if [ ! -f "$CORE/Cargo.lock" ]; then
    cargo generate-lockfile --manifest-path "$CORE/Cargo.toml"
fi
for package in drasi-computation-source-postgres drasi-computation-bootstrap-postgres drasi-computation-transformer-query drasi-computation-source-http; do
    cargo build --locked --manifest-path "$CORE/Cargo.toml" -p "$package" --lib --features "$package/dynamic-plugin"
    cp "$CARGO_TARGET_DIR/debug/lib${package//-/_}.$extension" "$DEMO/.native/plugins/"
done
cargo build --locked -p ignite-capacity --lib --features dynamic-plugin
cp "$CARGO_TARGET_DIR/debug/libdrasi_computation_ignite_capacity.$extension" "$DEMO/.native/plugins/"
cargo build --locked -p ignite-decisions --lib --features dynamic-plugin
cp "$CARGO_TARGET_DIR/debug/libdrasi_computation_ignite_decisions.$extension" "$DEMO/.native/plugins/"
if [ "$extension" = dylib ]; then
    python3 "$DEMO/ops/macho-align.py" "$DEMO"/.native/plugins/*.dylib
fi
cargo build --locked -p ignite-config -p ignite-workloads
"$CARGO_TARGET_DIR/debug/ignite-config" "$DEMO/.native/plugins" > "$DEMO/.native/server.json.tmp"
mv "$DEMO/.native/server.json.tmp" "$DEMO/.native/server.json"
"$SERVER/target/debug/drasi-server" validate --config "$DEMO/.native/server.json" --plugins-dir "$DEMO/.native/plugins"
python3 "$DEMO/ops/native-runtime.py" stamp
