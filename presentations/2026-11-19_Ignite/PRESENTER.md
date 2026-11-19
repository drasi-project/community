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
the next slide, a dashboard, or the incident channel. Your clicker works
whichever window is in front.

Some beats have buttons, such as **Summaries: Europe + US** or **Fail EU-A**. They
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
| Header | **READY** or **NOT READY**, whether the projector window is connected, **Ignite template assets missing** if they are, and the talk timer. It starts at 00:00 when you press **Open projector window** (**Restart timer** resets it) and turns amber at 22 minutes; tick **Short on time** if you're past about 11:30 when Recovery starts |
| **The current beat** (left) | Part number and name, the beats in this part with their numbers (beats are numbered across the whole talk, as in the script below; the current one also shows beside **what the projector shows**), **◀ Back / Next ▶** and **what the projector shows** (always in the same place), the beat's buttons and their result, **Click on the projector** (what to press or scroll on the projected channel), what to say, what to watch for, a preview of the next beat, what to do if it breaks (with **Use recordings from here on**), and how to undo a wrong button |
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
routes are applied**, or **Rejected by Drasi** with its reason.

**Short on time:** tick it to skip the beats marked optional (the late request
view). Their notes disappear from the sequence.

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

| Time | Beats | Part | Projector shows | You |
|---|---|---|---|---|
| 0:00 | 1–8 | Intro | Walk-in, Who I am, Drasi, the project, Grafana · Operations, Why Drasi exists, Change-driven computation with Drasi | Walk-in; who I am; the project; what I'll show you; why Drasi exists |
| 2:00 | 9–11 | What Drasi is | Simple concepts · one query, several sources, Simple concepts · many queries and reactions, Simple concepts · queries build on queries | Sources, Continuous Queries, Reactions, and queries building on queries |
| 4:30 | 12–15 | Observability and the scenario | Observability issues, What organizations want, What organizations want · with Drasi, The scenario | Observability: the issues, what organizations want, and the scenario |
| 7:00 | 16–19 | Scene 1 · Status from telemetry and configuration | Grafana · Operations | Telemetry + configuration → queries → Prometheus reaction → Grafana |
| 9:30 | 20–22 | Scene 2 · The absence of change | Grafana · Operations | The absence of change: requests with no answer by their deadline (trueLater) |
| 12:00 | 23–27 | Scene 3 · Compliance meets resilience | Grafana · Operations | Summaries Europe-only puts failover at risk; compliance allows the US and it's safe |
| 15:00 | 28–32 | Scene 4 · Failure, proposal, approval | Grafana · Operations, Mattermost · #incident-ai-support | EU-A fails; Drasi's proposal is a card in Mattermost; approve; verified; restore |
| 19:30 | 33–38 | Plugins, form factors and close | Plugin ecosystem, Run Drasi your way, What you saw, Learn more, Session survey | Plugins; run it your way; what we showed; links; the session survey, then questions |
| 22:00 | | Questions | Session survey | Take questions |

## The script

The desk shows one beat at a time, numbered across the whole talk (the desk shows the same
numbers). Press **Next** (or your clicker, or →) to move on: the projector follows. Beats marked optional are skipped when **Short on time** is ticked.
Say it in your own words.

### 1. Intro (2 minutes)

1. **Walk-in.**\
   Before the session: the walk-in slide stays up while people arrive.\
   Opening the projector window starts the talk timer at 00:00 (Restart timer in the header resets it).\
   Press Next when you start talking.
   - **Press:** Start demo traffic
   - **Watch for:** Press Start demo traffic before you start talking, so the dashboard has several minutes of data by scene 1. Press it at least 5 minutes after any reset.
2. **Who I am.**\
   Good morning, my name is Allen Jones.\
   I'm a member of the Azure Incubations team at Microsoft.
3. **Drasi, the project.**\
   For the last few years I've been working on an open-source project named Drasi.\
   We started it in Azure Incubations and open-sourced it in October 2024.\
   Since January 2025 Drasi has been a CNCF Sandbox project, and it's in active development with a growing community of users and contributors.
