import { scenes, type SceneId, type StageView } from "./scenes";
import type { ConfigId } from "./codeSlides";
import type { HowId } from "./howItWorks";

// The single source for what the presenter says and does. The presenter desk shows it beat by
// beat, and PRESENTER.md's run of show and script are generated from it
// (cd demo/ui && npm run sync-presenter). A test fails if PRESENTER.md drifts.

// Buttons the desk shows next to a beat. Each changes the live demo.
export type ActionId = "start-traffic" | "stop-traffic" | "fail-eu-a" | "approve-change"
  | "summaries-europe" | "summaries-anywhere" | "eu-a-half" | "eu-a-full" | "restore-eu-a"
  | "provision-us-c";

export type Beat = {
  show: StageView;     // what the projector shows during this beat (Next switches it)
  say: string;         // what to say, in your own words
  do?: ActionId[];     // the buttons to press, in order, before or while saying it
  click?: string;      // what to press on the projected screen itself (the audience sees it)
  watch?: string;      // what appears on screen
  optional?: true;     // skipped when "Short on time" is ticked
  expect?: "rejected"; // the approval is meant to be rejected (the out-of-date proposal)
  recording?: string;  // the recording shown instead in recording mode (default: the part's)
  // Dashboard panels to highlight on the projector, as "<dashboard uid>.<panel key>": the panel's
  // "key" in the dashboard JSON, or its title in lower case with dashes (a row covers its panels).
  highlight?: string[];
  // The Drasi configuration behind the highlighted panel, shown above it (presentation/codeSlides.ts).
  config?: (ConfigId | HowId)[];
};
export type Cue = { summary: string; beats: Beat[]; ifBroken: string; recording?: string };

