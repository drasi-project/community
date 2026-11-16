# Ignite presentation and demo delivery plan

## 1. Goal and decisions

Build one repeatable demonstration: **keep a customer-support assistant working when shared AI capacity becomes constrained**. Grafana reveals the operational problem; Drasi maintains policy, allocation, and resilience assessments; an operator approves recovery; fresh measurements and completed tasks confirm the outcome.

Deliver a working local demo, a mostly live Reveal.js presentation, and complete speaker notes for [THR1029: Monitoring without polling: Real-time, change-driven monitoring](https://ignite.microsoft.com/en-US/sessions/1787856602420001EGgA-1791588144696001Cv8p?source=sessions), **November 19, 2026, 25 minutes**. Target 22 minutes of content and three minutes of reserve.

**Audience format:** three clean slides only: title; Drasi concepts and scenario; closing. The bulk of the talk runs in actual Grafana dashboards, with brief Langfuse/Drasi Server drill-downs. Presenter instructions and controls remain private; the custom diagnostic console is not the projected demonstration.

### Confirmed decisions

| Area | Decision |
|---|---|
| Hosting | Stock Drasi Server, using the existing local Core and Server checkouts on `agentofreality-parallel-computation-graph-recovery` |
| Demo ownership | Custom native plugins and all demo-specific code/configuration belong in `community/presentations/2026-11-19_Ignite/demo/` |
| Platform work | Fix required bugs or missing functionality in `drasi-core`/`drasi-lib` or `drasi-server`, with regression coverage in the owning repository; do not switch to a custom host |
| Presentation | Reveal.js, support material, notes, and replay assets belong in `community/presentations/2026-11-19_Ignite/presentation/` |
| Operating environment | Local, offline-capable stack using Podman; no conference-network dependency |

Use the existing checkouts and branches in place. Record matched source revisions and plugin builds as development progresses, then freeze a qualified release.

### Starting point and scope

The [research](research/research_001.md), [architecture diagram](research/drasi-ai-observability-demo.png), and [UI concepts](ui_mocks/ui_mocks_001.md) establish the direction. The [gallery](ui_mocks/drasi-ignite-ui-mockups.html) contains synthetic charts and non-operative controls, not working Grafana/Langfuse integrations. `demo/` now contains the working local stack and audience application; `presentation/` contains its Reveal scenes, notes and replay manifest. Release qualification is documented in [the developer guide](../DEVELOPERS.md); human rehearsal and venue checks remain separate gates.

Keep the published session promises within this one application: endpoint failure and alerting, stalled support tasks, missed completion targets, missing heartbeats, and human-approved automated remediation. “Without polling” describes Drasi's change-driven and time-aware processing; Prometheus scraping and Grafana evaluation remain periodic.

Simulate model responses, endpoint regions, and capacity limits. Application requests, telemetry, database updates, Drasi computations, and integrations must be real. No real GPUs, customer data, paid model service, autonomous operations agent, Kubernetes, or additional observability products are needed.

## 2. Demo scenario

### Main incident

Two workloads share three compatible model endpoints. Support processes synthetic restricted records and is permitted only in Europe; public-document batch work may use any endpoint. These are fictional customer rules, not a general claim about GDPR.

| Endpoint | Modeled budget, tokens/minute | Initial reservation |
|---|---:|---|
| EU-A | 100,000 | Support: 60,000 |
| EU-B | 100,000 | Batch: 80,000 |
| US-C | 100,000 | None |

Fail EU-A. EU-B has only 20,000 spare capacity, while US-C is available but prohibited for support. Drasi proposes moving batch to US-C and support to EU-B. An operator approves the current proposal; the gateway adopts it for new and queued requests; the monitoring stack and task records demonstrate recovery.

Initially, allocate each workload to one destination with its full reservation. Preserve both workload commitments rather than silently dropping batch demand. A bounded exact search is sufficient for this small model; enforce policy, availability, freshness, model compatibility, and capacity, with deterministic tie-breaking that minimizes movement. Do not hard-code the stage solution.

### Variations

| Variation | Input change | What it demonstrates |
|---|---|---|
| Stalled task | Leave a queued task unchanged past its completion deadline | Drasi detects a missing outcome without another task event |
| Missing heartbeat | Pause endpoint reporting | Evidence becomes stale without another report |
| Stale proposal | Change an allowed destination while a plan awaits approval | Eligibility changes and the obsolete approval is rejected |
| Healthy but fragile | Restore health; place support on EU-A, batch on US-C, and leave EU-B idle; reduce EU-B's budget to 50,000 | Current work still fits, but losing EU-A would leave insufficient permitted capacity for support |
| Restore resilience | Raise EU-B's budget to 100,000 | The hypothetical EU-A-loss assessment becomes feasible without a real outage |

The resilience branch is read-only and evaluates a named failure, not a universal availability guarantee. Use shortened, visible demo deadlines; begin with a 15-second task target and an 8-second report-expiry interval, then tune against measured stage pacing.

## 3. Architecture and essential behavior

```text
Support + batch clients -> gateway -> simulated HTTP model endpoints
                         -> authoritative task/configuration data in PostgreSQL

Application/gateway -> OTel Collector -> Prometheus -> Grafana
                                    -> Langfuse traces

PostgreSQL bootstrap/CDC + selected observations + Grafana alert webhooks
  -> Drasi Server
     context/time-aware queries
     -> policy, allocation, resilience plugins
     -> result queries
     -> live UI, Grafana assessments/annotations, Langfuse assessments

Current candidate -> operator approval -> control service -> gateway adoption
  -> fresh observations and task completion -> recovery confirmation
```

Server owns graph configuration, lifecycle, management, and query-result streaming. The separate control service owns approvals and audit; the gateway enforces policy and admission at request dispatch. Use real Grafana and Langfuse interfaces, not replicas of their product screens.

Reuse [Move-a-wall](../../../../drasi-server/examples/move-a-wall/README.md) for native-plugin hosting, [Trading](../../../../drasi-server/examples/trading/README.md) for mixed-source bootstrap, and [GPU lab shared code](../../../../drasi-server/examples/gpu-cluster-lab/shared/README.md) for domain patterns. The GPU lab's Server deployment is unfinished; migrating it is not a prerequisite.

### Implementation rules

| Concern | Required behavior |
|---|---|
| Real computation | Scenario controls change inputs only. Queries and plugins derive the displayed conditions and decisions; adapters only translate and publish |
| Identity and freshness | Use stable endpoint/workload/task/trace identities, versioned decision inputs, and a reset/run epoch. Separate freshness timestamps from meaningful model changes so identical heartbeats do not invalidate every plan |
| Unknown state | Stale/missing evidence or failed computation means unknown, not healthy, permitted, or proven infeasible |
| Approval | Recheck policy, input version, and freshness; apply commands idempotently; keep candidate, committed, applied, and recovered states distinct |
| Enforcement | Check permission before transmitting request content. Reallocation does not migrate in-flight inference |
| Recovery | Require the adopted allocation plus fresh successful requests and resolution of affected tasks; preserve historical deadline breaches |
| Lifecycle | Bootstrap before declaring readiness; preserve authoritative data on restart; reconcile uncertain application outcomes; make scenario reset explicit and isolated |
| UI delivery | Use Server's snapshot-first query attachment, replacing state on reconnect. Do not combine a separate snapshot request and volatile SSE stream and assume no gap |
| Access and telemetry | Keep administration/secrets private, scope cleanup to this demo, use low-cardinality metrics, and show publication failures explicitly |

Prefer Rust for native plugins and backend services, with React/TypeScript for the custom UI and presentation. Settle the Rego evaluator, allocation representation, database transaction/bootstrap mode, product versions, and iframe-versus-tab navigation during the first phase.

## 4. Delivery phases

The critical path is **integration proof -> instrumented workload -> continuous context -> decision models -> verified recovery -> presentation -> qualification**. Layout and notes can progress alongside implementation once contracts are stable.

### Phase 1 - Prove the Server-hosted integration

**Dependencies:** None.

- Build local Server, required Core plugins, and a minimal community plugin against matching SDK/contracts. Package compatible binaries together.
- Prove PostgreSQL bootstrap and CDC through a context query, custom continuous model, result query, and Server browser subscription. Include populated startup, restart, and scenario reset.
- Prove one real request reaches Prometheus/Grafana and Langfuse, and Grafana firing/resolved webhooks reach Drasi.
- Qualify Podman compatibility, laptop resource use, trace-ingestion latency, and product navigation. Define shared record identities, versioning, and authoritative state.

**Exit:** The complete thin integration works, survives restart/reconnect, and has reproducible configuration. Fix platform gaps at their owning layer before dependent work proceeds.

### Phase 2 - Build the instrumented workloads

**Dependencies:** Phase 1's build and contracts.

- Implement support and batch clients, gateway routing/admission/queues, three endpoint simulators, and durable task outcomes.
- Add explicit failure, throttling, report-pause, restore, and fixture-reset controls.
- Instrument success, errors, latency, retries, queue depth, token usage, and capacity. Provision Grafana dashboards and alerts from configuration.

**Exit:** Actual traffic produces baseline measurements; injected failure produces a genuine Grafana alert without relying on Drasi decisions. Restricted support content cannot reach US-C.

### Phase 3 - Maintain context and detect missing progress

**Dependencies:** Phases 1-2.

- Connect database changes, selected observations, and normalized alert transitions to context and impact queries.
- Implement time-aware task deadlines and heartbeat expiry; preserve breach history after active conditions clear.
- Handle duplicate/out-of-order inputs and capture real intermediate evidence for inspection.

**Exit:** Unchanged task and heartbeat records cross their deadlines through Drasi's scheduled evaluation, not an application polling loop. Feed loss produces a visible stale/unknown state.

### Phase 4 - Implement the decision plugins

**Dependencies:** Phase 3's coherent context.

- Build policy, allocation, and resilience native plugins under `demo/plugins/`, sharing domain types and constraints.
- Evaluate real Rego rules, produce explained eligibility facts, and calculate complete feasible allocations.
- Publish current candidate and named-failure assessments as queryable facts, including their input basis and validity.
- Exercise capacities and policy combinations beyond the scripted fixture.

**Exit:** The model derives the required reallocation, invalidates it when inputs change, and correctly distinguishes current feasibility from EU-A-loss resilience.

### Phase 5 - Apply and verify recovery

**Dependencies:** Phase 4 and the gateway/control data model.

- Implement version-checked approval, audit, idempotent application, and gateway acknowledgement.
- Derive recovery from fresh operational evidence and completed tasks, not command success.
- Publish Grafana annotations/summary metrics and Langfuse assessments linked to the affected traces, with retryable delivery failures.

**Exit:** Duplicate approvals do not duplicate effects; stale plans are rejected; policy changes remain enforced during application; recovery stays unconfirmed until the configured observation criteria are met.

### Phase 6 - Complete the audience views

**Dependencies:** Stable results from Phase 5; layout can begin earlier.

| View | Required content |
|---|---|
| Grafana operations | Success/latency, queues, endpoint errors, workload throughput, allocation, alerts, and compact Drasi assessments with freshness |
| Grafana Policy and recovery | Maintained policy eligibility, configured/proposed allocation, decision events, current recovery, and resilience |
| Drasi Server drill-down | One actual continuous query and its maintained results; a brief optional tool visit, not a custom dashboard tour |
| Langfuse trace | One affected support task's attempts, policy rejection, queueing, completion, and independent Drasi assessment |

Use supported product panels, scores, metadata, and links. Keep historical snapshots visibly distinct from live data, and make all screens legible at projector resolution without essential scrolling.

**Exit:** The incident can be presented without terminal work, fabricated product UI, or unexplained numbers.

### Phase 7 - Complete Reveal.js and speaker materials

**Dependencies:** Stable scene/navigation contracts; final notes depend on Phase 6.

- Build the three-slide frame and the private speaking route in Section 5 with locally bundled assets.
- Navigate directly from the concepts slide into actual Grafana; use native dashboard links for detail and the closing slide. Backend state continues independently of slide navigation.
- Keep navigation separate from fault injection, approval, and reset. Speaker previews must be read-only and must not start another workload.
- Build a minimal private presenter desk with notes, timer, view navigation, explicit controls, and replay selection.
- Maintain one source of scene notes for Reveal, the desk, and printable speaker materials.

**Exit:** Slide/tool navigation never changes backend state; only the three slides are projected outside actual tools; notes remain private; the main route fits 22 minutes.

### Phase 8 - Qualify, package, and rehearse

**Dependencies:** Phases 1-7.

- Exercise restart/reset, browser reconnect, stale feeds, duplicate/out-of-order events, concurrent approvals, policy changes during application, failed acknowledgement, and unavailable publication targets.
- Run five incident/recovery/reset cycles and a 60-minute soak. Measure startup/reset times and request-to-alert, input-to-assessment, and applied-to-recovery latency.
- Produce a versioned offline bundle, operating instructions, known limitations, and labeled recordings of genuine runs.
- Complete three timed rehearsals, including an offline run and a deliberate fallback, using the actual laptop, displays, and clicker.

**Exit:** Another person can start/reset the system from its README; the presenter can complete the live route and switch to replay within 15 seconds.

For platform changes in any phase, add focused regression coverage in Core/library or Server and rerun the dependent demo checks. Track such fixes as phase dependencies and adjust the schedule rather than bypassing required behavior.

## 5. Presentation and speaker notes

### Audience route

| Time | Scene | Main action / message |
|---|---|---|
| 0:00-0:30 | Title slide | Introduce the session; no concepts or dashboard content |
| 0:30-2:30 | Concepts/scenario slide | Explain change-driven computation, the four Drasi roles, and the two-workload scenario |
| 2:30-5:00 | Grafana operations | Establish actual measurements, workloads and processing restrictions |
| 5:00-8:00 | Grafana incident | Fail EU-A; show the alert, queue and overdue tasks; optionally inspect one Langfuse trace |
| 8:00-13:00 | Grafana policy/recovery | Explain policy and coordinated allocation; approve privately; observe actual recovery |
| 13:00-15:00 | Drasi Server | Briefly show one actual continuous query and heartbeat expiry |
| 15:00-17:00 | Grafana policy | Change policy and show stale approval rejection in decision events |
| 17:00-20:30 | Grafana resilience | Show healthy current traffic but insufficient EU-A-loss recovery capacity, then restore it |
| 20:30-22:00 | Closing slide | Summarize change-driven computation alongside existing tools |
| 22:00-25:00 | Reserve | Questions, latency, or recovery from a presentation problem |

Use three simple slides. Advancing past slide two enters Grafana directly; its native links provide policy/recovery detail and the closing slide. Private speaking cues are separate from slide navigation.

Keep Grafana the operational home, Drasi the decision explanation, and Langfuse a short AI-behavior drill-down. Explain feasibility as conditional on the model, token costs as simulated estimates, and automation as human-approved. A missed task target is not by itself a long-term SLO calculation.

### Speaker package

Draft notes after the first working incident and refine them during rehearsal. Each scene needs its purpose/time budget, starting state, spoken explanation, exact action, evidence to point at, expected wait and cutoff, transition, and short/failure alternative. Use common scene IDs for notes, fixtures, and recordings.

Deliver:

- Scene notes available privately through Reveal/the presenter desk.
- A printable runbook with preflight, scenario transitions, reset, and troubleshooting.
- A shorter route that trims inspector depth and the standalone policy-edit scene while retaining failure, recovery, and resilience.
- Labeled replay clips for failure/deadline, recovery, policy invalidation, and resilience; replay disables mutation controls.

Do not return from replay to live unless the actual scenario state is known.

## 6. Deliverables and operating requirements

Paths below are relative to `community/presentations/2026-11-19_Ignite/`.

| Location | Deliverables |
|---|---|
| `demo/plugins/`, `demo/domain/` | Native policy/allocation/resilience plugins and shared domain logic |
| `demo/drasi/`, `demo/database/`, `demo/policies/`, `demo/scenarios/` | Server graph/query configuration, migrations, policies, and reproducible fixtures |
| `demo/control/`, `demo/workloads/`, `demo/ui/` | Approval/publication services, gateway/simulated workloads, and custom views |
| `demo/observability/`, `demo/ops/`, `demo/tests/` | Provisioned stack, startup/readiness/reset/stop/package tooling, and acceptance coverage |
| `presentation/` | Reveal shell, supporting visuals, notes, presenter desk, and local fallback media |
| `plan/` | This plan and existing research/design references |

Provide a single documented operating interface for prerequisite checks, startup, readiness/status, reset, logs, verification, shutdown, and packaging. Normal restart preserves data; reset is explicit. Keep credentials out of source, bind administrative access locally, and scope ports, storage, and cleanup to this stack.

Build Server, standard plugins, and community plugins from matching local sources. Record revisions, binary compatibility, locked dependencies, and image versions; rebuild affected artifacts after platform changes. Ship the assets needed for offline use, attribution, and concise installation/troubleshooting documentation.

## 7. Milestones and completion

| Date | Target |
|---|---|
| Nov 8-9 | Phase 1 integration proof and initial contracts; identify/fix platform gaps |
| Nov 10 | Phases 2-3: instrumented incident and time-aware detection; first notes |
| Nov 11-12 | Phases 4-5: decision plugins and approved, verified recovery |
| Nov 13-14 | Phases 6-7: final views, Reveal shell, and complete speaker materials |
| Nov 15-16 | Feature-complete gate, failure qualification, recordings, and first timed rehearsal |
| Nov 17 | Fix rehearsal blockers and freeze the matched release |
| Nov 18 | Venue/display check and fallback rehearsal; no elective dependency changes |
| Nov 19 | Confirm session time, preflight, warm/reset the baseline, and present |

The schedule assumes focused daily work. Address bootstrap, plugin compatibility, Langfuse resource/ingestion behavior, and product navigation early. If delivery slips, reduce animation, presenter previews, optional slides, and inspector polish before reducing correctness or the central story. A recorded product drill-down is a disclosed presentation fallback, not proof that its live integration is complete.

**Done means:** a reproducible Server-hosted demo with working integrations and required platform fixes; a rehearsed 22-minute presentation; complete private/printable speaker materials; and an offline release with tested recovery/replay options.

### Implementation references

- [Drasi Server configuration and APIs](../../../../drasi-server/README.md)
- [Native component catalogue](../../../../drasi-core/components/computation-plugins/README.md) and [PostgreSQL bootstrap/CDC](../../../../drasi-core/components/computation-plugins/sources/postgres/README.md)
- [Time-aware query examples](../../../../drasi-core/shared-tests/src/use_cases/sensor_heartbeat/queries.rs)
- [Reveal.js speaker view](https://revealjs.com/speaker-view/)
- [Langfuse deployment](https://langfuse.com/self-hosting/deployment/docker-compose) and [OTel ingestion](https://langfuse.com/integrations/native/opentelemetry)