4. **Grafana · Operations.**\
   Here's what I'm going to show you today.\
   This is a live AI service and its monitoring: a Grafana dashboard like the ones most teams already run.
   - **Watch for:** The live dashboard, charts moving; no highlight.
5. **Grafana · Operations.**\
   The top row is what most teams already have: OpenTelemetry, Prometheus and Grafana.\
   It tells you something is wrong.
   - **Watch for:** The top row outlined, labelled What most teams have.
6. **Grafana · Operations.**\
   The bottom row is Drasi: who's affected, what to do about it, and whether you'd survive the next failure.\
   Answers that update the moment things change, beside the monitoring you already use.\
   In a few minutes I'm going to break this service on purpose.\
   Watch which row notices first.
   - **Watch for:** The Decision Support row outlined, labelled Added by Drasi.
7. **Why Drasi exists.**\
   Drasi was started because, in our experience, detecting meaningful changes in data as they happen, and reacting to them, is harder than it should be.\
   There are two common approaches.\
   Polling: query everything every few seconds, diff it with last time, and write code to react.\
   It's simple, but slow to notice, wasteful, and it misses changes that happen in between.\
   Or stream processing: consume an event stream and react to every event.\
   That's powerful, and every event arrives, but working out what an event means for the current state, joining it with other sources, and noticing when something didn't happen, means building and running your own state: windows, joins, state stores, ordering and late events.
8. **Change-driven computation with Drasi.**\
   Drasi takes a change-driven approach.\
   You describe the result you want, as a query.\
   Changes flow in from the systems you already have; Drasi keeps the result current as the data changes, and as time passes, across sources; and a reaction acts on just what changed: what was added, updated or removed.\
   No polling loop, and no state store to build and run yourself.

**If it breaks:** The slides don't depend on the demo. Carry on.

### 2. What Drasi is (2½ minutes)

9. **Simple concepts · one query, several sources.**\
   Drasi is built around three simple, familiar concepts: Sources, Continuous Queries and Reactions.\
   Sources bring in every change, here from a database, telemetry and Kubernetes.\
   A continuous query describes what you care about across them, as one graph query, and Drasi does the joining and keeps its answer current as the data changes.\
   A reaction does something with each change to the answer.
10. **Simple concepts · many queries and reactions.**\
   And it grows from there: many sources, many queries, many reactions, each answer kept current as the data changes.\
   TODO: mention continuous models (policy, optimization) here or in scene 3?
11. **Simple concepts · queries build on queries.**\
   And queries can build on each other's results, forming a computation graph.\
   In this demo, the health checks feed a deployment-health query, which feeds the workload status and the plan; policy input feeds the policy model, which feeds the plan too; and the results fan out to several reactions.\
   When one input changes, only what depends on it updates.

**If it breaks:** Carry on; the slides don't depend on the demo.

### 3. Observability and the scenario (2½ minutes)

12. **Observability issues.**\
   Drasi is a general-purpose technology for building change-driven systems in any domain.\
   But few domains show the need and value better than observability, so today I'll show you some examples of how Drasi works in an observability use case.\
   Some common issues organizations report: too much data, too many tools, lots of noise, and dashboards that show symptoms, not answers, because the context lives somewhere else.
13. **What organizations want.**\
   What organizations want is more context, joined across systems; the signal raised above the noise; outcomes, including what didn't happen; action backed by evidence, with safeguards; all in the tools they already use.
14. **What organizations want · with Drasi.**\
   We believe Drasi can provide much of this: it keeps that connected context current as data changes in the systems you already have, and delivers the answers into the tools you already use.\
   Let me show you a few examples.
