# Presenter guide

Everything you need to give this talk, without needing to know how the demo is
built. To install and start the demo, see [SETUP.md](SETUP.md). For what the demo
is about, see [README.md](README.md).

**The talk in one line:** slides on Drasi and observability, then about 12 minutes
of live demo in four scenes: status from telemetry and configuration, the absence
of change, compliance meeting resilience, and a failure for which Drasi proposes a
fix within the rules, you approve it, and Drasi confirms it worked.

- Session: THR1029, *Monitoring without polling*, 25 minutes
- About 22 minutes of content and 3 minutes of questions

## Contents

1. [How it works on stage](#how-it-works-on-stage)
2. [Set up your screens](#set-up-your-screens)
3. [Before you present](#before-you-present)
4. [The presenter desk](#the-presenter-desk)
5. [Run of show](#run-of-show) and [the script](#the-script)
6. [Glossary](#glossary) and [questions to expect](#questions-to-expect)
7. [Reading the dashboard](#reading-the-dashboard), [the incident channel](#the-incident-channel) and [one late request](#one-late-request)
8. [When something goes wrong](#when-something-goes-wrong)
9. [After the talk](#after-the-talk)

## How it works on stage

There are two windows:

- **The projector window** is what the audience sees: slides, the Grafana
  dashboard with the how-it-works cards and Drasi configuration over it, and the
  team's incident channel in Mattermost (#incident-ai-support). You click in it only on the incident channel, to
  scroll to a card or press **Approve** on Drasi's proposal; the desk says when.
- **The presenter desk** is on your laptop screen. It shows what to say, the one
  or two buttons for this moment, and what to watch for.

The talk is a sequence of **beats**. Press **Next** on the desk (or your clicker,
→, Space or Page Down) and the projector moves to whatever the next beat needs:
the next slide, a dashboard, or the incident channel. Your clicker works
whichever window is in front.

Some beats have buttons, such as **Provision US-C** or **Fail EU-A**. They
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
   then go back to the first beat (**Jump to a part** › 1. Intro). The projector
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
| Header | **READY** or **NOT READY**, whether the projector window is connected, **Ignite template assets missing** if they are, and the talk timer. It starts at 00:00 when you press **Open projector window** (**Restart timer** resets it) and turns amber at 22 minutes; tick **Short on time** if you're past about 15:30 when Scene 4 starts |
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

**Short on time:** tick it to skip the beats marked optional (scene 3's optimization
model, scene 4's Restore EU-A). Their notes disappear from the sequence.

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
| 9:30 | 20–21 | Scene 2 · The absence of change | Grafana · Operations | The absence of change: requests with no answer by their deadline (trueLater) |
| 12:00 | 22–26 | Scene 3 · Compliance meets resilience | Grafana · Operations | Summaries Europe-only puts failover at risk; compliance allows the US and it's safe |
| 15:00 | 27–31 | Scene 4 · Failure, proposal, approval | Grafana · Operations, Mattermost · #incident-ai-support | EU-A fails; Drasi's proposal is a card in Mattermost; approve; verified; restore |
| 19:30 | 32–38 | What you saw, form factors, extensibility and close | What you saw, Run Drasi your way, Extensibility, Thank you, Learn more, Session survey | What you saw; form factors; extensibility; thank you; learn more; the session survey, then questions |
| 22:00 | | Questions | Session survey | Take questions |

## The script

The desk shows one beat at a time, numbered across the whole talk (the desk shows the same
numbers). Press **Next** (or your clicker, or →) to move on: the projector follows. Beats marked optional are skipped when **Short on time** is ticked.
Say it in your own words.

### 1. Intro (2 minutes)

1. **Walk-in.**\
   Before the session: leave the walk-in slide up while people arrive.\
   Opening the projector window starts the talk timer at 00:00 (Restart timer resets it).\
   Press Next when you start talking.
   - **Press:** Start demo traffic
   - **Watch for:** Press Start demo traffic before you start talking, so the dashboard has several minutes of data by scene 1. Press it at least 5 minutes after any reset.
2. **Who I am.**\
   Good morning, my name is Allen Jones.\
   I'm a member of the Azure Incubations team at Microsoft.
3. **Drasi, the project.**\
   For the last few years I've been working on an open-source project called Drasi.\
   Drasi is a general-purpose platform for building change-driven systems.\
   We started Drasi in Azure Incubations and open-sourced it in October 2024.\
   In January 2025 it became a CNCF Sandbox project, and it's in active development with a growing community of users and contributors.
4. **Grafana · Operations.**\
   Although Drasi is domain agnostic, the nature of the observability domain makes it particularly applicable.\
   So today I am going to show you some examples of how Drasi can help with the observability challenges faced by organizations, and what I show you will be based around this Grafana dashboard, which will be familiar to many people.
   - **Watch for:** The live dashboard, charts moving; no highlight.
5. **Grafana · Operations.**\
   The top section shows the kind of telemetry most organizations already have and use: OpenTelemetry metrics, collected by Prometheus, and displayed here in Grafana.
   - **Watch for:** The top row outlined, labelled The basics.
6. **Grafana · Operations.**\
   The lower section shows higher-level, higher-value information that uses richer context to provide in situ answers and guidance to people, and this is powered by Drasi.\
   I will talk about each of these tiles in a moment, but first I will tell you a little more about Drasi.
   - **Watch for:** The Decision Support row outlined, labelled Richer context, answers, guidance · Powered by Drasi.
7. **Why Drasi exists.**\
   We started Drasi because detecting meaningful change as it happens, and reacting to it, is harder than it should be.\
   Teams usually poll: query everything every few seconds, diff with last time, and write code to react.\
   It's simple, but slow, wasteful, and it misses changes in between.\
   Or they use stream processing: every event arrives, but working out what it means for the current state, joining other sources, and noticing what didn't happen means building and running your own state: windows, joins, state stores, late events.\
   Each of these has its place and can be the correct approach under certain circumstances, but for detecting specific meaningful change as it happens and being able to respond, we wanted to make it easier.
8. **Change-driven computation with Drasi.**\
   Drasi observes low-level change feeds.\
   You describe the result you want as a query, and Drasi keeps that result current as the data changes, and as time passes, across sources.\
   A reaction then acts on just what changed in the result: what was added, updated or deleted.\
   No polling loop, and no state store to build and run yourself.

**If it breaks:** The slides don't depend on the demo. Carry on.

### 2. What Drasi is (2½ minutes)

9. **Simple concepts · one query, several sources.**\
   Drasi is built on three simple concepts: Sources, Continuous Queries and Reactions.\
   Sources bring in low-level changes, here from a database, telemetry and Kubernetes.\
   A continuous query describes what you care about across them in one graph query; Drasi does the joining and keeps the result current.\
   A reaction acts on changes to that result, not the sources.
10. **Simple concepts · many queries and reactions.**\
   It scales out from there: many sources, many queries, many reactions, each result kept current as the data changes.
11. **Simple concepts · queries build on queries.**\
   And, coming soon, queries and models can build on each other's results: a computation graph.\
   Here, health checks feed deployment-health, which feeds workload status and the plan; policy input feeds the Rego policy, which feeds the plan too; and the results fan out to several reactions.\
   When one input changes, only what depends on it updates.

**If it breaks:** Carry on; the slides don't depend on the demo.

### 3. Observability and the scenario (2½ minutes)

12. **Observability issues.**\
   Back to observability.\
   Most organizations understand why it matters, but they report the same issues: fragmented tools; noisy alerts, yet real failures noticed late; context that lives elsewhere, so you see symptoms, not answers; rising telemetry costs; green components while customers suffer; and AI services that look healthy while getting it wrong.\
   Despite all the investment, customers often report issues first, and on-call engineers are left connecting the dots.
13. **What organizations want.**\
   What organizations want is context, joined across systems, that answers: who's affected, what changed, what should we do, and did it work.\
   Situations, not signals; outcomes, including what didn't happen; evidence-backed action, with approval; all in the tools they already use.
14. **What organizations want · with Drasi.**\
   We believe Drasi has a role to play here: bringing together that context from disparate systems, keeping it current as things change, and making it available where it is needed most.
15. **The scenario.**\
   So imagine an organization with two important AI workloads.\
   Customer support needs 60k tokens a minute, on EU-A; document summaries need 80k, on EU-B.\
   Each deployment has 100k of capacity, and US-C, in the US, is being provisioned.\
   Support's data is restricted and must stay in Europe; summaries' classification is under review, so compliance keeps them in Europe too.\
   The SLO: customers get an answer within 15 seconds.\
   And changes to where workloads run need human approval.\
   The tools are OpenTelemetry, Prometheus and Grafana, Langfuse for traces, Mattermost for incidents and PostgreSQL.\
   A real environment would add ticketing, paging, a service catalog and CI/CD; Drasi connects to those the same way.\
   For each scene: what's needed, shown live, then how it works.

**If it breaks:** Carry on; the slides don't depend on the demo.

### 4. Scene 1 · Status from telemetry and configuration (2½ minutes)

16. **Grafana · Operations.**\
   Back to the dashboard, starting simple.\
   The charts are moving because there's simulated traffic running locally.\
   The team wants one view of each deployment's health and each workload's status, and that means joining telemetry with configuration data.
   - **Watch for:** Charts moving. If the desk says Traffic: stopped, press Start demo traffic in All demo controls.
17. **Grafana · Operations.**\
   How do these tiles work?\
   Health checks arrive as OpenTelemetry metrics every second; deployments, workloads, routes and policy come from the AI service's database.\
   Two continuous queries, deployment-health and tile-workloads, join them, and a Prometheus reaction publishes each result as a metric for Grafana.\
   No polling job, no glue code.
   - **Watch for:** The how-it-works card above one outline around Deployment status and Workload status.
18. **Grafana · Operations.**\
   Here's deployment-health: the latest health check per deployment, from the OpenTelemetry source.\
   A value of 2 means failed.\
   Now we've just provisioned extra capacity: US-C, in the US.\
   Watch it appear in Deployment status within a second, with no dashboard or query change: Drasi picks it up the moment the new deployment record appears, joined with its health checks.
   - **Press:** Provision US-C
   - **Watch for:** EU-A, EU-B Healthy; within a second of pressing, a US-C Healthy row appears. Workload status is unchanged (no workload runs on US-C). No channel card.
19. **Grafana · Operations.**\
   And tile-workloads joins that health with configuration: where each workload runs, and whether policy allows it there.\
   So it tells you whether each workload is served somewhere healthy and allowed.
   - **Watch for:** Workload status: Support · EU-A and Summaries · EU-B Healthy; the tile-workloads query above it.

**If it breaks:** If the tiles are empty, traffic or Drasi isn't running: describe the how-it-works card and move on. If US-C doesn't appear within a few seconds, carry on; scene 3 needs it, so check the desk shows Provision US-C ✓ before then.

### 5. Scene 2 · The absence of change (2½ minutes)

20. **Grafana · Operations.**\
   Some of the most important situations are things that didn't happen.\
   A request that breaches the 15-second SLO produces no event.\
   The overdue-tasks query lists every unanswered request, and a time condition makes Drasi re-check each one at its deadline.\
   The result goes to Grafana, and to the incident channel when support falls behind.
   - **Watch for:** The how-it-works card above Support overdue.
21. **Grafana · Operations.**\
   Here's overdue-tasks. drasi.trueLater becomes true at the request's deadline, so the request joins the result at that moment, with no new event, and leaves it when it's answered.\
   It's the exact list of late requests, not a sampled count.\
   It's zero now; you'll see it climb in scene 4.
   - **Watch for:** Support overdue 0; the overdue-tasks query above it, the trueLater line highlighted.

**If it breaks:** Describe it from the card; scene 4 shows it live.

### 6. Scene 3 · Compliance meets resilience (3 minutes)

22. **Grafana · Operations.**\
   US-C is provisioned, yet failover is still at risk: compliance won't let summaries run in the US.\
   Failover readiness asks: if any one deployment failed, would every workload still have somewhere allowed to run?\
   Right now, no, so Recommended action says add capacity.\
   Behind both: two continuous models, a Rego policy and an optimization plan, re-run whenever policy, deployments or health change.
   - **Watch for:** Failover readiness: At risk: losing EU-A leaves no valid plan; Recommended action: Add capacity (both yellow); the how-it-works card above.
23. **Grafana · Operations.**\
   The policy, in Rego: a workload may only use the deployments it's allowed, and restricted data stays in its region.\
   The allowed destinations are just rows in the database.
   - **Watch for:** The Rego rules above Recommended action.
24. **Grafana · Operations.**\
   And the what-if: the same optimization, solved again without each healthy deployment in turn.\
   Lose EU-A, and support and summaries would both need EU-B, which only has 100k.
   - **Watch for:** The loss scenario above Failover readiness.
25. **Grafana · Operations (optional).**\
   The optimization model itself: one yes-or-no choice per workload and allowed deployment, every workload placed once, each healthy deployment within capacity, fewest moves first.
26. **Grafana · Operations.**\
   Now the review clears, and compliance allows summaries in the US: one row in the policy data.\
   Drasi re-runs the policy and the what-if at once: summaries could fail over to US-C, so we'd survive losing any one deployment, and the capacity warning clears.
   - **Press:** Summaries: Europe + US
   - **Watch for:** Failover readiness: Survives losing any one deployment (green); Recommended action: None. (Drasi also posts Failover restored in the channel.)

**If it breaks:** If Summaries: Europe + US is disabled, press Provision US-C first (it's in All demo controls). If Failover readiness doesn't turn green within about 10 seconds, say what it would show and move on.

### 7. Scene 4 · Failure, proposal, approval (4½ minutes)

27. **Grafana · Operations.**\
   Now EU-A, which serves customer support, goes down.\
   Nobody tells Drasi: the health checks fail, Deployment status shows Failed within seconds, and Recommended action proposes a fix within policy: move summaries to US-C and support to EU-B.\
   Support overdue starts climbing: scene 2's query, live.
   - **Press:** Fail EU-A
   - **Watch for:** Within about 8 seconds: EU-A Failed; Recommended action: Move summaries to US-C and support to EU-B (orange); Failover readiness: Already degraded; Support overdue above 0 (needs demo traffic).
28. **Mattermost · #incident-ai-support.**\
   On-call engineers don't watch dashboards all day.\
   Drasi posted in the incident channel as soon as EU-A failed, with the proposed change and the facts behind it, before Grafana's alert.\
   A reaction posts the card; once it's approved, another applies the routes through the gateway's configuration API, and the recovery-verified query checks it worked.
   - **Click on the projector:** Scroll up to the orange Proposed change card so the whole card shows. 
   - **Watch for:** Red Deployment failed: EU-A, then the orange Proposed change card with Approve and Reject; the how-it-works card top right.
29. **Mattermost · #incident-ai-support.**\
   I approve it here, as the on-call engineer would: that's the change approval.\
   Drasi checks the approval still matches its current plan, then applies the new routes.
   - **Click on the projector:** Approve on the newest orange Proposed change card.  Backup on the desk: Approve the proposed change (it presses the same button)
   - **Watch for:** Approved by @oncall; in the thread, blue Applied: Move summaries to US-C and support to EU-B.
30. **Grafana · Operations.**\
   Applied isn't fixed.\
   Drasi waits until the backlog has cleared and support requests are being answered on the new route, then marks it Verified.\
   Support overdue is back to zero.
   - **Watch for:** Recommended action: Verifying, then Verified (green); Workload status: Support · EU-B, Summaries · US-C; Support overdue 0. In the channel thread: Verified: support has recovered. Verified needs demo traffic.
31. **Grafana · Operations (optional).**\
   Later, EU-A is restored: every deployment is healthy, and we'd survive losing any one again.\
   Nothing moves back; Drasi only proposes a change when the facts call for one.
   - **Press:** Restore EU-A
   - **Watch for:** All Healthy; Failover readiness: Survives losing any one deployment; no new channel card.

**If it breaks:** If nothing changes within 30 seconds, press Use recordings from here on. If Approve doesn't respond, press Approve the proposed change on the desk; never approve twice. If it stays Verifying, demo traffic has stopped: press Start demo traffic.

### 8. What you saw, form factors, extensibility and close (2½ minutes)

32. **What you saw.**\
   So I hope you can see that Drasi has a valuable role to play in observability.\
   We saw status built from telemetry and configuration; requests that missed their deadline, noticed at that moment; one policy change re-checking resilience at once; and a fix proposed within policy, approved in chat and verified.\
   All by connecting sources, configuring continuous queries and models, and connecting reactions.\
   But remember: Drasi is domain-agnostic, a platform for change-driven solutions in any domain.
33. **Run Drasi your way.**\
   With that in mind, Drasi comes in several form factors: embed drasi-lib to build change-driven applications; run Drasi Server as a process or a container, configured in YAML, as this demo does; or run Drasi for Kubernetes, where each part scales on its own.\
   TODO: confirm the Node.js, Python and .NET bindings for Ignite; drasi.io lists drasi-lib as a Rust crate.
34. **Extensibility.**\
   Drasi is also highly extensible.\
   Sources, reactions and continuous models are all plugins, with a growing ecosystem coming in and going out, and you can build your own: this demo adds a Prometheus reaction.\
   Queries can be written in openCypher or GQL.
35. **Thank you.**\
   And with that, thank you for your time.
36. **Learn more.**\
   Here is a list of ways you can learn more.
37. **Session survey.**\
   Please visit the session survey and give us your feedback, so we can improve future sessions.\
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
| Source | A connection to a system Drasi observes for changes; here, PostgreSQL's change feed and the health checks' OpenTelemetry metrics |
| Continuous query | A query whose result set Drasi keeps current as data changes and as time passes; written in openCypher or GQL |
| Continuous model | Logic that Drasi re-runs when its inputs change: here a Rego policy, and an optimizer for the planner and the what-if |
| Reaction | Receives a query's result changes (added, updated, deleted) and acts on them: here Prometheus, the gateway's configuration API, Grafana, Langfuse, the incident channel and the presenter desk |
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

## One late request

No beat shows it, but if someone asks who is waiting, open **Show something on
the projector** and press **Oldest late request** (there while support has late
requests).
The projector shows the oldest late support request from Drasi's list: the
customer's question, each attempt to send it (for example "Sent to EU-A: failed
(HTTP 503) × 12"), and the moment its 15-second deadline passed and Drasi added it
to the overdue list. Press **Next** or go back to the beat to continue.

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
a recording. Keep pressing **Next** as usual: scene 4 shows its recordings (failure,
recovery, back to normal); scenes 1–3 have none, so the projector keeps the live
dashboard and its overlays, which you can describe; the slides still appear. The buttons are hidden. Stay in recording mode for the rest of the
talk.

**You pressed the wrong button.** Back and Next never undo anything. Use the
opposite button in **All demo controls**: **Summaries: Europe only** undoes
**Summaries: Europe + US**; to undo **Fail EU-A**, press **Restore EU-A**. **Provision US-C** has no undo during
the talk (a reset removes US-C). If
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
