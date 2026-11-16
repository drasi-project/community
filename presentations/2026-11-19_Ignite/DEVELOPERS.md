# Developer guide

**Who this is for:** anyone changing the demo's code, queries, dashboards or
slides, or checking that it still works. You don't need this to run or present
the demo: for that, see [SETUP.md](SETUP.md) and [PRESENTER.md](PRESENTER.md).

**What it covers:** how the demo is built, where each part lives, how to make
common changes, how to test them, and the known issues.

## How the demo fits together

The [system architecture](presentation/diagrams/architecture.svg) and
[Drasi graph](presentation/diagrams/drasi-graph.svg) diagrams show this visually.

1. A **workload service** (Rust) plays the AI service. It accepts support and
   summary requests, queues them, and sends them through a **gateway** to three
   **simulated model deployments** (EU-A, EU-B, US-C). The gateway checks the
   placement policy before sending anything. Every request, attempt, deployment
   report and approval is written to **PostgreSQL**.
2. **Drasi Server** reads PostgreSQL's change feed. Its continuous queries
   (Cypher) and continuous models (native plugins) keep the answers current:
   overdue requests, stale deployments, allowed destinations, the best allocation,
   the "if EU-A fails" check and verified recovery.
3. The workload service **subscribes to Drasi's results** (snapshot first, then
   changes). It uses them to approve recovery, and republishes them as Grafana
   metrics and annotations and as Langfuse trace scores.
4. The workload service also sends its own **telemetry** through the
   OpenTelemetry Collector to Prometheus (metrics) and Langfuse (traces).
   **Grafana** shows both, and its alerts call back into the workload service.
5. A small **web app** serves the slides and the private presenter desk. Its controls only change inputs (traffic, deployment faults, policy,
   approvals); everything on the dashboards is derived.

Deployments, regions and capacity are simulated. Everything else is real.

### Rules the code follows

- **Unknown is never healthy.** Stale or missing evidence shows as Unknown. Every
  Drasi value published to Grafana carries a "valid until" time and disappears
  when it expires.
- **Inputs, not answers.** Controls change inputs only. Policy, allocation and
  recovery are computed by Drasi, never by the dashboards or the web app.
- **Snapshot first.** Clients attach to a query and replace their state from its
  snapshot, then apply changes. On reconnect they start again from a snapshot.
- **Epoch and revision.** A scenario reset starts a new `run_epoch`; meaningful
  configuration changes increase `revision`. Proposals, approvals and recovery
  are tied to both, so stale approvals are rejected.
- **Approvals are idempotent.** Each approval has a command ID; retrying reuses it.

## Where things live

All paths are under `demo/` unless noted.

| Path | What it is |
|---|---|
| `workloads/` | The workload service (Rust crate `ignite-workloads`): gateway, simulators, traffic generator, approvals, Drasi subscriptions (`observer.rs`, `decisions.rs`), Grafana/Langfuse publishing (`dashboard.rs`, `telemetry.rs`) |
| `domain/` | Shared domain logic (crate `ignite-domain`): the model, Rego policy evaluation and allocation search |
| `plugins/capacity`, `plugins/decisions` | The demo's continuous models, as Drasi native plugins: capacity, policy, allocation and resilience |
| `drasi/*.cypher` | The continuous queries |
| `ops/config/` | Generates Drasi Server's configuration (`.native/server.json`): every source, query, model and connection |
| `policies/placement.rego` | The placement policy, used by both Drasi and the gateway |
| `database/` | PostgreSQL schema (`schema.sql`, `workload.sql`) and scenario reset (`reset.sql`) |
| `observability/` | Container configuration: Collector, Prometheus, Grafana dashboards, alerts and data source, and an integration test probe |
| `observability/grafana/dashboards.py` | Generates the two audience dashboards |
| `ui/` | The web app (Vite, React, Reveal.js) and its tests and scripts |
| `../presentation/` | Slide content and speaker notes, built by `ui/` (see below) |
| `tests/` | Integration and regression tests (Python) |
| `ops/` | Build, run and release scripts |
| `compose.yaml` | The ten supporting containers |
| `.env` | Generated local passwords. Private; never commit or print |
| `.native/` | Build output, generated configuration and test evidence. Ignored |

