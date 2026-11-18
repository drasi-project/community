# Developer guide

**Who this is for:** anyone changing the demo's code, queries, dashboards or
slides, or checking that it still works. You don't need this to run or present
the demo: for that, see [SETUP.md](SETUP.md) and [PRESENTER.md](PRESENTER.md).

**What it covers:** how the demo is built, where each part lives, how to make
common changes, how to test them, and the known issues.

## How the demo fits together

The [system architecture](presentation/diagrams/architecture.svg) and
[Drasi graph](presentation/diagrams/drasi-graph.svg) diagrams show this visually.
[DRASI.md](DRASI.md) covers every Drasi component, its configuration, and what is
simulated.

1. The **AI service** (Rust crate `ignite-workloads`) is the application and the
   systems around it. It accepts support and summary requests, queues them, and
   its **gateway** sends each to the deployment its route names, among three
   **simulated model deployments** (EU-A, EU-B, US-C). A **health monitor**
   checks each deployment every second and exports what it found as
   OpenTelemetry metrics (`deployment_health_probe_*`). Two small APIs stand in for external
   systems: a **change-approval tool** (`POST /approvals`, which only records an
   approval) and the gateway's **configuration API** (`POST /routes/apply`,
   `/approvals/{id}/reject`, `/approvals/{id}/verified`, called only by Drasi).
   **Mattermost**'s card buttons reach the approval tool through
   `POST /mattermost/actions`, which records the approval exactly as
   `POST /approvals` does (and updates the card); `POST /mattermost/approve-latest`
   presses Approve on the newest proposal card for the desk's backup button.
   Every request, attempt, scenario setting and approval is a row in
   **PostgreSQL**. The AI service reads none of Drasi's results.
2. **Drasi Server** reads PostgreSQL's change feed, and the health monitor's
   probe metrics through its OpenTelemetry source (`IGNITE_HEALTH_SOURCE=otel`
   in `ports.env`; with `postgres` the monitor writes `deployment_health_reports` and Drasi
   reads health from PostgreSQL instead). Its continuous queries
   (Cypher) prepare the inputs for two of Drasi's standard continuous models,
   configured by files in `drasi/`: a Rego **policy** (`drasi.standard/policy`),
   and an **optimizer** (`drasi.standard/optimization`, on `good_lp`) that
   produces the **plan** and its **what-if** (is there still a plan if any one
   healthy deployment fails?). Further queries turn the results into one answer
   per Grafana tile, decide whether a pending approval still matches the current
   plan, and decide whether recovery is verified. Seven event queries
   (`drasi/incident-*.cypher`) turn changes into incident cards: one row per card.
3. Drasi's **reactions** deliver the answers: a Prometheus endpoint that
   Prometheus scrapes for Grafana's tiles and alert; HTTP calls to the gateway
   configuration API (apply, reject, verified), Grafana's annotations API and
   Langfuse's scores API; an SSE stream for the presenter desk; and the
   incident cards, posted to Mattermost's REST API as the `drasi` bot.
4. The AI service also sends its own **telemetry** through the OpenTelemetry
   Collector to Prometheus (metrics) and Langfuse (traces), and the Collector
   forwards the health probes to Drasi (`observability/collector-drasi.yaml`). **Grafana** shows
   both and raises the alerts, which also post to **Mattermost**'s
   `#incident-ai-support`, the incident channel where Drasi posts its incident
   cards.
5. A small **web app** serves the projected stage (slides, the dashboard
   embedded from Grafana, the incident channel embedded from Mattermost, the late
   request view) and the private presenter desk
   that drives it. Its controls only change inputs (traffic, deployment faults,
   policy, capacity, approvals); every answer comes from Drasi.

Deployments, their health checks, capacity, the approval tool and the gateway
configuration API are simulated. Everything else is real.

### Rules the code follows

- **Unknown is never healthy.** Stale or missing evidence shows as Unknown: a
  deployment's health expires 8 seconds after its last probe (`drasi.trueLater`), the models emit
  `unknown` rather than guess, and Grafana shows a Drasi tile only while the
  Prometheus reaction reports its input loaded (`drasi_reaction_input_ready`).
