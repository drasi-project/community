# Presenter guide

Everything you need to give this talk, without needing to know how the demo is
built. To install and start the demo, see [SETUP.md](SETUP.md). For what the demo
is about, see [README.md](README.md).

**The talk in one line:** a few slides, then about 17 minutes of live demo in
which an AI service fails, Drasi shows who is affected and works out a fix within
the rules, you approve it, and Drasi confirms it worked.

- Session: THR1029, *Monitoring without polling*, 25 minutes
- About 22 minutes of content and 3 minutes of questions

## Contents

1. [How it works on stage](#how-it-works-on-stage)
2. [Set up your screens](#set-up-your-screens)
3. [Before you present](#before-you-present)
4. [The presenter desk](#the-presenter-desk)
5. [Run of show](#run-of-show) and [the script](#the-script)
6. [Glossary](#glossary) and [questions to expect](#questions-to-expect)
7. [Reading the dashboard](#reading-the-dashboard), [the incident channel](#the-incident-channel) and [the late request view](#the-late-request-view)
8. [When something goes wrong](#when-something-goes-wrong)
9. [After the talk](#after-the-talk)

## How it works on stage

There are two windows:

- **The projector window** is what the audience sees: slides, the Grafana
  dashboard, the team's incident channel in Mattermost (#incident-ai-support) and
  one late customer request. You click in it only on the incident channel, to
  scroll to a card or press **Approve** on Drasi's proposal; the desk says when.
- **The presenter desk** is on your laptop screen. It shows what to say, the one
  or two buttons for this moment, and what to watch for.

The talk is a sequence of **beats**. Press **Next** on the desk (or your clicker,
→, Space or Page Down) and the projector moves to whatever the next beat needs:
the next slide, a dashboard, or a "What Drasi just did" visual. Your clicker works
whichever window is in front.

Some beats have buttons, such as **Fail EU-A** or **Summaries: Europe only**. They
appear in the beat itself: press them when you reach the beat, while you say its
words. Nothing in the demo changes unless you press a button; **Back** and
**Next** never change the demo. If you forget a button, Next reminds you once.

Approving is done where the audience can see it: on the beats that approve, the
desk shows **Click on the projector** and you press **Approve** on Drasi's
proposal card in the projected channel. The desk keeps one backup button,
**Approve the proposed change**, which presses that same button for you.

Your clicker moves Back and Next. Buttons need a click with the trackpad or
mouse: desk buttons on the desk, card buttons on the projector.

If the desk isn't open, → / Space / Page Down (or the clicker) on the projector
window still steps through the slides and dashboards on its own, but nothing in
the demo changes. Open the desk to give the talk.

## Set up your screens

1. Connect the projector and set the displays to **extend**, not mirror
   (macOS: System Settings › Displays › Use as: Extended display). If they mirror,
   the audience sees your desk.
2. In Chrome, open the desk: http://localhost:19580/presenter (`localhost`, not
   `127.0.0.1`: the projected Mattermost channel only stays signed in on `localhost`)
3. On the desk, press **Open projector window**. Drag that window onto the
   projector and make it full screen (Ctrl ⌘ F). The desk's header then says
   **Projector window connected**.
   Set the projector to **1920×1080 at 100% scaling** (macOS: Displays ›
   1920×1080; Windows: Display › Scale 100%), as Ignite asks for live demos. The
   projected stage is the Ignite template's 16:9 slide, drawn at exactly
   1920×1080 there; on any other screen it is scaled to fit, with black bars if
   the shape differs. Nothing is shown in the bottom 78 px, which Ignite keeps
   clear for closed captions.
4. Keep the desk on your laptop screen. Turn on Do Not Disturb, quit chat apps,
   and plug in power.

## Before you present

**The day before:** rehearse the whole talk at least once with the live demo, and
once using only the recordings (see [When something goes wrong](#when-something-goes-wrong)).

**30 minutes before:**

1. Start the demo ([SETUP.md › Start the demo](SETUP.md#start-the-demo)).
2. Optional: sign in to Langfuse **privately** on the laptop, in case someone asks
   to see it ([SETUP.md › Get ready to present](SETUP.md#get-ready-to-present)).
3. [Set up your screens](#set-up-your-screens).
4. Sign the projector in to Mattermost: on the desk, open **Before and after the
   talk** and press **Sign the projector in to Mattermost** (it signs this browser
   in as the on-call engineer; no password is shown). Then **Show something on the
   projector** › **Mattermost · #incident-ai-support** should show the channel,
   not a sign-in page. Go back to the first beat afterwards.
5. On the desk, open **Before and after the talk** and check it says
   **✓ Starting state**. If not, press **Reset scenario…** then **Confirm reset**.
6. Check the header says **READY**, and doesn't say **Ignite template assets
   missing** (if it does, see [SETUP.md › Add the Ignite template artwork and
   fonts](SETUP.md#8-add-the-ignite-template-artwork-and-fonts)).
7. Step through a few beats with your clicker to check it moves the projector,
   then go back to the first beat (**Jump to a part** › 1. Title). The projector
   then shows the Ignite walk-in slide until you start.

**Before you start talking:** on the walk-in beat, press **Start demo traffic**
(next to the beat; don't mention it), so the charts have several minutes of
history by the time the dashboard appears. Press it at least 5 minutes before
the dashboard, and reset at least 5 minutes before that: after a reset the Model
calls chart shows the previous run for 5 minutes. If traffic is already
running, the button is ticked and there's nothing to press. Traffic stops by
itself after 90 minutes. If the desk ever shows **Traffic: stopped** during the talk, press
**Start demo traffic** again.

**If the demo won't start or the header stays NOT READY:** give the talk in
recording mode. Press **Use recordings for the rest of the talk** in the warning
at the top of the desk. Everything else works the same.

## The presenter desk

Top to bottom:

| Part | What it's for |
|---|---|
| Header | **READY** or **NOT READY**, whether the projector window is connected, **Ignite template assets missing** if they are, and the talk timer. It starts by itself on your first **Next** and turns amber at 22 minutes; tick **Short on time** if you're past about 11:30 when Recovery starts |
| **The current beat** (left) | Part number and name, dots for the beats in this part, **◀ Back / Next ▶** and **what the projector shows** (always in the same place), the beat's buttons and their result, **Click on the projector** (what to press or scroll on the projected channel), what to say, what to watch for, a preview of the next beat, what to do if it breaks (with **Use recordings from here on**), and how to undo a wrong button |
| **Drasi's answers** (right) | Support overdue, Recommended action and Failover readiness, as on the dashboard, and Recovery: Awaiting evidence or Verified after a change is applied |
| **Live demo** (right) | Each deployment's health and capacity; whether traffic is running; where the summaries may run |
| **Routes** (right) | Where support and summaries run now and Drasi's proposed plan, with a line saying whether a proposal is waiting for Approve, and the latest approval's status |
| Jump to a part | Go straight to any part of the talk (also moves the projector) |
| All demo controls | Every button, if you need one out of order, plus single requests and changes to any deployment |
| Show something on the projector | Put any slide, dashboard or visual on the projector without changing your place |
| Recordings (fallback) | Show one recording on the projector, or switch to recording mode for the rest of the talk |
| Questions to expect, and glossary | The same as in this guide |
| Before and after the talk | Starting-state check, the projector's Mattermost sign-in, reset, full settings, and whether the Ignite template assets are present |

**Buttons:** the button for the current beat is bright. A ✓ means it's already
done (traffic running, EU-A failed, the change approved). Grey dashed means it
can't be pressed now (for example, **Approve the proposed change** while there's
no proposal). **Fail EU-A** is red.
After pressing, the result appears right under the buttons: **Done.** On the
approving beats a second line follows Drasi's answer, whichever button was
pressed: waiting for Approve, checking, **Done: Drasi checked it and the new
routes are applied**, or in green **Rejected by Drasi, as intended** for the
approval that's meant to be rejected.

**Short on time:** tick it to skip the beats marked optional (the policy change
and the late request view). Their buttons and notes disappear from the sequence.

**The approval flow.** Drasi's planner works out a change and re-works it whenever
anything changes; Drasi posts each new proposal as a card in #incident-ai-support
with **Approve** and **Reject**. Nothing changes until someone approves. Pressing
**Approve** (on the card, or the desk's backup, which presses the newest
proposal's Approve) sends it to the change-approval tool, which only records it,
and the card then says who approved it. Drasi checks it: if the proposal and the
facts it was worked out from (policy, capacity, health) are unchanged, Drasi's
reaction applies the new routes through the gateway's configuration API;
otherwise it rejects the approval with a reason. Either way Drasi replies in the
card's thread, and the desk shows the same answer. An approved card loses its
buttons, so it can't be approved twice. When the facts change before anyone
approves, Drasi posts **Out of date** and, if a change still fits, a new
proposal: approve the new one.

<!-- generated from presentation/notes.ts: do not edit; run npm run sync-presenter in demo/ui -->

## Run of show

| Time | Part | Projector shows | You |
|---|---|---|---|
| 0:00 | Title | Walk-in, Title | Walk-in slide while people arrive; then introduce yourself and the talk |
| 0:30 | Observability and Drasi | Observability today, Drasi: change-driven computation | The observability challenge, and Drasi in three parts |
| 2:00 | The scenario | A typical scenario, The scenario · deployments, The scenario · rules, The scenario · three answers, Bringing it together when EU-A fails | A typical scenario: two workloads, the rules, three questions; then what Drasi brings together |
| 3:30 | Healthy service | Grafana · Operations | Walk through the healthy dashboard |
| 6:00 | Failure | Grafana · Operations | Fail EU-A; Drasi's row is ahead of the alert |
| 9:00 | Who is affected | Mattermost · #incident-ai-support, Who is affected: the continuous query, One late request | The incident channel: Drasi's failure and overdue cards; the query and its live list |
| 11:30 | Recovery | Mattermost · #incident-ai-support, Grafana · Operations | Approve Drasi's proposed change on its card; optionally show an out-of-date proposal rejected |
| 17:00 | Healthy but fragile | Grafana · Operations, Mattermost · #incident-ai-support, What Drasi just did | Healthy but fragile: EU-A returns with half its capacity |
| 20:30 | Close | Close, Session survey | Four takeaways, the session survey, then questions |
| 22:00 | Questions | Session survey | Take questions |

## The script

The desk shows one beat at a time. Press **Next** (or your clicker, or →) to move on: the
projector follows. Beats marked optional are skipped when **Short on time** is ticked.
Say it in your own words.

### 1. Title (30 seconds)

1. **Walk-in.** Before the session: the Ignite walk-in slide stays up while people arrive. Press Next when you start talking; that starts the talk timer.
   - **Press:** Start demo traffic
   - **Watch for:** Press Start demo traffic here, before you start talking, so the dashboard has several minutes of data by the time it appears; the desk shows Traffic: running for … Press it at least 5 minutes after any reset (the Model calls chart shows the previous run for 5 minutes after one). If traffic is already running, the button is ticked.
2. **Title.** Introduce yourself. Your monitoring tells you something is wrong. This talk is about getting the next answers too: who is affected, what to do, and whether it worked, worked out the moment things change, in the tools you already use.

**If it breaks:** The slides don't depend on the demo. Carry on.

### 2. Observability and Drasi (1½ minutes)

1. **Observability today.** Today's systems produce a lot of signals: metrics, logs, traces and health checks, plus the business data, the policies and the change process around them, spread across many tools. When something goes wrong, someone has to work out what's going on, what to do about it, and what doing it will cause. The context for those answers is spread across systems; bringing it together is what makes them possible.
2. **Drasi: change-driven computation.** Drasi makes that simpler and quicker. It's open-source change-driven computation, in three parts. Sources bring in every change as it happens, from databases, telemetry and other systems. Continuous Models do the work: continuous queries keep a result up to date as data changes and as time passes, and policy and optimization models re-run whenever their inputs change. Reactions deliver each change to the tools people already use. You configure it; you don't write the plumbing.

**If it breaks:** Carry on; the slides don't depend on the demo.

### 3. The scenario (1½ minutes)

1. **A typical scenario.** Let's look at a typical scenario.
2. **The scenario · deployments.** A company runs two AI workloads on the same limited AI model capacity: an assistant that answers customer-support questions, and a background job that writes document summaries. The capacity is split across three deployments, EU-A and EU-B in Europe and US-C in the United States, each with 100k tokens a minute.
3. **The scenario · rules.** Two kinds of rule. Data compliance: customer support handles restricted data, so it may only run in Europe; summaries use public data and can run anywhere. Business requirements: customers expect an answer within 15 seconds, and infrastructure changes, like moving work between deployments, require human approval.
4. **The scenario · three answers.** When something goes wrong, the business needs three answers: who is affected, what's the fastest fix that stays within policy, and how do we know when it's resolved? You're about to see Drasi answer all three.
5. **Bringing it together when EU-A fails.** Here's what we'll watch when EU-A fails. Drasi brings together the telemetry, the application's own data and the policy, and works with the tools the team already uses: Grafana for dashboards and alerts, the incident channel for messaging, and the change-approval tool, where a ticketing system would sit. We'll see it answer those three questions, sooner.

**If it breaks:** Carry on; the demo doesn't depend on the slides.

### 4. Healthy service (2½ minutes)

1. **Grafana · Operations.** This is the AI service's Grafana dashboard: the kind of monitoring most teams already run, with metrics from real requests and alerts on them.
   - **Watch for:** Charts moving (traffic started on the walk-in beat; if the desk says Traffic: stopped, press Start demo traffic in All demo controls).
2. **Grafana · Operations.** The top row, AI Deployments, is ordinary monitoring, measured from real requests: calls, errors, answer time, the support queue and Grafana alerts. OpenTelemetry, Prometheus and Grafana; nothing new.
   - **Watch for:** Support errors 0%, Support queue 0 to 2 (green).
3. **Grafana · Operations.** The bottom row, Decision Support, is Drasi's answers, worked out from changes as they happen: changes in the AI service's database, where it records every request, route, policy and approval, and its health checks, which arrive as ordinary OpenTelemetry metrics every second. A Drasi reaction publishes each answer as a Prometheus metric, so Grafana draws it and you can alert on it like anything else; another posts to the team's incident channel when something needs them. Let's take them one at a time.
   - **Watch for:** The whole Decision Support row outlined.
4. **Grafana · Operations.** Support overdue: customer requests past their 15-second deadline. Above it is the query that drives it: every request not yet answered or cancelled, and drasi.trueLater, which becomes true at the request's deadline. Drasi re-checks just that request at that moment, so it joins the list with no new event, and leaves it when it's answered.
   - **Watch for:** Support overdue 0; the overdue-tasks query above it, the trueLater line highlighted.
5. **Grafana · Operations.** Deployment status: each deployment's latest health check. Every second the AI service probes each deployment and exports the result as an ordinary OpenTelemetry metric; the Collector forwards it to Drasi's OpenTelemetry source, and this query keeps the latest probe per deployment: 2 means failed. Each reading is good for 8 seconds, so if the probes stop, the status turns Unknown as time passes.
   - **Watch for:** Deployment status: EU-A, EU-B, US-C Healthy; the deployment-health query above it.
6. **Grafana · Operations.** Workload status joins three things: where each workload runs, what the policy says about that deployment, and that deployment's health. So it tells you whether each workload is being served somewhere it's allowed to run.
   - **Watch for:** Workload status: Support · EU-A and Summaries · EU-B Healthy; the tile-workloads query above it.
7. **Grafana · Operations.** Recommended action comes from two of Drasi's standard models. First the policy, as these Rego rules: a workload may only use the deployments it's allowed, and restricted data must stay in its region. The allowed destinations are data, so a compliance change is just a change to some rows, and Drasi re-checks every workload and deployment when one changes.
   - **Watch for:** Recommended action: None (green); the Rego rules above it.
8. **Grafana · Operations.** Then the optimization model, configured by this file: one yes or no choice per workload and allowed deployment, every workload placed exactly once, each healthy deployment within its capacity, and the fewest moves first. Drasi re-solves whenever health, capacity or policy changes, proposes the result, and follows it through approval, applying and verification. Right now there's nothing to do.
   - **Watch for:** Recommended action: None (green); the optimization model above it.
9. **Grafana · Operations.** Failover readiness is a what-if from the same model: the loss scenario solves again without each healthy deployment in turn. If any one failed, would every workload still have somewhere allowed to run? A game-day question, answered all the time. No new tool to watch: the answers arrive where you already look.
   - **Watch for:** Failover readiness: Survives losing any one deployment; the scenario above it.

**If it breaks:** If the charts are empty, traffic isn't running: press Start demo traffic and keep talking.

### 5. Failure (3 minutes)

1. **Grafana · Operations.** Now I'll make EU-A, the deployment serving customer support, fail.
   - **Press:** Fail EU-A
   - **Watch for:** Within about 8 seconds: Deployment status EU-A Failed, and Recommended action names a change.
2. **Grafana · Operations.** Look at the bottom row first, Decision Support: EU-A Failed, and Recommended action already names a fix: move summaries to US-C and support to EU-B. Nobody told Drasi EU-A was down: its health probes failed, and the deployment-health query you saw turned that into Failed. And it has worked out a change that fits the rules. We'll come back to it.
   - **Watch for:** Recommended action: Move summaries to US-C and support to EU-B (orange).
3. **Grafana · Operations.** Back on the dashboard, the top half catches up: calls fail, errors and the queue climb, and Grafana's alert fires; it posts to the channel too. Something is wrong.
   - **Watch for:** Support errors red, Support queue rising; Alerts firing lists Model calls failing, then Support requests overdue (Drasi), an alert on Drasi's answer.
4. **Grafana · Operations.** And Support overdue is climbing. Each request has a 15-second deadline. When a request passes it, nothing happens to that request: no new row, no new metric. Drasi counts it at that exact moment anyway, because its query re-checks each request at its deadline. On the chart, the red overdue line follows the orange waiting line 15 seconds later. Nothing re-scans the database: Drasi reacts to each change and each deadline, which is why it had a fix proposed before Grafana's alert fired. Let's see what the team saw.
   - **Watch for:** Support overdue above 0; on the right-hand chart, the red line trails the orange one.

**If it breaks:** If nothing changes within 30 seconds, press Use recordings from here on and say it's a recording.

### 6. Who is affected (2½ minutes)

1. **Mattermost · #incident-ai-support.** The team doesn't live on a dashboard, though. This is their incident channel in Mattermost. Drasi posted here as soon as EU-A failed, with its proposed change already waiting and an Approve button. Look at the times: Drasi's cards came before Grafana's alert, which posts here too. We'll come back to the proposal.
   - **Click on the projector:** If Drasi's red Deployment failed: EU-A card isn't in view, scroll up to it. 
   - **Watch for:** Red card Deployment failed: EU-A, then the orange Proposed change card with Approve and Reject; Grafana's alert after them.
2. **Mattermost · #incident-ai-support.** Which customers are affected? Drasi posted that too: Support requests overdue, as soon as the first support request passed its deadline, with a link to the tile on the dashboard. A count tells you it's bad. Let's see who.
   - **Click on the projector:** If the red Support requests overdue card isn't in view, scroll to it. 
   - **Watch for:** Red card Support requests overdue, with the count when posted and a link to Operations › Support overdue.
3. **Who is affected: the continuous query.** This is the Support overdue query from earlier, now with its live result: not just a count, but the exact list of late requests with their traces, kept current request by request. With Prometheus you'd get a number from a gauge or a rule evaluated every so often; here it's the set itself, from one query
   - **Watch for:** Under the query: how many support requests are late, and the oldest few with their traces.
4. **One late request (optional).** Here's one of those customers: their question, every attempt to send it to EU-A failing, and the moment its deadline passed and Drasi listed it. The same request is a trace in Langfuse, so whoever picks it up has the detail.
   - **Watch for:** The oldest late request, with its attempts and the moment Drasi listed it.

**If it breaks:** If the channel shows no cards, say Drasi posts them there and go on to the query. If the list is empty, traffic has stopped: say what it would show. The request beat is skipped automatically when nothing is late.

### 7. Recovery (5½ minutes)

1. **Mattermost · #incident-ai-support.** Back to the proposal. On paper there's plenty of room: two healthy deployments with 200k, and we need 140k. But US-C has the free room, and customer support isn't allowed there: restricted data stays in Europe. The planner, an optimizer Drasi re-runs on every change, respects that: move summaries to US-C, which frees EU-B for support. Two moves, both within policy, and the card shows the facts it was based on: that's the Rego policy and the optimization model you saw earlier, re-solved on the new facts.
   - **Click on the projector:** Scroll up to the orange Proposed change card so the whole card shows. 
   - **Watch for:** The orange Proposed change card: Move summaries to US-C and support to EU-B. Why EU-A is down; Policy; Capacity; Based on revision and plan.
2. **Mattermost · #incident-ai-support (optional).** Before anyone approves it, compliance restricts summaries to Europe too. Drasi re-runs the policy and the planner at once: with EU-A down, support and summaries can't both fit in EU-B, so no change fits now, and Drasi says the proposal is out of date. Dropping work is a decision for a person, not the planner.
   - **Press:** Summaries: Europe only
   - **Watch for:** A grey card Out of date: the facts changed, Now: No valid plan. The proposal above it still has its Approve button.
3. **Mattermost · #incident-ai-support (optional).** Suppose someone approves the proposal anyway. The approval goes to the change-approval tool, and Drasi rejects it: every proposal carries the facts it was based on, and those facts changed. Nothing is changed.
   - **Click on the projector:** Approve on the orange proposal card, above the grey Out of date card (scroll up a little if needed) (it's meant to be rejected).  Backup on the desk: Approve the proposed change (it presses the same button)
   - **Watch for:** The card says Approved by @oncall; then, in its thread, Rejected by Drasi: nothing was changed, with Drasi's reason.
4. **Mattermost · #incident-ai-support (optional).** Compliance allows the US for summaries again, and Drasi posts a new proposal straight away. To Drasi, a policy change is just another change: it re-runs the policy and the planner every time.
   - **Press:** Summaries: Europe + US
   - **Watch for:** A new orange Proposed change card at the bottom, with a new plan in Based on.
5. **Mattermost · #incident-ai-support.** Now I approve the current proposal, here on its card, as the on-call engineer would. The approval goes to the change-approval tool; Drasi checks it still matches its current plan, and its reaction applies the new routes through the gateway's configuration API.
   - **Click on the projector:** Approve on the newest orange Proposed change card, at the bottom.  Backup on the desk: Approve the proposed change (it presses the same button)
   - **Watch for:** The card says Approved by @oncall at …; in its thread, blue Applied: Move summaries to US-C and support to EU-B.
6. **Grafana · Operations.** Look at Recommended action: it says Verifying. Applied isn't the same as fixed: another Drasi query waits until the backlog from before the change has cleared and at least three support requests have been answered on the new route. Only then does it say Verified.
   - **Watch for:** Recommended action goes from Verifying: … to Verified: … (green), often within 30 seconds of approval. If it's already Verified, say so.
7. **Grafana · Operations.** The rest of the dashboard agrees: the queue has drained, Support overdue is back to 0 and errors are falling. EU-A is still down: we routed around it, within policy. And in the channel, the proposal's thread now tells the whole story: proposed, approved by the on-call engineer, applied, and verified by Drasi from the evidence. Approving a change isn't the same as fixing the problem; Drasi checked the outcome.
   - **Watch for:** Support errors falling, Support queue back to 0 to 2, Support overdue 0.

**If it breaks:** If Approve on the card doesn't respond, press Approve the proposed change on the desk: it presses the same button. Never approve twice. If Recommended action stays Verifying, say it's applied but not yet verified, and carry on. If the proposal or the policy change doesn't behave as described, press Use recordings from here on.

### 8. Healthy but fragile (3½ minutes)

1. **Grafana · Operations.** Later that day EU-A is fixed, but it comes back with only 50k of capacity. Nothing is broken: every deployment is healthy, every service-health signal is green. But Failover readiness says No.
   - **Press:** EU-A back at 50k
   - **Watch for:** Deployment status all Healthy, EU-A 50k. Failover readiness: No, not if EU-B fails. Recommended action: Add capacity: losing EU-B leaves support nowhere to run.
2. **Grafana · Operations.** That's the what-if model: for each deployment, would every workload still fit somewhere allowed if that one failed? It re-asks whenever anything changes. A game-day question, answered all the time. And it's a metric, so you can alert on it.
3. **Mattermost · #incident-ai-support.** Drasi posted it in the channel, while everything is green. Support now runs on EU-B. If EU-B failed, support would need 60k in Europe, and EU-A has only 50k. Nobody would notice until EU-B fell over.
   - **Watch for:** Orange card Failover at risk: losing EU-B leaves support nowhere to run.
4. **Mattermost · #incident-ai-support.** Give EU-A its full capacity back, and Drasi says we're safe again, without any outage.
   - **Press:** EU-A back to 100k
   - **Watch for:** Green card Failover restored. (On the dashboard, Failover readiness: Yes, and Recommended action: None.)
5. **What Drasi just did.** One capacity change, and Drasi re-checked every what-if. Are we safe from the next failure? Answered before it happens.

**If it breaks:** If Failover readiness doesn't change, press Use recordings from here on. If the channel shows no card, stay on the dashboard: it shows the same answer.

### 9. Close (1½ minutes)

1. **Close.** Keep your monitoring tools, and add change-driven computation next to them. Drasi turned changing facts (requests, health, capacity, policy, approvals and time) into live answers: who is late, what fits the rules, whether the fix worked, and whether we're safe from the next failure. To learn more, go to drasi.io, on screen.
2. **Session survey.** Thank you. Please scan the code and tell us how we did in the session survey. Questions?
   - **Watch for:** The survey slide with its QR code stays up during questions.
3. **Session survey.** After questions, stop the demo traffic. Leave the survey slide up.
   - **Press:** Stop demo traffic

**If it breaks:** Summarize in words. Never project the desk.

## Glossary

| Term | Meaning |
|---|---|
| Deployment | One place the AI model runs: EU-A, EU-B (Europe) or US-C (United States) |
| Capacity | How much a deployment can handle, in tokens per minute (100k each) |
| Customer support / document summaries | The two workloads: support needs 60k, summaries 80k |
| Plan | Where each workload should run. Drasi's planner proposes one; you approve it, and Drasi applies it if it's still current |
| Policy | The rules: support only in Europe; summaries anywhere |
| Source | How Drasi hears about changes; here, the database's live list of changes and the health checks' OpenTelemetry metrics |
| Continuous query | A question whose answer Drasi keeps up to date as data changes and time passes |
| Continuous model | Logic that Drasi re-runs when its inputs change: here a Rego policy, and an optimizer for the planner and the what-if |
| Reaction | Delivers each change in an answer to another system: here Prometheus, the gateway's configuration API, Grafana, Langfuse and the presenter desk |
| Trace | One customer request, start to finish, in Langfuse |

## Questions to expect

| Question | Answer |
|---|---|
| How is this different from a Prometheus alert? | Prometheus evaluates numbers on an interval. Drasi keeps the actual list of affected requests current as changes arrive, joins in business facts like policy and approvals, and re-checks an item at the moment a deadline passes. Its answers still go to Prometheus and alerts, as you saw. |
| Couldn't I just export 'age of the oldest waiting request'? | That's one number. Drasi gives the exact set of late requests with their traces, updated request by request, without writing that logic yourself. |
| Isn't trueLater just a timer? | It's a re-check of one item at one moment. The claim is no scheduled scans or recomputation, not no clock. |
| Grafana refreshes every 5 seconds. Isn't that polling? | That's Prometheus and Grafana collecting and drawing, as they always do. Deciding who is late, what's allowed and which plan fits happens in Drasi as changes and deadlines arrive. |
| Was the alert slow on purpose? | The alert is measured over a short window, as most are. The point isn't speed: Drasi has a plan that fits policy, not just a signal. |
| Does it only work with PostgreSQL? | No. Drasi has sources for other databases, Kafka, Kubernetes, OpenTelemetry and HTTP. This demo uses two: PostgreSQL's change feed, and OpenTelemetry for the deployments' health checks. |
| Are the policy, planner and what-if part of Drasi? | Yes. They're Drasi's standard policy and optimization models. The demo supplies only configuration: the Rego rules, and a placement model of workloads, deployments, constraints and objectives, solved with the open-source good_lp solver. Drasi re-runs them whenever their inputs change. |
| Who changes the routes? | Drasi's reaction calls the gateway's configuration API, only for an approval that still matches Drasi's current plan. The AI service just applies the call, and refuses it if the scenario changed in between. |
| How do the answers reach Grafana? | A Drasi reaction keeps every answer as a Prometheus metric. Prometheus scrapes it and Grafana shows it. |
| What happens when Drasi restarts? | It rebuilds its state from the database and resumes the change feed. Until then the tiles show Unknown. |
| How does it scale? | This demo isn't a benchmark. Each query only keeps what it needs, here the unanswered requests. For performance work, see drasi.io. |
| Anything else | "Good question. I'll follow up; drasi.io, on screen, is a good place to start." |

<!-- end generated -->

## Reading the dashboard

Colours mean the same everywhere: **red** broken, **orange** needs action or
waiting, **green** OK, **grey** nothing to show. **Unknown** means Drasi's answer is
out of date or unavailable; it never means healthy. Every panel's (i) icon says
what it shows.

**AI service · Operations**

| Row | Panel | Meaning |
|---|---|---|
| AI Deployments (top: today's monitoring) | Model calls | Calls to the AI model per second, both workloads, last 30 seconds |
| | Support errors | Share of support calls failing, last 30 seconds |
| | Answer time p95 | 95th-percentile time from a customer's question to its answer. **No answers** during the outage |
| | Support queue | Support requests waiting for an answer. One or two in flight is normal (green) |
| | Model calls by workload (chart) | Answered and failed calls for each workload |
| | Support requests waiting (chart) | Orange: waiting. Red: overdue, from Drasi. The red line trails the orange one by the 15-second deadline |
| | Alerts firing | Grafana's own list of its alert rules firing now, with how long: "Model calls failing", and "Support requests overdue (Drasi)", an alert on Drasi's answer. Each also posts to #incident-ai-support |
| Decision Support (bottom: Drasi) | Support overdue | Support requests past their 15-second deadline |
| | Deployment status | Each deployment's health as Drasi derives it from its health-probe telemetry: Healthy, Over quota or Failed; Unknown if its probes have been silent for 8 seconds |
| | Workload status | Each workload where it runs now, for example **Support · EU-A**: Healthy, Over quota or Failing as its deployment is; Not allowed when policy no longer allows it there; Unknown if its deployment has been silent for 8 seconds |
| | Recommended action | Drasi's recommended change and where it is: **None**; **Approve: Move summaries to US-C and support to EU-B** (approve it on its card in #incident-ai-support); **Applying: …**, then **Verifying: …** until the backlog has cleared and support is answered on the new route; **Verified: …** (green); **No change fits policy and capacity**; **Add capacity: losing EU-B leaves support nowhere to run** |
| | Failover readiness | If any one deployment failed, would every workload still have somewhere allowed to run? **No** names the deployment whose loss would hurt. **Already one down** while a deployment has failed |

The deadlines (15-second target, 8-second silence) are shortened for the demo.

## The incident channel

**#incident-ai-support** in Mattermost, projected signed in as the on-call
engineer. Drasi posts a card when something changes; Grafana posts its alerts.
The newest post is at the bottom.

| Card | Colour | When |
|---|---|---|
| Deployment failed: EU-A | red | A deployment's probes have shown Failed for 3 seconds |
| Support requests overdue | red | Support requests have passed their 15-second deadline; links to the dashboard tile |
| Proposed change: … | orange | Drasi's plan for the current facts, with Why, Policy, Capacity and Based on, and **Approve** / **Reject**. After a press it says who approved or rejected it, and its buttons go |
| Out of date: the facts changed | grey | The proposal above no longer matches Drasi's plan; approving it would be rejected. Now says the new change or **No valid plan** |
| Applied / Verified / Rejected by Drasi | blue / green / orange | Drasi's answer to an approval, in the approved card's thread |
| Failover at risk: losing EU-B leaves support nowhere to run | orange | Everything is healthy, but losing one deployment would leave a workload nowhere to run |
| Failover restored | green | Losing any one deployment again leaves a valid plan |

## The late request view

In **Who is affected**, the projector shows the oldest late support request from
Drasi's list: the customer's question, each attempt to send it (for example
"Sent to EU-A: failed (HTTP 503) × 12"), and the moment its 15-second deadline
passed and Drasi added it to the overdue list. The beat is skipped automatically
if nothing is late.

The same request is a trace in Langfuse (its trace ID is at the bottom). If
someone asks to see Langfuse, open it on your laptop from **Show something on the
projector** › Langfuse, which opens privately; in Langfuse, a request shows its
input, each **attempt · EU-A** (red when it failed) and **model · EU-B** when
answered, and the *drasi_recovery* score, which Drasi adds to requests answered
on a route it verified.

## When something goes wrong

**The live demo stops behaving as the script says.** Press **Use recordings from
here on** (in **If it breaks**), or **Use recordings for the rest of the talk**
when the header says NOT READY. The projector shows this part's recording,
labelled **REPLAY · NOT LIVE**, and plays it silently: talk over it, and say it's
a recording. Keep pressing **Next** as usual: each remaining live part shows its
recording (failure, recovery, policy change, healthy but fragile), and the slides
still appear. The buttons are hidden. Stay in recording mode for the rest of the
talk.

**You pressed the wrong button.** Back and Next never undo anything. Use the
opposite button in **All demo controls**: **Summaries: Europe + US** undoes
**Europe only**; **EU-A back to 100k** undoes **EU-A back at 50k**; to undo **Fail
EU-A**, use **Any deployment** › EU-A › Restore › **Apply to deployment**. If
you're not sure what state the demo is in, switch to recordings.

**You pressed Next before the beat's button.** The desk stops you once: "You
haven't pressed Fail EU-A yet". Press the button, or press Next again to move on
without it.

| On stage | Do this |
|---|---|
| Charts are empty, or Traffic: stopped | Press **Start demo traffic** (All demo controls) and keep talking |
| Tiles show Unknown | Drasi's answer is out of date. Say so; never call it healthy. Press **Use recordings from here on** |
| Nothing happens 30 seconds after Fail EU-A | Press **Use recordings from here on** |
| Recommended action stays at Verifying | Say it's applied but not yet verified; keep going. Never approve twice |
| Approve on the card does nothing | Press **Approve the proposed change** on the desk: it presses the same button |
| The channel shows Mattermost's sign-in page | Press **Sign the projector in to Mattermost** (Before and after the talk), then show the channel again |
| The channel shows its sign-in page again, or "session expired" after signing in | The desk must be open at `http://localhost:19580`, not `127.0.0.1` (Mattermost's cookies only reach the projected channel from `localhost`). The web app moves `127.0.0.1` to `localhost` by itself; if the address bar still says `127.0.0.1`, open http://localhost:19580/presenter and reopen the projector window |
| No cards appear in the channel | Say Drasi posts them there, and stay on the dashboard: the Decision Support row shows the same answers |
| The projector shows the wrong thing | Press **Back**, then **Next**, or use **Jump to a part** |
| The projector window closed | Press **Open projector window** in the header and put it back on the projector |
| You lost your place | **Jump to a part** |

Golden rules: never project the desk, never call Unknown healthy, approve once,
and never reset during the talk.

## After the talk

1. Press **Stop demo traffic**.
2. Open **Before and after the talk**, press **Reset scenario…** then **Confirm
   reset**, so the next presenter starts clean.
3. If you're finished for the day, [stop the demo](SETUP.md#stop-the-demo).