**Presentation sources** (`presentation/`):

| File | Role |
|---|---|
| `scenes.ts` | The three slides, the speaking cues and product URLs |
| `notes.ts` | Speaker notes: the single source for the presenter desk and for the run of show and script in `PRESENTER.md` |
| `Explanation.tsx` | Slide content |
| `Audience.tsx` | Slide and replay display, and navigation into Grafana |
| `Presenter.tsx` | The private presenter desk |
| `replay.ts` | The recorded fallback clips |

**Scripts** (`ops/`):

| Script | What it does |
|---|---|
| `observability-stack.sh` | Runs `podman compose` for this demo only. `env` creates `.env` |
| `observability-build.sh` | Builds the test probe container image |
| `native-build.sh` | Builds Drasi Server and its UI, the Core plugins, the demo plugins, the config generator and the workload service; writes and validates `server.json`; stamps the build |
| `native-runtime.py` | `init` (create database), `migrate`, `reset`, `run` (start Server), `sql`, `stamp` |
| `workload.py` | `run` (start the workload service), `status`, `context`, `decisions`, `control`, `client`, `reset` |
| `macho-align.py` | Works around a macOS linker bug (see Known issues) |
| `release.py` | Automated cycles, soak test, preflight, release stamp and verify |

### Ports

| Port | Service |
|---|---|
| 18421 | Drasi Server API and UI (`/ui/`) |
| 18422 | Drasi HTTP webhook source (used by the test probe) |
| 18500 | Workload service: gateway, controls, alert webhook |
| 18531–18533 | Simulated deployments EU-A, EU-B, US-C |
| 18580 | Slides and presenter desk |
| 13000 / 13001 | Grafana / Langfuse |
| 19090 / 14318 | Prometheus / OpenTelemetry Collector |
| 15432 | PostgreSQL (database and user `ignite`) |

Everything listens on loopback only and has no authentication. Don't expose it.

## Making changes

Rebuild with `./ops/native-build.sh` (with `.env` loaded) after any Rust, Cypher,
Rego or configuration change. Server and the workload service refuse to start if
their files changed since the last build. Restart order is always Server, then
the workload service, then the web app.

| To change… | Edit | Then |
|---|---|---|
| A continuous query | `drasi/<name>.cypher` | Rebuild; restart Server and the workload service |
| Which queries exist or how they connect | `ops/config/src/main.rs` | Rebuild; restart both |
| Policy rules | `policies/placement.rego` | Rebuild; restart both |
| Allocation or domain logic | `domain/src/` | Rebuild; restart both |
| A continuous model plugin | `plugins/*/src/lib.rs` | Rebuild; restart both |
| Gateway, traffic, approvals or publishing | `workloads/src/` | Rebuild; restart the workload service |
| Grafana dashboards | `observability/grafana/dashboards.py` | Run it from `demo/`, then reload Grafana (below) |
| Grafana alerts | `observability/grafana/provisioning/alerting/` | Reload Grafana |
| Speaker notes or cue timings | `presentation/notes.ts`, `scenes.ts` | `cd ui && npm run sync-presenter` (updates `PRESENTER.md`), then `npm run build` |
| Slides | `presentation/Explanation.tsx` | `cd ui && npm run build` |
| Desk or app behaviour | `presentation/Presenter.tsx`, `ui/src/` | `cd ui && npm run build`; update `PRESENTER.md` if a desk label changes |
| Database schema | `database/workload.sql` | `python3 ops/native-runtime.py migrate` with Server and workload stopped |

Reload Grafana dashboards and alerts without restarting:

```sh
set -a; . ./.env; set +a
for kind in dashboards alerting; do
  curl -s -u "admin:$GRAFANA_ADMIN_PASSWORD" -X POST \
    http://localhost:13000/api/admin/provisioning/$kind/reload
done
```

Panel names are referenced by `presentation/notes.ts` (and so `PRESENTER.md`),
`ui/scripts/audience-review.mjs` (by panel ID) and `tests/workload-telemetry.py`.
Update them together.

## Testing

Run from `demo/` with `.env` loaded (`set -a; . ./.env; set +a`).