- **Inputs, not answers.** Controls change inputs only. Policy, plans, approval
  checks and recovery are computed by Drasi, never by the AI service, the
  dashboards or the web app.
- **Snapshot first.** The desk, the query slide, the Prometheus reaction and the
  models load a snapshot, then apply changes. On reconnect they start again
  from a snapshot.
- **Epoch and revision.** A scenario reset starts a new `run_epoch`; every
  change to routes, capacity or allowed destinations raises
  `routing_config.revision`. A plan carries its revision and a `basis` hash of
  the facts it solved, so an approval that no longer matches is rejected.
- **Idempotent reactions.** The gateway configuration API records each
  `x-drasi-delivery-id`, acts only on a `pending` (or, for verified, `applied`)
  approval, and applies routes only if the revision still equals the plan's.

## Where things live

All paths are under `demo/` unless noted.

| Path | What it is |
|---|---|
| `workloads/` | The AI service (`ignite-workloads`): gateway and retries (`main.rs`), simulated deployments and health monitor (`main.rs`; its probe metrics in `probe.rs`), traffic (`traffic.rs`), demo controls and reset (`scenario.rs`), the approval tool and gateway configuration API (`external.rs`), telemetry (`telemetry.rs`) |
| `settings/` | Crate `ignite-settings`: reads `ports.env` for every Rust program |
| `drasi/policy/` | The placement rules (`placement.rego`) and the configuration (`policy.json`) of Drasi's Rego policy, `drasi.standard/policy` (drasi-core crate `drasi-computation-transformer-policy`) |
| `drasi/optimization/` | The placement model, its single-loss what-if and its preconditions: the configuration (`optimization.json`) of Drasi's optimization, `drasi.standard/optimization` (drasi-core crate `drasi-computation-transformer-optimization`, `good_lp` with `microlp`) |
| `plugins/prometheus` | Prometheus reaction, `ignite/prometheus-exporter` (crate `ignite-prometheus`) |
| `plugins/prometheus/examples/ignite.json` | The Prometheus reaction's metric mapping in this graph |
| `drasi/*.cypher` | The 47 continuous queries; `incident-*` build the incident cards |
| `ops/config/` | Generates Drasi Server's configuration (`.native/server.json`): every source, query, model, reaction and connection |
| `database/` | PostgreSQL schema (`schema.sql`, `workload.sql`) and scenario seed (`reset.sql`) |
| `observability/` | Container configuration: Collector, Prometheus, Grafana dashboards, alerts and data source, and an integration test probe |
| `observability/grafana/dashboards.py` | Generates the audience dashboard, `ai-operations` (AI Deployments and Decision Support rows); `fixture.py` serves sample `/metrics` for each tile state |
| `ui/` | The web app (Vite, React, Reveal.js) and its tests and scripts |
| `../presentation/` | Slide content and speaker notes, built by `ui/` (see below) |
| `tests/` | Integration and regression tests (Python) |
| `ops/` | Build, run and release scripts |
| `compose.yaml` | The ten supporting containers |
| `ports.env` | Every port and name this copy uses (see [Ports](#ports)) |
| `.env` | Generated local passwords and the Grafana token. Private; never commit or print |
| `.native/` | Build output, generated configuration and test evidence. Ignored |

**Presentation sources** (`presentation/`):

| File | Role |
|---|---|
| `scenes.ts` | The slides, the talk's parts and timings, dashboard and product URLs |
| `notes.ts` | The script: each part's beats (what the projector shows, what to say, which desk buttons, what to click on the projected channel, what to watch for, which panels to highlight), the glossary and the questions to expect. The single source for the desk and `PRESENTER.md` |
| `script.ts` | Generates `PRESENTER.md`'s run of show, script, glossary and questions from `notes.ts` |
| `Audience.tsx` | The projected stage: always the Ignite template's 1920×1080 slide, scaled to fit the window. Slides, the dashboard embedded in Grafana kiosk mode at 1.35×, the incident channel (`incident`: Mattermost's `#incident-ai-support`, framed at 1.5× so it lays out 1280 px wide; the projector's browser is signed in as `oncall` through the AI service's `/mattermost/sign-in`), both ending at the caption line (y 1002) and kept loaded once shown, the late request view and recordings. Keys pressed here (a clicker) are passed to the desk; if no desk answers within half a second, the stage steps through the talk's views on its own |
| `Presenter.tsx`, `desk.css` | The presenter desk: the current beat with its buttons and what to click on the projector, a mirror of Drasi's answers, the live demo and routes, and the collapsible extras. Approving happens on the projected proposal card; the desk's backup, **Approve the proposed change**, calls `POST /mattermost/approve-latest`, which presses that card's Approve, and the desk reports Drasi's answer from `tile-last-approval`. **Next** moves to the next beat and switches the projector, warning once if the beat's button wasn't pressed. Recording mode (`fallback` in the stage state) shows each live part's recording instead |
| `Explanation.tsx`, `slides.css` | Slide content and styles. `slides.css` holds the Ignite template's dark theme: its colours, Segoe UI, and the layout grid in pixels on a 1920×1080 slide (92 px side margins, titles at y 72, body from y 226, nothing below y 1002, the closed-caption clear space). Template artwork and fonts load from `/template/` (see `ops/template-assets.py` below) |
| `Highlight.tsx`, `panels.ts` | Highlights the dashboard panels a beat talks about (a beat's `highlight` in `notes.ts`, as `<dashboard uid>.<panel key>`): dims the rest of the projected dashboard and outlines the panels, placed from the dashboard JSON's `gridPos` the way Grafana lays it out. A panel's key is its `key` in the dashboard JSON, else its title in lower case with dashes; a row's key covers its panels. A test fails if a highlight names no panel |
| `QuerySlide.tsx` | The continuous query slide: `demo/drasi/overdue-tasks.cypher` laid out for the projector (a test checks it matches the file), and Drasi's live list of late requests |
| `RequestView.tsx` | One late request: its question, attempts and the moment Drasi listed it, from the AI service's `/tasks/{id}` |
| `replay.ts` | The recorded fallback clips |

The desk and the projector are two windows of the same browser; they talk over a
`BroadcastChannel` (`demo/ui/src/stage.ts`). The desk owns the position (part,
beat, what's shown) and stores it in `localStorage`.

**Scripts** (`ops/`):

| Script | What it does |
|---|---|
| `observability-stack.sh` | Runs `podman compose` for this copy only (project `$IGNITE_PROJECT`). `env` creates `.env`; `up` also writes Prometheus's scrape target for the Drasi reaction and, when it starts Grafana, makes sure `.env` has a working `IGNITE_GRAFANA_TOKEN` |
| `observability-env.py` | Creates `.env`; `grafana-token` creates or checks the `drasi-annotations` service-account token |
| `observability-build.sh` | Builds the test probe container image (`localhost/$IGNITE_PROJECT-probe:phase1`) |
| `native-build.sh` | Builds Drasi Server and its UI; the Core plugins (PostgreSQL source and bootstrap, OpenTelemetry source, query, policy and optimization transformers, HTTP and SSE sinks); the demo's Prometheus reaction; the config generator and the AI service. Aligns the plugin libraries (`macho-align.py`), writes and validates `server.json` and stamps the build |
| `native-runtime.py` | `init` (create database), `migrate`, `reset`, `run` (delete earlier runs' rows and recorded deliveries, export the reactions' secrets, then start Server), `sql`, `stamp` |
| `workload.py` | `run` (start the AI service), `status` (its `/health`), `approve`, `configure`, `control`, `client`, `reset` |
| `settings.py` | Reads `ports.env` for Python and shell (`eval "$(python3 ops/settings.py)"`) |
| `macho-align.py` | Works around a macOS linker bug (see Known issues). Like `native-build.sh`, it replaces plugin files rather than rewriting them in place, because macOS kills a process that loads a library changed in place |
| `release.py` | Automated cycles, soak test, preflight, release stamp and verify |
| `template-assets.py` | Extracts the Ignite template's artwork, the session survey slide and Segoe UI into `ui/public/template/` (git-ignored, never committed: they are Microsoft's). See SETUP.md |

### Ports

Every port, the container project name, the PostgreSQL publication and slot,
and the browser storage key come from `demo/ports.env`. Compose (through
`ops/observability-stack.sh`), the `ops/` scripts, the Server config generator,
the AI service, the web app and the tests all read it; an exported variable of
the same name overrides a line, and `IGNITE_SETTINGS=<file>` selects another
file. `ports.env` holds this copy's values (each original port +1000, project
`ignite-native`), so it runs beside the original demo, whose values are in
`ports.default.env`. Grafana's dashboard links carry the Langfuse port:
regenerate them with `observability/grafana/dashboards.py` after changing it.

| Setting | This copy | Original | Service |
|---|---|---|---|
| `IGNITE_DRASI_API_PORT` | 19421 | 18421 | Drasi Server API and UI (`/ui/`) |
| `IGNITE_DRASI_METRICS_PORT` | 19423 | 18423 | Drasi Prometheus reaction (`/metrics`) |
| `IGNITE_DRASI_SSE_PORT` | 19424 | 18424 | Drasi SSE reaction for the desk (`/events`, `/snapshot/{query}`) |
| `IGNITE_DRASI_OTLP_PORT` | 19427 | 18427 | Drasi OpenTelemetry source (OTLP/gRPC, loopback), fed the health probes by the Collector |
| `IGNITE_WORKLOAD_PORT` | 19500 | 18500 | AI service: gateway, controls, approval tool, gateway configuration |
| `IGNITE_DEPLOYMENT_{EU_A,EU_B,US_C}_PORT` | 19531–19533 | 18531–18533 | Simulated deployments |
| `IGNITE_UI_PORT` / `_UI_TEST_PORT` / `_UI_FIXTURE_PORT` | 19580 / 19581 / 19582 | 18580 / 18581 / 18582 | Slides and presenter desk; the browser tests' copy and fixture server |
| `IGNITE_GRAFANA_PORT` / `IGNITE_LANGFUSE_PORT` | 14000 / 14001 | 13000 / 13001 | Grafana / Langfuse |
| `IGNITE_PROMETHEUS_PORT` / `IGNITE_OTLP_PORT` | 20090 / 15318 | 19090 / 14318 | Prometheus / OpenTelemetry Collector |
| `IGNITE_POSTGRES_PORT` | 16432 | 15432 | PostgreSQL (database and user `ignite`) |
| `IGNITE_PROBE_PORT` | 19888 | 18888 | Integration test probe |
| `IGNITE_MATTERMOST_PORT` | 19065 | 18065 | Mattermost, the incident channel |
| `IGNITE_PROJECT` | `ignite-native` | `ignite-observability` | Podman compose project and probe image name |
| `IGNITE_PG_PUBLICATION`, `IGNITE_PG_SLOT` | `ignite_rework` | `ignite_native` | Logical replication read by Drasi |
| `IGNITE_STORAGE_KEY` | `ignite-native-stage-v1` | `ignite-presentation-stage-v3` | Presenter stage in browser storage |
| `IGNITE_HEALTH_SOURCE` | `otel` | `postgres` | Where Drasi learns deployment health: the probes' OpenTelemetry metrics, or `deployment_health_reports` rows. Read by the AI service, the Server configuration generator and `ops/observability-stack.sh` (which adds the Collector's Drasi pipeline) |

