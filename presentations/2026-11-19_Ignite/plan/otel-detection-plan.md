# Deployment health from telemetry: plan

## Summary

Today the AI service's health monitor writes each deployment's status into
`ignite_reports`, and Drasi reads it through PostgreSQL. After this change the
monitor only **emits probe metrics** through OpenTelemetry. The Collector
forwards them to **Drasi's native OpenTelemetry source** (drasi-core PR 1030),
and a continuous query decides **Healthy, Failed or Unknown**. Unknown means
the deployment went silent for 8 s. Nothing about health passes through
PostgreSQL.

The decision enters the graph as rows shaped like `ignite_reports`
(`deployment-health`, label `DeploymentHealth`). The six queries that read
health today read those rows instead and are otherwise unchanged:
- the planner and what-if (`plan-bins`);
- the Decision Support tiles (`tile-deployments`, `tile-capacity`);
- the incident cards (`incident-failed`, `incident-deployments`);
- recovery verification (`recovery-observation-context`).

One setting, `IGNITE_HEALTH_SOURCE` in `demo/ports.env` (`postgres` or `otel`),
switches the AI service, the Server configuration and the Collector together.
The default is `postgres`, which is exactly today's demo.

| Piece | Ready now (committed, tested) | Waits for the merge |
|---|---|---|
| 1. Synthetic prober | Probe gauges in the AI service; `ignite_reports` writes behind the setting | — |
| 2. Collector | Overlay pipeline to Drasi, applied only with `otel` | — |
| 3. Drasi queries and generator | `deployment-health`, the rewritten health queries, generator wiring, plugin build step | Building the plugin and generating the `otel` graph against it |
| 4. Tests | Unit and in-process query tests (pass); OTLP sender; live integration test | Running the live test on the switched stack |
| 5. Docs | Drafts below | Applying them at the switch |

What remains after PR 1030 merges:
1. Rebase.
2. Build: the PR changes the native plugin-services ABI, so the Server and every
   native plugin are rebuilt together.
3. Validate the generated graph.
4. Switch the setting.
5. Run the tests.

Rollback is the same setting set back to `postgres`.

**Effort and risk for Nov 19.**
- **Effort:** about half a day after the merge, most of it building and the
  test run.
- **Risk:** medium. The prober, Collector path and query logic are proven. The
  unproven part is the native source inside the demo's generated graph:
  plugin loading, its `out` port, and a query with a non-bootstrapping source
  input. That part can only be tested once the merged plugin builds.
- **Fallback:** if the switch isn't validated by the morning of Nov 19, present
  with `postgres`.

## How it works

```text
AI service health monitor (every 1 s: GET each deployment's /health)
  → ignite_probe_status / _up / _latency_ms {deployment, run_epoch}   observable gauges (delta)
  → OTLP/HTTP → Collector: metrics/drasi pipeline (ignite_probe_* only, no batch, no retry)
  → OTLP/gRPC → host.containers.internal:$IGNITE_DRASI_OTLP_PORT
  → Drasi drasi.network/otel-source (127.0.0.1)
       (:Service {name:'ignite-workload'})-[:REPORTS]->
       (:Metric {name:'ignite_probe_status', value, observedAt, receivedAt})
       id metric:svc:ignite-workload:ignite_probe_status:deployment=EU-A,run_epoch=<uuid>
  → deployment-health: {id, run_epoch, status, observed_ms, expires_ms = observed_ms + 8000}
  → plan-bins → plan, resilience (planner, what-if)
    tile-deployments, tile-capacity → Prometheus reaction → Decision Support tiles
    incident-failed, incident-deployments → incident-channel cards
    recovery-observation-context → recovery-results
```

Design choices:

- **Observable gauges, not gauges.** In opentelemetry_sdk 0.33.1 a synchronous
  `Gauge` re-exports its last value on every collection (cumulative), so a
  silent deployment would look alive forever. An `ObservableGauge` is forced
  to delta temporality (`pipeline.rs`: `ObservableGauge ⇒ last_value(Delta)`).
  It exports only what its callback observed, and the callback observes
  readings younger than 1.5 s.
- **Identity attributes `deployment, run_epoch`.**
  - The source puts identity attributes only in the Metric's id, not in its
    properties. `deployment-health` reads them with
    `split(elementId(m), …)`.
  - Each run gets its own Metric node, so the last run's status never leaks
    into a new run. The health queries already filter
    `r.run_epoch = s.run_epoch`.
  - Metric nodes never expire. The source's `maxMetrics` is raised to 1,000,000
    and a Server restart clears them.
