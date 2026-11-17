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
7. [Reading the dashboards](#reading-the-dashboards) and [the late request view](#the-late-request-view)
8. [When something goes wrong](#when-something-goes-wrong)
9. [After the talk](#after-the-talk)

## How it works on stage

There are two windows:

- **The projector window** is what the audience sees: slides, Grafana dashboards
  and one late customer request. You never click in it.
- **The presenter desk** is on your laptop screen. It shows what to say, the one
  or two buttons for this moment, and what to watch for.

The talk is a sequence of **beats**. Press **Next** on the desk (or your clicker,
→, Space or Page Down) and the projector moves to whatever the next beat needs:
the next slide, a dashboard, or a "What Drasi just did" visual. Your clicker works
whichever window is in front.

Some beats have buttons, such as **Fail EU-A** or **Approve selected plan**. They
appear in the beat itself: press them when you reach the beat, while you say its
words. Nothing in the demo changes unless you press a button; **Back** and
**Next** never change the demo. If you forget a button, Next reminds you once.

Your clicker moves Back and Next. Buttons need a click on the desk with the
trackpad or mouse.

If the desk isn't open, → / Space / Page Down (or the clicker) on the projector
window still steps through the slides and dashboards on its own, but nothing in
the demo changes. Open the desk to give the talk.

## Set up your screens

1. Connect the projector and set the displays to **extend**, not mirror
   (macOS: System Settings › Displays › Use as: Extended display). If they mirror,
   the audience sees your desk.
2. In Chrome, open the desk: http://127.0.0.1:19580/presenter
3. On the desk, press **Open projector window**. Drag that window onto the
   projector and make it full screen (Ctrl ⌘ F). The desk's header then says
   **Projector window connected**.
4. Keep the desk on your laptop screen. Turn on Do Not Disturb, quit chat apps,
   and plug in power.

## Before you present

**The day before:** rehearse the whole talk at least once with the live demo, and
once using only the recordings (see [When something goes wrong](#when-something-goes-wrong)).

**30 minutes before:**

1. Start the demo ([SETUP.md › Start the demo](SETUP.md#start-the-demo)).
2. Optional: sign in to Langfuse **privately** on the laptop, in case someone asks
   to see it ([SETUP.md › Get ready to present](SETUP.md#get-ready-to-present)).
   Nothing on the projector needs a sign-in.
3. [Set up your screens](#set-up-your-screens).
4. On the desk, open **Before and after the talk** and check it says
   **✓ Starting state**. If not, press **Reset scenario…** then **Confirm reset**.
5. Check the header says **READY**.
6. Step through a few beats with your clicker to check it moves the projector,
   then go back to the first beat (**Jump to a part** › 1. Title).

**5 minutes before:** press **Start demo traffic** (All demo controls), so the
charts have history when you get to them. Traffic stops by itself after 90
minutes. If the desk ever shows **Traffic: stopped** during the talk, press
**Start demo traffic** again.

**If the demo won't start or the header stays NOT READY:** give the talk in
recording mode. Press **Use recordings for the rest of the talk** in the warning
at the top of the desk. Everything else works the same.

## The presenter desk

Top to bottom:

| Part | What it's for |
|---|---|
| Header | **READY** or **NOT READY**, whether the projector window is connected, and the talk timer. It starts by itself on your first **Next** and turns amber at 22 minutes; tick **Short on time** if you're past about 11:30 when Recovery starts |
| **The current beat** (left) | Part number and name, dots for the beats in this part, **◀ Back / Next ▶** and **what the projector shows** (always in the same place), the beat's buttons and their result, what to say, what to watch for, a preview of the next beat, what to do if it breaks (with **Use recordings from here on**), and how to undo a wrong button |
| **Drasi's answers** (right) | The same answers the audience sees: Support overdue, Action needed, Recovery, Safe if one fails? |
| **Live demo** (right) | Each deployment's health and capacity; whether traffic is running; where the summaries may run |
| **Recovery plan** (right) | Where support and summaries run now, the proposed plan, and the plan you selected for approval, with a line saying whether it needs your approval, and the latest approval status |
| Jump to a part | Go straight to any part of the talk (also moves the projector) |
| All demo controls | Every button, if you need one out of order, plus single requests and changes to any deployment |
| Show something on the projector | Put any slide, dashboard or visual on the projector without changing your place |
| Recordings (fallback) | Show one recording on the projector, or switch to recording mode for the rest of the talk |
| Questions to expect, and glossary | The same as in this guide |
| Before and after the talk | Starting-state check, reset, full settings |

**Buttons:** the button for the current beat is bright. A ✓ means it's already
done (traffic running, plan selected, EU-A failed). Grey dashed means it can't be
pressed yet (for example, Approve before you've selected a plan). **Fail EU-A**
is red.
After pressing, the result appears right under the buttons: **Done.**, or in green
**Rejected, as intended** for the approval that's meant to be rejected.

**Short on time:** tick it to skip the beats marked optional (the policy change
and the late request view). Their buttons and notes disappear from the sequence.

**The approval flow.** Drasi's planner proposes a plan and re-works it whenever
anything changes; nothing changes until you approve it. **Select proposed plan**
takes a copy; **Approve selected plan** sends it to the change-approval tool,
which only records it. Drasi then checks it: if the plan and the facts it was
worked out from (policy, capacity, health) are unchanged, Drasi's reaction
applies the new routes through the gateway's configuration API; otherwise it
rejects the approval with a reason, which the desk shows. After an approval,
your selection clears, so you can't approve twice by accident. That's also true
after a rejected approval: select the plan again before approving.

<!-- generated from presentation/notes.ts: do not edit; run npm run sync-presenter in demo/ui -->

## Run of show

| Time | Part | Projector shows | You |
|---|---|---|---|
| 0:00 | Title | Title | Introduce yourself and the talk |
| 0:30 | The scenario | The scenario | Set up the scenario and the three questions |
| 2:00 | Drasi in one picture | Drasi in one picture | Explain Drasi's parts and what is built in |
| 3:30 | Healthy service | Grafana · Operations, What Drasi just did | Walk through the healthy dashboard |
| 6:00 | Failure | Grafana · Operations, What Drasi just did | Fail EU-A |
| 9:00 | Who is affected | Who is affected: the continuous query, One late request | Show who is affected: the query and its live list |
| 11:30 | Recovery | Grafana · Recovery plan, What Drasi just did, Grafana · Operations | Approve the proposed plan; optionally show a stale approval rejected |
| 17:00 | Healthy but fragile | Grafana · Recovery plan, Grafana · Operations, What Drasi just did | Healthy but fragile: EU-A returns with half its capacity |
| 20:30 | Close | Close | Four takeaways, then questions |
| 22:00 | Questions | Close | Take questions |

## The script

The desk shows one beat at a time. Press **Next** (or your clicker, or →) to move on: the
projector follows. Beats marked optional are skipped when **Short on time** is ticked.
Say it in your own words.

### 1. Title (30 seconds)

1. **Title.** Introduce yourself. Your monitoring tells you something is wrong. This talk is about getting the next answers too: who is affected, what to do, and whether it worked, worked out the moment things change, in the tools you already use.

**If it breaks:** The slides don't depend on the demo. Carry on.

### 2. The scenario (1½ minutes)

1. **The scenario.** A company runs two AI workloads on the same limited AI model capacity: an assistant that answers customer-support questions, and a background job that writes document summaries. The capacity is split across three deployments, EU-A and EU-B in Europe and US-C in the United States, each with 100k tokens a minute.
2. **The scenario.** Two rules. Customer support handles restricted data, so it may only run in Europe; summaries use public data and can run anywhere. Customers expect an answer within 15 seconds, and moving work between deployments needs a person's approval.
3. **The scenario.** When EU-A fails, the business needs three answers: who is affected right now, what's the fastest fix that stays within policy, and how do we know it's really fixed? You're about to see Drasi answer all three.

**If it breaks:** Carry on; the demo doesn't depend on the slides.

### 3. Drasi in one picture (1½ minutes)

1. **Drasi in one picture.** Drasi is open-source change-driven computation. Sources bring in every change as it happens: here, from the AI service's database. Continuous queries keep a result up to date as changes arrive, and as time passes. Continuous models run logic such as policies and planners on those results and re-run it whenever an input changes. Reactions deliver each change to other systems.
2. **Drasi in one picture.** What's what. Drasi does the hard part: it keeps results up to date incrementally as changes arrive, re-checks them when time passes a deadline, and re-runs the models whenever their inputs change. For this demo we wrote three plugins: a policy model that runs Rego rules, an optimizer for the planner and the what-if, and a reaction that publishes answers to Prometheus. The rules and settings are their configuration. The AI service, its change-approval tool and its gateway configuration API are the systems Drasi works with. 'Without polling' is about computing the answers: nothing is recomputed on a schedule. Health checks still run every second, as in any system, and Prometheus and Grafana still collect and draw on their own schedule; Drasi reacts to what changes.

**If it breaks:** Carry on.

### 4. Healthy service (2½ minutes)

1. **Grafana · Operations.** This is the AI service's Grafana dashboard. The top half is ordinary monitoring, measured from real requests: calls, errors, answer time, the support queue and Grafana alerts. OpenTelemetry, Prometheus and Grafana; nothing new.
   - **Press:** Start demo traffic
   - **Watch for:** Charts moving. Support errors 0%, Support queue 0 to 2 (green). (Start demo traffic only if it isn't already running.)
2. **Grafana · Operations.** The bottom half is Drasi's answers, worked out from changes in the AI service's database rather than from metrics. The service records every request and attempt, and checks each deployment's health every second. Drasi turns that into: overdue support requests, each deployment's status, recovery, what action is needed, and whether we'd survive losing any one deployment. A Drasi reaction publishes them as ordinary Prometheus metrics, so you can alert on them like anything else.
   - **Watch for:** Support overdue 0, all deployments Healthy, Recovery Nothing to verify, Action needed None, Safe if one fails? Yes.
3. **What Drasi just did.** So far Drasi has read every change in the database, kept its queries current, run the policy, planner and what-if, and its reaction has published the answers for Grafana. Nothing new to watch: the answers sit where you already look.

**If it breaks:** If the charts are empty, traffic isn't running: press Start demo traffic and keep talking.

### 5. Failure (3 minutes)

1. **Grafana · Operations.** Now I'll make EU-A, the deployment serving customer support, fail.
   - **Press:** Fail EU-A
   - **Watch for:** Within about 8 seconds: Deployment status EU-A Failed, and Action needed Approve proposed plan.
2. **Grafana · Operations.** Look at the bottom row first: EU-A Failed, and Action needed already says Approve proposed plan. Drasi has worked out a fix that fits the rules. We'll come back to it.
3. **Grafana · Operations.** The top half does its job: calls fail, errors and the queue climb, and Grafana's alert fires. Something is wrong.
   - **Watch for:** Support errors red, Support queue rising, Grafana alerts firing 1 or 2 (the second is an alert on Drasi's overdue answer).
4. **Grafana · Operations.** And Support overdue is climbing. Each request has a 15-second deadline. When a request passes it, nothing happens to that request: no new row, no new metric. Drasi counts it at that exact moment anyway, because its query re-checks each request at its deadline. On the chart, the red overdue line follows the orange waiting line 15 seconds later. I'll show you how in a moment.
   - **Watch for:** Support overdue above 0; on the right-hand chart, the red line trails the orange one.
5. **What Drasi just did.** Drasi saw the failed attempts and EU-A's health turn to Failed, worked out which requests were late as each deadline passed, and re-ran the planner. It didn't re-scan the database; it reacted to each change and each deadline. And notice the order: Drasi had a plan ready before Grafana's alert fired.

**If it breaks:** If nothing changes within 30 seconds, press Use recordings from here on and say it's a recording.

### 6. Who is affected (2½ minutes)

1. **Who is affected: the continuous query.** Which customers are affected? This is the actual query behind Support overdue. 1: every request, as it arrives from the database. 2: not answered yet. 3: this condition becomes true at the deadline, and Drasi re-checks just that request then. 4: the result is the list itself, not just a count, and here it is, live. With Prometheus you'd get a number from a gauge or a rule evaluated every so often; here it's the exact set of late requests, kept current request by request, from one query.
   - **Watch for:** Under the query: how many support requests are late, and the oldest few with their traces.
2. **One late request (optional).** Here's one of those customers: their question, every attempt to send it to EU-A failing, and the moment its deadline passed and Drasi listed it. The same request is a trace in Langfuse, so whoever picks it up has the detail.
   - **Watch for:** The oldest late request, with its attempts and the moment Drasi listed it.

**If it breaks:** If the list is empty, traffic has stopped: say what it would show. The request beat is skipped automatically when nothing is late.

### 7. Recovery (5½ minutes)

1. **Grafana · Recovery plan.** This dashboard is all Drasi. Capacity: 300k in total, 200k on healthy deployments, 140k needed. On paper there's plenty. But US-C has the free room, and customer support isn't allowed there.
   - **Watch for:** Customer support may run on: US-C Not allowed.
2. **Grafana · Recovery plan.** The planner, an optimizer Drasi re-runs on every change, respects that: move summaries to US-C, which frees EU-B for support. I'll take this plan to our change-approval tool, here on my laptop.
   - **Press:** Select proposed plan
   - **Watch for:** Proposed plan: support → EU-B, summaries → US-C. (Selecting changes nothing on the projector.)
3. **Grafana · Recovery plan (optional).** While it waits for approval, compliance restricts summaries to Europe too. The answer changes at once: no valid plan. Plans keep both workloads running, and with EU-A down, support and summaries can't both fit in EU-B. Dropping work is a decision for a person, not the planner.
   - **Press:** Summaries: Europe only
   - **Watch for:** Document summaries may run on: US-C Not allowed. Action needed and Proposed plan: No valid plan.
4. **Grafana · Recovery plan (optional).** If I approve the plan I selected a moment ago, Drasi rejects it: every plan carries the facts it was based on, and those facts changed.
   - **Press:** Approve selected plan (it's meant to be rejected)
   - **Watch for:** Last approval: Rejected.
5. **Grafana · Recovery plan (optional).** Compliance allows the US for summaries again, and the plan comes straight back.
   - **Press:** Summaries: Europe + US
   - **Watch for:** Action needed: Approve proposed plan.
6. **What Drasi just did (optional).** The policy change came in as a change like any other. Drasi re-ran the policy and the planner, saw that the approval no longer matched its current plan, and its reaction rejected it in the approval tool, with the reason.
7. **Grafana · Recovery plan.** Now I approve the current plan. Drasi checks it still matches its plan, and its reaction applies the new routes through the gateway's configuration API.
   - **Press:** Select proposed plan, then Approve selected plan
   - **Watch for:** Last approval: Applied, awaiting evidence. Running now changes. Action needed None. Recovery Awaiting evidence.
8. **Grafana · Recovery plan.** Look at Recovery. Approving isn't enough to call it fixed: another Drasi query waits until the queue has cleared and at least three support requests have been answered on the new route. Only then does it say Verified.
   - **Watch for:** Recovery goes from Awaiting evidence to Verified, often within 30 seconds of approval. If it's already Verified, say so.
9. **Grafana · Operations.** On Operations: the queue has drained, Support overdue is back to 0 and errors are falling. EU-A is still down: we routed around it, within policy.
10. **What Drasi just did.** Drasi kept the policy and the plan current as health and capacity changed, applied the approved plan, and then checked the outcome before recording it as verified. Approving a change isn't the same as fixing the problem.

**If it breaks:** If Recovery stays Awaiting evidence, say it's applied but not yet verified, and carry on. Never approve twice. If the plan or the policy change doesn't behave as described, press Use recordings from here on.

### 8. Healthy but fragile (3½ minutes)

1. **Grafana · Recovery plan.** Later that day EU-A is fixed, but it comes back with only 50k of capacity. Nothing is broken.
   - **Press:** EU-A back at 50k
   - **Watch for:** Deployment capacity EU-A 50k. Safe if one fails? No: not if EU-B fails. Action needed: At risk.
2. **Grafana · Operations.** Every service-health signal is green. But Drasi says we're not safe. Support now runs on EU-B. If EU-B failed, support would need 60k in Europe, and EU-A has only 50k.
3. **Grafana · Recovery plan.** That's the what-if model: for each deployment, would every workload still fit somewhere allowed if that one failed? It re-asks whenever anything changes. A game-day question, answered all the time. And it's a metric, so you can alert on it.
4. **Grafana · Recovery plan.** Give EU-A its full capacity back, and the answer is Yes again, without any outage.
   - **Press:** EU-A back to 100k
   - **Watch for:** Safe if one fails? Yes. Action needed None.
5. **What Drasi just did.** One capacity change, and Drasi re-checked every what-if. Are we safe from the next failure? Answered before it happens.

**If it breaks:** If Safe if one fails? doesn't change, press Use recordings from here on.

### 9. Close (1½ minutes)

1. **Close.** Keep your monitoring tools, and add change-driven computation next to them. Drasi turned changing facts (requests, health, capacity, policy, approvals and time) into live answers: who is late, what fits the rules, whether the fix worked, and whether we're safe from the next failure. To learn more, go to drasi.io, on screen. Thank you. Questions?
2. **Close.** After questions, stop the demo traffic.
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
| Source | How Drasi hears about changes; here, the database's live list of changes |
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
| Does it only work with PostgreSQL? | No. Drasi has sources for other databases, Kafka, Kubernetes, OpenTelemetry and HTTP. This demo uses PostgreSQL's change feed. |
| Are the policy, planner and what-if part of Drasi? | Not yet built in: they're plugins written for this demo, in the repository. One runs Rego rules; the other is an optimizer on the open-source good_lp solver. Both are generic: the rules, workloads and capacities are configuration. Drasi runs them as continuous models and re-runs them when their inputs change. |
| Who changes the routes? | Drasi's reaction calls the gateway's configuration API, only for an approval that still matches Drasi's current plan. The AI service just applies the call, and refuses it if the scenario changed in between. |
| How do the answers reach Grafana? | A Drasi reaction keeps every answer as a Prometheus metric. Prometheus scrapes it and Grafana shows it. |
| What happens when Drasi restarts? | It rebuilds its state from the database and resumes the change feed. Until then the tiles show Unknown. |
| How does it scale? | This demo isn't a benchmark. Each query only keeps what it needs, here the unanswered requests. For performance work, see drasi.io. |
| Anything else | "Good question. I'll follow up; drasi.io, on screen, is a good place to start." |

<!-- end generated -->

## Reading the dashboards

Colours mean the same everywhere: **red** broken, **orange** needs action or
waiting, **green** OK, **grey** nothing to show. **Unknown** means Drasi's answer is
out of date or unavailable; it never means healthy.

**AI service · Operations**

| Area | Tile | Meaning |
|---|---|---|
| Monitoring (top) | Model calls | Calls to the AI model per second, last 30 seconds |
| | Support errors | Share of support calls failing, last 30 seconds |
| | Answer time p95 | 95th-percentile time from a customer's question to its answer. **No answers** during the outage |
| | Support queue | Support requests waiting for an answer. One or two in flight is normal (green) |
| | Grafana alerts firing | Grafana alert rules firing: "Model calls failing", and "Support requests overdue", an alert on Drasi's answer |
| | Model calls (chart) | Answered and failed calls for each workload |
| | Support requests waiting (chart) | Orange: waiting. Red: overdue, from Drasi. The red line trails the orange one by the 15-second deadline |
| Drasi's answers (bottom) | Support overdue | Support requests past their 15-second deadline |
| | Deployment status | Each deployment's latest health report: Healthy, Over quota or Failed; Unknown if it has been silent for 8 seconds |
| | Recovery | Nothing to verify, Awaiting evidence, or Verified |
| | Action needed | None; Approve proposed plan; No valid plan; At risk (healthy, but losing one deployment would leave no valid plan) |
| | Safe if one fails? | If any one deployment failed, would every workload still fit somewhere allowed? **No** names the deployment whose loss would hurt. **Already one down** while a deployment has failed: the question doesn't apply |

Dashed green lines on the charts mark Drasi's decision events (plans, approvals,
verified recovery), which Drasi posts to Grafana.

**AI service · Recovery plan** (everything on it comes from Drasi)

| Tile | Meaning |
|---|---|
| Action needed, Recovery, Safe if one fails? | As above |
| Last approval | None yet; Approved, applying; Applied, awaiting evidence; Applied and verified; Rejected: out of date |
| Running now | Where each workload runs and the capacity it needs. **held: not allowed** when policy no longer allows it there |
| Proposed plan | The planner's plan, in orange when it differs from what's running. **Same as running now** or **No valid plan** otherwise |
| Customer support / Document summaries may run on | Drasi's Rego policy answer for each deployment: Allowed or Not allowed |
| Decision events | Newest first: plans, approvals, rejections with the reason, verified recovery |
| Capacity | Total, on healthy deployments, and needed by the two workloads |
| Deployments | Each deployment's capacity, orange when below its normal 100k; **Failed** while it is down |

The deadlines (15-second target, 8-second silence) are shortened for the demo.

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
| Recovery stays Awaiting evidence | Say it's applied but not yet verified; keep going. Never approve twice |
| Approve is grey | Press **Select proposed plan** first |
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
