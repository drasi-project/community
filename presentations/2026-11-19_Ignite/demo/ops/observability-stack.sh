#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export DOCKER_CONFIG="$ROOT/observability/compose-client"
if [[ "${1:-}" == env ]]; then
  exec python3 "$ROOT/ops/observability-env.py"
fi
if [[ ! -f "$ROOT/.env" ]]; then
  echo "Run $ROOT/ops/observability-stack.sh env first." >&2
  exit 1
fi
# Ports, project name and keys (ports.env, or IGNITE_SETTINGS); compose reads them from the environment.
eval "$(python3 "$ROOT/ops/settings.py")"
# Prometheus scrapes Drasi's Prometheus reaction on the host.
mkdir -p "$ROOT/.native/prometheus-targets"
printf '[{"targets":["host.containers.internal:%s"],"labels":{"component":"drasi"}}]\n' \
  "$IGNITE_DRASI_METRICS_PORT" > "$ROOT/.native/prometheus-targets/drasi.json"
compose() { podman compose --project-name "$IGNITE_PROJECT" --env-file "$ROOT/.env" -f "$ROOT/compose.yaml" "$@"; }
case "${1:-}" in
  up)
    compose "$@"
    # Drasi's annotation reaction needs a Grafana service-account token (IGNITE_GRAFANA_TOKEN in .env).
    services=(); for argument in "${@:2}"; do [[ "$argument" == -* ]] || services+=("$argument"); done
    if [[ ${#services[@]} -eq 0 || " ${services[*]} " == *" grafana "* ]]; then
      python3 "$ROOT/ops/observability-env.py" grafana-token
    fi
    ;;
  down|stop|start|restart|ps|logs|pull|exec)
    exec podman compose --project-name "$IGNITE_PROJECT" --env-file "$ROOT/.env" -f "$ROOT/compose.yaml" "$@"
    ;;
  *) echo "Usage: $0 env|up|down|stop|start|restart|ps|logs|pull|exec [compose arguments]" >&2; exit 2 ;;
esac