export const notes: Record<SceneId, Cue> = {
  intro: {
    summary: "Walk-in; who I am; the project; what I'll show you; why Drasi exists",
    beats: [
      { show: "walkin", do: ["start-traffic"],
        say: "Before the session: the walk-in slide stays up while people arrive. Opening the projector window starts the talk timer at 00:00 (Restart timer in the header resets it). Press Next when you start talking.",
        watch: "Press Start demo traffic before you start talking, so the dashboard has several minutes of data by scene 1. Press it at least 5 minutes after any reset." },
      { show: "title",
        say: "Good morning, my name is Allen Jones. I'm a member of the Azure Incubations team at Microsoft." },
      { show: "project",
        say: "For the last few years I've been working on an open-source project named Drasi. We started it in Azure Incubations and open-sourced it in October 2024. Since January 2025 Drasi has been a CNCF Sandbox project, and it's in active development with a growing community of users and contributors." },
      { show: "ops",
        say: "Here's what I'm going to show you today. This is a live AI service and its monitoring: a Grafana dashboard like the ones most teams already run.",
        watch: "The live dashboard, charts moving; no highlight." },
      { show: "ops", highlight: ["ai-operations.ai-deployments"], config: ["label-standard"],
        say: "The top row is what most teams already have: OpenTelemetry, Prometheus and Grafana. It tells you something is wrong.",
        watch: "The top row outlined, labelled The basics." },
      { show: "ops", highlight: ["ai-operations.decision-support"], config: ["label-drasi"],
        say: "The bottom row is context, answers and guidance, powered by Drasi: who's affected, what to do about it, and whether you'd survive the next failure. Answers that update the moment things change, beside the monitoring you already use. In a few minutes I'm going to break this service on purpose. Watch which row notices first.",
        watch: "The Decision Support row outlined, labelled Context, answers, guidance · Powered by Drasi." },
      { show: "problem",
        say: "Drasi was started because, in our experience, detecting meaningful changes in data as they happen, and reacting to them, is harder than it should be. There are two common approaches. Polling: query everything every few seconds, diff it with last time, and write code to react. It's simple, but slow to notice, wasteful, and it misses changes that happen in between. Or stream processing: consume an event stream and react to every event. That's powerful, and every event arrives, but working out what an event means for the current state, joining it with other sources, and noticing when something didn't happen, means building and running your own state: windows, joins, state stores, ordering and late events." },
      { show: "change-driven",
        say: "Drasi takes a change-driven approach. You describe the result you want, as a query. Changes flow in from the systems you already have; Drasi keeps the result current as the data changes, and as time passes, across sources; and a reaction acts on just what changed: what was added, updated or removed. No polling loop, and no state store to build and run yourself." },
    ],
    ifBroken: "The slides don't depend on the demo. Carry on.",
  },
  drasi: {
    summary: "Sources, Continuous Queries, Reactions, and queries building on queries",
    beats: [
      { show: "drasi-1",
        say: "Drasi is built around three simple, familiar concepts: Sources, Continuous Queries and Reactions. Sources bring in every change, here from a database, telemetry and Kubernetes. A continuous query describes what you care about across them, as one graph query, and Drasi does the joining and keeps its answer current as the data changes. A reaction does something with each change to the answer." },
      { show: "drasi-2",
        say: "And it grows from there: many sources, many queries, many reactions, each answer kept current as the data changes. TODO: mention continuous models (policy, optimization) here or in scene 3?" },
      { show: "drasi-3",
        say: "And, coming soon, queries and models can build on each other's results, forming a computation graph. In this demo, the health checks feed a deployment-health query, which feeds the workload status and the plan; policy input feeds the policy model, which feeds the plan too; and the results fan out to several reactions. When one input changes, only what depends on it updates." },
    ],
    ifBroken: "Carry on; the slides don't depend on the demo.",
  },
  observability: {
    summary: "Observability: the issues, what organizations want, and the scenario",
    beats: [
      { show: "issues",
        say: "Drasi is a general-purpose technology for building change-driven systems in any domain. But few domains show the need and value better than observability, so today I'll show you some examples of how Drasi works in an observability use case. A typical team has telemetry in one set of tools, dashboards, alerts and incidents in another, and the context, deployments, the service catalog, business outcomes, in yet more. Organizations report the same issues: the tools are fragmented; alerts are noisy, yet important failures still arrive late; the context lives somewhere else, so you see symptoms, not answers; telemetry costs keep growing; healthy components don't mean customers got what they needed; and AI services can look healthy while getting it wrong. The on-call engineer is the one joining it all up." },
      { show: "wants",
        say: "What organizations want is more context, joined across systems; the signal raised above the noise; outcomes, including what didn't happen; action backed by evidence, with safeguards; all in the tools they already use." },
      { show: "wants-drasi",
        say: "We believe Drasi can provide much of this: it keeps that connected context current as data changes in the systems you already have, and delivers the answers into the tools you already use. Let me show you a few examples." },
      { show: "scenario-map",
        say: "The scenario: monitoring two AI workloads. Customer support needs 60k tokens a minute and runs on EU-A; document summaries need 80k and run on EU-B. Each deployment has 100k of capacity, and US-C in the US is extra capacity being provisioned. Support handles restricted data, which stays in Europe. Summaries' classification is under review, so until that clears, compliance keeps them in Europe too. Customers expect an answer within 15 seconds, and workload deployment changes need human approval. And the team already has its tools: OpenTelemetry, Prometheus and Grafana, Langfuse for traces, Mattermost for incidents, and PostgreSQL for the service's own data. In a real environment there'd also be ticketing, on-call paging, a service catalog and CI/CD; Drasi connects to those the same way. I won't walk through it as if I'm running the process; for each step I'll say what needs doing, show it, then show how it works." },
    ],
    ifBroken: "Carry on; the slides don't depend on the demo.",
  },
  scene1: {
    summary: "Telemetry + configuration → queries → Prometheus reaction → Grafana",
    beats: [
      { show: "ops",
        say: "Let's start simple. The team wants one view of each deployment's health and each workload's status, and that means joining telemetry with configuration data. This is their Grafana dashboard: the top row is ordinary monitoring, the bottom row is Drasi's answers.",
        watch: "Charts moving. If the desk says Traffic: stopped, press Start demo traffic in All demo controls." },
      { show: "ops", highlight: ["ai-operations.deployment-status+workload-status"], config: ["how-status"],
        say: "How do these two tiles work? Two sources: the health checks, which arrive as ordinary OpenTelemetry metrics every second, and the AI service's own database, where it keeps its deployments, workloads, routes and policy. Two continuous queries join them, and a Prometheus reaction publishes each answer as a metric, so Grafana draws it. No polling job, and no glue code.",
        watch: "The how-it-works card above Deployment status and Workload status, with an arrow to each." },
      { show: "ops", highlight: ["ai-operations.deployment-status"], config: ["deployment-health"],
        say: "Here's the first query: the latest health check per deployment, from the OpenTelemetry source. 2 means failed.",
        watch: "Deployment status: EU-A, EU-B Healthy (US-C is added in scene 3); the deployment-health query above it." },
      { show: "ops", highlight: ["ai-operations.workload-status"], config: ["tile-workloads"],
        say: "And the second joins that health with configuration: where each workload runs, and whether policy allows it there. So it says whether each workload is served somewhere healthy that it's allowed to run.",
        watch: "Workload status: Support · EU-A and Summaries · EU-B Healthy; the tile-workloads query above it." },
    ],
    ifBroken: "If the tiles are empty, traffic or Drasi isn't running: describe the how-it-works card and move on.",
  },
  scene2: {
    summary: "The absence of change: requests with no answer by their deadline (trueLater)",
    beats: [
      { show: "ops", highlight: ["ai-operations.support-overdue"], config: ["how-overdue"],
        say: "Some of the most important situations are things that didn't happen. A customer request that gets no answer within 15 seconds produces no event: no new row, no new metric. Here the source is the requests in the database; the query lists every request not yet answered, and a time condition makes Drasi re-check each one at its deadline. The answer goes to Grafana, and to the incident channel when support falls behind.",
        watch: "The how-it-works card above Support overdue." },
      { show: "ops", highlight: ["ai-operations.support-overdue"], config: ["overdue-tasks"],
        say: "This is the query. drasi.trueLater becomes true at the request's deadline, so the request joins the list at that moment with no new event, and leaves it when it's answered. The exact list of late requests, kept current request by request. TODO: a live trigger here (for example a control that slows support answers), or say we'll see it climb in scene 4.",
        watch: "Support overdue 0; the overdue-tasks query above it, the trueLater line highlighted." },
      { show: "ops", highlight: ["ai-operations.deployment-status"], config: ["deployment-health"], optional: true,
        say: "The same idea keeps Deployment status honest: each health reading is good for 8 seconds, so if the checks stop arriving, the deployment turns Unknown as time passes rather than staying green. TODO: keep this beat?" },
    ],
    ifBroken: "Describe it from the card; scene 4 shows it live.",
  },
  scene3: {
    summary: "Summaries Europe-only puts failover at risk; compliance allows the US and it's safe",
    beats: [
      { show: "ops", do: ["provision-us-c"], highlight: ["ai-operations.deployment-status"],
        say: "We've provisioned extra capacity: US-C, in the US. Watch it appear on the dashboard: one new row, and no dashboard change. But failover is still at risk: compliance won't let summaries run in the US yet.",
        watch: "TODO (live texts from the port agent): Deployment status gains US-C Healthy; Failover readiness stays At risk." },
      { show: "ops", highlight: ["ai-operations.recommended-action+failover-readiness"], config: ["how-readiness"],
        say: "While summaries' classification is under review, compliance won't let them be processed in the US. Failover readiness asks: if any one deployment failed, would every workload still have somewhere allowed to run? Right now, no, and Recommended action says add capacity. Behind it: the policy and the deployments from the database, health from OpenTelemetry, and two of Drasi's continuous models, a Rego policy and an optimization model, re-run whenever any of them changes.",
        watch: "Failover readiness: At risk: losing EU-A leaves no valid plan; Recommended action: Add capacity (both yellow); the how-it-works card above." },
      { show: "ops", highlight: ["ai-operations.recommended-action"], config: ["policy"],
        say: "The policy, as Rego: a workload may only use the deployments it's allowed, and restricted data stays in its region. The allowed destinations are data, rows in the database.",
        watch: "The Rego rules above Recommended action." },
      { show: "ops", highlight: ["ai-operations.failover-readiness"], config: ["what-if"],
        say: "And the what-if: the same optimization solved again without each healthy deployment in turn. Lose EU-A, and support and summaries would both need EU-B, which only has 100k.",
        watch: "The loss scenario above Failover readiness." },
      { show: "ops", highlight: ["ai-operations.recommended-action"], config: ["optimization"], optional: true,
        say: "The optimization model itself: one yes-or-no choice per workload and allowed deployment, every workload placed once, each healthy deployment within capacity, fewest moves first." },
      { show: "ops", do: ["summaries-anywhere"], highlight: ["ai-operations.failover-readiness", "ai-operations.recommended-action"],
        say: "Now the review clears: compliance agrees summaries can run in the US. That's one row in the policy data. Drasi re-runs the policy and the what-if at once: summaries could fail over to US-C, so we'd survive losing any one deployment, and the capacity warning goes away.",
        watch: "Failover readiness: Survives losing any one deployment (green); Recommended action: None. (Drasi also posts Failover restored in the channel.)" },
    ],
    ifBroken: "If Failover readiness doesn't turn green within about 10 seconds, say what it would show and move on.",
  },
  scene4: {
    summary: "EU-A fails; Drasi's proposal is a card in Mattermost; approve; verified; restore",
    recording: "failure",
    beats: [
      { show: "ops", do: ["fail-eu-a"], highlight: ["ai-operations.deployment-status", "ai-operations.recommended-action"],
        say: "Now EU-A, the deployment serving customer support, goes down. Nobody tells Drasi: its health checks fail, and within seconds Deployment status says Failed and Recommended action names a fix: move summaries to US-C and support to EU-B, within policy. Support overdue starts climbing too: that's scene 2's query, live.",
        watch: "Within about 8 seconds: EU-A Failed; Recommended action: Move summaries to US-C and support to EU-B (orange); Support overdue above 0." },
      { show: "incident", config: ["how-proposal"],
        click: "Scroll up to the orange Proposed change card so the whole card shows",
        say: "The team doesn't live on a dashboard. In their incident channel, Drasi posted as soon as EU-A failed, with the proposed change and the facts it was based on, before Grafana's alert arrived. How: the same policy and optimization models, with a reaction that posts the card; and when someone approves it, another reaction applies the routes through the gateway's configuration API, and a query checks it worked.",
        watch: "Red Deployment failed: EU-A, then the orange Proposed change card with Approve and Reject; the how-it-works card top right." },
      { show: "incident", do: ["approve-change"], recording: "recovery",
        click: "Approve on the newest orange Proposed change card",
        say: "I approve it here, as the on-call engineer would. Drasi checks the approval still matches its current plan, and applies the new routes.",
        watch: "Approved by @oncall; in the thread, blue Applied: Move summaries to US-C and support to EU-B." },
      { show: "ops", highlight: ["ai-operations.recommended-action", "ai-operations.support-overdue"], recording: "recovery",
        say: "Applied isn't fixed: another query waits until the backlog has cleared and support requests are answered on the new route, then says Verified. Support overdue is back to 0.",
        watch: "Recommended action: Verifying, then Verified (green); Support overdue 0." },
      { show: "ops", do: ["restore-eu-a"], highlight: ["ai-operations.decision-support"], recording: "resilience", optional: true,
        say: "Later EU-A is back at full capacity: every deployment Healthy, and we'd survive losing any one again. Nothing moves back; Drasi only proposes a move when the facts call for one.",
        watch: "All Healthy; Failover readiness: Survives losing any one deployment; no new channel card." },
    ],
    ifBroken: "If nothing changes within 30 seconds, press Use recordings from here on. If Approve doesn't respond, press Approve the proposed change on the desk; never approve twice.",
  },
  closing: {
    summary: "Plugins; run it your way; what we showed; links; the session survey, then questions",
    beats: [
      { show: "plugins",
        say: "Everything you saw was configuration over plugins. Sources, reactions and continuous models are all plugins, and there's a growing ecosystem of them: databases, streams and APIs, Kubernetes and live data feeds coming in; APIs, agents, messaging and databases going out. And you can build your own: this demo adds one, the Prometheus reaction. Continuous queries can be written in openCypher or GQL; this demo uses Cypher." },
      { show: "forms",
        say: "And you can run it your way. Embed it as a library in Rust, Node.js, Python or .NET to build change-driven applications; run Drasi Server as a process or a container, configured in YAML, which is what this demo runs on; or deploy it on Kubernetes, where each part scales on its own. TODO: confirm availability of each at Ignite." },
      { show: "closing", say: "So what did we see? Status worked out from telemetry and configuration together. Requests that got no answer, noticed at their deadline. A compliance change that made the service more resilient. And a fix, proposed within policy, approved in chat and verified. Each one is richer context from data that already existed in different places, and none of it needed an integration project: connect sources, configure continuous models, connect reactions." },
      { show: "links", say: "To learn more, start at drasi.io; the code, and this demo, are on GitHub." },
      { show: "survey", say: "Thank you. Please scan the code and tell us how we did in the session survey. Questions?",
        watch: "The survey slide with its QR code stays up during questions." },
      { show: "survey", do: ["stop-traffic"], say: "After questions, stop the demo traffic. Leave the survey slide up." },
    ],
    ifBroken: "Summarize in words. Never project the desk.",
  },
};

