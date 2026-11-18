# Work remaining

Open items for the rework (`agentofreality-ignite-native`), most important first.
Remove an item when it is done.

## Before the talk

- **Human rehearsal on Nov 18** on the presentation laptop, with only this stack
  running (Podman's 7.45 GiB VM can't hold both stacks). Phase 8a recommends GO
  for this version after that rehearsal.
- **Reset at least 5 minutes before the talk:** the Model calls chart shows the
  previous run for 5 minutes after a reset.
- **Not yet run:** `ops/release.py soak` (1 hour) and `ops/release.py stamp`.

## Speaker material

- **"Drasi in one picture", second beat.** About 110 spoken words (about 45
  seconds). Keep for now; the presenter will review and decide whether to trim.

## Known compromises

- **`recovery-results`** still uses the query-to-query workaround: a query can't
  yet bootstrap from its upstream queries' snapshots.
- **The capacity tile's query** reads the PostgreSQL tables directly rather than
  another query's result.
- **Drasi Server lists no reactions** (`/instances/ignite/reactions` returns an
  empty list), so its UI shows none, although the graph has five.
- **Grafana annotations and Langfuse scores** are best effort: their reactions
  skip after 3 retries, so an outage loses those events, and annotations can be
  duplicated on a retry or restart.
- **"Grafana alerts firing" tile** recomputes the two alert rules in PromQL, so it
  flips within seconds; Grafana's real alert fires about 20 seconds after the
  failure. The script doesn't point at the tile for the "plan before the alert"
  comparison.
- **Restarting a single query** isn't supported (queries fed by other queries get
  only their changes); restart the whole of Drasi Server.
- **The HTTP sink option `unroutedChanges`** exists only on this drasi-core branch.

## After the talk

- **Push and merge.** The rework branches in community, drasi-core and
  drasi-server are local only. Push them, and open pull requests for the
  drasi-core and drasi-server fixes.
- **Drasi Server UI issues:** component details requested by ID; 14 px text in
  the expanded query view.
- **Podman memory.** The Podman VM has 7.45 GiB; running both stacks at once ran
  out of memory. Give it more memory (the Mac has 32 GB) before running both again.

## Things to Change in the Presentation

- **1. Title Slide**
  - change "Answers that update the moment things change, beside the monitoring you already use" to "???"
- **4, 5 and 7: done, waiting for your review.** One dashboard with "AI
  Deployments" and "Decision Support" rows, (i) descriptions and menus that stay
  visible; Change status, Recommended action and Failover readiness replace
  Recovery, Action needed and Safe if one fails?; the Recovery plan dashboard and
  the desk's select and approve buttons are replaced by Drasi's cards in a real
  Mattermost incident channel (`#incident-ai-support`), approved on the projected
  card. See `plan/incident-channel-plan.md`.
  