15. **The scenario.**\
   The scenario: an AI service with two workloads, customer support, which handles restricted data and must answer within 15 seconds, and document summaries, which use public data.\
   They run on deployments of the same AI model: EU-A and EU-B in Europe, and US-C in the US, extra capacity that's just been provisioned.\
   There's policy: restricted data stays in Europe, and for now summaries stay in Europe too.\
   Changes need a person's approval.\
   And the team already has its tools: OpenTelemetry, Prometheus and Grafana, Langfuse for traces, and Mattermost for incidents.\
   I won't walk through it as if I'm running the process; for each step I'll say what needs doing, show it, then show how it works.

**If it breaks:** Carry on; the slides don't depend on the demo.

### 4. Scene 1 · Status from telemetry and configuration (2½ minutes)

16. **Grafana · Operations.**\
   Let's start simple.\
   The team wants one view of each deployment's health and each workload's status, and that means joining telemetry with configuration data.\
   This is their Grafana dashboard: the top row is ordinary monitoring, the bottom row is Drasi's answers.
   - **Watch for:** Charts moving. If the desk says Traffic: stopped, press Start demo traffic in All demo controls.
17. **Grafana · Operations.**\
   How do these two tiles work?\
   Two sources: the health checks, which arrive as ordinary OpenTelemetry metrics every second, and the AI service's own database, where it keeps its deployments, workloads, routes and policy.\
   Two continuous queries join them, and a Prometheus reaction publishes each answer as a metric, so Grafana draws it.\
   No polling job, and no glue code.
   - **Watch for:** The how-it-works card above Deployment status and Workload status, with an arrow to each.
18. **Grafana · Operations.**\
   Here's the first query: the latest health check per deployment, from the OpenTelemetry source.\
   2 means failed.
   - **Watch for:** Deployment status: EU-A, EU-B, US-C Healthy; the deployment-health query above it.
19. **Grafana · Operations.**\
   And the second joins that health with configuration: where each workload runs, and whether policy allows it there.\
   So it says whether each workload is served somewhere healthy that it's allowed to run.
   - **Watch for:** Workload status: Support · EU-A and Summaries · EU-B Healthy; the tile-workloads query above it.

**If it breaks:** If the tiles are empty, traffic or Drasi isn't running: describe the how-it-works card and move on.

### 5. Scene 2 · The absence of change (2½ minutes)

20. **Grafana · Operations.**\
   Some of the most important situations are things that didn't happen.\
   A customer request that gets no answer within 15 seconds produces no event: no new row, no new metric.\
   Here the source is the requests in the database; the query lists every request not yet answered, and a time condition makes Drasi re-check each one at its deadline.\
   The answer goes to Grafana, and to the incident channel when support falls behind.
   - **Watch for:** The how-it-works card above Support overdue.
21. **Grafana · Operations.**\
   This is the query. drasi.trueLater becomes true at the request's deadline, so the request joins the list at that moment with no new event, and leaves it when it's answered.\
   The exact list of late requests, kept current request by request.\
   TODO: a live trigger here (for example a control that slows support answers), or say we'll see it climb in scene 4.
   - **Watch for:** Support overdue 0; the overdue-tasks query above it, the trueLater line highlighted.
22. **Grafana · Operations (optional).**\
   The same idea keeps Deployment status honest: each health reading is good for 8 seconds, so if the checks stop arriving, the deployment turns Unknown as time passes rather than staying green.\
   TODO: keep this beat?

**If it breaks:** Describe it from the card; scene 4 shows it live.

### 6. Scene 3 · Compliance meets resilience (3 minutes)

23. **Grafana · Operations.**\
   We've provisioned extra capacity in US-C, but compliance won't let document summaries be processed in the US.\
   Failover readiness asks: if any one deployment failed, would every workload still have somewhere allowed to run?\
   Right now, no, and Recommended action says add capacity.\
   Behind it: the policy and the deployments from the database, health from OpenTelemetry, and two of Drasi's continuous models, a Rego policy and an optimization model, re-run whenever any of them changes.
   - **Watch for:** Failover readiness: At risk: losing EU-A leaves no valid plan; Recommended action: Add capacity (both yellow); the how-it-works card above.