// Every beat's number across the whole talk (1, 2, 3, …), for referring to beats; shown on the desk
// and in PRESENTER.md, never on the projector. Renumbers itself when beats change.
export const beatNumber = (cue: number, beat: number) =>
  scenes.slice(0, cue).reduce((n, scene) => n + notes[scene.id].beats.length, 0) + beat + 1;

// A beat's script split into sentences, so the desk and PRESENTER.md start each on its own line.
export const sentences = (text: string) => text.split(/(?<=[.?!])\s+(?=["“(A-Z0-9])/);

export const glossary: readonly [string, string][] = [
  ["Deployment", "One place the AI model runs: EU-A, EU-B (Europe) or US-C (United States)"],
  ["Capacity", "How much a deployment can handle, in tokens per minute (100k each)"],
  ["Customer support / document summaries", "The two workloads: support needs 60k, summaries 80k"],
  ["Plan", "Where each workload should run. Drasi's planner proposes one; you approve it, and Drasi applies it if it's still current"],
  ["Policy", "The rules: support only in Europe; summaries in Europe at the start, then anywhere once the US is allowed"],
  ["Source", "How Drasi hears about changes; here, the database's live list of changes and the health checks' OpenTelemetry metrics"],
  ["Continuous query", "A question whose answer Drasi keeps up to date as data changes and time passes"],
  ["Continuous model", "Logic that Drasi re-runs when its inputs change: here a Rego policy, and an optimizer for the planner and the what-if"],
  ["Reaction", "Delivers each change in an answer to another system: here Prometheus, the gateway's configuration API, Grafana, Langfuse, the incident channel and the presenter desk"],
  ["Trace", "One customer request, start to finish, in Langfuse"],
];

export const questions: readonly [string, string][] = [
  ["How is this different from a Prometheus alert?", "Prometheus evaluates numbers on an interval. Drasi keeps the actual list of affected requests current as changes arrive, joins in business facts like policy and approvals, and re-checks an item at the moment a deadline passes. Its answers still go to Prometheus and alerts, as you saw."],
  ["Couldn't I just export 'age of the oldest waiting request'?", "That's one number. Drasi gives the exact set of late requests with their traces, updated request by request, without writing that logic yourself."],
  ["Isn't trueLater just a timer?", "It's a re-check of one item at one moment. The claim is no scheduled scans or recomputation, not no clock."],
  ["Grafana refreshes every 5 seconds. Isn't that polling?", "That's Prometheus and Grafana collecting and drawing, as they always do. Deciding who is late, what's allowed and which plan fits happens in Drasi as changes and deadlines arrive."],
  ["Was the alert slow on purpose?", "The alert is measured over a short window, as most are. The point isn't speed: Drasi has a plan that fits policy, not just a signal."],
  ["Does it only work with PostgreSQL?", "No. Drasi has sources for other databases, Kafka, Kubernetes, OpenTelemetry and HTTP. This demo uses two: PostgreSQL's change feed, and OpenTelemetry for the deployments' health checks."],
  ["Are the policy, planner and what-if part of Drasi?", "Yes. They're Drasi's standard policy and optimization models. The demo supplies only configuration: the Rego rules, and a placement model of workloads, deployments, constraints and objectives, solved with the open-source good_lp solver. Drasi re-runs them whenever their inputs change."],
  ["Who changes the routes?", "Drasi's reaction calls the gateway's configuration API, only for an approval that still matches Drasi's current plan. The AI service just applies the call, and refuses it if the routing configuration changed in between."],
  ["How do the answers reach Grafana?", "A Drasi reaction keeps every answer as a Prometheus metric. Prometheus scrapes it and Grafana shows it."],
  ["What happens when Drasi restarts?", "It rebuilds its state from the database and resumes the change feed. Until then the tiles show Unknown."],
  ["How does it scale?", "This demo isn't a benchmark. Each query only keeps what it needs, here the unanswered requests. For performance work, see drasi.io."],
  ["Anything else", "\"Good question. I'll follow up; drasi.io, on screen, is a good place to start.\""],
];

export const actionLabels: Record<ActionId, string> = {
  "start-traffic": "Start demo traffic", "stop-traffic": "Stop demo traffic", "fail-eu-a": "Fail EU-A",
  "approve-change": "Approve the proposed change",
  "summaries-europe": "Summaries: Europe only", "summaries-anywhere": "Summaries: Europe + US",
  "eu-a-half": "EU-A back at 50k", "eu-a-full": "EU-A back to 100k", "restore-eu-a": "Restore EU-A",
  "provision-us-c": "Provision US-C",
};
