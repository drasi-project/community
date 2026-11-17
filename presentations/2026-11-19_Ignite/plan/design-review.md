# Design review of the talk and demo

Three independent reviewers studied every screen in every demo state (37 captures)
and the presenter documents:

- a first-time, non-technical presenter (133 findings, 28 high)
- a visual designer (159 findings, 41 high)
- an observability engineer in the audience (128 findings, 13 high)

The detailed findings are in the session's `review1/` folder. This document is
the summary and the decisions taken.

## Summary

1. **Two control surfaces.** The presenter clicks on the projector (Grafana links,
   Back, Langfuse rows) and on the laptop desk. The clicker goes to whichever
   window has focus. This is the biggest risk on stage.
2. **The desk isn't built for a live talk.** It's twice a laptop screen tall, the
   current action is buried under a long script, the controls are far from the
   instructions, and expected outcomes look like errors.
3. **Projected screens are built for a laptop, not a hall.** Small text, wasted
   space, tool chrome (Grafana toolbar, Langfuse sidebar and promo), a white
   Langfuse screen after a dark deck, and colours that mean different things on
   different tiles.
4. **The story contradicts itself at key moments.**
   - Errors stay red next to "Verified".
   - The stale-approval demo approves a plan that changes nothing.
   - Support moves back to EU-A without approval.
   - "Safe if EU-A fails?" says "EU-A is down now".
   - "No valid plan" shows as "Unknown".
5. **It's unclear what is Drasi and what is demo code.** The slides call app work
   "reactions" and never say which parts are built in.
6. **Terminology drifts:** summaries, batch and background job; plan, routes and
   allocation; capacity and budget; continuous models and continuous monitors.
7. **"Who is affected" is answered with a number.** Drasi's list of late requests
   is never shown where it matters.

## Decisions

### One control surface

The projected window becomes a stage that shows slides, Grafana and Langfuse
inside it, scaled for the room. The presenter desk drives it: **Next** (keyboard,
clicker or button on the desk) moves to the next beat and changes the projector.
The presenter never clicks on the projector, and focus problems disappear.

### Presenter desk

- A no-scroll "now" view: the current beat in large type, the one or two buttons
  for that beat beside it, what to watch for, and what to do if it breaks.
- Results appear next to the button that was pressed.
- Expected outcomes are shown as expected, not as errors.
- Everything else (all controls, recordings, reset, Q&A crib) is in collapsible
  sections.

### Script

Each cue is split into beats: say a little, click, watch, say. The language is
plain, with a short glossary. Each "What Drasi did" visual has its own line.

### Story changes

- **Order:** title → scenario → Drasi in one picture → healthy → failure → who is
  affected (the query slide, with Drasi's live list of late requests linking to
  their traces in Langfuse) → recovery → healthy but fragile → close.
- **Policy is folded into recovery as optional beats.** Select the plan, compliance
  restricts summaries (no valid plan), approve (rejected because the facts
  changed), restore the rule, select and approve again. The stale approval is now
  a real plan.
- **Healthy but fragile no longer moves work without approval.** After recovery,
  EU-A comes back with only 50k. The what-if becomes "Safe if any one deployment
  fails?" and answers "No: not if EU-B fails". Restoring EU-A to 100k answers Yes.
- **Verified** needs the backlog cleared and at least three support requests
  answered on the new route.
- **Errors** are measured over 30 seconds, so they clear by the time recovery is
  verified.
- **Drasi versus demo code:** a slide states plainly what is built into Drasi
  (sources, continuous queries, running your models, delivering results) and
  what was written for the demo (the policy, planner and what-if models, and the
  AI service that applies plans).
- **"Without polling":** the Drasi slide states what it means: no scheduled scans
  or re-computation; Prometheus and Grafana still refresh to display.

### Projected screens

- Grafana is shown in kiosk mode, filling the screen, with larger tiles.
- **Colours:** red means broken; amber means action or waiting; green means OK;
  grey means not allowed or not applicable.
- Changed values are highlighted. The proposed plan shows "Same as current" or
  "No valid plan".
- A large tile shows the latest approval.
- Langfuse is shown in dark mode, zoomed, with plain trace and step names.
- **One glossary everywhere:**
  - deployment
  - customer support
  - document summaries
  - request
  - plan
  - capacity
  - sources → continuous queries → continuous models → reactions

## Process

After the fixes, the captures are retaken and fresh reviewers repeat the review.
This continues until a round finds nothing significant.

## Rounds

Each round reviewed a full run of the talk, captured beat by beat through the
presenter desk (`demo/ui/scripts/walkthrough.mjs`). Fresh reviewers each time.

| Round | Presenter (high / all) | Visual (high / all) | Audience (high / all) |
|---|---|---|---|
| 1: the original | 28 / 133 | 41 / 159 | 13 / 128 |
| 2: after the redesign | 6 / 81 | 12 / 109 | 7 / 98 |
| 3 | 5 / 61 | 6 / 103 | 5 / 86 |

Round 2 and 3 high findings were all fixed:

- one name for each tile
- stable desk controls
- Next warns about a missed button
- a single recording mode
- a rejected approval clears the selection
- the timer starts by itself
- wrong-press recovery
- honest wording on polling and on what is Drasi versus demo code
- the policy model's query input shown
- dashboard tiles that didn't fit, weren't red when broken, or didn't show a failed deployment

What remains is medium and low polish (wording, spacing, small layout details),
listed in the round 3 findings files.
