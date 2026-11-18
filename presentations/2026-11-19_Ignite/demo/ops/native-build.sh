#!/usr/bin/env bash
set -euo pipefail
DEMO="$(cd "$(dirname "$0")/.." && pwd)"
ROOT="$(cd "$DEMO/../../../.." && pwd)"
# The next-version checkouts beside this one (ops/settings.py uses the same names).
SERVER="$ROOT/${IGNITE_DRASI_SERVER:-drasi-server-next}"
CORE="$ROOT/${IGNITE_DRASI_CORE:-drasi-core-next}"
# At most 4 parallel jobs; one shared build folder for the copy (see demo/.cargo/config.toml).
export CARGO_BUILD_JOBS="$(( ${CARGO_BUILD_JOBS:-4} > 4 ? 4 : ${CARGO_BUILD_JOBS:-4} ))"
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/.cargo-target-next}"
export JQ_LIB_DIR="${JQ_LIB_DIR:-/opt/homebrew/lib}"
mkdir -p "$DEMO/.native/plugins"
# Install as a new file: macOS kills a process that loads a library overwritten
# in place, because the kernel keeps the old file's code signature.
install_plugin() {
    cp "$1" "$DEMO/.native/plugins/.$(basename "$1").tmp"
    mv -f "$DEMO/.native/plugins/.$(basename "$1").tmp" "$DEMO/.native/plugins/$(basename "$1")"
}
case "$(uname -s)" in
    Darwin) extension=dylib ;;
    Linux) extension=so ;;
    *) echo "Unsupported native platform" >&2; exit 1 ;;
esac
cd "$DEMO"
npm --prefix "$SERVER/ui" run build
mkdir -p "$DEMO/.native/ui/dist"
rsync -a --delete "$SERVER/ui/dist/" "$DEMO/.native/ui/dist/"
cargo build --locked --manifest-path "$SERVER/Cargo.toml" --bin drasi-server
if [ ! -f "$CORE/Cargo.lock" ]; then
    cargo generate-lockfile --manifest-path "$CORE/Cargo.toml"
fi
for package in drasi-computation-source-postgres drasi-computation-bootstrap-postgres drasi-computation-transformer-query drasi-computation-sink-http drasi-computation-sink-sse; do
    cargo build --locked --manifest-path "$CORE/Cargo.toml" -p "$package" --lib --features "$package/dynamic-plugin"
    install_plugin "$CARGO_TARGET_DIR/debug/lib${package//-/_}.$extension"
done
# Drasi's native OpenTelemetry source (drasi-core PR 1030). The generator wires it only when IGNITE_HEALTH_SOURCE=otel (plan/otel-detection-plan.md).
if [ -f "$CORE/components/computation-plugins/sources/otel/Cargo.toml" ]; then
    package=drasi-computation-source-otel
    cargo build --locked --manifest-path "$CORE/Cargo.toml" -p "$package" --lib --features "$package/dynamic-plugin"
    install_plugin "$CARGO_TARGET_DIR/debug/lib${package//-/_}.$extension"
fi
# This demo's native plugins: Rego policy, good_lp optimizer (plan and resilience), Prometheus reaction.
for package in ignite-policy ignite-optimizer ignite-prometheus; do
    cargo build --locked -p "$package" --lib --features "$package/dynamic-plugin"
    install_plugin "$CARGO_TARGET_DIR/debug/libdrasi_computation_${package//-/_}.$extension"
done
# Plugins earlier versions of the graph used.
rm -f "$DEMO"/.native/plugins/libdrasi_computation_{ignite_capacity,ignite_decisions,source_http}.$extension
if [ "$extension" = dylib ]; then
    python3 "$DEMO/ops/macho-align.py" "$DEMO"/.native/plugins/*.dylib
fi
cargo build --locked -p ignite-config -p ignite-workloads
"$CARGO_TARGET_DIR/debug/ignite-config" "$DEMO/.native/plugins" > "$DEMO/.native/server.json.tmp"
mv "$DEMO/.native/server.json.tmp" "$DEMO/.native/server.json"
"$CARGO_TARGET_DIR/debug/drasi-server" validate --config "$DEMO/.native/server.json" --plugins-dir "$DEMO/.native/plugins"
python3 "$DEMO/ops/native-runtime.py" stamp