Everything listens on loopback only and has no authentication. Don't expose it.

## Making changes

Rebuild with `./ops/native-build.sh` (with `.env` loaded) after any Rust, Cypher,
Rego or configuration change. Server and the AI service refuse to start if their
files changed since the last build. They don't depend on each other: restart
only the one you changed, then reload the desk.

| To change… | Edit | Then |
|---|---|---|
| A continuous query | `drasi/<name>.cypher` | Rebuild; restart Server |
| Which components exist or how they connect | `ops/config/src/main.rs` | Rebuild; restart Server |
| An incident card's text or trigger | `drasi/incident-<card>.cypher` (every string is built there); its colour, field titles and buttons are `incident_routes` in `ops/config/src/main.rs` | `cargo test --locked -p ignite-config`; rebuild; restart Server |
| Policy rules | `drasi/policy/placement.rego` (its input and output mapping: `drasi/policy/policy.json`; what it decides: `drasi/policy-input.cypher`) | Rebuild; restart Server |
| The optimizer's inputs, constraints, objectives, what-if or preconditions | `drasi/optimization/optimization.json` (its inputs: `drasi/plan-*.cypher`; its results: `plan-`, `assignment-`, `resilience-` and `loss-results.cypher`) | `cargo test --locked -p ignite-config`; rebuild; restart Server |
| Which tile rows become which metrics | `plugins/prometheus/examples/ignite.json` (and Grafana, below) | Rebuild; restart Server |
| The Prometheus reaction's code | `plugins/prometheus/src/` | Test the crate (below); rebuild; restart Server |
| Gateway, traffic, simulations, approval tool or gateway configuration API | `workloads/src/` | Rebuild; restart the AI service |
| Mattermost's settings | `MM_*` variables of the `mattermost` service in `compose.yaml` | `./ops/observability-stack.sh up -d --wait mattermost` (recreates it; data stays in the `mattermost-data` volume and the `mattermost` database) |
| Mattermost's users, team, channel, bot, tokens, webhook | `ops/mattermost.py` | `python3 ops/mattermost.py bootstrap` (idempotent; secrets to `.env`, IDs to `.native/mattermost.json`). If the channel was recreated, rebuild (`ops/native-build.sh`): the config generator writes its ID into Server's configuration. `python3 ops/mattermost.py clear` empties the channel, as the desk's Reset does |
| Grafana dashboards | `observability/grafana/dashboards.py` | Run it from `demo/`, then `./ops/observability-stack.sh restart grafana` (Grafana doesn't notice regenerated files through the Podman mount) |
| Grafana alerts | `observability/grafana/provisioning/alerting/` | Reload Grafana (below). Both rules also post to #incident-ai-support (contact point `incident-channel`). The **Alerts firing** panel is Grafana's own alert list, showing rules labelled `component: workload`; give a new rule that label to list it |
| The script, beats or timings | `presentation/notes.ts`, `scenes.ts` | `cd ui && npm run sync-presenter` (updates `PRESENTER.md`), then `npm run build` |
| Slides | `presentation/Explanation.tsx`, `slides.css` (the query slide: `QuerySlide.tsx`, which a test keeps in step with `drasi/overdue-tasks.cypher`) | `cd ui && npm run build` |
| Desk or app behaviour | `presentation/Presenter.tsx`, `ui/src/` | `cd ui && npm run build`; update `PRESENTER.md` if a desk label changes |
| Database schema | `database/workload.sql` | `python3 ops/native-runtime.py migrate` with Server and the AI service stopped |
| The diagrams | `presentation/diagrams/*.svg` (template colours and Segoe UI, as the slides) | `node ui/scripts/render-diagrams.mjs` re-renders each PNG at 3840×2160 |
| Template artwork, survey QR code, fonts | Not in the repository: `ops/template-assets.py` extracts them to `ui/public/template/` (git-ignored; see SETUP.md). `vite.config.ts` serves that folder at `/template/` as it is | Run the script again |

Reload Grafana's alerting without restarting:

```sh
set -a; . ./.env; set +a; eval "$(python3 ops/settings.py)"
curl -s -u "admin:$GRAFANA_ADMIN_PASSWORD" -X POST \
  http://localhost:$IGNITE_GRAFANA_PORT/api/admin/provisioning/alerting/reload
```

Panel titles and values are referenced by `presentation/notes.ts` (and so
`PRESENTER.md`) and `tests/workload-telemetry.py`. Update them together.

## Testing

Run from `demo/` with `.env` loaded (`set -a; . ./.env; set +a`).

**Quick checks** (no running demo needed). Build and test one crate at a time;
builds go to the shared `.cargo-target-next` beside the checkouts
(`demo/.cargo/config.toml`, at most 4 jobs). Never use `--workspace` in
drasi-core or drasi-server.

| Crate | Build | Test |
|---|---|---|
| Prometheus reaction | `cargo build --locked -p ignite-prometheus --lib --features ignite-prometheus/dynamic-plugin` | `cargo test --locked -p ignite-prometheus --lib` |
| AI service | `cargo build --locked -p ignite-workloads` | `cargo test --locked -p ignite-workloads`; its database tests reset this copy's scenario and are ignored by default: `cargo test --locked -p ignite-workloads -- --ignored --test-threads=1` |
| Settings | | `cargo test --locked -p ignite-settings --lib` |
| Config generator and incident cards | `cargo check --locked -p ignite-config` | `cargo test --locked -p ignite-config`: the card templates render to the contract's Mattermost posts, and every `incident-*` query runs through drasi-core with a test clock (debounce, one card per event, the 60 s window, nothing after a restart or a reset). `./ops/native-build.sh` runs the generator and `drasi-server validate` on its output |

The Prometheus reaction keeps its logic in plain modules with host-free tests
(the text format) and has one SDK-level test of its factory. The policy and the
optimizer are Drasi's own crates, tested in `drasi-core-next`
(`cargo test -p drasi-computation-transformer-policy`,
`-p drasi-computation-transformer-optimization`). `ignite-config`'s tests run
the demo's result queries over the optimizer's output, and
`tests/decision-integration.py` checks the talk's outcomes live (the failure
plan, infeasible after the policy edit, at risk at EU-A 50k).

