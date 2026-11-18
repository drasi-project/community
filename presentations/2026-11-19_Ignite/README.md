# Drasi at Microsoft Ignite 2026

The presentation and live demo for **Monitoring without polling: Real-time,
change-driven monitoring** (session THR1029, Microsoft Ignite, November 19, 2026,
25 minutes).

| I want to… | Read |
|---|---|
| Understand what the talk and demo are about | This page |
| Set up and run the demo | [SETUP.md](SETUP.md) |
| Hear the incident from the on-call engineer's side | [STORY.md](STORY.md) |
| Give the talk or run the demo | [PRESENTER.md](PRESENTER.md) |
| Understand exactly what Drasi does in the demo | [DRASI.md](DRASI.md) |
| Change or test the demo | [DEVELOPERS.md](DEVELOPERS.md) |

## In one paragraph

A company runs a customer-support AI assistant and a background
document-summary job. Both draw on the same limited AI model capacity, split
across a few deployments, and support data may only be processed in Europe. When the
capacity serving support fails, Grafana shows the errors and the growing queue, as it
should. But the operator still has to work out which customers are now waiting
too long, where the work is allowed to go, whether the move will fit, and
whether the fix actually worked. Drasi answers those questions with
**change-driven computation**: it keeps the answers current as the data changes
and as time passes, and puts them in the tools the team already uses.

## The talk

A few slides, in the Microsoft Ignite session template, frame about 17 minutes
of live demo:

1. **Title:** Monitoring without polling (after the Ignite walk-in slide).
2. **Observability today, and Drasi:** plenty of data, not enough context
   (fragmented, noisy yet late, symptoms not answers, healthy isn't working,
   context lives elsewhere); what would help; then Drasi, a change-driven
   computation platform that keeps that context current inside the tools teams
   already use, in three parts: sources, continuous models (query, policy,
   optimization) and reactions.
3. **A typical scenario:** two AI workloads sharing limited AI capacity across
   three deployments, the data-compliance rules and business requirements they
   must follow, and three questions for when something goes wrong: who is
   affected, what's the fastest fix within policy, and how do we know when it's
   resolved? Then what Drasi brings together (telemetry, application data,
   policy) and the tools it works with (Grafana, messaging, change management).
4. **Live demo** in Grafana and the team's incident channel, with the Drasi
   configuration behind each Decision Support panel and one late customer
   request on the way.
5. **Close:** keep your monitoring tools, add change-driven computation; then
   the Ignite session survey during questions.

The presenter drives everything from a private presenter desk; the projector
follows. See [PRESENTER.md](PRESENTER.md).

## The situation it mirrors

Most teams adding AI features hit the same constraints:

- **Capacity is shared and limited.** AI model capacity comes with token or
  request budgets, and several workloads compete for it.
- **Not all work can go everywhere.** Data residency, customer contracts and
  internal policy decide which workloads may use which regions.
- **Success means the customer got an answer in time.** A healthy deployment does
  not mean nobody is waiting.
- **Fixes need human approval.** Moving workloads changes cost, compliance and
  risk, so someone signs off.

The demo models this with two workloads and AI model capacity split across three
deployments, two in Europe and one in the United States (EU-A, EU-B, US-C):

| | Data | Allowed locations | Normally runs on |
|---|---|---|---|
| Customer support | Restricted | Europe only | EU-A (needs 60k tokens/min) |
| Document summaries | Public | Europe at the start; anywhere once compliance allows the US (during the talk) | EU-B (needs 80k tokens/min) |

EU-A, EU-B and US-C each have a capacity of 100k tokens/min. US-C starts idle.

## What the familiar tools do

The stack is what many teams already run:

- **OpenTelemetry** collects measurements from real HTTP model calls.
- **Prometheus** stores them; **Grafana** charts them and raises alerts.
- **Langfuse** records each AI request as a trace.

These tools are good at "what is happening": error rates, latency, queue depth
and individual traces.

## The questions they leave to the operator

When EU-A fails, the dashboards turn red. The hard questions remain:

1. **Which customer requests have missed their deadline?** A request that is
   simply waiting produces no new event. It becomes late because time passed.
2. **Is a deployment's status still trustworthy?** If its health checks go
   quiet, the last value looks fine but is no longer evidence of anything.
3. **Where may this work go?** US-C has room, but policy forbids support there.
4. **What plan fits everything?** EU-B has only 20k spare. The answer is a
   system-wide move: summaries to US-C, which frees EU-B for support.
5. **Is the approved plan still valid?** If policy or capacity changed after it
   was proposed, approving it would apply a plan built on stale facts.
6. **Did it actually recover?** Changing a route is not proof. Customers must be
   getting answers again.
7. **Are we safe from the next failure?** Everything can look healthy while
   losing one deployment would leave nowhere legal for support to go.

Alert rules work on aggregated metrics. They can't track individual requests or
combine live business context such as policy, capacity and current routes. So
during an incident, an engineer does that in their head or from a runbook, and
it gets harder as deployments, workloads and rules grow.

