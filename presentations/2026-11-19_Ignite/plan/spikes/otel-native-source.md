# Spike: native OTel source (drasi-core PR 1030)

Question: will the new native OTel source (`drasi.network/otel-source`, PR 1030, head
`cde3da20`) let Drasi detect deployment health (Failed / Healthy / Unknown) from telemetry
and feed it into the planner and Decision Support?

Status: **blocked at step 2 (build).** Awaiting a decision on how to proceed.

## Summary

- **What it projects:** the same graph and identities as the legacy source:
  - `(:Service)-[:REPORTS]->(:Metric)`;
  - `DEPENDS_ON` from client spans, with TTL;
  - `LogEvent` from error logs, with TTL;
  - `Heartbeat`.

  It has **no per-span nodes, span status or duration**. So "real call outcomes from spans"
  is not available from this source in any form.
- **Main gain over legacy:** it is a native graph component. The demo generator can wire it
  with ordinary relationships next to the native PostgreSQL source, so graph queries can join
  OTel and PG nodes. This is the gap the legacy spike could not close.
- **Build: blocked.** The plugin needs shared-crate and ABI changes in PR 1030 that our branch
  lacks:
  - Most importantly, the optional plugin-services ABI goes from **v1 to v2**, with an exact
    match required.
  - A plugin built on PR 1030 is refused by our Server, and our demo's native plugins would
    be refused by a PR 1030 Server.

## Step 1: what the source does (from README, DESIGN.md, CONFIGURATION.md at `pr-1030`)

| Aspect | Behaviour |
|---|---|
| Metrics | Gauge and sum only (histograms rejected). `metricAllowlist` and `metricIdentityAttributes`. Identity attributes only suffix the element id (`metric:{svc}:{name}:k=v`); they are **not properties** |
| Spans | Only spans of `spanKinds` (default `CLIENT`) naming a callee via `destinationAttributes` produce `DEPENDS_ON {lastSeen}`. No span node, status or duration |
| Logs | `LogEvent {service, severity, body, eventName?, observedAt}` at or above `logMinSeverity`, with TTL |
| Heartbeat | `Heartbeat {lastSeen}` from a metric or log event |
| Expiry | `DEPENDS_ON` and `LogEvent` only. **Service, Metric and Heartbeat never expire** |
| Bootstrap / restart | "Ingress is volatile; there is no durable admission, replay or bootstrap." State survives Stop/Start of the same object, but **not reconstruction or restart**. The legacy `durability` WAL is removed |
| Ports | One `out` port (graph changes, volatile producer progress). No bootstrap output |
| Transport | gRPC and/or HTTP protobuf (gzip accepted; no OTLP/JSON). Binds loopback by default; non-loopback needs `authToken` or `allowUnauthenticated: true` |

## Step 2: build attempt

The plugin directory and workspace member were copied onto `agentofreality-ignite-native`.
Then:

```text
$ cargo build -p drasi-computation-source-otel --features dynamic-plugin
package `drasi-computation-source-otel` depends on `drasi-computation-network-common` with
feature `grpc-listener` but `drasi-computation-network-common` does not have that feature.
```

The trial was reverted, and the drasi-core tree is clean.

What is missing on our branch:

- **network-common:**
  - the `grpc-listener` feature;
  - the `grpc_incoming` module (moved from the gRPC source);
  - `Ingress::admit_all` (all-or-nothing group admission, about 300 lines in `ingress.rs`).
- **plugin-sdk:** `ConfigOption::name()` (trivial).
- **ABI (`components/computation-plugin-abi/src/services.rs`):** `VERSION: 1 → 2`. ADMIT now
  carries `wait`, and "Hosts and plugins require an exact match". `admit_all` and the
  back-pressure path depend on it.
- The OTel commit's shared changes do **not** apply cleanly to our branch: `git apply --check`
  fails on `ingress.rs`, the gRPC source and network README. They build on the PR's earlier
  commits (recovery-branch fixes, `1b000ce6`, `fb3bcd31`, …). Divergence `HEAD..pr-1030`:
  - plugin-sdk, contracts and host-sdk: 41 files, +2.8k/−0.5k;
  - network-common: +0.8k.

Consequence: using the native source means the Server **and every demo native plugin**
(PostgreSQL source and bootstrap, query, sinks, Ignite policy/optimizer/prometheus) must be
rebuilt against PR 1030's drasi-core.

## Options

1. **Separate checkout of PR 1030 plus a matching Server.** Work in a separate folder with
   drasi-core at `pr-1030` and drasi-server beside it; build the Server and all demo plugins
   there; test on spare ports.
   - Unknowns: whether drasi-server's branch and the Ignite plugins compile against PR
     1030's host-sdk changes, and roughly 1–2 h of build time.
   - Disk cost: a second full target dir, about 10–20 GB.
   - Needs a worktree or clone, which this folder normally avoids.
2. **Merge PR 1030 into `agentofreality-ignite-native`.** Real test in place, but it changes
   the branch the demo runs from, two days before the talk. The rebuild would also force a
   demo restart.
3. **Wait for the merge**, then rebase the demo branch and test.
4. **Use the legacy source now.** Its projection and identities are the same, so a later
   switch is mostly config. The legacy source cannot join into the demo's graph-defined
   queries (see `otel-legacy-source.md`).

## Step 4 (provisional, from the docs)

- **Probe-based Failed / Healthy:** yes, as with the legacy source.
- **Joining with PostgreSQL in the demo's graph shape:** expected yes, because it is a native
  graph component. Not tested.
- **Unknown via `trueLater` on `receivedAt`:** yes. After a Server restart a silent
  deployment is absent rather than Unknown, unless the query starts from PG deployments
  joined to Metric.
- **Real call outcomes from spans:** no, this source does not project span status. Use a
  gauge or counter (`ignite_workload_requests{status}` or a per-endpoint failure gauge)
  instead.
