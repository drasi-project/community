import type { SceneId, StageView } from "./scenes";

// The single source for what the presenter says and does. The presenter desk shows it beat by
// beat, and PRESENTER.md's run of show and script are generated from it
// (cd demo/ui && npm run sync-presenter). A test fails if PRESENTER.md drifts.

// Buttons the desk shows next to a beat. Each changes the live demo.
export type ActionId = "start-traffic" | "stop-traffic" | "fail-eu-a" | "select-plan" | "approve-plan"
  | "summaries-europe" | "summaries-anywhere" | "eu-a-half" | "eu-a-full";

export type Beat = {
  show: StageView;     // what the projector shows during this beat (Next switches it)
  say: string;         // what to say, in your own words
  do?: ActionId[];     // the buttons to press, in order, before or while saying it
  watch?: string;      // what appears on screen
  optional?: true;     // skipped when "Short on time" is ticked
  expect?: "rejected"; // the approval is meant to be rejected
  recording?: string;  // the recording shown instead in recording mode (default: the part's)
};
export type Cue = { summary: string; beats: Beat[]; ifBroken: string; recording?: string };

export const notes: Record<SceneId, Cue> = {
  title: {
    summary: "Introduce yourself and the talk",
    beats: [{
      show: "title",
      say: "Introduce yourself. Your monitoring tells you something is wrong. This talk is about getting the next answers too: who is affected, what to do, and whether it worked, worked out the moment things change, in the tools you already use.",
    }],
    ifBroken: "The slides don't depend on the demo. Carry on.",
  },
  concepts: {
    summary: "Set up the scenario and the three questions",
    beats: [
      { show: "concepts", say: "A company runs two AI workloads on the same limited AI model capacity: an assistant that answers customer-support questions, and a background job that writes document summaries. The capacity is split across three deployments, EU-A and EU-B in Europe and US-C in the United States, each with 100k tokens a minute." },
      { show: "concepts", say: "Two rules. Customer support handles restricted data, so it may only run in Europe; summaries use public data and can run anywhere. Customers expect an answer within 15 seconds, and moving work between deployments needs a person's approval." },
      { show: "concepts", say: "When EU-A fails, the business needs three answers: who is affected right now, what's the fastest fix that stays within policy, and how do we know it's really fixed? You're about to see Drasi answer all three." },
    ],
    ifBroken: "Carry on; the demo doesn't depend on the slides.",
  },
  drasi: {
    summary: "Explain Drasi's parts and what is built in",
    beats: [
      { show: "drasi", say: "Drasi is open-source change-driven computation. Sources bring in every change as it happens: here, from the AI service's database. Continuous queries keep a result up to date as changes arrive, and as time passes. Continuous models run logic such as policies and planners on those results and re-run it whenever an input changes. Reactions deliver each change to other systems." },
      { show: "drasi", say: "What's what. Drasi does the hard part: it keeps results up to date incrementally as changes arrive, re-checks them when time passes a deadline, and re-runs the models whenever their inputs change. For this demo we wrote three plugins: a policy model that runs Rego rules, an optimizer for the planner and the what-if, and a reaction that publishes answers to Prometheus. The rules and settings are their configuration. The AI service, its change-approval tool and its gateway configuration API are the systems Drasi works with. 'Without polling' is about computing the answers: nothing is recomputed on a schedule. Health checks still run every second, as in any system, and Prometheus and Grafana still collect and draw on their own schedule; Drasi reacts to what changes." },
    ],
    ifBroken: "Carry on.",
  },
  normal: {
    summary: "Walk through the healthy dashboard",
    beats: [
      { show: "ops", do: ["start-traffic"],
        say: "This is the AI service's Grafana dashboard. The top half is ordinary monitoring, measured from real requests: calls, errors, answer time, the support queue and Grafana alerts. OpenTelemetry, Prometheus and Grafana; nothing new.",
        watch: "Charts moving. Support errors 0%, Support queue 0 to 2 (green). (Start demo traffic only if it isn't already running.)" },
      { show: "ops",
        say: "The bottom half is Drasi's answers, worked out from changes in the AI service's database rather than from metrics. The service records every request and attempt, and checks each deployment's health every second. Drasi turns that into: overdue support requests, each deployment's status, recovery, what action is needed, and whether we'd survive losing any one deployment. A Drasi reaction publishes them as ordinary Prometheus metrics, so you can alert on them like anything else.",
        watch: "Support overdue 0, all deployments Healthy, Recovery Nothing to verify, Action needed None, Safe if one fails? Yes." },
      { show: "drasi-healthy",
        say: "So far Drasi has read every change in the database, kept its queries current, run the policy, planner and what-if, and its reaction has published the answers for Grafana. Nothing new to watch: the answers sit where you already look." },
    ],
    ifBroken: "If the charts are empty, traffic isn't running: press Start demo traffic and keep talking.",
  },
  failure: {
    summary: "Fail EU-A",
    recording: "failure",
    beats: [
      { show: "ops", do: ["fail-eu-a"], say: "Now I'll make EU-A, the deployment serving customer support, fail.",
        watch: "Within about 8 seconds: Deployment status EU-A Failed, and Action needed Approve proposed plan." },
      { show: "ops", say: "Look at the bottom row first: EU-A Failed, and Action needed already says Approve proposed plan. Drasi has worked out a fix that fits the rules. We'll come back to it." },
      { show: "ops", say: "The top half does its job: calls fail, errors and the queue climb, and Grafana's alert fires. Something is wrong.",
        watch: "Support errors red, Support queue rising, Grafana alerts firing 1 or 2 (the second is an alert on Drasi's overdue answer)." },
      { show: "ops", say: "And Support overdue is climbing. Each request has a 15-second deadline. When a request passes it, nothing happens to that request: no new row, no new metric. Drasi counts it at that exact moment anyway, because its query re-checks each request at its deadline. On the chart, the red overdue line follows the orange waiting line 15 seconds later. I'll show you how in a moment.",
        watch: "Support overdue above 0; on the right-hand chart, the red line trails the orange one." },
      { show: "drasi-failure", say: "Drasi saw the failed attempts and EU-A's health turn to Failed, worked out which requests were late as each deadline passed, and re-ran the planner. It didn't re-scan the database; it reacted to each change and each deadline. And notice the order: Drasi had a plan ready before Grafana's alert fired." },
    ],
    ifBroken: "If nothing changes within 30 seconds, press Use recordings from here on and say it's a recording.",
  },
  who: {
    summary: "Show who is affected: the query and its live list",
    beats: [
      { show: "query", say: "Which customers are affected? This is the actual query behind Support overdue. 1: every request, as it arrives from the database. 2: not answered yet. 3: this condition becomes true at the deadline, and Drasi re-checks just that request then. 4: the result is the list itself, not just a count, and here it is, live. With Prometheus you'd get a number from a gauge or a rule evaluated every so often; here it's the exact set of late requests, kept current request by request, from one query.",
        watch: "Under the query: how many support requests are late, and the oldest few with their traces." },
      { show: "request", optional: true, say: "Here's one of those customers: their question, every attempt to send it to EU-A failing, and the moment its deadline passed and Drasi listed it. The same request is a trace in Langfuse, so whoever picks it up has the detail.",
        watch: "The oldest late request, with its attempts and the moment Drasi listed it." },
    ],
    ifBroken: "If the list is empty, traffic has stopped: say what it would show. The request beat is skipped automatically when nothing is late.",
  },
  recovery: {
    summary: "Approve the proposed plan; optionally show a stale approval rejected",
    recording: "recovery",
    beats: [
      { show: "plan", say: "This dashboard is all Drasi. Capacity: 300k in total, 200k on healthy deployments, 140k needed. On paper there's plenty. But US-C has the free room, and customer support isn't allowed there.",
        watch: "Customer support may run on: US-C Not allowed." },
      { show: "plan", do: ["select-plan"], say: "The planner, an optimizer Drasi re-runs on every change, respects that: move summaries to US-C, which frees EU-B for support. I'll take this plan to our change-approval tool, here on my laptop.",
        watch: "Proposed plan: support → EU-B, summaries → US-C. (Selecting changes nothing on the projector.)" },
      { show: "plan", optional: true, recording: "policy", do: ["summaries-europe"], say: "While it waits for approval, compliance restricts summaries to Europe too. The answer changes at once: no valid plan. Plans keep both workloads running, and with EU-A down, support and summaries can't both fit in EU-B. Dropping work is a decision for a person, not the planner.",
        watch: "Document summaries may run on: US-C Not allowed. Action needed and Proposed plan: No valid plan." },
      { show: "plan", optional: true, recording: "policy", do: ["approve-plan"], expect: "rejected", say: "If I approve the plan I selected a moment ago, Drasi rejects it: every plan carries the facts it was based on, and those facts changed.",
        watch: "Last approval: Rejected." },
      { show: "plan", optional: true, recording: "policy", do: ["summaries-anywhere"], say: "Compliance allows the US for summaries again, and the plan comes straight back.",
        watch: "Action needed: Approve proposed plan." },
      { show: "drasi-policy", optional: true, say: "The policy change came in as a change like any other. Drasi re-ran the policy and the planner, saw that the approval no longer matched its current plan, and its reaction rejected it in the approval tool, with the reason." },
      { show: "plan", do: ["select-plan", "approve-plan"], say: "Now I approve the current plan. Drasi checks it still matches its plan, and its reaction applies the new routes through the gateway's configuration API.",
        watch: "Last approval: Applied, awaiting evidence. Running now changes. Action needed None. Recovery Awaiting evidence." },
      { show: "plan", say: "Look at Recovery. Approving isn't enough to call it fixed: another Drasi query waits until the queue has cleared and at least three support requests have been answered on the new route. Only then does it say Verified.",
        watch: "Recovery goes from Awaiting evidence to Verified, often within 30 seconds of approval. If it's already Verified, say so." },
      { show: "ops", say: "On Operations: the queue has drained, Support overdue is back to 0 and errors are falling. EU-A is still down: we routed around it, within policy." },
      { show: "drasi-recovery", say: "Drasi kept the policy and the plan current as health and capacity changed, applied the approved plan, and then checked the outcome before recording it as verified. Approving a change isn't the same as fixing the problem." },
    ],
    ifBroken: "If Recovery stays Awaiting evidence, say it's applied but not yet verified, and carry on. Never approve twice. If the plan or the policy change doesn't behave as described, press Use recordings from here on.",
  },
  resilience: {
    summary: "Healthy but fragile: EU-A returns with half its capacity",
    recording: "resilience",
    beats: [
      { show: "plan", do: ["eu-a-half"], say: "Later that day EU-A is fixed, but it comes back with only 50k of capacity. Nothing is broken.",
        watch: "Deployment capacity EU-A 50k. Safe if one fails? No: not if EU-B fails. Action needed: At risk." },
      { show: "ops", say: "Every service-health signal is green. But Drasi says we're not safe. Support now runs on EU-B. If EU-B failed, support would need 60k in Europe, and EU-A has only 50k." },
      { show: "plan", say: "That's the what-if model: for each deployment, would every workload still fit somewhere allowed if that one failed? It re-asks whenever anything changes. A game-day question, answered all the time. And it's a metric, so you can alert on it." },
      { show: "plan", do: ["eu-a-full"], say: "Give EU-A its full capacity back, and the answer is Yes again, without any outage.",
        watch: "Safe if one fails? Yes. Action needed None." },
      { show: "drasi-resilience", say: "One capacity change, and Drasi re-checked every what-if. Are we safe from the next failure? Answered before it happens." },
    ],
    ifBroken: "If Safe if one fails? doesn't change, press Use recordings from here on.",
  },
  closing: {
    summary: "Four takeaways, then questions",
    beats: [
      { show: "closing", say: "Keep your monitoring tools, and add change-driven computation next to them. Drasi turned changing facts (requests, health, capacity, policy, approvals and time) into live answers: who is late, what fits the rules, whether the fix worked, and whether we're safe from the next failure. To learn more, go to drasi.io, on screen. Thank you. Questions?" },
      { show: "closing", do: ["stop-traffic"], say: "After questions, stop the demo traffic." },
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
  ["Source", "How Drasi hears about changes; here, the database's live list of changes"],
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
  ["Does it only work with PostgreSQL?", "No. Drasi has sources for other databases, Kafka, Kubernetes, OpenTelemetry and HTTP. This demo uses PostgreSQL's change feed."],
  ["Are the policy, planner and what-if part of Drasi?", "Not yet built in: they're plugins written for this demo, in the repository. One runs Rego rules; the other is an optimizer on the open-source good_lp solver. Both are generic: the rules, workloads and capacities are configuration. Drasi runs them as continuous models and re-runs them when their inputs change."],
  ["Who changes the routes?", "Drasi's reaction calls the gateway's configuration API, only for an approval that still matches Drasi's current plan. The AI service just applies the call, and refuses it if the scenario changed in between."],
  ["How do the answers reach Grafana?", "A Drasi reaction keeps every answer as a Prometheus metric. Prometheus scrapes it and Grafana shows it."],
  ["What happens when Drasi restarts?", "It rebuilds its state from the database and resumes the change feed. Until then the tiles show Unknown."],
  ["How does it scale?", "This demo isn't a benchmark. Each query only keeps what it needs, here the unanswered requests. For performance work, see drasi.io."],
  ["Anything else", "\"Good question. I'll follow up; drasi.io, on screen, is a good place to start.\""],
];

export const actionLabels: Record<ActionId, string> = {
  "start-traffic": "Start demo traffic", "stop-traffic": "Stop demo traffic", "fail-eu-a": "Fail EU-A",
  "select-plan": "Select proposed plan", "approve-plan": "Approve selected plan",
  "summaries-europe": "Summaries: Europe only", "summaries-anywhere": "Summaries: Europe + US",
  "eu-a-half": "EU-A back at 50k", "eu-a-full": "EU-A back to 100k",
};