## What Drasi adds

Drasi is **change-driven computation**. Instead of re-running checks on a
schedule, it reacts to each change as it happens, and to time passing, and keeps
its answers current. It has four kinds of component, all working in this demo:

| Component | What it does in this demo | Built into Drasi? |
|---|---|---|
| **Sources** | Read the service database's change feed (requests, attempts, capacity, allowed destinations and approvals) and each deployment's health checks, as OpenTelemetry metrics every second | Yes: the PostgreSQL and OpenTelemetry sources |
| **Continuous queries** | Cypher queries kept current as data changes and as time passes, for example *overdue support requests* (past 15 seconds), *deployment status* (Unknown after 8 silent seconds), *is this approval still valid?* and *verified recovery*, plus one query per Grafana tile | Yes |
| **Continuous models** | Logic re-run whenever its inputs change. Here: **policy** (where each workload may run: Rego rules, given as configuration), **planner** (an optimizer that finds the best complete plan within policy and capacity) and **what-if** (if any one deployment failed, would every workload still fit?) | Yes: Drasi's standard policy and optimization components. The demo supplies the rules and the model as configuration |
| **Reactions** | Deliver every change in an answer: as Prometheus metrics for Grafana; to the gateway's configuration API, applying an approved plan only if it still matches Drasi's current plan and otherwise rejecting it; as Grafana annotations and Langfuse scores; as incident cards in Mattermost; and to the presenter desk | Yes: the HTTP and SSE reactions. The Prometheus reaction was written for this demo |

During the talk, the presenter says which of these components produced what
the audience just saw; after the healthy-but-fragile part, a short **What Drasi
just did** visual shows it.

## How the story unfolds on stage

1. **Healthy service.** Real traffic flows. Support runs on EU-A, summaries on
   EU-B. Grafana's AI Deployments row is ordinary monitoring; its Decision
   Support row is Drasi's answers. Summaries start Europe-only, so Failover
   readiness is at risk (losing EU-A would leave both workloads needing EU-B).
   Compliance allows summaries in the US, Drasi re-runs the policy and the
   what-if, and readiness turns to "Survives losing any one deployment".
2. **Failure.** EU-A fails. Within seconds Drasi, reading EU-A's health-check
   telemetry, shows it Failed and has a plan ready; errors and the queue rise and Grafana alerts. Support requests count as
   overdue the moment each passes its 15-second deadline, with no new event.
3. **Who is affected.** Drasi's failure and overdue cards in the incident channel,
   then one of those customers' requests, attempt by attempt.
4. **Recovery.** Support may not run on US-C, but summaries may. The planner's plan
   moves summaries to US-C, freeing EU-B for support. The presenter approves the
   plan on its card; **Verified** appears only once the queue is answered and support
   requests succeed on the new route.
5. **Healthy but fragile.** EU-A comes back with only 50k of capacity. Everything
   is green, but Drasi says we're not safe: if EU-B failed, support would have
   nowhere allowed to go. Restoring EU-A to 100k flips the answer back, with no
   outage.

## What is real and what is simulated

- **Simulated:** model responses, the three deployments, their capacity and
  health checks, and the two systems Drasi calls: a change-approval tool and the
  gateway's configuration API. No real GPUs, paid model services or customer data.
- **Real:** the HTTP requests, the telemetry in OpenTelemetry, Prometheus,
  Grafana and Langfuse, the database changes, and everything the talk says Drasi
  does: its queries, the policy, the plans, checking and applying approvals, and
  verifying recovery. See [DRASI.md](DRASI.md#3-what-is-simulated).

## Why people should care

- **It fits what they already have.** Nobody replaces Grafana, Prometheus or
  Langfuse. Drasi feeds them answers.
- **It answers "so what?", not just "what?".** Dashboards say something is wrong.
  Drasi says which customers are affected, what is allowed, what will fit, and
  whether the fix worked.
- **Time is a first-class input.** Missed deadlines and silent deployments are
  detected as they happen, not at the next check.
- **Decisions stay valid.** Proposals track the facts they depend on, so a stale
  plan cannot be approved by accident.
- **It is a pattern for AI operations.** Shared capacity, policy-bound placement
  and human-approved remediation are everyday problems for teams running AI, and
  the same approach applies well beyond this demo.

## Architecture

- [System architecture](presentation/diagrams/architecture.svg): the AI service,
  the observability tools and Drasi Server
- [Inside Drasi](presentation/diagrams/drasi-graph.svg): the demo's 59 components:
  sources, continuous queries, continuous models and reactions

## What's in this folder

| Folder | Contents |
|---|---|
| [presentation/](presentation/) | Slides, speaker notes and diagrams |
| [demo/](demo/) | Demo code and local services (see [DEVELOPERS.md](DEVELOPERS.md)) |
| [plan/](plan/) | Original design, research and UI concepts |