24. **Grafana · Operations.**\
   The policy, as Rego: a workload may only use the deployments it's allowed, and restricted data stays in its region.\
   The allowed destinations are data, rows in the database.
   - **Watch for:** The Rego rules above Recommended action.
25. **Grafana · Operations.**\
   And the what-if: the same optimization solved again without each healthy deployment in turn.\
   Lose EU-A, and support and summaries would both need EU-B, which only has 100k.
   - **Watch for:** The loss scenario above Failover readiness.
26. **Grafana · Operations (optional).**\
   The optimization model itself: one yes-or-no choice per workload and allowed deployment, every workload placed once, each healthy deployment within capacity, fewest moves first.
27. **Grafana · Operations.**\
   Now compliance agrees summaries can run in the US: they use public data.\
   That's one row in the policy data.\
   Drasi re-runs the policy and the what-if at once: summaries could fail over to US-C, so we'd survive losing any one deployment, and the capacity warning goes away.
   - **Press:** Summaries: Europe + US
   - **Watch for:** Failover readiness: Survives losing any one deployment (green); Recommended action: None. (Drasi also posts Failover restored in the channel.)

**If it breaks:** If Failover readiness doesn't turn green within about 10 seconds, say what it would show and move on.

### 7. Scene 4 · Failure, proposal, approval (4½ minutes)

28. **Grafana · Operations.**\
   Now EU-A, the deployment serving customer support, goes down.\
   Nobody tells Drasi: its health checks fail, and within seconds Deployment status says Failed and Recommended action names a fix: move summaries to US-C and support to EU-B, within policy.\
   Support overdue starts climbing too: that's scene 2's query, live.
   - **Press:** Fail EU-A
   - **Watch for:** Within about 8 seconds: EU-A Failed; Recommended action: Move summaries to US-C and support to EU-B (orange); Support overdue above 0.
29. **Mattermost · #incident-ai-support.**\
   The team doesn't live on a dashboard.\
   In their incident channel, Drasi posted as soon as EU-A failed, with the proposed change and the facts it was based on, before Grafana's alert arrived.\
   How: the same policy and optimization models, with a reaction that posts the card; and when someone approves it, another reaction applies the routes through the gateway's configuration API, and a query checks it worked.
   - **Click on the projector:** Scroll up to the orange Proposed change card so the whole card shows. 
   - **Watch for:** Red Deployment failed: EU-A, then the orange Proposed change card with Approve and Reject; the how-it-works card top right.
30. **Mattermost · #incident-ai-support.**\
   I approve it here, as the on-call engineer would.\
   Drasi checks the approval still matches its current plan, and applies the new routes.
   - **Click on the projector:** Approve on the newest orange Proposed change card.  Backup on the desk: Approve the proposed change (it presses the same button)
   - **Watch for:** Approved by @oncall; in the thread, blue Applied: Move summaries to US-C and support to EU-B.
31. **Grafana · Operations.**\
   Applied isn't fixed: another query waits until the backlog has cleared and support requests are answered on the new route, then says Verified.\
   Support overdue is back to 0.
   - **Watch for:** Recommended action: Verifying, then Verified (green); Support overdue 0.
32. **Grafana · Operations (optional).**\
   Later EU-A is back at full capacity: every deployment Healthy, and we'd survive losing any one again.\
   Nothing moves back; Drasi only proposes a move when the facts call for one.
   - **Press:** Restore EU-A
   - **Watch for:** All Healthy; Failover readiness: Survives losing any one deployment; no new channel card.

**If it breaks:** If nothing changes within 30 seconds, press Use recordings from here on. If Approve doesn't respond, press Approve the proposed change on the desk; never approve twice.

### 8. Plugins, form factors and close (2½ minutes)

33. **Plugin ecosystem.**\
   Everything you saw was configuration over plugins.\
   Sources, reactions and continuous models are all plugins, and there's a growing ecosystem of them: databases, streams and APIs, Kubernetes and live data feeds coming in; APIs, agents, messaging and databases going out.\
   And you can build your own: this demo adds one, the Prometheus reaction.\
   Continuous queries can be written in openCypher or GQL; this demo uses Cypher.