| Command | Checks |
|---|---|
| `cd ui && npm test` | Slides and desk against a test fixture, at 1280×720 and 1920×1080: every slide inside the template's margins and above the caption line, dashboards and the incident channel ending at it, and the stage still working without the template assets; approving on a card (a stand-in channel page, `tests/mattermost-stand-in.html`, shows the fixture's cards) and through the desk's backup, including the out-of-date proposal rejected; and that `PRESENTER.md`'s script matches `notes.ts`. Builds into `ui/dist-test`, so it never replaces what a running demo serves |
| `python3 observability/grafana/fixture.py [state]` | Serves sample `/metrics` for each tile state on the reaction's port, for checking Grafana; only while Drasi Server is stopped |
| `python3 tests/release-test.py`, `python3 tests/native-runtime-test.py` | Release and bundle guards |
| `python3 tests/otlp.py --self-test` | The tests' OTLP sender: protobuf encoding of a probe metric |

**Against the running demo:** these read every answer from Drasi's outputs
(`tests/drasi_outputs.py`: the desk stream, the Prometheus reaction and Server's
query results), never from the AI service. Their `reset()` resets the scenario and then allows
summaries in the US, as the desk's **Summaries: Europe + US** does, so they start
safe; `reset(us=False)` leaves the seed, with failover at risk.

