# Spike: legacy OTel source in the native computation graph

Question: can `drasi-source-otel` (legacy plugin) feed native continuous queries in the
Server that runs the Ignite demo, so deployment health is detected from telemetry rather than
from the stub's fault mode written to PostgreSQL?

## Summary

| # | Question | Answer |
|---|----------|--------|
| 1a | Server loads the legacy OTel plugin dynamically | **Yes**, proven |
| 1b | Legacy source runs inside the computation graph, and native queries consume it | **Yes, for ordinary `queries:`**, proven |
| 1c | A query can take OTel *and* the demo's native PostgreSQL source as inputs | **Not proven.** Supported in code for ordinary queries; the demo's graph-defined queries do not bind legacy sources |
| 2 | Bootstrap / restart | Queries start **empty** (no bootstrap). After a restart, all Metric nodes are **lost** until each series reports again |
| 3 | Working proof: value change + `trueLater` silence | **Yes**, proven end-to-end |
| 4 | Collector → host gRPC via `host.containers.internal` | **Yes**, proven with a throwaway Collector on the same image. The demo Collector needs a config change and a restart |
| 5 | Blockers for Nov 19 | No hard blocker for an OTel-only health query. The fragile parts are wiring it into the demo's graph queries and the data model gaps below |

**Recommendation:** use the legacy source **as is** for an OTel-only, Server-side health
query (ordinary `queries:` entry). Feed it from a dedicated probe gauge whose
`service.name` is the deployment. Do **not** try to join it into the demo's graph-defined
queries before Nov 19. A native OTLP source is the right long-term answer, not a two-day job.

## Evidence

Spike setup: throwaway Server on 19721 (OTLP gRPC 19717, OTLP/HTTP 19718), own config under
`/tmp/otel-spike`. The binary is the existing `.cargo-target/debug/drasi-server`. Plugins are
copied, not built (see Risks: disk). The demo stack was not touched; its Server still
returned 200 after the spike.

- OTel plugin: prebuilt `libdrasi_source_otel.dylib` copied from
  `computation-graph/drasi-server/target/debug/plugins` (built Nov 16). It loads against this
  Server: `[cdylib] source: otel (sdk=0.19.0 core=0.1.3 plugin=0.1.3)`. A build from this
  branch would be needed for the demo.
- Query plugin: the demo's `libdrasi_computation_transformer_query.dylib`.

### 1. Loading and wiring

- Instance config accepts legacy `sources:`, ordinary `queries:` and `computation:` side by
  side (`drasi-server/src/config/types.rs:330-347`). All of them are built into one DrasiLib
  core (`src/server.rs:579-692`). "ComputationGraph is the only runtime."
- Each legacy source becomes a graph record with a borrowed `LegacySource` resource
  `source-host/<id>`, bound to `SourcePluginHost`
  (`drasi-core/lib/src/computation/runtime/managed.rs:585-597`, `runtime/source.rs`).
- Ordinary queries are hosted as native `drasi.query/continuous` components. Their legacy
  inputs are subscribed automatically. Log line: `Query '…' subscribing to OTel source '…'`.
- Config (YAML; the generator would emit the same JSON):

  ```yaml
  sources:
    - {kind: otel, id: otel, autoStart: true, grpcBind: "0.0.0.0:19717",
       metricAllowlist: [ignite_probe_up], metricIdentityAttributes: [endpoint]}
  queries:
    - name: health
      queryLanguage: Cypher        # default is GQL; Cypher functions are missing under GQL
      inputs: [otel]
      query: |
        MATCH (s:Service)-[:REPORTS]->(m:Metric {name: 'ignite_probe_up'})
        RETURN s.name AS deployment, m.value AS up,
               CASE WHEN m.value < 1 THEN 'Failed' ELSE 'Healthy' END AS status
  ```

- **Mixing with the demo's PostgreSQL source.** The legacy-input binding in
  `runtime/mod.rs:616-700` only applies to queries configured with `query_config`
  (ordinary queries). The demo generator (`demo/ops/config/src/main.rs`) emits graph queries
  with `query` + `execution` and wires inputs as graph relationships. Graph `connections`
  resolve only among graph components (`drasi-server/src/computation/authoring.rs:721-740`).
  So a demo graph query cannot name the OTel source as an input today.

  I attempted an ordinary query with `inputs: [otel, postgres]` over a pruned copy of the
  demo graph with a throwaway Postgres. Results:
  - The config validated.
  - With the OTel input, Server startup hung.
  - Without it (PG-only control), the query stayed `Stopped` and the slot never went active.
  - The hand-pruned graph is likely missing generator-wired pieces (bootstrap provider,
    consumer progress). This is unresolved inside the time box.
- Also hit: graph authoring (`computation.graph`) wraps all config as `Literal`, so it cannot
  express the PG source's secret `password`. Only the canonical `definition` form can.

### 2. Bootstrap and restart