- **The existing meter.** The probes share the AI service's `ignite.workload`
  meter and `service.name=ignite-workload`, so there is no extra provider.
  Putting the deployment in `service.name` would need one MeterProvider per
  deployment and would still not carry the run.
- **Expiry from the probe's time.** `expires_ms` is `observedAt` (the SDK's
  collection time, within 1.5 s of the probe) plus 8 s, as `ignite_reports`
  uses the monitor's time plus 8 s.
- **Loopback listener.** The source binds `127.0.0.1`. The Collector reaches
  it through `host.containers.internal`, as Prometheus reaches the metrics
  reaction, so no token is needed.

## 1. Synthetic prober (ready)

**Changed:**
- `workloads/src/probe.rs`: new module.
- `workloads/src/telemetry.rs`: registers the probe gauges.
- `workloads/src/main.rs` (`report_once`): records a probe for every reporting
  deployment and writes `ignite_reports` only with `IGNITE_HEALTH_SOURCE=postgres`.
- `ports.env` and `ports.default.env`: `IGNITE_DRASI_OTLP_PORT` (19427 / 18427)
  and `IGNITE_HEALTH_SOURCE=postgres`.
- A missing key means `postgres`, and an invalid value stops the AI service at
  startup.

**Gauges:**
- `ignite_probe_status`: 0 ok, 1 throttled, 2 failed, 3 unknown.
- `ignite_probe_up`: 1 when ok.
- `ignite_probe_latency_ms`: the `/health` round trip.

In `postgres` mode the probes are still exported. The running Collector sends
them only to Prometheus, as new series.

**Tests:** `cargo test -p ignite-workloads --bins` (19 pass). The probe tests
collect through a real `SdkMeterProvider` and check:
- one series per deployment and run;
- a silent deployment stops being exported;
- a new reading replaces the last, and a new run is a new series.

**Possible later:** probe the deployment's request path (`/infer`) instead of
`/health`. `/infer` writes task state and uses token quota, so it needs a
dedicated probe request type.

## 2. Collector (ready)

**Changed:**
- `observability/collector-drasi.yaml`: a `metrics/drasi` pipeline.
  - **Filter:** `filter/drasi-probes` keeps only `ignite_probe_*`.
  - **Exporter:** `otlp_grpc/drasi` to
    `host.containers.internal:${env:IGNITE_DRASI_OTLP_PORT}`, with no retry or
    queue: a late probe would look fresh.
- `compose.health-otel.yaml`: passes that file as a second `--config` and sets
  the port.
- `ops/observability-stack.sh`: adds `compose.health-otel.yaml` only when
  `IGNITE_HEALTH_SOURCE=otel`.
- `compose.yaml` and `collector.yaml` are unchanged. With `postgres` the
  rendered Collector service is identical to today's.

**Tested:**
- `otelcol-contrib validate` of the merged configuration passes.
- A throwaway Collector (the demo's image and its real `collector.yaml` plus
  the overlay) forwarded probes to a source listening on loopback only:
  - Healthy (0) → Failed (2) → Unknown (−1, 8 s after the last probe) →
    Healthy;
  - a non-probe metric was filtered out.
- The source in that test was the legacy `drasi-source-otel`, whose projection
  and identities the native source keeps (PR 1030 DESIGN.md, "Differences").

## 3. Drasi queries and configuration generator (ready; the `otel` graph waits)

**Changed:**
- `drasi/deployment-health.cypher`: new.
- `ops/config/src/main.rs`:
  - `Health::from_settings()`.
  - `queries(health)`: in `otel` mode it inserts `deployment-health` (input
    `otel`, label `DeploymentHealth`). Every query that reads `ignite_reports`
    gets the label rewritten to `DeploymentHealth` and `deployment-health`
    added to its inputs.
  - `joins()`: maps `DEPLOYMENT_REPORT` and `ROUTE_REPORT` to the new label.
  - The PostgreSQL source and bootstrap stop replicating `ignite_reports`.
  - The `otel` component (`drasi.network/otel-source`) is configured by
    `otel_configuration()` and edged into `deployment-health`. It has no
    bootstrap edge, because its state is volatile.
  - With `otel` and no plugin, generation fails with "IGNITE_HEALTH_SOURCE=otel
    needs the native OTel source plugin".
- `ops/native-build.sh`: builds and installs `drasi-computation-source-otel`
  once `drasi-core/components/computation-plugins/sources/otel` exists. Until
  then it does nothing.

**Not changed:**
- The query files: the rewrite happens in the generator, so `postgres` uses
  the files byte for byte.
