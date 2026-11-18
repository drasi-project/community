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
# Health from telemetry: the Collector also sends the health probes to Drasi's OTel source.
files=(-f "$ROOT/compose.yaml")
case "${IGNITE_HEALTH_SOURCE:-postgres}" in
  postgres) ;;
  otel) files+=(-f "$ROOT/compose.health-otel.yaml") ;;
  *) echo "IGNITE_HEALTH_SOURCE must be postgres or otel" >&2; exit 2 ;;
esac
compose() { podman compose --project-name "$IGNITE_PROJECT" --env-file "$ROOT/.env" "${files[@]}" "$@"; }
case "${1:-}" in
  up)
    services=(); for argument in "${@:2}"; do [[ "$argument" == -* ]] || services+=("$argument"); done
    mattermost=false
    [[ ${#services[@]} -eq 0 || " ${services[*]} " == *" mattermost "* ]] && mattermost=true
    # Mattermost's database lives in this copy's PostgreSQL; create it before Mattermost starts.
    if $mattermost; then
      compose up -d --wait postgres
      python3 "$ROOT/ops/mattermost.py" database
    fi
    compose "$@"
    # Drasi's annotation reaction needs a Grafana service-account token (IGNITE_GRAFANA_TOKEN in .env).
    if [[ ${#services[@]} -eq 0 || " ${services[*]} " == *" grafana "* ]]; then
      python3 "$ROOT/ops/observability-env.py" grafana-token
    fi
    # Users, team, #incident-ai-support, bot, tokens and Grafana's webhook (secrets in .env).
    if $mattermost; then
      status=0; python3 "$ROOT/ops/mattermost.py" bootstrap || status=$?
      if [[ $status -eq 3 ]]; then
        # The alert webhook URL changed: recreate Grafana so its contact point uses it.
        compose up -d --wait grafana
        python3 "$ROOT/ops/observability-env.py" grafana-token
      elif [[ $status -ne 0 ]]; then
        exit $status
      fi
    fi
    ;;
  down|stop|start|restart|ps|logs|pull|exec)
    exec podman compose --project-name "$IGNITE_PROJECT" --env-file "$ROOT/.env" "${files[@]}" "$@"
    ;;
  *) echo "Usage: $0 env|up|down|stop|start|restart|ps|logs|pull|exec [compose arguments]" >&2; exit 2 ;;
esac
