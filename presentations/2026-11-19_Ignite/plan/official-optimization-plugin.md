# Replacing the local optimizer plugin with `drasi.standard/optimization`

## Summary

**Yes, with general changes.** One `drasi.standard/optimization` component
replaces both local factories (`ignite/assignment-plan` and
`ignite/assignment-resilience`). Planning, the "remove each healthy deployment"
what-if, and the never-guess behaviour are all configuration plus queries.

- **Model.** Expressed entirely in the official model language: binary assignment
  over allowed candidates, exactly-one, capacity, a peak-use variable, and the
  lexicographic objective (fewest moves, then lowest peak). Eligibility comes from
  `policy-results` (unchanged columns) and health from `plan-bins`.
- **What-if.** A `loss` scenario re-solves without each healthy deployment. The
  status row now carries a roll-up, so the first loss that leaves no plan is
  `head(scenarios.loss.failures)`.
- **Unknown.** New `preconditions`: if health is unknown, a policy decision is
  missing or undecided, an input has not loaded, or inputs disagree on the
  revision, the status is `unknown`. No plan and no scenarios are published.
- **Approvals.** `basis` = `signature` (the inputs solved) and
  `candidate_id` = new `solutionSignature` (the published solution). Both are
  stable across input order and change when the facts or the plan change.
- **Downstream.** `plan-results`, `assignment-results`, `resilience-results` and
  `loss-results` are rewritten over the new rows with **the same columns**
  (except `LossScenario.assignments`, unused). Tiles, approval checks and incident
  cards need no change.
- **Proof.** Seven SDK-harness tests in drasi-core-next reproduce every required
  outcome. The rewritten queries were evaluated with drasi-core's query engine
  against the plugin's status rows.
- **Effort.** About half a day: four query files, two new queries, one component
  configuration, generator wiring, and deleting `plugins/optimizer`. This is
  blocked only on Server loading the merged plugins (in progress).

For the talk: the plan, the what-if and "unknown, never a guess" are now shown as
standard Drasi configuration rather than demo code.

## Mapping

| Local plugin (contracts §5, §7) | Official plugin |
|---|---|
| Two factories, ports `items`, `bins`, `eligibility` | One component, port `in`. Sets select rows by result label: `PlanContext`, `PlanItem`, `PlanBin`, `PlanCandidate`, `PolicyResult` |
| Edge item→bin when allowed, healthy and demand ≤ capacity | `pairs` = `PlanCandidate` rows `inSet` allowed decisions, `inSet` healthy bins, `capacity_tpm >= demand_tpm`. The pairs come from a candidates query, because the plugin has no join by design |
| `assignEachItemOnce`, `binCapacity` | Constraints `assign-once` (forEach items, `== 1`) and `capacity` (forEach healthy bins) |
| `fewestMoves`, then `lowestPeakUtilisation` | Objectives `moves` (assign where `target != current`), then `peak` (continuous `peak`, with `demand·x − peak·capacity ≤ 0` per bin) |
| Deterministic ties | Built in: sets are materialized in key order |
| `Plan.state` feasible / infeasible / unknown | Status `optimal` → feasible, `infeasible` → infeasible, `unknown`/`limit`/`error` → unknown (in `plan-results`) |
| `basis`, `candidate_id` | `signature`, `solutionSignature` |
| `revision`, `run_epoch` | `provenance` from the `context` set (one `ignite_scenario` row) |
| `assignments` JSON, `moves`, `peak_utilisation`, `reason` | `summaries.assignments`, `summaries.moved`, `objectives.moves`, `objectives.peak`, `detail`; text built in `plan-results` |
| `solved_ms` | `drasi.changeDateTime(p).epochMillis`: the effective time of the triggering input |
| `Resilience`: state, first_failure, baseline, unhealthy_bins | Status-row `scenarios.loss` (`values`, `feasible`, `failures[{value,status,detail}]`) and `setSizes.unhealthy` |
| `LossScenario` per healthy bin | Scenario rows (`scenarioLabel: LossScenario`, id `scenario/loss/<bin>`), with `feasible` and `detail`. Feasibility only, so no per-loss assignments |
| Unknown on: health outside the lists, missing or unknown eligibility, revision/run disagreement, not loaded | Preconditions `health-known`, `policy-known`, `policy-complete` (every candidate has a decision; stricter than today), `same-revision`, `same-run`, `same-run-policy`, `context-loaded`, `items-loaded`, `bins-loaded`, `candidates-loaded` |
| Timeout → unknown | `timeLimitMs` → status `limit` → unknown |
| `coalesceMs` 100 | `debounceMs` 100, `maxDelayMs` 1000 |
| `failed` ↔ `throttled` leaves basis unchanged | `plan-bins` projects `health` (healthy / unhealthy / unknown), so the signature never sees the raw status |
| Previous answer stays until the new solve finishes | Same: rows are replaced only when a solve completes, and the roll-up stays until the next analysis |

