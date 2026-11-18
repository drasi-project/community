# Moving the demo to the merged drasi-core (PR 1030)

## Summary

The demo runs today on `drasi-core` and `drasi-server` branch
`agentofreality-ignite-native`. The next version builds from three sibling
checkouts in `ignite-native/`, each on branch `agentofreality-ignite-native-next`:

| Checkout | Role | Local commits on top of origin |
|---|---|---|
| `drasi-core-next` | Merged drasi-core (PR 1030, plugin-services ABI 2) | `0e0e3ed4` HTTP sink `unroutedChanges: ignore` |
| `drasi-server-next` | Drasi Server adapted to it (`0e039e5`) | `a4eed95` builds against `../drasi-core-next` |
| `community-next` | This demo (linked worktree of `community`) | the demo changes below |

Builds go to `ignite-native/.cargo-target-next`, so the running demo's build folder is never touched.

| Area | State |
|---|---|
| Drasi Server binary | Builds (`cargo build -p drasi-server`) |
| Demo plugins (`ignite-prometheus`, `ignite-optimizer`) | Build and pass their tests. Drasi's `drasi.standard/policy` replaced `ignite-policy`; the optimizer is being replaced by the official plugin (another workstream) |
| Config generator (`ignite-config`) | Adapted. Its tests pass, and `drasi-server validate` accepts the current graph: 57 components, 9 plugins |
| Build and run scripts | Point at the `-next` checkouts and build folder |
| Live switch | Not done. It needs the user's go-ahead (see [Switch-over](#switch-over)) |

Two stacks can't run at once (Podman memory), so the live switch is one short,
reversible window: stop the native processes, start the same processes from
`community-next` on the same ports, then verify.

## What changed

### drasi-core-next

- **`unroutedChanges: ignore`** (HTTP sink): the demo's d34892e7 re-applied to
  the new sink. Changing it is a Pause-boundary update. Tests: `cargo test -p
  drasi-computation-sink-http`, 40 passed. This commit isn't upstream yet.
- **Native queries with bootstrap edges:** the demo's ef341573 doesn't need porting.
  drasi-core-next has it as `edge_bootstrap`.

### drasi-server-next

Origin `0e039e5` already does what the demo needs:

- Reads the plugin ABI family and version from the new plugin metadata.
- `sourceProgress` names its query's `IndexBackend` (`indexes`) and depends on it.
- Includes the `memoryIndexes` volatility fix (drasi-server#243).

### community-next

| Commit | Change |
|---|---|
| `d471d69` | Plugin and `ops/config` path dependencies point at `../drasi-core-next` |
| `4bafcc8` | `ignite-prometheus`: `DescribeRequest::unkeyed_sink` now takes the permanent-failure policy (`Stall`) |
| `4cf30b5` | `ignite-prometheus`: golden text test refreshed for the metric renames in 68b457e (this failure predates the migration) |
| `395b8b3`, `c45c814` | HTTP reactions: `failurePolicy` is now `onPermanentFailure`. `strict` → `stall` (gateway), `skip` → `skip`. Docs updated |
| `2fd85ab` | `sourceProgress` recipes add `"indexes":"indexes"` and an `IndexBackend` dependency on it |
| `c9bc740` | `ops/settings.py` defines the checkouts (`IGNITE_DRASI_CORE`, `IGNITE_DRASI_SERVER`, defaults `drasi-core-next`, `drasi-server-next`) and the build folder (`.cargo-target-next`). `native-build.sh`, `native-runtime.py`, `release.py` and `.cargo/config.toml` use them |

### Behavior differences to know

- **Gateway reaction failures.** Under `stall`, a permanent failure records a
  terminal failure on the sink, and the sink delivers nothing more until an
  operator acts. Permanent means any 4xx except 408/409/425/429. The gateway
  answers 400 or 404 to a bad apply, so such an apply now stops the gateway
  reaction instead of retrying it indefinitely. Either way the change never
  lands. Transient failures, such as an outage, are still retried in place.
- **Source progress inherits index durability.** The demo uses `memoryIndexes`,
  so progress stays volatile, as before.

## Checklist before the switch

| # | Item | Owner | State |
|---|---|---|---|
| 1 | `drasi-core-next`: HTTP sink `unroutedChanges` | this workstream | Done |
| 2 | Demo plugins and `ignite-config` build and test against drasi-core-next | this workstream | Done |
| 3 | Drasi Server builds from `drasi-server-next` | this workstream | Done |
| 4 | Generated graph passes `drasi-server validate` | this workstream | Done (Postgres health source) |
| 5 | Official policy plugin replaces `ignite-policy` (generator, queries, `native-build.sh`) | policy workstream | Done: `7d0cd2d`. `native-build.sh` builds `drasi-computation-transformer-policy` and removes the old `ignite_policy` library |
| 6 | Official optimization plugin replaces `ignite-optimizer` | optimization workstream | In progress |
| 7 | Native OTel source path (`IGNITE_HEALTH_SOURCE=otel`): the generator's `REPORTS` join fix (`fc035a1`), then validate | OTel workstream | In progress |
| 8 | Full `ops/native-build.sh` in community-next: UI, Server, all plugins, generator, `validate`, `stamp` | this workstream | Re-run after items 5–7 |
| 9 | Re-run every crate's tests: `cargo test -p ignite-prometheus`, `-p ignite-config`, `-p ignite-workloads`, plus the replacement plugins' | all | After items 5–7 |
| 10 | `community-next/.../demo/.env` and `.native/mattermost.json` copied from the running demo (both ignored by git) | switch operator | `mattermost.json` copied. Copy `.env` in the window's preparation |
| 11 | Presenter UI: `npm ci && npm run build` in `community-next/.../demo/ui` | switch operator | Not done |
| 12 | DEVELOPERS.md and SETUP.md: branch names, `.cargo-target-next`, the dependency table | this workstream | After the switch is accepted |
| 13 | Upstream: propose `unroutedChanges` to drasi-core, so the local commit can go | later | Not proposed |

## Switch-over

The next stack replaces the running one on the same ports: both trees use the
same `ports.env`, including `IGNITE_PROJECT=ignite-native`. Drasi Server keeps
no state on disk (`memoryIndexes`), so it rebuilds every query from PostgreSQL
on start in either direction. The containers (PostgreSQL, Grafana, Mattermost,
Prometheus, Collector, Langfuse) keep running and keep their volumes.

### Prepare (no downtime)

1. Checklist items 5–11 are done, and `ops/native-build.sh` in `community-next`
   finishes with `validate` OK.
2. Compare what the containers mount:
   `git -C community-next diff agentofreality-ignite-native -- presentations/2026-11-19_Ignite/demo/observability presentations/2026-11-19_Ignite/demo/compose.yaml presentations/2026-11-19_Ignite/demo/compose.health-otel.yaml`.
   If that prints anything, step 3 of the window recreates the containers.
   Otherwise they stay as they are.
3. Compare the schema: `git -C community-next diff agentofreality-ignite-native -- presentations/2026-11-19_Ignite/demo/database`.
   If that prints anything, step 4 of the window applies the new schema.

### Window (a few minutes, after the user's go-ahead)

In `community/presentations/2026-11-19_Ignite/demo` (running):

1. Stop the presenter UI (terminal 3), the AI service (terminal 2) and Drasi
   Server (terminal 1) with Ctrl-C.

In `community-next/presentations/2026-11-19_Ignite/demo`:

2. Load the environment in each terminal: `set -a; . ./.env; set +a`.
3. Only if prepare step 2 showed changes: `./ops/observability-stack.sh up -d --wait`.
   Same project name, so the containers are recreated in place: the volumes
   stay and the mounts move to this folder.
4. Only if prepare step 3 showed changes: `python3 ops/native-runtime.py migrate`.
5. Terminal 1: `python3 ops/native-runtime.py run`. Terminal 2: `python3 ops/workload.py run`.
   Terminal 3: `cd ui && npm run preview`.

### Verify

- The presenter desk header reports ready, and
  `curl -s 127.0.0.1:19423/metrics | grep drasi_reaction_input_ready` shows 1 for
  every output.
- `curl -s -o /dev/null -w '%{http_code}' 127.0.0.1:19421/health` returns 200.
- `python3 tests/native-integration.py` and `python3 tests/recovery-lifecycle.py`
  (the approve, apply and verify path through the gateway reaction) pass.
- Grafana's Operations dashboard and `#incident-ai-support` update on a
  scenario change.

### Roll back

Stop the three processes in `community-next`. If the window recreated the
containers, run `./ops/observability-stack.sh up -d --wait` from
`community/presentations/2026-11-19_Ignite/demo`. Then start the three
processes there as before. The running demo's checkouts, binaries
(`.cargo-target`) and `.native/` were never changed. If step 4 applied a
schema change, roll back only if the old service can't run on the new schema.