- The Rego policy and the optimizer: they read the same `plan-bins` and
  `policy-*` rows.

**Tested now:**
- **Generator output:** `postgres` mode is identical to today's: same graph
  after normalising generated IDs, same 2,674,863 bytes.
- **Mode tests:**
  - `health_from_postgres_is_the_graph_of_today`;
  - `health_from_telemetry_reads_probe_rows_instead_of_reports`;
  - `otel_source_configuration_uses_documented_fields`, against PR 1030
    CONFIGURATION.md.
- **Queries through drasi-core with OTel-shaped elements** (Service, Metric
  and REPORTS with the source's ids; volatile on restart):
  - `failed_card_from_telemetry_is_the_failed_card_from_reports`: the same
    cards at the same times as the `ignite_reports` test, across a restart.
  - `deployment_tile_from_telemetry_detects_failure_silence_and_recovery`:
    0 → 2 and 1 → −1 after 8 s of silence → 0, and a new run ignores the last
    run's probes.
  - `plan_bins_from_telemetry`: `failed`, `ok`, `unknown`.
- `cargo test -p ignite-config --bin ignite-config`: 18 pass.

**Waits for the merge:**
- Generating the `otel` graph against the real plugin: the factory, the `out`
  port and stream, and query construction with a source input that has no
  bootstrap port.
- Then `drasi-server validate`.

## 4. Tests (ready)

- `tests/otlp.py`: an OTLP metrics sender with hand-encoded protobuf over
  OTLP/HTTP. Drasi's source refuses OTLP/JSON, and the Collector accepts
  protobuf. It has `--self-test` (wire format) and a `probe()` helper. Its
  bytes are identical to the sender used in the end-to-end runs.
- `tests/health-telemetry-integration.py`: a live gate, which with `postgres`
  prints "not applicable" and exits 0. It checks:
  - **Baseline:** every deployment Healthy from probes, and `ignite_reports`
    not written.
  - **Collector path:** a synthetic `TEST-PROBE` sent through the Collector
    reaches `deployment-health`.
  - **Detection:** `/control fail` gives Failed, and the plan moves work off
    EU-A.
  - **Unknown:** `pause-reports` gives Unknown, not before 6 s.
  - **Recovery:** back to Healthy.
  - Evidence goes to `.native/evidence/health-telemetry.json`.
- After the switch, the existing live gates exercise the telemetry path
  unchanged:
  - `decision-integration.py`, `incident-channel-integration.py`,
    `recovery-integration.py`;
  - `native-integration.py` for restarts.

## 5. Switch-over (after PR 1030 merges)

1. **Rebase.** Rebase `agentofreality-ignite-native` in drasi-core onto the
   merged main, and drasi-server if its host SDK must follow. Expect
   conflicts only where this branch changed shared crates.
2. **Build.** `./ops/native-build.sh` builds the Server, every native plugin
   and the OTel source together. Plugin-services ABI v2 requires an exact
   host/plugin match.
3. **Unit tests.** `cargo test -p ignite-config --bin ignite-config` and
   `cargo test -p ignite-workloads --bins`.
4. **Check the `otel` graph without switching.**
   `IGNITE_HEALTH_SOURCE=otel .cargo-target/debug/ignite-config .native/plugins > /tmp/otel.json`,
   then `drasi-server validate --config /tmp/otel.json --plugins-dir .native/plugins`.
5. **Switch.** Set `IGNITE_HEALTH_SOURCE=otel` in `ports.env` and run
   `./ops/native-build.sh` (regenerates `server.json`).
6. **Restart.** Stop the Server and the AI service. Run
   `./ops/observability-stack.sh up -d --wait collector`, which recreates the
   Collector with the overlay. Then start
   `python3 ops/native-runtime.py run` and `python3 ops/workload.py run`.
7. **Live gates.** Run `python3 tests/health-telemetry-integration.py`, then
   the decision, incident-channel and recovery gates.
8. **Docs.** Apply the wording in section 7, and add the new test to
   DEVELOPERS.md's test table.

**Rollback.**
1. Set `IGNITE_HEALTH_SOURCE=postgres`.
2. Run `./ops/native-build.sh`.
3. Restart the Server and the AI service.
4. Run `./ops/observability-stack.sh up -d --wait collector`.

The `postgres` graph does not need the OTel plugin. Reset once so
`ignite_reports` starts fresh.

## 6. Risks and known differences

- **Absent rather than Unknown.**
  - At a new run, or right after a Server restart, a deployment has no
    `DeploymentHealth` row until its first probe (about 1–2 s). Today the
    reset seeds an `unknown` report instead.
  - A deployment that is silent across a Server restart stays absent: no
    status tile row, no planner bin, not listed as "not reporting". The
    planner still sends it nothing.
  - Fixing this needs the health rows anchored on `ignite_deployments`, which
    Drasi's Cypher can't express as an optional match. A later option is a
    second adapter that emits `unknown` for deployments with no probe row.
- **Volatile ingress.** The native source has no durability and no bootstrap.
  Probes every second make that harmless here.
- **Latency.** Failed appears about 2–3 s after the fault: probe, 1 s export,
  Collector, source. Today it is about 1 s. The talk's "within about 8
  seconds" still holds.
- **What is "detected".** The prober still asks each simulated deployment's
  `/health`, which reports its fault mode, as a real health check would. What
  changes is that Drasi derives Failed and Unknown from telemetry, including
  silence, rather than reading a status row.
- **No call outcomes from spans.** The native source projects only
  `DEPENDS_ON` from client spans, with no status or duration. Gateway outcomes
  could come later from `ignite_workload_requests{endpoint,status}` (a
  cumulative counter) with identity attributes `[endpoint, workload, status]`.
  `ignite_workload_last_request_failed` is not a health signal: it changes
  only when traffic reaches the endpoint and is re-exported forever.
- **Untested until the merge.** The native plugin inside the generated graph
  (see step 3). The PR's ABI change means nothing built before the rebase can
  be mixed with it.
- **Port.** `IGNITE_DRASI_OTLP_PORT` (19427) must be free on the host at the
  switch.

## 7. Docs wording (apply at the switch)

**DRASI.md, Sources table.** Add a row:

> | `otel` | `drasi.network/otel-source` | The AI service's health probes, from the Collector | OTLP/gRPC on `127.0.0.1:$IGNITE_DRASI_OTLP_PORT`. Allowlist `ignite_probe_status`; identity attributes `deployment`, `run_epoch`; no bootstrap (volatile, refreshed every second) |

**DRASI.md, published tables.** Replace "`ignite_tasks`, `ignite_attempts` and
`ignite_reports`. … Drasi sees health only through reports." with:

> `ignite_tasks` and `ignite_attempts`. Health is not in PostgreSQL: Drasi
> derives it from the health monitor's probe metrics.

**DRASI.md, query table.** Add `deployment-health` (label `DeploymentHealth`):

> Each deployment's latest probe for this run: `ok`, `throttled`, `failed`,
> from `ignite_probe_status`; expires 8 s after the probe

In `plan-bins`, `recovery-observation-context` and `incident-deployments`,
replace "latest report" with "latest probe".

**DRASI.md, simulated table, Health monitor.**

> The AI service probes each stub every second and exports the result as
> OpenTelemetry metrics (`ignite_probe_*`); the Collector forwards them to
> Drasi. The desk's faults set the stubs' mode

**DRASI.md, Worth knowing.** Add:

> - **Drasi detects failure from telemetry.** No component tells Drasi a
>   deployment is down. Probe metrics arrive through Drasi's OpenTelemetry
>   source; a continuous query turns them into Healthy, Over quota or Failed,
>   and into Unknown when a deployment goes quiet for 8 seconds, using
>   `drasi.trueLater`.

**STORY.md.** Replace "Deployment status had EU-A as Failed, from its health
checks." with:

> Deployment status had EU-A as Failed. Nobody told Drasi that: it worked it
> out from EU-A's health-check telemetry.

**PRESENTER.md, Deployment status tile.**

> Each deployment's health as Drasi derives it from its health-probe
> telemetry: Healthy, Over quota or Failed; Unknown if its probes have been
> silent for 8 seconds

**PRESENTER.md, failed card.**

> A deployment's probes have shown Failed for 3 seconds

Optionally, change the card text in `incident-failed.cypher` the same way
("has reported Failed" → "has probed Failed"). That changes the `postgres`
card too, so do it only at the switch.

**Script line (Failure beat):**

> Nobody told Drasi EU-A was down. The health checks are ordinary
> OpenTelemetry metrics; Drasi's OpenTelemetry source turns them into a live
> graph, and a continuous query decides Healthy, Failed or, when a deployment
> goes quiet, Unknown.

**DEVELOPERS.md, tests table.**

> | `python3 tests/health-telemetry-integration.py` | Health from telemetry: detection, Unknown on silence, recovery, the Collector path (applies with `IGNITE_HEALTH_SOURCE=otel`) | Yes |
>
> | `python3 tests/otlp.py --self-test` | The test OTLP sender's protobuf encoding | No |