**Behaviour differences (all acceptable for the demo):**

- **Stricter unknown.** A candidate pair without a policy decision is now
  `unknown`, not silently ignored.
- **Infeasible wording.** Text comes from the plugin's `detail`. When an item has
  no option, it reads "No allowed, healthy destination with room for support".
  The local "needs N, more than any … can take" variant is folded into this.
- **Loss scenarios are feasibility-only.** `loss-results` no longer has
  `assignments`, which only the integration test's evidence dump captured.

## Upstream changes (drasi-core-next, `agentofreality-ignite-native-next`)

Commits `b282af95`, `6b663dd0` and `26267f08`, only in
`components/computation-plugins/transformers/optimization`. All tests pass:
27 (18 existing and 9 new), clippy `-D warnings` clean, `--features
dynamic-plugin` builds. The README documents each change.

| Change | Why it is general |
|---|---|
| **`preconditions`** `{name, set, where?, min?, max?}`, and status **`unknown`** with `precondition` on the status row | Any continuously re-solved model can see partial or untrusted inputs: during bootstrap, from stale telemetry, or with an undecided policy. Declaring "do not answer unless…" avoids publishing a confident plan from incomplete facts. Without it, consumers must re-derive completeness in separate queries that race the solver. |
| **`solutionSignature`** on the status row | Approve-then-apply, change tickets and idempotent actuation need a stable identity for the exact published solution, alongside `signature` for the inputs. |
| **`summaries`** `{name, variable, where?, properties?}`: solution lists on the status row | Solution rows arrive as separate changes and vanish when the result is not optimal. Consumers that act on the whole solution need it atomically in one row. Aggregating rows downstream cannot express "empty" or "not optimal" reliably. |
| **Scenario roll-up** `scenarios.<name>` = `{values, feasible, failures}` plus `scenariosSignature` | N-1 analysis asks "does every single loss leave a solution, and if not, which loss first?" A one-row answer avoids aggregating scenario rows downstream. The latest analysis stays until replaced, so consumers never see a transient gap. |
| **Upstream-snapshot initialization** (opt-in `output_snapshots`, shared SDK `UpstreamSnapshots`) | Any volatile transformer fed by continuous queries must start from its inputs' current rows. Query bootstrap rows are not forwarded downstream. |
| **`setSizes`** on the status row | Observability: why a model is empty or small. It also lets consumers read derived counts, such as unhealthy members, without another query. |

New tests (`src/component/tests/placement.rs`, a generic placement example): the
seed needs no moves and is safe; failure plan; policy edit → infeasible; capacity
loss reported until capacity returns; one failure risks the next loss; unknown on
incomplete inputs (5 cases) and recovery; stale-approval identities.
Configuration validation covers the new fields.

## Evidence

SDK harness (`cargo test -p drasi-computation-transformer-optimization`), Ignite
shapes and values:

| Required outcome | Test | Result |
|---|---|---|
| Seed: feasible, 0 moves, safe | `placement_seed_needs_no_moves_and_survives_any_single_loss` | support→EU-A, batch→EU-B; 3 losses, 0 failures |
| EU-A failed: support→EU-B, summaries (batch)→US-C, 2 moves; at risk, first failure EU-B | `placement_failure_plan_then_policy_edit_is_infeasible` | as required. EU-B loss detail names `[support]` |
| Then support loses EU-B: infeasible, no Assignment rows, `candidate_id` empty | same | `constraint assign-once[support] has no eligible variables and cannot hold` |
| Healthy, support on EU-B, batch on US-C, EU-A 50k: "not if EU-B fails"; safe at 100k | `placement_capacity_loss_is_reported_until_capacity_returns` | first failure EU-B (support); safe after restore |
| US-C failed: first failure EU-A | `placement_with_one_failure_first_risks_the_next_loss` | EU-A |
| Expired report, undecided or missing policy, other revision, not loaded → unknown | `placement_is_unknown_rather_than_guessed_on_incomplete_inputs`, `placement_unknown_recovers_and_scenarios_return` | `unknown`, no Assignment or LossScenario rows, roll-up null; recovers when health returns |
| Stable basis and candidate; stale approvals detectable | `placement_identities_detect_stale_approvals` | Same facts in any order → same `signature` and `solutionSignature`. Demand or revision change → new basis. Different plan → new candidate |

Downstream queries were evaluated with drasi-core's `QueryBuilder` and the Cypher
function set, fed status rows shaped exactly as the plugin emits them. Results:

- seed → `feasible`, `"{\"batch\":\"EU-B\",\"support\":\"EU-A\"}"`, reason "Everything already runs … (peak use 80%)", resilience `safe`;
- failure → reason "Move batch to US-C and support to EU-B (2 moves, peak use 80%)", resilience `at_risk`, `first_failure` EU-B, reason "Losing EU-B leaves no plan: No allowed, healthy destination with room for support";
- infeasible → reason "No allowed, healthy destination with room for support", resilience `at_risk` "No plan even now: …";
- unknown health → "Health of EU-B is unknown"; unknown policy → "Policy for batch on US-C is unknown"; `limit` → unknown;
- scenarios not yet analysed → resilience `unknown` "The loss analysis has not finished".

Two drasi-core Cypher quirks found and avoided:

- `reduce` evaluates its body without the function registry, and list indexing
  fails inside it.
- `'x' + f(...)[0]` indexes the concatenation, so the index needs parentheses.

These may be worth separate issues.

Not yet run: the full graph under Server. `drasi-server validate` does not parse
Cypher, and Server wiring belongs to the port agent.

## Proposed configuration

### Component `plan` (`drasi.standard/optimization`, port `in` → `out`)

Replaces the `plan` and `resilience` components. Inputs to `in`: `plan-context`,
`plan-items`, `plan-bins`, `plan-candidates`, `policy-results`.

