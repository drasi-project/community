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
        say: "Before the session: leave the walk-in slide up while people arrive. Opening the projector window starts the talk timer at 00:00 (Restart timer resets it). Press Next when you start talking.",
        watch: "Press Start demo traffic before you start talking, so the dashboard has several minutes of data by scene 1. Press it at least 5 minutes after any reset." },
      { show: "title",
        say: "Good morning, my name is Allen Jones. I'm a member of the Azure Incubations team at Microsoft." },
      { show: "project",
        say: "For the last few years I've been working on an open-source project called Drasi. Drasi is a general-purpose platform for building change-driven systems. We started Drasi in Azure Incubations and open-sourced it in October 2024. In January 2025 it became a CNCF Sandbox project, and it's in active development with a growing community of users and contributors." },
      { show: "ops",
        say: "Although Drasi is domain agnostic, the nature of the observability domain makes it particularly applicable. So today I am going to show you some examples of how Drasi can help with the observability challenges faced by organizations, and what I show you will be based around this Grafana dashboard, which will be familiar to many people.",
        watch: "The live dashboard, charts moving; no highlight." },
      { show: "ops", highlight: ["ai-operations.ai-deployments"], config: ["label-standard"],
        say: "The top section shows the kind of telemetry most organizations already have and use: OpenTelemetry metrics, collected by Prometheus, and displayed here in Grafana.",
        watch: "The top row outlined, labelled The basics." },
      { show: "ops", highlight: ["ai-operations.decision-support"], config: ["label-drasi"],
        say: "The lower section shows higher-level, higher-value information that uses richer context to provide in situ answers and guidance to people, and this is powered by Drasi. I will talk about each of these tiles in a moment, but first I will tell you a little more about Drasi.",
        watch: "The Decision Support row outlined, labelled Richer context, answers, guidance · Powered by Drasi." },
      { show: "problem",
        say: "We started Drasi because detecting meaningful change as it happens, and reacting to it, is harder than it should be. Teams usually poll: query everything every few seconds, diff with last time, and write code to react. It's simple, but slow, wasteful, and it misses changes in between. Or they use stream processing: every event arrives, but working out what it means for the current state, joining other sources, and noticing what didn't happen means building and running your own state: windows, joins, state stores, late events. Each of these has its place and can be the correct approach under certain circumstances, but for detecting specific meaningful change as it happens and being able to respond, we wanted to make it easier." },
      { show: "change-driven",
        say: "Drasi observes low-level change feeds. You describe the result you want as a query, and Drasi keeps that result current as the data changes, and as time passes, across sources. A reaction then acts on just what changed in the result: what was added, updated or deleted. No polling loop, and no state store to build and run yourself." },
    ],
    ifBroken: "The slides don't depend on the demo. Carry on.",
  },
  drasi: {
    summary: "Sources, Continuous Queries, Reactions, and queries building on queries",
    beats: [
      { show: "drasi-1",
        say: "Drasi is built on three simple concepts: Sources, Continuous Queries and Reactions. Sources bring in low-level changes, here from a database, telemetry and Kubernetes. A continuous query describes what you care about across them in one graph query; Drasi does the joining and keeps the result current. A reaction acts on changes to that result, not the sources." },
      { show: "drasi-2",
        say: "It scales out from there: many sources, many queries, many reactions, each result kept current as the data changes." },
      { show: "drasi-3",
        say: "And, coming soon, queries and models can build on each other's results: a computation graph. Here, health checks feed deployment-health, which feeds workload status and the plan; policy input feeds the Rego policy, which feeds the plan too; and the results fan out to several reactions. When one input changes, only what depends on it updates." },
    ],
    ifBroken: "Carry on; the slides don't depend on the demo.",
  },
  observability: {
    summary: "Observability: the issues, what organizations want, and the scenario",
    beats: [
      { show: "issues",
        say: "Back to observability. Most organizations understand why it matters, but they report the same issues: fragmented tools; noisy alerts, yet real failures noticed late; context that lives elsewhere, so you see symptoms, not answers; rising telemetry costs; green components while customers suffer; and AI services that look healthy while getting it wrong. Despite all the investment, customers often report issues first, and on-call engineers are left connecting the dots." },
      { show: "wants",
        say: "What organizations want is context, joined across systems, that answers: who's affected, what changed, what should we do, and did it work. Situations, not signals; outcomes, including what didn't happen; evidence-backed action, with approval; all in the tools they already use." },
      { show: "wants-drasi",
        say: "We believe Drasi has a role to play here: bringing together that context from disparate systems, keeping it current as things change, and making it available where it is needed most." },
      { show: "scenario-map",
        say: "So imagine an organization with two important AI workloads. Customer support needs 60k tokens a minute, on EU-A; document summaries need 80k, on EU-B. Each deployment has 100k of capacity, and US-C, in the US, is being provisioned. Support's data is restricted and must stay in Europe; summaries' classification is under review, so compliance keeps them in Europe too. The SLO: customers get an answer within 15 seconds. And changes to where workloads run need human approval. The tools are OpenTelemetry, Prometheus and Grafana, Langfuse for traces, Mattermost for incidents and PostgreSQL. A real environment would add ticketing, paging, a service catalog and CI/CD; Drasi connects to those the same way. For each scene: what's needed, shown live, then how it works." },
    ],
    ifBroken: "Carry on; the slides don't depend on the demo.",
  },
  scene1: {
    summary: "Telemetry + configuration → queries → Prometheus reaction → Grafana",
    beats: [
      { show: "ops",
        say: "Back to the dashboard, starting simple. The charts are moving because there's simulated traffic running locally. The team wants one view of each deployment's health and each workload's status, and that means joining telemetry with configuration data.",
        watch: "Charts moving. If the desk says Traffic: stopped, press Start demo traffic in All demo controls." },
      { show: "ops", highlight: ["ai-operations.deployment-status+workload-status"], config: ["how-status"],
        say: "How do these tiles work? Health checks arrive as OpenTelemetry metrics every second; deployments, workloads, routes and policy come from the AI service's database. Two continuous queries, deployment-health and tile-workloads, join them, and a Prometheus reaction publishes each result as a metric for Grafana. No polling job, no glue code.",
        watch: "The how-it-works card above one outline around Deployment status and Workload status." },
      { show: "ops", do: ["provision-us-c"], highlight: ["ai-operations.deployment-status"], config: ["deployment-health"],
        say: "Here's deployment-health: the latest health check per deployment, from the OpenTelemetry source. A value of 2 means failed. Now we've just provisioned extra capacity: US-C, in the US. Watch it appear in Deployment status within a second, with no dashboard or query change: Drasi picks it up the moment the new deployment record appears, joined with its health checks.",
        watch: "EU-A, EU-B Healthy; within a second of pressing, a US-C Healthy row appears. Workload status is unchanged (no workload runs on US-C). No channel card." },
      { show: "ops", highlight: ["ai-operations.workload-status"], config: ["tile-workloads"],
        say: "And tile-workloads joins that health with configuration: where each workload runs, and whether policy allows it there. So it tells you whether each workload is served somewhere healthy and allowed.",
        watch: "Workload status: Support · EU-A and Summaries · EU-B Healthy; the tile-workloads query above it." },
    ],
    ifBroken: "If the tiles are empty, traffic or Drasi isn't running: describe the how-it-works card and move on. If US-C doesn't appear within a few seconds, carry on; scene 3 needs it, so check the desk shows Provision US-C ✓ before then.",
  },
  scene2: {
    summary: "The absence of change: requests with no answer by their deadline (trueLater)",
    beats: [
      { show: "ops", highlight: ["ai-operations.support-overdue"], config: ["how-overdue"],
        say: "Some of the most important situations are things that didn't happen. A request that breaches the 15-second SLO produces no event. The overdue-tasks query lists every unanswered request, and a time condition makes Drasi re-check each one at its deadline. The result goes to Grafana, and to the incident channel when support falls behind.",
        watch: "The how-it-works card above Support overdue." },
      { show: "ops", highlight: ["ai-operations.support-overdue"], config: ["overdue-tasks"],
        say: "Here's overdue-tasks. drasi.trueLater becomes true at the request's deadline, so the request joins the result at that moment, with no new event, and leaves it when it's answered. It's the exact list of late requests, not a sampled count. It's zero now; you'll see it climb in scene 4.",
        watch: "Support overdue 0; the overdue-tasks query above it, the trueLater line highlighted." },
    ],
    ifBroken: "Describe it from the card; scene 4 shows it live.",
  },
  scene3: {
    summary: "Summaries Europe-only puts failover at risk; compliance allows the US and it's safe",
    beats: [
      { show: "ops", highlight: ["ai-operations.recommended-action+failover-readiness"], config: ["how-readiness"],
        say: "US-C is provisioned, yet failover is still at risk: compliance won't let summaries run in the US. Failover readiness asks: if any one deployment failed, would every workload still have somewhere allowed to run? Right now, no, so Recommended action says add capacity. Behind both: two continuous models, a Rego policy and an optimization plan, re-run whenever policy, deployments or health change.",
        watch: "Failover readiness: At risk: losing EU-A leaves no valid plan; Recommended action: Add capacity (both yellow); the how-it-works card above." },
      { show: "ops", highlight: ["ai-operations.recommended-action"], config: ["policy"],
        say: "The policy, in Rego: a workload may only use the deployments it's allowed, and restricted data stays in its region. The allowed destinations are just rows in the database.",
        watch: "The Rego rules above Recommended action." },
      { show: "ops", highlight: ["ai-operations.failover-readiness"], config: ["what-if"],
        say: "And the what-if: the same optimization, solved again without each healthy deployment in turn. Lose EU-A, and support and summaries would both need EU-B, which only has 100k.",
        watch: "The loss scenario above Failover readiness." },
      { show: "ops", highlight: ["ai-operations.recommended-action"], config: ["optimization"], optional: true,
        say: "The optimization model itself: one yes-or-no choice per workload and allowed deployment, every workload placed once, each healthy deployment within capacity, fewest moves first." },
      { show: "ops", do: ["summaries-anywhere"], highlight: ["ai-operations.failover-readiness", "ai-operations.recommended-action"],
        say: "Now the review clears, and compliance allows summaries in the US: one row in the policy data. Drasi re-runs the policy and the what-if at once: summaries could fail over to US-C, so we'd survive losing any one deployment, and the capacity warning clears.",
        watch: "Failover readiness: Survives losing any one deployment (green); Recommended action: None. (Drasi also posts Failover restored in the channel.)" },
    ],
    ifBroken: "If Summaries: Europe + US is disabled, press Provision US-C first (it's in All demo controls). If Failover readiness doesn't turn green within about 10 seconds, say what it would show and move on.",
  },
  scene4: {
    summary: "EU-A fails; Drasi's proposal is a card in Mattermost; approve; verified; restore",
    recording: "failure",
    beats: [
      { show: "ops", do: ["fail-eu-a"], highlight: ["ai-operations.deployment-status", "ai-operations.recommended-action"],
        say: "Now EU-A, which serves customer support, goes down. Nobody tells Drasi: the health checks fail, Deployment status shows Failed within seconds, and Recommended action proposes a fix within policy: move summaries to US-C and support to EU-B. Support overdue starts climbing: scene 2's query, live.",
        watch: "Within about 8 seconds: EU-A Failed; Recommended action: Move summaries to US-C and support to EU-B (orange); Failover readiness: Already degraded; Support overdue above 0 (needs demo traffic)." },
      { show: "incident", config: ["how-proposal"],
        click: "Scroll up to the orange Proposed change card so the whole card shows",
        say: "On-call engineers don't watch dashboards all day. Drasi posted in the incident channel as soon as EU-A failed, with the proposed change and the facts behind it, before Grafana's alert. A reaction posts the card; once it's approved, another applies the routes through the gateway's configuration API, and the recovery-verified query checks it worked.",
        watch: "Red Deployment failed: EU-A, then the orange Proposed change card with Approve and Reject; the how-it-works card top right." },
      { show: "incident", do: ["approve-change"], recording: "recovery",
        click: "Approve on the newest orange Proposed change card",
        say: "I approve it here, as the on-call engineer would: that's the change approval. Drasi checks the approval still matches its current plan, then applies the new routes.",
        watch: "Approved by @oncall; in the thread, blue Applied: Move summaries to US-C and support to EU-B." },
      { show: "ops", highlight: ["ai-operations.recommended-action", "ai-operations.support-overdue"], recording: "recovery",
        say: "Applied isn't fixed. Drasi waits until the backlog has cleared and support requests are being answered on the new route, then marks it Verified. Support overdue is back to zero.",
        watch: "Recommended action: Verifying, then Verified (green); Workload status: Support · EU-B, Summaries · US-C; Support overdue 0. In the channel thread: Verified: support has recovered. Verified needs demo traffic." },
      { show: "ops", do: ["restore-eu-a"], highlight: ["ai-operations.decision-support"], recording: "resilience", optional: true,
        say: "Later, EU-A is restored: every deployment is healthy, and we'd survive losing any one again. Nothing moves back; Drasi only proposes a change when the facts call for one.",
        watch: "All Healthy; Failover readiness: Survives losing any one deployment; no new channel card." },
    ],
    ifBroken: "If nothing changes within 30 seconds, press Use recordings from here on. If Approve doesn't respond, press Approve the proposed change on the desk; never approve twice. If it stays Verifying, demo traffic has stopped: press Start demo traffic.",
  },
  closing: {
    summary: "What you saw; form factors; extensibility; thank you; learn more; the session survey, then questions",
    beats: [
      { show: "closing", say: "So I hope you can see that Drasi has a valuable role to play in observability. We saw status built from telemetry and configuration; requests that missed their deadline, noticed at that moment; one policy change re-checking resilience at once; and a fix proposed within policy, approved in chat and verified. All by connecting sources, configuring continuous queries and models, and connecting reactions. But remember: Drasi is domain-agnostic, a platform for change-driven solutions in any domain." },
      { show: "forms",
        say: "With that in mind, Drasi comes in several form factors: embed drasi-lib to build change-driven applications; run Drasi Server as a process or a container, configured in YAML, as this demo does; or run Drasi for Kubernetes, where each part scales on its own. TODO: confirm the Node.js, Python and .NET bindings for Ignite; drasi.io lists drasi-lib as a Rust crate." },
      { show: "plugins",
        say: "Drasi is also highly extensible. Sources, reactions and continuous models are all plugins, with a growing ecosystem coming in and going out, and you can build your own: this demo adds a Prometheus reaction. Queries can be written in openCypher or GQL." },
      { show: "thanks", say: "And with that, thank you for your time." },
      { show: "links", say: "Here is a list of ways you can learn more." },
      { show: "survey", say: "Please visit the session survey and give us your feedback, so we can improve future sessions. Questions?",
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
  ["Source", "A connection to a system Drasi observes for changes; here, PostgreSQL's change feed and the health checks' OpenTelemetry metrics"],
  ["Continuous query", "A query whose result set Drasi keeps current as data changes and as time passes; written in openCypher or GQL"],
  ["Continuous model", "Logic that Drasi re-runs when its inputs change: here a Rego policy, and an optimizer for the planner and the what-if"],
  ["Reaction", "Receives a query's result changes (added, updated, deleted) and acts on them: here Prometheus, the gateway's configuration API, Grafana, Langfuse, the incident channel and the presenter desk"],
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