| Command | Checks | Changes data? |
|---|---|---|
| `cd ui && npm run test:live` | Slides and desk against the real services | No |
| `python3 tests/workload-integration.py` | The application and simulated deployments as Drasi sees them: deployment status, overdue requests and their history | Yes |
| `python3 tests/workload-telemetry.py` | The application's metrics and Langfuse traces, Drasi's overdue metric, and Grafana's alerts on both | Yes |
| `python3 tests/decision-integration.py` | Policy, plan and what-if | Yes |
| `python3 tests/health-telemetry-integration.py` | Health from telemetry: probes through the Collector reach Drasi; Failed and the planner's move, Unknown after 8 s of silence, recovery; `deployment_health_reports` not written. Applies with `IGNITE_HEALTH_SOURCE=otel` | Yes; resets at the start and the end |
| `python3 tests/mattermost-actions.py` | Mattermost's card buttons reach the approval tool (approve, reject, an earlier run's card), the desk's backup press, the projector sign-in | Yes: adds approvals Drasi rejects (made-up plans); deletes its cards |
| `python3 tests/incident-channel-integration.py` | Drasi's incident cards in #incident-ai-support in the talk's order: the seed's at risk, restored once summaries may run in the US, failure, proposal, overdue, approve → Applied → Verified in the card's thread, then Restore EU-A posts nothing and moves nothing back; each posted once | Yes; resets at the start and the end (clears the channel) |
| `python3 tests/recovery-integration.py` | Approval checked by Drasi, routes changed by its gateway reaction, stale rejection, verified recovery, annotations and scores | Yes |
| `node ui/scripts/walkthrough.mjs capture <folder>` | The whole talk through the desk, beat by beat, as a presenter would; saves the projector and the desk at every beat (about 12 minutes) | Yes; resets at the start and the end |
| `node ui/scripts/walkthrough.mjs record` | The same, and re-records the four fallback recordings at 1920×1080 (16:9) | Yes; resets at the start and the end |