```json
{
  "stream": "plan/out",
  "debounceMs": 100,
  "maxDelayMs": 1000,
  "timeLimitMs": 2000,
  "statusLabel": "PlanStatus",
  "scenarioLabel": "LossScenario",
  "sets": [
    {
      "name": "context",
      "label": "PlanContext",
      "key": [
        "run_epoch"
      ]
    },
    {
      "name": "items",
      "label": "PlanItem",
      "key": [
        "id"
      ]
    },
    {
      "name": "bins",
      "label": "PlanBin",
      "key": [
        "id"
      ]
    },
    {
      "name": "healthy",
      "label": "PlanBin",
      "key": [
        "id"
      ],
      "where": [
        {
          "property": "health",
          "op": "==",
          "value": "healthy"
        }
      ]
    },
    {
      "name": "unhealthy",
      "label": "PlanBin",
      "key": [
        "id"
      ],
      "where": [
        {
          "property": "health",
          "op": "==",
          "value": "unhealthy"
        }
      ]
    },
    {
      "name": "decisions",
      "label": "PolicyResult",
      "key": [
        "subject",
        "target"
      ]
    },
    {
      "name": "allowed",
      "label": "PolicyResult",
      "key": [
        "subject",
        "target"
      ],
      "where": [
        {
          "property": "state",
          "op": "==",
          "value": "allowed"
        },
        {
          "property": "allowed",
          "op": "==",
          "value": true
        }
      ]
    },
    {
      "name": "candidates",
      "label": "PlanCandidate",
      "key": [
        "subject",
        "target"
      ]
    },
    {
      "name": "pairs",
      "label": "PlanCandidate",
      "key": [
        "subject",
        "target"
      ],
      "where": [
        {
          "inSet": "allowed",
          "match": {
            "subject": "subject",
            "target": "target"
          }
        },
        {
          "inSet": "healthy",
          "match": {
            "target": "id"
          }
        },
        {
          "property": "capacity_tpm",
          "op": ">=",
          "valueProperty": "demand_tpm"
        }
      ]
    }
  ],
  "variables": [
    {
      "name": "assign",
      "type": "binary",
      "over": "pairs",
      "output": {
        "label": "Assignment",
        "properties": [
          "subject",
          "target",
          "current",
          "demand_tpm"
        ]
      }
    },
    {
      "name": "peak",
      "type": "continuous",
      "lower": 0,
      "output": {
        "emit": false
      }
    }
  ],
  "constraints": [
    {
      "name": "assign-once",
      "forEach": "items",
      "terms": [
        {
          "variable": "assign",
          "match": {
            "subject": "id"
          }
        }
      ],
      "op": "==",
      "rhs": 1
    },
    {
      "name": "capacity",
      "forEach": "healthy",
      "terms": [
        {
          "variable": "assign",
          "match": {
            "target": "id"
          },
          "coefficient": {
            "property": "demand_tpm"
          }
        }
      ],
      "op": "<=",
      "rhs": {
        "property": "capacity_tpm"
      }
    },
    {
      "name": "peak-use",
      "forEach": "healthy",
      "where": [
        {
          "property": "capacity_tpm",
          "op": ">",
          "value": 0
        }
      ],
      "terms": [
        {
          "variable": "assign",
          "match": {
            "target": "id"
          },
          "coefficient": {
            "property": "demand_tpm"
          }
        },
        {
          "variable": "peak",
          "coefficient": {
            "property": "capacity_tpm",
            "of": "group",
            "scale": -1
          }
        }
      ],
      "op": "<=",
      "rhs": 0
    }
  ],
  "objectives": [
    {
      "name": "moves",
      "sense": "minimize",
      "terms": [
        {
          "variable": "assign",
          "where": [
            {
              "property": "target",
              "op": "!=",
              "valueProperty": "current"
            }
          ]
        }
      ]
    },
    {
      "name": "peak",
      "sense": "minimize",
      "terms": [
        {
          "variable": "peak"
        }
      ]
    }
  ],
  "scenarios": [
    {
      "name": "loss",
      "set": "healthy",
      "property": "id"
    }
  ],
  "provenance": {
    "set": "context",
    "properties": [
      "run_epoch",
      "revision"
    ]
  },
  "preconditions": [
    {
      "name": "context-loaded",
      "set": "context",
      "min": 1,
      "max": 1
    },
    {
      "name": "items-loaded",
      "set": "items",
      "min": 1
    },
    {
      "name": "bins-loaded",
      "set": "bins",
      "min": 1
    },
    {
      "name": "health-known",
      "set": "bins",
      "max": 0,
      "where": [
        {
          "property": "health",
          "op": "notIn",
          "value": [
            "healthy",
            "unhealthy"
          ]
        }
      ]
    },
    {
      "name": "policy-known",
      "set": "decisions",
      "max": 0,
      "where": [
        {
          "property": "state",
          "op": "notIn",
          "value": [
            "allowed",
            "denied"
          ]
        }
      ]
    },
    {
      "name": "candidates-loaded",
      "set": "items",
      "max": 0,
      "where": [
        {
          "notInSet": "candidates",
          "match": {
            "id": "subject"
          }
        }
      ]
    },
    {
      "name": "policy-complete",
      "set": "candidates",
      "max": 0,
      "where": [
        {
          "notInSet": "decisions",
          "match": {
            "subject": "subject",
            "target": "target"
          }
        }
      ]
    },
    {
      "name": "same-revision",
      "set": "items",
      "max": 0,
      "where": [
        {
          "notInSet": "context",
          "match": {
            "run_epoch": "run_epoch",
            "revision": "revision"
          }
        }
      ]
    },
    {
      "name": "same-run",
      "set": "bins",
      "max": 0,
      "where": [
        {
          "notInSet": "context",
          "match": {
            "run_epoch": "run_epoch"
          }
        }
      ]
    },
    {
      "name": "same-run-policy",
      "set": "decisions",
      "max": 0,
      "where": [
        {
          "notInSet": "context",
          "match": {
            "run_epoch": "run_epoch"
          }
        }
      ]
    }
  ],
  "summaries": [
    {
      "name": "assignments",
      "variable": "assign",
      "properties": [
        "subject",
        "target"
      ]
    },
    {
      "name": "moved",
      "variable": "assign",
      "properties": [
        "subject",
        "target"
      ],
      "where": [
        {
          "property": "target",
          "op": "!=",
          "valueProperty": "current"
        }
      ]
    }
  ]
}
```

