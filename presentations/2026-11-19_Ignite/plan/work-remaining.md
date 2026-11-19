# Work remaining

Open items for the demo (community `agentofreality-ignite-new-flow`; drasi-core and
drasi-server `agentofreality-ignite-native-next`), most important first.
Remove an item when it is done.

## Before the talk

- **Human rehearsal of the new flow** on the presentation laptop, with only this
  stack running (Podman's 7.45 GiB VM can't hold both stacks).
- **Open script TODOs** (`presentation/notes.ts`, `scenes.ts`; they show on the desk
  and in PRESENTER.md):
  - confirm the speaker's role on the title slide;
  - where to introduce continuous models (policy, optimization): on the "many
    queries and reactions" slide or in scene 3;
  - scene 2: a live trigger for Support overdue (for example a control that slows
    support answers), or say it climbs in scene 4; and whether to keep the
    optional Unknown-after-8-seconds beat;
  - Run Drasi your way: confirm each form factor's availability at Ignite.
- **Reset at least 5 minutes before the talk:** the Model calls chart shows the
  previous run for 5 minutes after a reset.
- **Re-record the fallback recordings** (`node ui/scripts/walkthrough.mjs record`)
  once the flow is final: the current ones show the earlier flow, not scenes 1–4.
- **Not yet run:** `ops/release.py soak` (1 hour) and `ops/release.py stamp`.

## Known compromises

- **`recovery-results`** still uses the query-to-query workaround: a query can't
  yet bootstrap from its upstream queries' snapshots.
- **The capacity tile's query** reads the PostgreSQL tables directly rather than
  another query's result.
- **Drasi Server lists no reactions** (`/instances/ai-ops/reactions` returns an
  empty list), so its UI shows none, although the graph has six.
- **Grafana annotations and Langfuse scores** are best effort: their reactions
  skip after 3 retries, so an outage loses those events, and annotations can be
  duplicated on a retry or restart.
- **Restarting a single query** isn't supported (queries fed by other queries get
  only their changes); restart the whole of Drasi Server.
- **The HTTP sink option `unroutedChanges`** exists only on `drasi-core-next`
  (`0e0e3ed4`), not yet upstream.

## After the talk

- **Merge.** Community's `agentofreality-ignite-new-flow` and the
  `agentofreality-ignite-native-next` branches of drasi-core and drasi-server are
  pushed. Open pull requests for drasi-core's
  commits beyond `agentofreality-parallel-computation-graph-recovery` (the HTTP
  sink's `unroutedChanges`, and the policy and optimization changes) and for the
  community branch.
- **Drasi Server UI issues:** component details requested by ID; 14 px text in
  the expanded query view.
- **Podman memory.** The Podman VM has 7.45 GiB; running both stacks at once ran
  out of memory. Give it more memory (the Mac has 32 GB) before running both again.

## Things to Change in the Presentation

- **1. Title Slide**
  - change "Answers that update the moment things change, beside the monitoring you already use" to "???"
- **4, 5 and 7: done, waiting for your review.** One dashboard with "AI
  Deployments" and "Decision Support" rows, (i) descriptions and menus that stay
  visible; Workload status, Recommended action (with the change's progress),
  Failover readiness and Grafana's alert list replace Recovery, Action needed,
  Safe if one fails? and the alert count; the Recovery plan dashboard and
  the desk's select and approve buttons are replaced by Drasi's cards in a real
  Mattermost incident channel (`#incident-ai-support`), approved on the projected
  card. See `plan/incident-channel-plan.md`.