**Quick checks** (no running demo needed):

| Command | Checks |
|---|---|
| `CARGO_TARGET_DIR=.native/target cargo test --locked --workspace` | Domain logic, plugins, workload service, config generator |
| `cd ui && npm test` | Slides and desk against a test fixture, at 1280×720 and 1920×1080; and that `PRESENTER.md`'s script matches `notes.ts` |
| `python3 tests/release-test.py`, `python3 tests/native-runtime-test.py` | Release and bundle guards |

**Against the running demo:**

| Command | Checks | Changes data? |
|---|---|---|
| `cd ui && npm run test:live` | Slides and desk against the real services | No |
| `node ui/scripts/audience-review.mjs inspect` | Both Grafana dashboards load with real data; saves screenshots | No |
| `python3 tests/workload-telemetry.py` | Metrics, Langfuse traces and Grafana alert for real requests | Yes |
| `python3 tests/decision-integration.py` | Policy, allocation and resilience results | Yes |
| `python3 tests/recovery-integration.py` | Approval, stale rejection, verified recovery and its expiry | Yes |
| `IGNITE_SKIP_SERVER_UI=1 IGNITE_AUDIENCE_WINDOW=1 node ui/scripts/audience-review.mjs record` | The whole talk end to end in real Grafana, and re-records the four replay clips | Yes; resets at the start and restores the baseline |

After `record`, run `cd ui && npm run build` so the app serves the new clips.
Tests that change data must not run while someone else is using the demo.
`IGNITE_SKIP_SERVER_UI=1` skips the Drasi Server UI step (see Known issues); the
query source is still checked through the API.

**Tests that start their own processes** (stop Server and the workload first):
`tests/native-integration.py`, `tests/workload-lifecycle.py`,
`tests/recovery-lifecycle.py`.

**Release qualification** (long-running, before an event):

```sh
python3 ops/release.py cycles     # five full incident/recovery/reset cycles
python3 ops/release.py soak       # one real hour of traffic with an incident
python3 ops/release.py preflight  # everything attached, fresh and unchanged
python3 ops/release.py stamp      # record the exact build as the release
python3 ops/release.py verify     # check nothing changed since the stamp
```

Any later change invalidates the stamp; rebuild, retest and stamp again.
Evidence from every run, including failures, is kept under `.native/evidence/`.

## Dependencies on Drasi Core and Server

The demo builds against the sibling `drasi-core` and `drasi-server` checkouts
on `agentofreality-parallel-computation-graph-recovery`, not published packages.
Problems in Drasi itself are fixed in those repositories, not worked around
here, except where noted below.

## Known issues

| Issue | Effect | Status |
|---|---|---|
| `recovery-results` can't use the snapshot-bootstrapped query type, which can't yet publish results to Server | It runs as an ordinary query fed by its two upstream queries instead. It only rebuilds its state when the whole graph starts, so restart all of Server, never just this query | Waiting for a Drasi Core fix. Then switch `recovery-results` in `ops/config/src/main.rs` back to `drasi.experimental/volatile-query`, bootstrapped from its two upstream queries' snapshots, with a `catalog` dependency |
| Drasi Server UI requests component details by ID, but the API now expects names | The UI shows no query text or configuration | Drasi Server issue, not yet filed |
| Drasi Server's expanded query view uses 14 px text | Too small to read on a projector; use browser zoom | Drasi Server issue, not yet filed |
| A macOS linker release leaves some plugin libraries unloadable ("mis-aligned LINKEDIT string pool") | `ops/macho-align.py` repairs them during the build | Apple toolchain bug |
| Retries have no limit | A deployment left failed or throttled for hours creates tens of thousands of attempts. The workload service then can't read the attempt history (it shows Unknown), a reset can exceed Drasi's 64 MB message limit, and Server startup slows to minutes | Open; avoid leaving faults on |

## Data and safety

- Never run `podman compose down -v`, `podman system prune` or broad `kill`
  commands. They delete this demo's database or affect other containers.
- Never delete `.env` while the container volumes exist; passwords are set only once.
- Stop processes by exact PID or with Ctrl-C, and never start a second copy of a
  service on the same port.
- Before a long PostgreSQL outage, stop Server first.
