import type { SceneId } from "./scenes";

// The single source for what the presenter does. The presenter desk shows these
// cue by cue, and PRESENTER.md's run of show and script are generated from them
// (cd demo/ui && npm run sync-presenter). A test fails if PRESENTER.md drifts.
export type Notes = {
  summary: string;  // a few words for the run-of-show table
  say: string;
  do: string;
  point: string;
  happens: string;
  ifBroken: string;
  next: string;
};

export const notes: Record<SceneId, Notes> = {
  title: {
    summary: "Introduce yourself and the session",
    say: "This talk is about adding change-driven computation alongside the observability tools you already use.",
    do: "Introduce yourself, then advance to the next slide.",
    point: "The session title.",
    happens: "Nothing changes in the demo.",
    ifBroken: "The slides don't depend on the demo. Carry on.",
    next: "First, what does Drasi add?",
  },
  concepts: {
    summary: "Set up the scenario, then Drasi's role",
    say: "Here's a situation many teams running AI will recognise. A company's AI assistant answers customer-support questions, and a background job summarizes documents. Both draw on the same limited AI model capacity, deployed in three regions, each with a budget of tokens per minute. Support data is restricted, so it must stay in Europe; the summaries use public data and can run anywhere. Customers expect an answer within 15 seconds, and moving work between regions needs someone's approval. Right now EU-A serves support, EU-B runs the summaries, and US-C in the United States is spare. When EU-A fails, customers stop getting answers, and the business needs to know three things: who is affected right now, what's the fastest fix that stays within company policy, and how do we know it's really fixed? Your monitoring tools will tell you something is wrong; answering those three questions is where Drasi comes in. Drasi is change-driven computation: sources bring in changes, continuous models keep results current as data changes and as time passes, and reactions publish results or take action. The AI models and capacity are simulated; the requests, monitoring and Drasi computation are real.",
    do: "When you've explained the slide, advance (or click Live demo · Grafana) to open Grafana.",
    point: "The capacity in each region and what is using it, the rules on the right, then the three questions. Finish on the Drasi section at the bottom: sources, continuous models, reactions.",
    happens: "Advancing opens Grafana's Operations dashboard in the same window.",
    ifBroken: "If Grafana doesn't open, open http://localhost:13000/d/ignite-workload in the projected window.",
    next: "Let's run this service in Grafana.",
  },
  normal: {
    summary: "Walk through the healthy dashboard",
    say: "The top half is a normal service dashboard measured from real requests: call rate, errors, latency, the support queue and a Grafana alert. OpenTelemetry collects the measurements, Prometheus stores them and Grafana shows them. The bottom half is what Drasi adds: customer impact, kept current continuously.",
    do: "Nothing to click. If traffic isn't running, click Start demo traffic on the desk.",
    point: "Support errors 0%, Support queue 0, Support overdue 0, all three deployments Healthy, Recovery None in progress, Action needed None.",
    happens: "The charts update every 5 seconds.",
    ifBroken: "If the charts are empty, traffic isn't running: start it and keep talking. Never wait on stage for a chart to fill.",
    next: "Now the deployment serving customer support fails.",
  },
  failure: {
    summary: "Fail EU-A",
    say: "The monitoring stack sees failed calls and a growing queue, as it should. Drasi answers a different question: which customer requests have now missed their deadline? That answer changes just because time passes, with no new event on the request.",
    do: "On the desk, choose Deployment EU-A and Action fail, then click Apply to deployment. Stay on the Grafana page. Optional: click Submit support request, then Open returned task trace to show that request in Langfuse.",
    point: "Model calls by outcome, Support errors, Support queue, Deployment status, Support overdue and Action needed.",
    happens: "In order: support → EU-A shows HTTP 503 in red; Support errors and Support queue rise; Deployment status shows EU-A Failed; after about 15 seconds Support overdue starts counting; after about 30 seconds the Grafana alert fires; Action needed changes to Approve proposed plan.",
    ifBroken: "If nothing happens within 30 seconds, play the failure recording.",
    next: "US-C has spare capacity. Can we use it?",
  },
  recovery: {
    summary: "Approve Drasi's proposed plan",
    say: "On paper we have plenty: 200k of healthy capacity for 140k of work. But US-C has the room, and policy denies it for support. Drasi's answer moves the summaries to US-C, which frees EU-B for support. Drasi keeps that complete plan current as things change. But approving a plan isn't the same as proving it worked.",
    do: "Click Policy and recovery in Grafana's links. On the desk, click Select proposed plan, then Approve selected plan, once.",
    point: "Capacity: Healthy 200k, Needed 140k. Customer support may run on: US-C Denied. Document summaries may run on: US-C Allowed. Proposed routes: support → EU-B and summaries → US-C.",
    happens: "Decision events records the approval; Current routes change; Recovery shows Awaiting evidence, then Verified once real support requests succeed for 5 seconds. Back on Operations, the queue drains and successful calls return.",
    ifBroken: "If Recovery stays at Awaiting evidence, say it's applied but not yet verified, and keep traffic running. Never approve a second time. If needed, play the recovery recording.",
    next: "Here's the continuous query behind one of those answers.",
  },
  change: {
    summary: "Show the query behind \"overdue\"",
    say: "This is a continuous query, not a scheduled check. Drasi keeps its result current as changes arrive, and re-evaluates it when time alone changes the answer. These results feed the models and dashboards you've just seen.",
    do: "Click Drasi queries in Grafana's links. Select the overdue-tasks query, click Expand query, and zoom the browser (Cmd +) so it's readable. Keep this to two minutes.",
    point: "The deadline condition in the query.",
    happens: "Nothing changes in the demo.",
    ifBroken: "If the query text doesn't appear (a known Drasi Server UI issue), describe the query in one sentence and go back to Grafana.",
    next: "Policy is another input to the same computation.",
  },
  policy: {
    summary: "Restrict summaries to Europe (optional)",
    say: "A proposal belongs to the facts that produced it. Change the policy and the answer changes, even though nothing has failed. An old approval can't silently apply a different plan.",
    do: "On the desk, click Select proposed plan. Then click Batch policy: Europe only. When US-C turns Denied for summaries, click Approve selected plan. Afterwards, click Batch policy: Europe + US to restore the policy.",
    point: "Document summaries may run on: US-C, and Decision events.",
    happens: "US-C changes to Denied for summaries, and the approval is rejected; the rejection appears in Decision events.",
    ifBroken: "Restore the policy (Batch policy: Europe + US) and describe what happened in words. Play the policy recording if needed.",
    next: "Finally: can a healthy service survive the next failure?",
  },
  resilience: {
    summary: "Cut EU-B's budget, then restore it",
    say: "Everything is healthy: support on EU-A, summaries on US-C. But EU-B now has only 50k of capacity, and support needs 60k. If EU-A failed, support would have nowhere it's allowed to go. A dashboard of current health can't tell you that.",
    do: "On the desk, click Healthy-fragile inputs · EU-B 50k. After explaining, click Restore EU-B capacity · 100k.",
    point: "Safe if EU-A fails? and Deployment budgets. Optionally show Operations: everything is green.",
    happens: "Safe if EU-A fails? changes to No. After restoring EU-B it changes back to Yes, with no outage.",
    ifBroken: "Play the resilience recording.",
    next: "That's the idea: change-driven computation alongside the tools you already use.",
  },
  closing: {
    summary: "Three takeaways",
    say: "Keep your observability tools. Add change-driven computation to connect changing context, keep decisions valid, and verify what actually happened.",
    do: "Click Closing slide in Grafana's links. Take questions. When you've finished, click Stop demo traffic on the desk.",
    point: "The three takeaways and the repository link.",
    happens: "The projected window returns to the slides.",
    ifBroken: "Summarize in words. Never project the desk.",
    next: "Questions.",
  },
};
