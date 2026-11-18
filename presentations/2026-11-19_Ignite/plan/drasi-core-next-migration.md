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
| Demo plugin (`ignite-prometheus`) | Builds and passes its tests. Drasi's `drasi.standard/policy` and `drasi.standard/optimization` replace `ignite-policy` and `ignite-optimizer` |
| Config generator (`ignite-config`) | Adapted. Its tests pass, and `drasi-server validate` accepts the graph with 9 plugins: 57 components (PostgreSQL health) or 59 (OTel health) |
| Build and run scripts | Point at the `-next` checkouts and build folder. `ops/native-build.sh` runs end to end, including `validate` and `stamp` |
| Live switch | Done 2026-11-18 06:09–06:13 PST, with health from telemetry (`IGNITE_HEALTH_SOURCE=otel`). See [Switch record](#switch-record) |

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
| 4 | Generated graph passes `drasi-server validate` | this workstream | Done: both health sources |
| 5 | Official policy plugin replaces `ignite-policy` (generator, queries, `native-build.sh`) | policy workstream | Done: `7d0cd2d`. `native-build.sh` builds `drasi-computation-transformer-policy` and removes the old `ignite_policy` library |
| 6 | Official optimization plugin replaces `ignite-optimizer` | optimization workstream | Done: `e3d5fbe`, `bbe44f0` |
| 7 | Native OTel source path (`IGNITE_HEALTH_SOURCE=otel`): the generator's `REPORTS` join fix (`fc035a1`), then validate | OTel workstream | Done: validates with 58 components |
| 8 | Full `ops/native-build.sh` in community-next: UI, Server, all plugins, generator, `validate`, `stamp` | this workstream | Done at `eab949d`. Re-run after any later change |
| 9 | Re-run every crate's tests: `cargo test -p ignite-prometheus`, `-p ignite-config`, `-p ignite-workloads`, plus the replacement plugins' | all | Done at `e3d5fbe`: 15, 20 and 19 passed (1 ignored) |
| 10 | `community-next/.../demo/.env` and `.native/mattermost.json` copied from the running demo (both ignored by git) | switch operator | Done. Copy them again if the running demo's files change |
| 11 | Presenter UI: `npm ci && npm run build` in `community-next/.../demo/ui` | switch operator | Done at `e3d5fbe` |
| 11a | Git-ignored assets the demo needs: the slide template (`demo/ui/public/template`, from `python3 ops/template-assets.py` or copied from the running demo; the web app serves that folder directly, with no rebuild) and the probe image's wheels (`demo/observability/probe/wheels`) | switch operator | Missed at the 2026-11-18 switch, so the deck lost its styling. Copied in afterwards |
| 12 | DEVELOPERS.md and SETUP.md: branch names, `.cargo-target-next`, the dependency table | this workstream | After the switch is accepted |
| 13 | Upstream: propose `unroutedChanges` to drasi-core, so the local commit can go | later | Not proposed |

## Switch-over

The next stack replaces the running one on the same ports: both trees use the
same `ports.env`, including `IGNITE_PROJECT=ignite-native`. Drasi Server keeps
no state on disk (`memoryIndexes`), so it rebuilds every query from PostgreSQL
on start in either direction. The containers (PostgreSQL, Grafana, Mattermost,
Prometheus, Collector, Langfuse) keep running and keep their volumes.

### Prepare (no downtime)

1. Checklist items 5–11a are done, and `ops/native-build.sh` in `community-next`
   finishes with `validate` OK.
   - Check the git-ignored files too: `git ls-files -o -i --exclude-standard --directory`
     in both demo folders must list the same runtime inputs. That is `.env`,
     `.native/mattermost.json`, `ui/public/template` (run `ops/template-assets.py`
     or copy it) and `observability/probe/wheels`.
   - Test evidence folders aren't needed.
2. Compare what the containers mount:
   `git -C community-next diff agentofreality-ignite-native -- presentations/2026-11-19_Ignite/demo/observability presentations/2026-11-19_Ignite/demo/compose.yaml presentations/2026-11-19_Ignite/demo/compose.health-otel.yaml`.
   If that prints anything, step 3 of the window recreates the containers.
   Otherwise they stay as they are. At `e3d5fbe` it prints nothing.
3. Compare the schema: `git -C community-next diff agentofreality-ignite-native -- presentations/2026-11-19_Ignite/demo/database`.
   If that prints anything, step 4 of the window applies the new schema. At
   `e3d5fbe` it prints nothing.

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

## Switch record

On 2026-11-18 the live demo moved to `community-next` (`1861cfe`) with health
from telemetry.

- **Stopped**, from `community` (binaries in `.cargo-target`): Server 57841,
  AI service 81845, UI 86617 (npm 86592).
- **Collector:** recreated alone with the drasi overlay
  (`./ops/observability-stack.sh up -d --wait --no-deps collector`, otel mode).
  Every other container kept running.
- **Grafana:** recreated from `community-next`
  (`./ops/observability-stack.sh up -d --wait --no-deps grafana`) when the
  dashboard changed. Until then it kept mounting the old folder's dashboards and
  alert provisioning: a container mounts files from the folder it was created
  from. Prometheus, Mattermost and the other containers still mount `community`
  files, which are identical on both branches. Recreate them from `community-next`
  if a change touches their files.
- **Started** detached from `community-next` (binaries in `.cargo-target-next`):
  Server 53843, AI service 53853, UI 53931. Logs are in `.native/`. The API
  answered 200 about 2 minutes after start.
- **Verified:**
  - all 12 `drasi_reaction_input_ready` series are 1;
  - `health-telemetry-integration.py` passes: Failed in 0.9 s, Unknown after
    8.9 s, Healthy in 1.1 s, and `ignite_reports` not written;
  - `incident-channel-integration.py`, up to Verified: failure card, proposal,
    Out of date, rejection, approval, Applied, Verified;
  - the desk (`/presenter`, live SSE), Grafana and the Prometheus scrape.

**Open:** `incident-channel-integration.py` fails its last check. After EU-A
recovers, a "Failover restored" card is posted about 4 s later, before the
test's at-risk step. That means the failover state passed through `at_risk`
between `degraded` and `safe`. A plain fail and restore (no policy edits, no
approval) goes from degraded straight to safe. This is for the
optimization workstream (`resilience-results`, `e3d5fbe`).

**Roll back:**
1. Stop 53843, 53853 and 53931.
2. From `community/.../demo`, run
   `./ops/observability-stack.sh up -d --wait --no-deps collector`, which
   recreates the Collector in postgres mode.
3. Start the three processes there, as in [Switch-over](#switch-over).
4. Run `python3 ops/workload.py reset`, so the AI service writes
   `ignite_reports` again.