34. **Run Drasi your way.**\
   And you can run it your way.\
   Embed it as a library in Rust, Node.js, Python or .NET to build change-driven applications; run Drasi Server as a process or a container, configured in YAML, which is what this demo runs on; or deploy it on Kubernetes, where each part scales on its own.\
   TODO: confirm availability of each at Ignite.
35. **What you saw.**\
   So what did we see?\
   Status worked out from telemetry and configuration together.\
   Requests that got no answer, noticed at their deadline.\
   A compliance change that made the service more resilient.\
   And a fix, proposed within policy, approved in chat and verified.\
   Each one is richer context from data that already existed in different places, and none of it needed an integration project: connect sources, configure continuous models, connect reactions.
36. **Learn more.**\
   To learn more, start at drasi.io; the code, and this demo, are on GitHub.
37. **Session survey.**\
   Thank you.\
   Please scan the code and tell us how we did in the session survey.\
   Questions?
   - **Watch for:** The survey slide with its QR code stays up during questions.
38. **Session survey.**\
   After questions, stop the demo traffic.\
   Leave the survey slide up.
   - **Press:** Stop demo traffic

**If it breaks:** Summarize in words. Never project the desk.

## Glossary

| Term | Meaning |
|---|---|
| Deployment | One place the AI model runs: EU-A, EU-B (Europe) or US-C (United States) |
| Capacity | How much a deployment can handle, in tokens per minute (100k each) |
| Customer support / document summaries | The two workloads: support needs 60k, summaries 80k |
| Plan | Where each workload should run. Drasi's planner proposes one; you approve it, and Drasi applies it if it's still current |
| Policy | The rules: support only in Europe; summaries in Europe at the start, then anywhere once the US is allowed |
| Source | How Drasi hears about changes; here, the database's live list of changes and the health checks' OpenTelemetry metrics |
| Continuous query | A question whose answer Drasi keeps up to date as data changes and time passes |
| Continuous model | Logic that Drasi re-runs when its inputs change: here a Rego policy, and an optimizer for the planner and the what-if |
| Reaction | Delivers each change in an answer to another system: here Prometheus, the gateway's configuration API, Grafana, Langfuse, the incident channel and the presenter desk |
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
| Who changes the routes? | Drasi's reaction calls the gateway's configuration API, only for an approval that still matches Drasi's current plan. The AI service just applies the call, and refuses it if the routing configuration changed in between. |
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
| | Recommended action | Drasi's recommended change and where it is: **None**; **Approve: Move summaries to US-C and support to EU-B** (approve it on its card in #incident-ai-support); **Applying: …**, then **Verifying: …** until the backlog has cleared and support is answered on the new route; **Verified: …** (green); **No change fits policy and capacity**; **Add capacity: losing EU-A leaves no valid plan** (yellow; the starting state, until summaries are also allowed in the US) |
| | Failover readiness | If any one deployment failed, would every workload still have somewhere allowed to run? **Survives losing any one deployment** (green); **At risk: losing EU-A leaves no valid plan** (yellow; the starting state); **Already degraded: EU-A down** (grey) while a deployment has failed |

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
| Failover at risk: losing EU-A leaves no valid plan | orange | Everything is healthy, but losing one deployment would leave no valid plan. Posted after every reset, while summaries are Europe-only |
| Failover restored | green | Losing any one deployment again leaves a valid plan: after **Summaries: Europe + US** |

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
recording (failure, recovery, back to normal), and the slides
still appear. The buttons are hidden. Stay in recording mode for the rest of the
talk.

**You pressed the wrong button.** Back and Next never undo anything. Use the
opposite button in **All demo controls**: **Summaries: Europe only** undoes
**Summaries: Europe + US**; to undo **Fail EU-A**, press **Restore EU-A**. If
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
