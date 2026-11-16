# Drasi at Microsoft Ignite 2026

The presentation and live demo for **Monitoring without polling: Real-time,
change-driven monitoring** (session THR1029, Microsoft Ignite, November 19, 2026,
25 minutes).

| I want to… | Read |
|---|---|
| Understand what the talk and demo are about | This page |
| Set up and run the demo | [SETUP.md](SETUP.md) |
| Give the talk or run the demo | [PRESENTER.md](PRESENTER.md) |
| Change or test the demo | [DEVELOPERS.md](DEVELOPERS.md) |

## In one paragraph

A company runs a customer-support AI assistant and a background
document-summary job. Both draw on the same limited AI model capacity, deployed
in a few regions, and support data may only be processed in Europe. When the
capacity serving support fails, Grafana shows the errors and the growing queue, as it
should. But the operator still has to work out which customers are now waiting
too long, where the work is allowed to go, whether the move will fit, and
whether the fix actually worked. Drasi answers those questions with
**change-driven computation**: it keeps the answers current as the data changes
and as time passes, and puts them in the tools the team already uses.

## The talk

Three short slides frame about 18 minutes of live demo in Grafana:

1. **Title:** Monitoring without polling.
2. **The scenario:** two AI workloads sharing limited AI capacity across three regions, the rules
   they must follow, and three business questions for when EU-A fails: who is
   affected, what's the fastest fix within company policy, and how do we know
   it's really fixed? It finishes with how Drasi answers them: sources,
   continuous models and reactions.
3. **Live demo** in Grafana, with brief looks at Langfuse and Drasi Server.
4. **Closing:** keep your observability tools, add change-driven computation,
   verify the outcome.

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

The demo models this with two workloads and AI model capacity deployed in three
regions (EU-A, EU-B, US-C):

| | Data | Allowed locations | Normally runs on |
|---|---|---|---|
| Customer support | Restricted | Europe only | EU-A (needs 60k tokens/min) |
| Document summaries | Public | Anywhere | EU-B (needs 80k tokens/min) |

EU-A, EU-B and US-C each have a 100k tokens/min budget. US-C starts idle.

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
2. **Is a deployment's status still trustworthy?** If reports stop, the last
   value looks fine but is no longer evidence of anything.
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

Drasi reacts to changes instead of re-running checks. In Drasi terms:

- **Sources** bring in changes: requests, deployment reports, configuration and
  alerts, from the service's PostgreSQL database.
- **Continuous models** keep answers current as data changes and as time
  passes. Drasi has several kinds: continuous queries, continuous policy and
  continuous monitors. This demo uses:
  - **Continuous queries** over requests and deployments. Some change just because
    time passes: a request becomes overdue at its deadline, and a deployment
    report becomes stale eight seconds after it arrived, with no further update
    to either.
  - **Continuous policy** for which deployments each workload may use.
  - **Continuous models** for the best complete allocation, and for whether
    recovery would still be possible if EU-A failed.
- **Reactions** publish the results: Grafana panels and annotations, scores on
  Langfuse traces, and the current proposal shown to the operator for approval.

The answers are continuously maintained, not recomputed on request. When an
input changes, only the affected answers update.

## How the story unfolds on stage

1. **Healthy service (Grafana).** Real traffic flows. Support runs on EU-A,
   summaries on EU-B, and the queue is empty.
2. **Failure (Grafana).** EU-A fails. Errors and the queue rise, and Grafana
   alerts. Drasi's overdue count climbs as support requests pass their deadline,
   with no new events needed. A Langfuse trace shows one affected request.
3. **Recovery (Grafana).** Drasi's policy shows support is denied on US-C but
   summaries are allowed there. Its proposal moves summaries to US-C and support
   to EU-B. The presenter approves it. Grafana then shows the route change, the
   queue draining, and recovery **Verified**, which appears only after real
   support requests succeed on the new route.
4. **Under the hood (Drasi Server).** A brief look at the actual continuous query
   behind "overdue".
5. **Policy change (Grafana).** Restricting summaries to Europe changes the
   answer immediately. An approval of the old plan is rejected because its facts
   are out of date.
6. **Healthy but fragile (Grafana).** Everything works, but EU-B's budget is cut.
   Drasi shows that losing EU-A would leave support nowhere allowed to go.
   Restoring EU-B's budget flips the answer back, without any real outage.

## What is real and what is simulated

- **Simulated:** model responses, deployment regions and capacity budgets. No real
  GPUs, paid model services or customer data.
- **Real:** the HTTP requests, the telemetry in OpenTelemetry, Prometheus,
  Grafana and Langfuse, the database changes, the policy checks, Drasi's queries
  and models, and the measured recovery.

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
- [Inside Drasi](presentation/diagrams/drasi-graph.svg): the demo's 25 components: sources,
  continuous queries, continuous models and outputs

## What's in this folder

| Folder | Contents |
|---|---|
| [presentation/](presentation/) | Slides, speaker notes and diagrams |
| [demo/](demo/) | Demo code and local services (see [DEVELOPERS.md](DEVELOPERS.md)) |
| [plan/](plan/) | Original design, research and UI concepts |