After `record`, run `cd ui && npm run build` so the app serves the new clips.
Tests that change data must not run while someone else is using the demo.

**Tests that start their own processes:**

| Command | Checks | Stop first |
|---|---|---|
| `python3 tests/native-integration.py` | Server lifecycle: bootstrap from populated tables, a restart that rebuilds every answer (including changes and an approval made while it was down), replication reconnect | Server |
| `python3 tests/workload-lifecycle.py` | Drasi's answers while the AI service or Server is down | Server and the AI service |
| `python3 tests/recovery-lifecycle.py` | The reactions while the systems they call are down: the gateway reaction applies once the API is back; Grafana and Langfuse outages don't hold up recovery | Server and the AI service |

Each leaves the processes it owns stopped.

**Release qualification** (long-running, before an event):

```sh
python3 ops/release.py cycles     # five full incident/recovery/reset cycles
python3 ops/release.py soak       # one real hour of traffic with an incident
python3 ops/release.py preflight  # everything attached, fresh and unchanged
python3 ops/release.py stamp      # record the exact build as the release
python3 ops/release.py verify     # check nothing changed since the stamp
```

`cycles`, `soak` and `preflight` read Drasi's outputs, like the integration tests.
Any later change invalidates the stamp; rebuild, retest and stamp again.
Evidence from every run, including failures, is kept under `.native/evidence/`.