### New and changed upstream queries (result label in brackets)

`plan-context` (`PlanContext`, new):

```cypher
MATCH (s:ignite_scenario)
RETURN s.run_epoch AS run_epoch, s.revision AS revision
```

`plan-candidates` (`PlanCandidate`, new; uses the existing `WORKLOAD_ROUTE` join):

```cypher
MATCH (w:ignite_workloads)-[:WORKLOAD_ROUTE]->(r:ignite_routes), (d:ignite_deployments)
WHERE r.run_epoch = w.run_epoch AND d.run_epoch = w.run_epoch
RETURN w.id AS subject, d.id AS target, w.run_epoch AS run_epoch,
       w.demand_tpm AS demand_tpm, r.deployment AS current, d.capacity_tpm AS capacity_tpm
```

`plan-bins` (`PlanBin`; projects `health` instead of `status`. Only the optimizer reads it):

```cypher
MATCH (d:ignite_deployments)-[:DEPLOYMENT_REPORT]->(r:ignite_reports),
      (d)-[:DEPLOYMENT_SCENARIO]->(s:ignite_scenario)
WHERE d.run_epoch = s.run_epoch AND r.run_epoch = s.run_epoch
RETURN d.id AS id, d.run_epoch AS run_epoch, d.capacity_tpm AS capacity_tpm,
       CASE WHEN drasi.trueLater(true, r.expires_ms) THEN 'unknown'
            WHEN r.status = 'ok' THEN 'healthy'
            WHEN r.status IN ['failed', 'throttled'] THEN 'unhealthy'
            ELSE 'unknown' END AS health
```

`plan-items` (`PlanItem`): unchanged text, add the label. `policy-results`
(`PolicyResult`): unchanged columns. If the optimizer reads the policy plugin
output directly, adjust the `decisions`/`allowed` filters to its properties.

### Rewritten result queries (same columns and labels as today)

`plan-results` (`PlanResult`):

