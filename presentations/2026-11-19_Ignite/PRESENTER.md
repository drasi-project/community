# Presenter guide

Everything you need to give this talk or run the demo, without needing to know
how it's built. To install and start the demo, see [SETUP.md](SETUP.md). To
understand what the demo is about, see [README.md](README.md).

**The talk in one line:** three short slides, then about 18 minutes live in
Grafana showing a real AI service fail, Drasi working out a valid recovery, you
approving it, and Drasi confirming it worked.

- Session: THR1029, *Monitoring without polling*, 25 minutes
- Aim for 22 minutes of content and 3 minutes of questions
- During the talk, the presenter desk shows the same notes as this guide, one cue
  at a time

## Contents

1. [Your screens](#your-screens)
2. [Before you present](#before-you-present)
3. [Run of show](#run-of-show)
4. [The script](#the-script)
5. [Reading the dashboards](#reading-the-dashboards)
6. [The presenter desk](#the-presenter-desk)
7. [When something goes wrong](#when-something-goes-wrong)
8. [After the talk](#after-the-talk)

## Your screens

You use two screens. **Never project the presenter desk.**

| Projected for the audience | Private, on your laptop |
|---|---|
| Slides: http://127.0.0.1:18580/present | Presenter desk: http://127.0.0.1:18580/presenter |
| Then Grafana, Langfuse and Drasi Server, in the same window | |

The projected window starts on the slides. Advancing past slide 2 opens Grafana
in the same window. From there, the links along the top of each Grafana dashboard
take you to the other dashboard, Langfuse, Drasi Server, and back to the closing
slide. You never need to switch windows on the projector.

The presenter desk is where you make things happen: start traffic, fail an
deployment, approve recovery, change policy. It also shows your notes for the
current cue. Moving between cues on the desk only changes your notes; it never
changes what the audience sees or the demo.

## Before you present

**The day before:** rehearse the whole talk at least once with the live demo, and
once using only the recordings (see [When something goes wrong](#when-something-goes-wrong)).
Printing this guide gives you a paper copy of the script.

**30 minutes before:**

1. Start the demo ([SETUP.md › Start the demo](SETUP.md#start-the-demo)).
2. Sign in to Grafana and Langfuse **privately**, before projecting either
   ([SETUP.md › Get ready to present](SETUP.md#get-ready-to-present) shows the passwords).
3. Open the slides on the projector and the presenter desk on your laptop.
4. Check the banner at the top of the desk says **READY**.
5. On the desk, check the starting state: all three deployments Healthy, support on
   EU-A, summaries on EU-B. If anything is different, use **Reset scenario…**.
6. In Grafana, close the left navigation menu so the dashboard fills the screen,
   and check the dashboard links are visible along the top.
7. Check text is readable from the back of the room; zoom the browser (Cmd +) if needed.
8. Test your clicker on the slides.

**2 minutes before:** click **Start demo traffic** on the desk so Grafana's charts
have data when you arrive. Traffic stops by itself after 30 minutes.

<!-- generated from presentation/notes.ts: do not edit; run npm run sync-presenter in demo/ui -->

## Run of show

| Time | Cue | Audience sees | You |
|---|---|---|---|
| 0:00 | Title | Title slide | Introduce yourself and the session |
| 0:30 | The scenario | Scenario slide | Set up the scenario, then Drasi's role |
| 2:30 | Healthy service | Grafana · Operations | Walk through the healthy dashboard |
| 5:00 | Failure and customer impact | Grafana · Operations | Fail EU-A |
| 8:00 | Recovery | Grafana · Policy and recovery | Approve Drasi's proposed plan |
| 13:00 | The continuous query | Drasi Server | Show the query behind "overdue" |
| 15:00 | Policy changes the answer | Grafana · Policy and recovery | Restrict summaries to Europe (optional) |
| 17:00 | Healthy but fragile | Grafana · Policy and recovery | Cut EU-B's budget, then restore it |
| 20:30 | Close | Closing slide | Three takeaways |
| 22:00 | Questions | Closing slide | Take questions |

## The script

The same notes appear cue by cue on the presenter desk, under **What to do**. The wording is a guide, not a script to read aloud.

### 1. Title (30 seconds) · Title slide

- **Say:** This talk is about adding change-driven computation alongside the observability tools you already use.
- **Do:** Introduce yourself, then advance to the next slide.
- **Point at:** The session title.
- **What happens:** Nothing changes in the demo.
- **If it breaks:** The slides don't depend on the demo. Carry on.
- **Next:** "First, what does Drasi add?"

### 2. The scenario (2 minutes) · Scenario slide

- **Say:** Here's a situation many teams running AI will recognise. A company's AI assistant answers customer-support questions, and a background job summarizes documents. Both draw on the same limited AI model capacity, deployed in three regions, each with a budget of tokens per minute. Support data is restricted, so it must stay in Europe; the summaries use public data and can run anywhere. Customers expect an answer within 15 seconds, and moving work between regions needs someone's approval. Right now EU-A serves support, EU-B runs the summaries, and US-C in the United States is spare. When EU-A fails, customers stop getting answers, and the business needs to know three things: who is affected right now, what's the fastest fix that stays within company policy, and how do we know it's really fixed? Your monitoring tools will tell you something is wrong; answering those three questions is where Drasi comes in. Drasi is change-driven computation: sources bring in changes, continuous models keep results current as data changes and as time passes, and reactions publish results or take action. The AI models and capacity are simulated; the requests, monitoring and Drasi computation are real.
- **Do:** When you've explained the slide, advance (or click Live demo · Grafana) to open Grafana.
- **Point at:** The capacity in each region and what is using it, the rules on the right, then the three questions. Finish on the Drasi section at the bottom: sources, continuous models, reactions.
- **What happens:** Advancing opens Grafana's Operations dashboard in the same window.
- **If it breaks:** If Grafana doesn't open, open http://localhost:13000/d/ignite-workload in the projected window.
- **Next:** "Let's run this service in Grafana."

### 3. Healthy service (2½ minutes) · Grafana · Operations

- **Say:** The top half is a normal service dashboard measured from real requests: call rate, errors, latency, the support queue and a Grafana alert. OpenTelemetry collects the measurements, Prometheus stores them and Grafana shows them. The bottom half is what Drasi adds: customer impact, kept current continuously.
- **Do:** Nothing to click. If traffic isn't running, click Start demo traffic on the desk.
- **Point at:** Support errors 0%, Support queue 0, Support overdue 0, all three deployments Healthy, Recovery None in progress, Action needed None.
- **What happens:** The charts update every 5 seconds.
- **If it breaks:** If the charts are empty, traffic isn't running: start it and keep talking. Never wait on stage for a chart to fill.
- **Next:** "Now the deployment serving customer support fails."

### 4. Failure and customer impact (3 minutes) · Grafana · Operations

- **Say:** The monitoring stack sees failed calls and a growing queue, as it should. Drasi answers a different question: which customer requests have now missed their deadline? That answer changes just because time passes, with no new event on the request.
- **Do:** On the desk, choose Deployment EU-A and Action fail, then click Apply to deployment. Stay on the Grafana page. Optional: click Submit support request, then Open returned task trace to show that request in Langfuse.
- **Point at:** Model calls by outcome, Support errors, Support queue, Deployment status, Support overdue and Action needed.
- **What happens:** In order: support → EU-A shows HTTP 503 in red; Support errors and Support queue rise; Deployment status shows EU-A Failed; after about 15 seconds Support overdue starts counting; after about 30 seconds the Grafana alert fires; Action needed changes to Approve proposed plan.
- **If it breaks:** If nothing happens within 30 seconds, play the failure recording.
- **Next:** "US-C has spare capacity. Can we use it?"

### 5. Recovery (5 minutes) · Grafana · Policy and recovery

- **Say:** On paper we have plenty: 200k of healthy capacity for 140k of work. But US-C has the room, and policy denies it for support. Drasi's answer moves the summaries to US-C, which frees EU-B for support. Drasi keeps that complete plan current as things change. But approving a plan isn't the same as proving it worked.
- **Do:** Click Policy and recovery in Grafana's links. On the desk, click Select proposed plan, then Approve selected plan, once.
- **Point at:** Capacity: Healthy 200k, Needed 140k. Customer support may run on: US-C Denied. Document summaries may run on: US-C Allowed. Proposed routes: support → EU-B and summaries → US-C.
- **What happens:** Decision events records the approval; Current routes change; Recovery shows Awaiting evidence, then Verified once real support requests succeed for 5 seconds. Back on Operations, the queue drains and successful calls return.
- **If it breaks:** If Recovery stays at Awaiting evidence, say it's applied but not yet verified, and keep traffic running. Never approve a second time. If needed, play the recovery recording.
- **Next:** "Here's the continuous query behind one of those answers."

### 6. The continuous query (2 minutes) · Drasi Server

- **Say:** This is a continuous query, not a scheduled check. Drasi keeps its result current as changes arrive, and re-evaluates it when time alone changes the answer. These results feed the models and dashboards you've just seen.
- **Do:** Click Drasi queries in Grafana's links. Select the overdue-tasks query, click Expand query, and zoom the browser (Cmd +) so it's readable. Keep this to two minutes.
- **Point at:** The deadline condition in the query.
- **What happens:** Nothing changes in the demo.
- **If it breaks:** If the query text doesn't appear (a known Drasi Server UI issue), describe the query in one sentence and go back to Grafana.
- **Next:** "Policy is another input to the same computation."

### 7. Policy changes the answer (2 minutes) · Grafana · Policy and recovery

- **Say:** A proposal belongs to the facts that produced it. Change the policy and the answer changes, even though nothing has failed. An old approval can't silently apply a different plan.
- **Do:** On the desk, click Select proposed plan. Then click Batch policy: Europe only. When US-C turns Denied for summaries, click Approve selected plan. Afterwards, click Batch policy: Europe + US to restore the policy.
- **Point at:** Document summaries may run on: US-C, and Decision events.
- **What happens:** US-C changes to Denied for summaries, and the approval is rejected; the rejection appears in Decision events.
- **If it breaks:** Restore the policy (Batch policy: Europe + US) and describe what happened in words. Play the policy recording if needed.
- **Next:** "Finally: can a healthy service survive the next failure?"

### 8. Healthy but fragile (3½ minutes) · Grafana · Policy and recovery

- **Say:** Everything is healthy: support on EU-A, summaries on US-C. But EU-B now has only 50k of capacity, and support needs 60k. If EU-A failed, support would have nowhere it's allowed to go. A dashboard of current health can't tell you that.
- **Do:** On the desk, click Healthy-fragile inputs · EU-B 50k. After explaining, click Restore EU-B capacity · 100k.
- **Point at:** Safe if EU-A fails? and Deployment budgets. Optionally show Operations: everything is green.
- **What happens:** Safe if EU-A fails? changes to No. After restoring EU-B it changes back to Yes, with no outage.
- **If it breaks:** Play the resilience recording.
- **Next:** "That's the idea: change-driven computation alongside the tools you already use."

### 9. Close (1½ minutes) · Closing slide

- **Say:** Keep your observability tools. Add change-driven computation to connect changing context, keep decisions valid, and verify what actually happened.
- **Do:** Click Closing slide in Grafana's links. Take questions. When you've finished, click Stop demo traffic on the desk.
- **Point at:** The three takeaways and the repository link.
- **What happens:** The projected window returns to the slides.
- **If it breaks:** Summarize in words. Never project the desk.
- **Next:** "Questions."

<!-- end generated -->

**Running late?** Tick **Short on time** on the desk and skip the policy change.
If you're still behind, keep the Drasi Server cue to one sentence.

## Reading the dashboards

**AI service · Operations**

| Area | Panel | Meaning |
|---|---|---|
| Service health (measured) | Model calls | Calls per second, both workloads |
| | Support errors | Share of support calls failing in the last minute |
| | Support p95 | 95th-percentile support call time |
| | Support queue | Support requests waiting for an answer |
| | Service alerts | Grafana's HTTP-failure alert |
| | Model calls by outcome | Calls per second by route and HTTP status: 200 ok, 503 failed, 429 over quota, 409 refused because the route changed |
| | Requests answered | Completed requests per second |
| Customer impact (Drasi) | Support overdue | Support requests past their 15-second target |
| | Deployment status | Each deployment's latest report; Unknown if none for 8 seconds |
| | Recovery | None in progress, Awaiting evidence, or Verified |
| | Action needed | None, Approve proposed plan, or No valid plan |

**AI service · Policy and recovery** (all from Drasi)

| Panel | Meaning |
|---|---|
| Action needed, Recovery | As above |
| Safe if EU-A fails? | Would every workload still fit somewhere allowed without EU-A? |
| Current routes / Proposed routes | Where each workload runs now, and Drasi's best plan |
| Customer support / Document summaries may run on | Policy decision for each deployment |
| Decision events | Approvals, rejections, route changes, verified recovery |
| Capacity | Total capacity, the part on healthy deployments, and what the workloads need |
| Deployment budgets | Simulated capacity of each deployment |

**Show Drasi decisions on charts** (switch at the top left of Operations, off by
default) draws a dashed marker on both charts at each Drasi decision: approval,
rejection, routes adopted, recovery verified. Hover over a marker to read it.
Turning it on during the recovery cue shows the approval, then errors falling and
successful calls returning just after it.

**Unknown** means Drasi's answer is stale or unavailable. It never means healthy.
The deadlines (15-second target, 8-second report expiry) are shortened for the
demo.

## The presenter desk

The desk only changes the demo's inputs. Everything on the dashboards is then
worked out from real requests and Drasi. Grafana refreshes every 5 seconds.

**Banner:** **READY** means the live demo is working and the controls are
available. **NOT READY** means Drasi's answers are unavailable or out of date:
the controls are disabled. Use a recording, or see Troubleshooting in SETUP.md.

**Cue panel**

| Control | What it does |
|---|---|
| Start timer | Starts the talk clock; it turns amber at 22 minutes |
| Previous cue / Next cue / Cue list | Changes your notes only |
| Short on time | Skips the policy change |
| Grafana, Langfuse and Drasi Server links | Open the tools in a new tab on your laptop |
| Show a slide in the projected window | Switches the projected window to the title, concepts or closing slide |

**What to do:** your notes for the current cue: what to say, what to do, what to
point at, what happens, what to do if it breaks, and the line into the next cue.

**Traffic and faults**

| Control | What it does |
|---|---|
| Start demo traffic / Stop demo traffic | One support and one summary request every 5 seconds, for up to 30 minutes |
| Submit support request / Submit batch request | Sends one request, then offers **Open returned task trace** to show it in Langfuse |
| Deployment, Action, **Apply to deployment** | Changes one deployment (below) |

| Action | Effect |
|---|---|
| fail | Every request to the deployment fails (HTTP 503) |
| throttle | Every request is rejected as over quota (HTTP 429). Not a slowdown |
| restore | Back to normal |
| pause-reports | The deployment keeps working but stops reporting; it shows Unknown after 8 seconds |
| resume-reports | Reporting resumes |

**Approving a plan**

| Control | What it does |
|---|---|
| Select proposed plan | Keeps a copy of Drasi's current proposal. If the proposal changes afterwards, the desk shows **STALE** |
| Approve selected plan | Applies the selected plan, after checking it's still valid. A stale plan is rejected. Clicking again retries the same approval |

**Scenario settings**

| Control | What it does |
|---|---|
| Batch policy: Europe only / Europe + US | Forbids or allows summaries on US-C |
| Healthy-fragile inputs · EU-B 50k | Everything healthy; support on EU-A, summaries on US-C; EU-B cut to 50k |
| Restore EU-B capacity · 100k | Restores EU-B |
| Show current settings | Shows the current configuration |
| Reset scenario… then Confirm reset | Restores the starting state and cancels waiting requests. History is kept |

**Recordings (fallback):** see below.

**Approval status:** the latest approval in plain words: approved and applying,
applied and waiting for evidence, recovery verified, or rejected with the reason.

## When something goes wrong

**Play a recording.** The desk's **Recordings (fallback)** panel has recordings of
the failure, recovery, policy and resilience parts.

1. On the desk, click the recording. Live controls are disabled while it's selected.
2. In the projected window, go back to the slides: click **Closing slide** in
   Grafana's links (or open http://127.0.0.1:18580/present). Instead of the
   slide, it shows the recording, labelled **REPLAY · NOT LIVE**. Press play.
3. Say clearly that it's a recording.
4. To go back to live: tick **I've checked the live demo is ready** on the desk,
   click **Return to verified live state**, then return to Grafana.

| On stage | Do this |
|---|---|
| Charts are empty | Start traffic and keep talking |
| Tiles show Unknown | Drasi's answer is out of date. Say so; never call it healthy. Move on or play a recording |
| The failure isn't showing after 30 seconds | Play the failure recording |
| Recovery stays at Awaiting evidence | Say it's applied but not yet verified; keep traffic running; play the recovery recording if needed |
| Approve selected plan is disabled | Click **Select proposed plan** first |
| The desk banner says NOT READY | Use recordings for the rest of the demo |
| You lost your place | Use the Cue list on the desk; it doesn't change the projected screen |

Golden rules: never project the desk, never call Unknown healthy, approve once,
and never reset during the talk.

## After the talk

1. Stop demo traffic.
2. Restore any deployment you changed, and the batch policy if you changed it.
3. Use **Reset scenario…** so the next presenter starts clean.
4. If you're finished for the day, [stop the demo](SETUP.md#stop-the-demo).