- The OTel source has no bootstrap provider (`src/lib.rs:149`, "There is no bootstrap
  snapshot"). Log line: `Bootstrap requested … but no bootstrap provider configured for OTel source`.
- Observed: queries return `[]` at start. After a Server restart, all three queries were
  `[]`. A deployment reappeared only when its next point arrived.
- Consequence: a deployment that never reports after a restart is **absent**, not "Unknown".
  "Unknown" requires an expected-deployments list joined in, which hits the mixing problem
  in 1c. The optional `durability` WAL replays projected changes, but was not tested.
- With the SDK's 1 s periodic export, normal series repopulate within about 1 s of restart.

### 3. Proof run (HTTP sender, hand-encoded OTLP protobuf, stdlib only)

```text
20:50:21 start                       health=[]  silent=[]
20:50:26 eu-a=1, us-b=1              health: eu-a Healthy, us-b Healthy
20:50:32 eu-a=0                      health: eu-a Failed (value 0.0), us-b Healthy
20:50:51 eu-a silent ~20s, us-b 2s   silent: [{deployment: eu-a, lastSeen: 04:50:29.813}]
20:50:55 eu-a=1                      health: eu-a Healthy; silent: []
```

Silence query (fires 10 s after the last point and retracts on the next one):

```cypher
MATCH (s:Service)-[:REPORTS]->(m:Metric {name: 'ignite_probe_up'})
WHERE drasi.trueLater(
        datetime(m.receivedAt) <= datetime.realtime() - duration({seconds: 10}),
        datetime(m.receivedAt) + duration({seconds: 10}))
RETURN s.name AS deployment, m.receivedAt AS lastSeen
```

Also confirmed:
- The allowlist drops other metrics.
- `elementId(m)` and `split(elementId(m), 'endpoint=')[1]` work under Cypher; they returned
  `eu-a`.

### 4. Collector to host

- Ran a throwaway `otel/opentelemetry-collector-contrib:0.165.0`, the demo's image, with an
  `otlp_grpc` exporter to `host.containers.internal:19717` (`tls.insecure: true`).
- Sent OTLP/HTTP to it and got: `health: eu-a Failed, us-b Healthy`.
- The Server bound `0.0.0.0:19717` for this. Loopback-only was not tested.
- The demo Collector has `otlp_grpc`, `groupbyattrs`, `transform` and `resource`.
- Integration needs these edits to `observability/collector.yaml`, then a Collector restart:
  - add the exporter;
  - add it to the `metrics` pipeline;
  - optionally add a `filter` for the allowlisted names.
- Podman on macOS does not mount `/tmp`, so mounted files must live under `/Users`.

### 5. Gaps and risks

- **Identity attributes are not properties.** `metricIdentityAttributes` only suffixes the
  element id (`metric:svc:<service>:<name>:endpoint=eu-a`). Read it with `elementId`/`split`,
  or put the deployment in `service.name`.
- **Unlisted attributes collapse series.** The gateway gauge carries `{workload, endpoint}`.
  With identity `[endpoint]`, `sales=0` overwrote `support=1` on eu-a. Use
  `[endpoint, workload]`.
- **The gateway gauge is not a health probe.** `ignite_workload_last_request_failed` only
  changes when a request is routed to that endpoint. The cumulative SDK re-exports the last
  value every second, so after failover EU-A stays "failed" with fresh `receivedAt`. Silence
  never triggers and recovery is never seen. A **synthetic per-deployment probe gauge** is
  needed for honest Failed/Healthy/Unknown.
- **Counters** (`ignite_workload_requests`) arrive as cumulative values. Rates need
  `drasi.previousValue` maths; this was not tested.
- **Histograms** are rejected (`mapping.rs:48`), so `ignite_workload_call_duration` and
  `…_answer` are unusable.
- **Traces:** only CLIENT spans make `DEPENDS_ON`. The gateway's spans are
  `SpanKind::Internal` (`workloads/src/telemetry.rs`), so they produce nothing. There is no
  per-span status anyway.
- **OTLP/HTTP is protobuf-only.** JSON is not supported. gRPC from the SDK or Collector is fine.
- **Metric nodes never expire.** A deployment removed from the scenario keeps its node until
  restart.
- **Build and disk:**
  - No OTel dylib from this branch exists; the proof used a Nov 16 build from another
    checkout.
  - Building one needs `cargo build -p drasi-source-otel --features dynamic-plugin`. It was
    not attempted: free disk was already 18 GiB (now 13 GiB), under the 40 GB rule.
- **Query language default is GQL.** Always set `queryLanguage: Cypher`.

## Integration effort (demo)

| Option | Work | Effort | Risk for Nov 19 |
|--------|------|--------|-----------------|
| A. OTel-only health query (recommended) | Build the plugin from this branch; add `sources` + one ordinary query to the generator; emit a probe gauge per deployment (`service.name` = deployment) from the probe container; Collector exporter + restart; UI reads the new query | ~0.5–1 day | Low–medium. Restart shows "absent", not "Unknown" |
| B. Feed OTel into the existing graph queries (tiles, incidents) | Make the generator emit `query_config`-style queries, or extend drasi-lib so graph queries bind legacy inputs; fix the mixed-input startup hang | 2–4 days, unknown | High |
| C. Native OTLP source plugin | New computation-plugin source with bootstrap/retained state, identity attrs as properties, metric TTL | 1–2 weeks | Not for Nov 19 |