```cypher
MATCH (p:PlanStatus)
WITH p,
     CASE p.status WHEN 'optimal' THEN 'feasible' WHEN 'infeasible' THEN 'infeasible' ELSE 'unknown' END AS state,
     coalesce(p.summaries.moved, []) AS moved,
     toString(toInteger(round(coalesce(p.objectives.peak, 0.0) * 100))) AS percent
WITH p, state, percent, size(moved) AS n,
     [m IN moved | m.subject + ' to ' + m.target] AS parts
WITH p, state, percent, n, parts, parts[0..size(parts) - 1] AS init
RETURN 'plan' AS id, state,
       CASE WHEN state = 'feasible' THEN p.solutionSignature ELSE '' END AS candidate_id,
       p.signature AS basis,
       p.provenance.revision AS revision,
       CASE WHEN state = 'feasible'
            THEN '{' + reduce(s = '', a IN p.summaries.assignments |
                   s + CASE WHEN s = '' THEN '' ELSE ',' END + '"' + a.subject + '":"' + a.target + '"') + '}'
            ELSE '{}' END AS assignments,
       toInteger(coalesce(p.objectives.moves, 0)) AS moves,
       coalesce(p.objectives.peak, 0.0) AS peak_utilisation,
       CASE WHEN state = 'unknown' AND p.precondition = 'health-known'
              THEN 'Health of ' + (split(split(p.detail, ': [')[1], ']')[0]) + ' is unknown'
            WHEN state = 'unknown' AND p.precondition = 'policy-known'
              THEN 'Policy for ' + replace((split(split(p.detail, ': [')[1], ']')[0]), ', ', ' on ') + ' is unknown'
            WHEN state = 'unknown' THEN coalesce(p.detail, 'The planner has not answered')
            WHEN state = 'infeasible' AND p.detail STARTS WITH 'constraint assign-once['
              THEN 'No allowed, healthy destination with room for ' + (split(split(p.detail, '[')[1], ']')[0])
            WHEN state = 'infeasible' THEN 'Allowed, healthy destinations cannot take everything together'
            WHEN n = 0
              THEN 'Everything already runs on an allowed, healthy destination with room (peak use ' + percent + '%)'
            ELSE 'Move ' + CASE WHEN n = 1 THEN head(parts)
                                ELSE reduce(s = '', x IN init | s + CASE WHEN s = '' THEN '' ELSE ', ' END + x)
                                     + ' and ' + last(parts) END
                 + ' (' + toString(n) + CASE WHEN n = 1 THEN ' move' ELSE ' moves' END
                 + ', peak use ' + percent + '%)'
       END AS reason,
       p.provenance.run_epoch AS run_epoch,
       drasi.changeDateTime(p).epochMillis AS solved_ms
```

`resilience-results` (`ResilienceResult`):

```cypher
MATCH (p:PlanStatus)
WITH p, p.scenarios.loss AS loss, head(coalesce(p.scenarios.loss.failures, [])) AS first,
     CASE p.status WHEN 'optimal' THEN 'feasible' WHEN 'infeasible' THEN 'infeasible' ELSE 'unknown' END AS baseline
WITH p, loss, first, baseline,
     CASE WHEN baseline = 'unknown' OR loss IS NULL THEN 'unknown'
          WHEN first IS NULL AND baseline = 'feasible' THEN 'safe'
          WHEN first IS NOT NULL AND first.status <> 'infeasible' THEN 'unknown'
          ELSE 'at_risk' END AS state
RETURN 'resilience' AS id, state,
       CASE WHEN state = 'at_risk' AND first IS NOT NULL THEN first.value ELSE '' END AS first_failure,
       baseline,
       coalesce(p.setSizes.unhealthy, 0) AS unhealthy_bins,
       CASE WHEN baseline = 'unknown' THEN coalesce(p.detail, 'The planner has not answered')
            WHEN loss IS NULL THEN 'The loss analysis has not finished'
            WHEN state = 'unknown' THEN 'Losing ' + first.value + ' could not be checked: ' + first.detail
            WHEN state = 'safe' THEN 'Losing any one healthy destination still leaves a plan'
            WHEN baseline = 'infeasible' AND p.detail STARTS WITH 'constraint assign-once['
              THEN 'No plan even now: No allowed, healthy destination with room for '
                   + (split(split(p.detail, '[')[1], ']')[0])
            WHEN baseline = 'infeasible'
              THEN 'No plan even now: Allowed, healthy destinations cannot take everything together'
            WHEN first.detail STARTS WITH 'constraint assign-once['
              THEN 'Losing ' + first.value + ' leaves no plan: No allowed, healthy destination with room for '
                   + (split(split(first.detail, '[')[1], ']')[0])
            ELSE 'Losing ' + first.value + ' leaves no plan: Allowed, healthy destinations cannot take everything together'
       END AS reason,
       p.signature AS basis, p.provenance.revision AS revision, p.provenance.run_epoch AS run_epoch
```