## Dependencies on Drasi Core and Server

The demo builds against sibling checkouts of Drasi Core and Server, not
published packages: `drasi-core-next` and `drasi-server-next` beside this
repository, both on branch `agentofreality-ignite-native-next`. They are the
merged drasi-core (PR 1030) and the Server adapted to it, on
`agentofreality-parallel-computation-graph-recovery`. `ops/settings.py` and
`ops/native-build.sh` name them (`IGNITE_DRASI_CORE`, `IGNITE_DRASI_SERVER`,
defaults `drasi-core-next` and `drasi-server-next`), and the demo's Rust crates
reference `../drasi-core-next` by path. Server and every plugin must come from
the same drasi-core build: the plugin services interface (version 2) requires an
exact match, so `native-build.sh` builds them together.

Problems in Drasi itself are fixed in those repositories, not worked around
here, except where [Known issues](#known-issues) says so. The checkouts carry
these commits beyond their upstream branch:

| Commit | Where | What the demo needs from it |
|---|---|---|
| `0e0e3ed4` | drasi-core: HTTP sink | `unroutedChanges: ignore`: a change whose operation has no template route completes without a request. The HTTP reactions route only additions (and updates, for approval annotations); without it every other change would be posted to `{baseUrl}/changes/{alias}`, which the receivers don't have, and the gateway reaction would stall |
| `b282af95`, `6b663dd0`, `26267f08`, `9afd6aa6` | drasi-core: `drasi.standard/optimization` | Preconditions, solution summaries and identities (`signature`, `solutionSignature`), the scenario roll-up, placement candidates separate from policy decisions, start from upstream snapshots, and `atomicScenarios` |
| `a79f7e27`, `854886c3`, `8d0dcce9` | drasi-core: `drasi.standard/policy` and the plugin SDK | Volatile transformers start from their upstream outputs' snapshots, and the policy emits the decisions for that snapshot at startup |
| `a4eed95` | drasi-server | Builds against `../drasi-core-next` |

## Known issues

| Issue | Effect | Status |
|---|---|---|
| A query fed by another query receives only that query's changes, not its bootstrap rows | Query-to-query edges are used only where the upstream's rows arrive live after startup: the models' results and the recovery chain (`recovery-task-context` and `recovery-observation-context` → `recovery-results`, a workaround: it should bootstrap from their snapshots). Tiles over rows that rarely change read PostgreSQL directly instead of an upstream query: `tile-capacity` joins the deployments and their health itself rather than reading `plan-bins`. These queries rebuild only when the whole graph starts, so restart all of Server, never one query | Open. Queries now have a `snapshot` output; try bootstrap edges from upstream queries in `ops/config/src/main.rs` |
| Drasi Server's API lists no reactions (`GET …/instances/ai-ops/reactions` is empty) | The Server UI shows the queries and sources but none of the six native sinks; see `.native/server.json` | Drasi Server issue, not yet filed |
| Drasi Server UI requests component details by ID, but the API now expects names | The UI shows no query text or configuration | Drasi Server issue, not yet filed |
| Drasi Server's expanded query view uses 14 px text | Too small to read on a projector; use browser zoom | Drasi Server issue, not yet filed |
| Grafana's annotations API has no idempotency key | A retried post whose response was lost, or a Server restart within 30 s of an approval event, can add the same decision event twice (same tag) | Accepted for the demo |
| A macOS linker release leaves some plugin libraries unloadable ("mis-aligned LINKEDIT string pool") | `ops/macho-align.py` repairs them during the build | Apple toolchain bug |
| The fallback recordings are long (failure about 2 minutes, recovery about 4) | The presenter talks over a lot of footage in recording mode | Open; trim with a video tool, or shorten the waits in `ui/scripts/walkthrough.mjs` and re-record |
| Retries have no limit | A deployment left failed or throttled for hours creates tens of thousands of attempts; a reset can exceed Drasi's 64 MB message limit, and Server startup slows to minutes | Open; avoid leaving faults on. A reset followed by a restart clears it, because startup deletes earlier runs' rows |
| Incident cards are posted, not updated | Mattermost assigns post IDs and the sink ignores responses, so only approval outcomes can reply in a thread (the approval records the card's post ID). Each post carries `pending_post_id` (run and card id), and Mattermost returns the existing post for a repeated one within its de-duplication window (about 30 s), so a retried delivery doesn't post a card twice; a Server restart within 60 s of an approval outcome still can. A state standing at a restart (EU-A already failed, a proposal) is not posted again; the next change is | Accepted for the demo |
| The incident card queries name the two workloads | `incident-policy` and the failure and outcome cards read the `support` and `batch` rows by name (`batch` is written "summaries"); sorted lists assume at most three items | Accepted for the demo; a third workload needs those queries extended |
| Each of the 33 queries that read PostgreSQL snapshots the tables in turn at startup | Startup time grows with the current tables: about 20 seconds after a reset, minutes with a long run's history | `ops/native-runtime.py run` deletes earlier runs' rows first, and the instance's activation timeout is 20 minutes |

## Data and safety

- Never run `podman compose down -v`, `podman system prune` or broad `kill`
  commands. They delete this demo's database or affect other containers.
- Never delete `.env` while the container volumes exist; passwords are set only once.
- Stop processes by exact PID or with Ctrl-C, and never start a second copy of a
  service on the same port.
- Before a long PostgreSQL outage, stop Server first.
- Grafana allows anonymous read-only viewing and embedding (`compose.yaml`) so the
  projected stage can show dashboards without a sign-in. It listens only on
  127.0.0.1; don't expose its port beyond this machine.
- Mattermost allows framing only from the web app's origins
  (`MM_SERVICESETTINGS_FRAMEANCESTORS`) and listens only on 127.0.0.1. Its card
  buttons call the AI service through `host.containers.internal`
  (`AllowedUntrustedInternalConnections`). `/mattermost/sign-in` and
  `/mattermost/actions` have no authentication beyond the loopback bind, like
  `POST /approvals`.
