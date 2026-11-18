import type { SceneId, StageView } from "./scenes";
import type { ConfigId } from "./codeSlides";

// The single source for what the presenter says and does. The presenter desk shows it beat by
// beat, and PRESENTER.md's run of show and script are generated from it
// (cd demo/ui && npm run sync-presenter). A test fails if PRESENTER.md drifts.

// Buttons the desk shows next to a beat. Each changes the live demo.
export type ActionId = "start-traffic" | "stop-traffic" | "fail-eu-a" | "approve-change"
  | "summaries-europe" | "summaries-anywhere" | "eu-a-half" | "eu-a-full";

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
  config?: ConfigId[];
};
export type Cue = { summary: string; beats: Beat[]; ifBroken: string; recording?: string };

export const notes: Record<SceneId, Cue> = {
  title: {
    summary: "Walk-in slide while people arrive; then introduce yourself and the talk",
    beats: [{
      show: "walkin", do: ["start-traffic"],
      say: "Before the session: the Ignite walk-in slide stays up while people arrive. Press Next when you start talking; that starts the talk timer.",
      watch: "Press Start demo traffic here, before you start talking, so the dashboard has several minutes of data by the time it appears; the desk shows Traffic: running for … Press it at least 5 minutes after any reset (the Model calls chart shows the previous run for 5 minutes after one). If traffic is already running, the button is ticked.",
    }, {
      show: "title",
      say: "Introduce yourself. Your monitoring tells you something is wrong. This talk is about getting the next answers too: who is affected, what to do, and whether it worked, worked out the moment things change, in the tools you already use.",
    }],
    ifBroken: "The slides don't depend on the demo. Carry on.",
  },
  concepts: {
    summary: "Set up the scenario and the three questions; what you'll see",
    beats: [
      { show: "concepts", say: "A company runs two AI workloads on the same limited AI model capacity: an assistant that answers customer-support questions, and a background job that writes document summaries. The capacity is split across three deployments, EU-A and EU-B in Europe and US-C in the United States, each with 100k tokens a minute." },
      { show: "concepts-rules", say: "Two kinds of rule. Data compliance: customer support handles restricted data, so it may only run in Europe; summaries use public data and can run anywhere. Business requirements: customers expect an answer within 15 seconds, and infrastructure changes, like moving work between deployments, require human approval." },
      { show: "concepts-questions", say: "When something goes wrong, the business needs three answers: who is affected, what's the fastest fix that stays within policy, and how do we know when it's resolved? You're about to see Drasi answer all three." },
      { show: "setup", say: "Here's the setup: two deployments in Europe, one in the United States, and the two workloads on them. The team already has monitoring, Prometheus and Grafana, and an incident channel. We're going to see what happens when EU-A fails, and how Drasi works with that existing observability, adding to it, to respond to events intelligently, and sooner." },
    ],
    ifBroken: "Carry on; the demo doesn't depend on the slides.",
  },
  drasi: {
    summary: "Explain Drasi's parts and what is built in",
    beats: [
      { show: "drasi", say: "Drasi is open-source change-driven computation, in three parts. Sources bring in every change as it happens: here, from the AI service's database and from its health-check telemetry. Continuous models do the work: continuous queries keep a result up to date as changes arrive, and as time passes, and policy and optimization models run logic on those results, re-running whenever an input changes. Reactions deliver each change to other systems." },
      { show: "drasi", say: "What's what. Drasi does the hard part: it keeps results up to date incrementally as changes arrive, re-checks them when time passes a deadline, and re-runs the models whenever their inputs change. The policy model that runs Rego rules and the optimizer behind the planner and the what-if are Drasi's own; this demo gives them configuration: the rules, and the placement model. We wrote one plugin, a reaction that publishes answers to Prometheus. The AI service, its change-approval tool and its gateway configuration API are the systems Drasi works with. 'Without polling' is about computing the answers: nothing is recomputed on a schedule. Health checks still run every second, as in any system, and Prometheus and Grafana still collect and draw on their own schedule; Drasi reacts to what changes." },
    ],
    ifBroken: "Carry on.",
  },
  normal: {
    summary: "Walk through the healthy dashboard",
    beats: [
      { show: "ops",
        say: "This is the AI service's Grafana dashboard: the kind of monitoring most teams already run, with metrics from real requests and alerts on them.",
        watch: "Charts moving (traffic started on the walk-in beat; if the desk says Traffic: stopped, press Start demo traffic in All demo controls)." },
      { show: "ops", highlight: ["ignite-workload.ai-deployments"],
        say: "The top row, AI Deployments, is ordinary monitoring, measured from real requests: calls, errors, answer time, the support queue and Grafana alerts. OpenTelemetry, Prometheus and Grafana; nothing new.",
        watch: "Support errors 0%, Support queue 0 to 2 (green)." },
      { show: "ops", highlight: ["ignite-workload.decision-support"],
        say: "The bottom row, Decision Support, is Drasi's answers, worked out from changes as they happen: changes in the AI service's database, where it records every request, route, policy and approval, and its health checks, which arrive as ordinary OpenTelemetry metrics every second. A Drasi reaction publishes each answer as a Prometheus metric, so Grafana draws it and you can alert on it like anything else; another posts to the team's incident channel when something needs them. Let's take them one at a time.",
        watch: "The whole Decision Support row outlined." },
      { show: "ops", highlight: ["ignite-workload.support-overdue"], config: ["overdue-tasks"],
        say: "Support overdue: customer requests past their 15-second deadline. Above it is the query that drives it: every request not yet answered or cancelled, and drasi.trueLater, which becomes true at the request's deadline. Drasi re-checks just that request at that moment, so it joins the list with no new event, and leaves it when it's answered.",
        watch: "Support overdue 0; the overdue-tasks query above it, the trueLater line highlighted." },
      { show: "ops", highlight: ["ignite-workload.deployment-status"], config: ["deployment-health"],
        say: "Deployment status: each deployment's latest health check. Every second the AI service probes each deployment and exports the result as an ordinary OpenTelemetry metric; the Collector forwards it to Drasi's OpenTelemetry source, and this query keeps the latest probe per deployment: 2 means failed. Each reading is good for 8 seconds, so if the probes stop, the status turns Unknown as time passes.",
        watch: "Deployment status: EU-A, EU-B, US-C Healthy; the deployment-health query above it." },
      { show: "ops", highlight: ["ignite-workload.workload-status"], config: ["tile-workloads"],
        say: "Workload status joins three things: where each workload runs, what the policy says about that deployment, and that deployment's health. So it tells you whether each workload is being served somewhere it's allowed to run.",
        watch: "Workload status: Support · EU-A and Summaries · EU-B Healthy; the tile-workloads query above it." },
      { show: "ops", highlight: ["ignite-workload.recommended-action"], config: ["policy"],
        say: "Recommended action comes from two of Drasi's standard models. First the policy, as these Rego rules: a workload may only use the deployments it's allowed, and restricted data must stay in its region. The allowed destinations are data, so a compliance change is just a change to some rows, and Drasi re-checks every workload and deployment when one changes.",
        watch: "Recommended action: None (green); the Rego rules above it." },
      { show: "ops", highlight: ["ignite-workload.recommended-action"], config: ["optimization"],
        say: "Then the optimization model, configured by this file: one yes or no choice per workload and allowed deployment, every workload placed exactly once, each healthy deployment within its capacity, and the fewest moves first. Drasi re-solves whenever health, capacity or policy changes, proposes the result, and follows it through approval, applying and verification. Right now there's nothing to do.",
        watch: "Recommended action: None (green); the optimization model above it." },
      { show: "ops", highlight: ["ignite-workload.failover-readiness"], config: ["what-if"],
        say: "Failover readiness is a what-if from the same model: the loss scenario solves again without each healthy deployment in turn. If any one failed, would every workload still have somewhere allowed to run? A game-day question, answered all the time. No new tool to watch: the answers arrive where you already look.",
        watch: "Failover readiness: Survives losing any one deployment; the scenario above it." },
    ],
    ifBroken: "If the charts are empty, traffic isn't running: press Start demo traffic and keep talking.",
  },
  failure: {
    summary: "Fail EU-A; Drasi's row and the incident channel are ahead of the alert",
    recording: "failure",
    beats: [
      { show: "ops", do: ["fail-eu-a"], highlight: ["ignite-workload.deployment-status"],
        say: "Now I'll make EU-A, the deployment serving customer support, fail.",
        watch: "Within about 8 seconds: Deployment status EU-A Failed, and Recommended action names a change." },
      { show: "ops", highlight: ["ignite-workload.deployment-status", "ignite-workload.recommended-action"],
        say: "Look at the bottom row first, Decision Support: EU-A Failed, and Recommended action already names a fix: move summaries to US-C and support to EU-B. Nobody told Drasi EU-A was down: its health probes failed, and the deployment-health query you saw turned that into Failed. And it has worked out a change that fits the rules. We'll come back to it.",
        watch: "Recommended action: Move summaries to US-C and support to EU-B (orange)." },
      { show: "incident",
        say: "The team doesn't live on a dashboard, though. This is their incident channel in Mattermost. Drasi posted here as soon as EU-A failed, and its proposed change is already waiting, with an Approve button. Grafana hasn't alerted yet. We'll come back to the proposal.",
        watch: "Red card Deployment failed: EU-A, then the orange Proposed change card with Approve and Reject, the newest at the bottom." },
      { show: "ops", highlight: ["ignite-workload.ai-deployments"],
        say: "Back on the dashboard, the top half catches up: calls fail, errors and the queue climb, and Grafana's alert fires; it posts to the channel too. Something is wrong.",
        watch: "Support errors red, Support queue rising; Alerts firing lists Model calls failing, then Support requests overdue (Drasi), an alert on Drasi's answer." },
      { show: "ops", highlight: ["ignite-workload.support-overdue", "ignite-workload.support-waiting"],
        say: "And Support overdue is climbing. Each request has a 15-second deadline. When a request passes it, nothing happens to that request: no new row, no new metric. Drasi counts it at that exact moment anyway, because its query re-checks each request at its deadline. On the chart, the red overdue line follows the orange waiting line 15 seconds later. Nothing re-scans the database: Drasi reacts to each change and each deadline, which is why its proposal was in the channel before Grafana's alert fired.",
        watch: "Support overdue above 0; on the right-hand chart, the red line trails the orange one." },
    ],
    ifBroken: "If nothing changes within 30 seconds, press Use recordings from here on and say it's a recording. If the channel shows no cards, say Drasi posts them there and stay on the dashboard: the same answers are on the Decision Support row.",
  },
  who: {
    summary: "Show who is affected: the overdue card, the query and its live list",
    beats: [
      { show: "incident",
        click: "If the red Support requests overdue card isn't in view, scroll up to it (Grafana's alerts may have posted below it)",
        say: "Which customers are affected? Drasi posted that too: Support requests overdue, as soon as the first support request passed its deadline, with a link to the tile on the dashboard. A count tells you it's bad. Let's see who.",
        watch: "Red card Support requests overdue, with the count when posted and a link to Operations › Support overdue." },
      { show: "query", say: "This is the Support overdue query from earlier, now with its live result: not just a count, but the exact list of late requests with their traces, kept current request by request. With Prometheus you'd get a number from a gauge or a rule evaluated every so often; here it's the set itself, from one query",
        watch: "Under the query: how many support requests are late, and the oldest few with their traces." },
      { show: "request", optional: true, say: "Here's one of those customers: their question, every attempt to send it to EU-A failing, and the moment its deadline passed and Drasi listed it. The same request is a trace in Langfuse, so whoever picks it up has the detail.",
        watch: "The oldest late request, with its attempts and the moment Drasi listed it." },
    ],
    ifBroken: "If the list is empty, traffic has stopped: say what it would show. The request beat is skipped automatically when nothing is late.",
  },
  recovery: {
    summary: "Approve Drasi's proposed change on its card; optionally show an out-of-date proposal rejected",
    recording: "recovery",
    beats: [
      { show: "incident",
        click: "Scroll up to the orange Proposed change card so the whole card shows",
        say: "Back to the proposal. On paper there's plenty of room: two healthy deployments with 200k, and we need 140k. But US-C has the free room, and customer support isn't allowed there: restricted data stays in Europe. The planner, an optimizer Drasi re-runs on every change, respects that: move summaries to US-C, which frees EU-B for support. Two moves, both within policy, and the card shows the facts it was based on: that's the Rego policy and the optimization model you saw earlier, re-solved on the new facts.",
        watch: "The orange Proposed change card: Move summaries to US-C and support to EU-B. Why EU-A is down; Policy; Capacity; Based on revision and plan." },
      { show: "incident", optional: true, recording: "policy", do: ["summaries-europe"],
        say: "Before anyone approves it, compliance restricts summaries to Europe too. Drasi re-runs the policy and the planner at once: with EU-A down, support and summaries can't both fit in EU-B, so no change fits now, and Drasi says the proposal is out of date. Dropping work is a decision for a person, not the planner.",
        watch: "A grey card Out of date: the facts changed, Now: No valid plan. The proposal above it still has its Approve button." },
      { show: "incident", optional: true, recording: "policy", do: ["approve-change"], expect: "rejected",
        click: "Approve on the orange proposal card, above the grey Out of date card (scroll up a little if needed)",
        say: "Suppose someone approves the proposal anyway. The approval goes to the change-approval tool, and Drasi rejects it: every proposal carries the facts it was based on, and those facts changed. Nothing is changed.",
        watch: "The card says Approved by @oncall; then, in its thread, Rejected by Drasi: nothing was changed, with Drasi's reason." },
      { show: "incident", optional: true, recording: "policy", do: ["summaries-anywhere"],
        say: "Compliance allows the US for summaries again, and Drasi posts a new proposal straight away. To Drasi, a policy change is just another change: it re-runs the policy and the planner every time.",
        watch: "A new orange Proposed change card at the bottom, with a new plan in Based on." },
      { show: "incident", do: ["approve-change"],
        click: "Approve on the newest orange Proposed change card, at the bottom",
        say: "Now I approve the current proposal, here on its card, as the on-call engineer would. The approval goes to the change-approval tool; Drasi checks it still matches its current plan, and its reaction applies the new routes through the gateway's configuration API.",
        watch: "The card says Approved by @oncall at …; in its thread, blue Applied: Move summaries to US-C and support to EU-B." },
      { show: "ops", highlight: ["ignite-workload.recommended-action"],
        say: "Look at Recommended action: it says Verifying. Applied isn't the same as fixed: another Drasi query waits until the backlog from before the change has cleared and at least three support requests have been answered on the new route. Only then does it say Verified.",
        watch: "Recommended action goes from Verifying: … to Verified: … (green), often within 30 seconds of approval. If it's already Verified, say so." },
      { show: "ops", highlight: ["ignite-workload.ai-deployments", "ignite-workload.support-overdue"],
        say: "The rest of the dashboard agrees: the queue has drained, Support overdue is back to 0 and errors are falling. EU-A is still down: we routed around it, within policy.",
        watch: "Support errors falling, Support queue back to 0 to 2, Support overdue 0." },
      { show: "incident",
        say: "And the channel has the whole story in one thread: proposed, approved by the on-call engineer, applied, and verified by Drasi from the evidence. Approving a change isn't the same as fixing the problem; Drasi checked the outcome.",
        watch: "Green Verified: support has recovered, in the approved card's thread." },
    ],
    ifBroken: "If Approve on the card doesn't respond, press Approve the proposed change on the desk: it presses the same button. Never approve twice. If Recommended action stays Verifying, say it's applied but not yet verified, and carry on. If the proposal or the policy change doesn't behave as described, press Use recordings from here on.",
  },
  resilience: {
    summary: "Healthy but fragile: EU-A returns with half its capacity",
    recording: "resilience",
    beats: [
      { show: "ops", do: ["eu-a-half"], highlight: ["ignite-workload.deployment-status", "ignite-workload.failover-readiness"],
        say: "Later that day EU-A is fixed, but it comes back with only 50k of capacity. Nothing is broken: every deployment is healthy, every service-health signal is green. But Failover readiness says No.",
        watch: "Deployment status all Healthy, EU-A 50k. Failover readiness: No, not if EU-B fails. Recommended action: Add capacity: losing EU-B leaves support nowhere to run." },
      { show: "incident",
        say: "Drasi posted it in the channel, while everything is green. Support now runs on EU-B. If EU-B failed, support would need 60k in Europe, and EU-A has only 50k. Nobody would notice until EU-B fell over.",
        watch: "Orange card Failover at risk: losing EU-B leaves support nowhere to run." },
      { show: "ops", highlight: ["ignite-workload.failover-readiness"],
        say: "That's the what-if model: for each deployment, would every workload still fit somewhere allowed if that one failed? It re-asks whenever anything changes. A game-day question, answered all the time. And it's a metric, so you can alert on it." },
      { show: "incident", do: ["eu-a-full"],
        say: "Give EU-A its full capacity back, and Drasi says we're safe again, without any outage.",
        watch: "Green card Failover restored. (On the dashboard, Failover readiness: Yes, and Recommended action: None.)" },
      { show: "drasi-resilience", say: "One capacity change, and Drasi re-checked every what-if. Are we safe from the next failure? Answered before it happens." },
    ],
    ifBroken: "If Failover readiness doesn't change, press Use recordings from here on. If the channel shows no card, stay on the dashboard: it shows the same answer.",
  },
  closing: {
    summary: "Four takeaways, the session survey, then questions",
    beats: [
      { show: "closing", say: "Keep your monitoring tools, and add change-driven computation next to them. Drasi turned changing facts (requests, health, capacity, policy, approvals and time) into live answers: who is late, what fits the rules, whether the fix worked, and whether we're safe from the next failure. To learn more, go to drasi.io, on screen." },
      { show: "survey", say: "Thank you. Please scan the code and tell us how we did in the session survey. Questions?",
        watch: "The survey slide with its QR code stays up during questions." },
      { show: "survey", do: ["stop-traffic"], say: "After questions, stop the demo traffic. Leave the survey slide up." },
    ],
    ifBroken: "Summarize in words. Never project the desk.",
  },
};