`assignment-results` (`AssignmentResult`):

```cypher
MATCH (a:Assignment)
RETURN a.subject AS id, a.subject AS item, a.target AS bin, a.current AS current,
       a.target <> a.current AS moved, a.demand_tpm AS demand_tpm, a.run_epoch AS run_epoch
```

`loss-results`:

```cypher
MATCH (l:LossScenario)
RETURN l.value AS id, l.value AS lost,
       CASE WHEN l.feasible = true THEN 'feasible' WHEN l.feasible = false THEN 'infeasible' ELSE 'unknown' END AS state,
       l.detail AS reason, l.signature AS basis, l.provenance.run_epoch AS run_epoch
```

## Migration steps

Do these on community-next once Server loads the merged plugins.

1. **Graph (`ops/config/src/main.rs`).**
   - Replace the `plan` and `resilience` components with one `plan` component:
     `drasi.standard/optimization`, the configuration above (for example
     `config/optimization.json`), loaded from the official plugin library.
   - Bind its `output_snapshots` dependency to the `nativeOutputSnapshots`
     resource listing the five upstream outputs, as the local plugins do today.
   - Wire `plan-context`, `plan-items`, `plan-bins`, `plan-candidates` and
     `policy-results` → `plan.in`.
   - Feed `plan-results`, `assignment-results`, `resilience-results` and
     `loss-results` from `plan`.
   - Give `plan-context`, `plan-items`, `plan-bins` and `plan-candidates` result
     labels. Keep `plan-bins` in `HEALTH_QUERIES`.
2. **Queries (`drasi/`).** Add `plan-context` and `plan-candidates`. Replace
   `plan-bins`, `plan-results`, `resilience-results`, `assignment-results` and
   `loss-results` with the text above. No other query changes: tiles, approval
   checks, incident cards and annotations read the same columns.
3. **Remove** `plugins/optimizer`, its build step in `native-build.sh` (ask the
   port agent), and its library from the plugin list.
4. **Contracts.** Update `contracts.md` §5 and §7 to point at this document.
   `LossScenario.assignments` is gone; `basis` and `candidate_id` are 16-hex
   FNV-1a signatures instead of SHA-256.
5. **Verify.**
   - Run `tests/decision-integration.py`: failure plan, policy edit, capacity
     beat, approvals and stale approvals.
   - Check the walkthrough's resilience tile and incident cards. Expect identical
     text except the infeasible wording noted above.

## Risks

- **Input bootstrap (fixed).**
  - On a real next Server, a query's bootstrap rows are hidden from downstream
    components, and the official volatile transformers could not read upstream
    snapshots.
  - Commit `26267f08` adds opt-in initialization through the SDK's shared
    `UpstreamSnapshots` (policy agent, `a79f7e27`). With an `output_snapshots`
    dependency bound, the first start reads each input edge's snapshot, solves
    and publishes before any live input. It skips envelopes the snapshot
    covers, and re-reads an upstream that restarts.
  - Tests: `solves_from_upstream_snapshots_and_skips_covered_input` and
    `reads_an_upstream_again_when_it_restarts`.
- **Detail parsing.** `plan-results` text depends on the plugin's `detail` format
  (`constraint <name>[<key>] …`, `precondition … : [<key>]`), which is now
  documented in the README.