export const glossary: readonly [string, string][] = [
  ["Deployment", "One place the AI model runs: EU-A, EU-B (Europe) or US-C (United States)"],
  ["Capacity", "How much a deployment can handle, in tokens per minute (100k each)"],
  ["Customer support / document summaries", "The two workloads: support needs 60k, summaries 80k"],
  ["Plan", "Where each workload should run. Drasi's planner proposes one; you approve it, and Drasi applies it if it's still current"],
  ["Policy", "The rules: support only in Europe; summaries anywhere"],
  ["Source", "How Drasi hears about changes; here, the database's live list of changes and the health checks' OpenTelemetry metrics"],
  ["Continuous query", "A question whose answer Drasi keeps up to date as data changes and time passes"],
  ["Continuous model", "Logic that Drasi re-runs when its inputs change: here a Rego policy, and an optimizer for the planner and the what-if"],
  ["Reaction", "Delivers each change in an answer to another system: here Prometheus, the gateway's configuration API, Grafana, Langfuse and the presenter desk"],
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
  ["Who changes the routes?", "Drasi's reaction calls the gateway's configuration API, only for an approval that still matches Drasi's current plan. The AI service just applies the call, and refuses it if the scenario changed in between."],
  ["How do the answers reach Grafana?", "A Drasi reaction keeps every answer as a Prometheus metric. Prometheus scrapes it and Grafana shows it."],
  ["What happens when Drasi restarts?", "It rebuilds its state from the database and resumes the change feed. Until then the tiles show Unknown."],
  ["How does it scale?", "This demo isn't a benchmark. Each query only keeps what it needs, here the unanswered requests. For performance work, see drasi.io."],
  ["Anything else", "\"Good question. I'll follow up; drasi.io, on screen, is a good place to start.\""],
];

export const actionLabels: Record<ActionId, string> = {
  "start-traffic": "Start demo traffic", "stop-traffic": "Stop demo traffic", "fail-eu-a": "Fail EU-A",
  "approve-change": "Approve the proposed change",
  "summaries-europe": "Summaries: Europe only", "summaries-anywhere": "Summaries: Europe + US",
  "eu-a-half": "EU-A back at 50k", "eu-a-full": "EU-A back to 100k",
};
