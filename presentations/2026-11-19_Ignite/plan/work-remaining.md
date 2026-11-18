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
- **4. Healthy Service**
  - The section headings should be more generic and typical of a dashboard like "AI Deployments" and "Decision Support" or something like that
  - THere used to be little (i) icons that gave extra info, they have gone and should return. The current 3 dots in top right disappear as you move the mouse across them making them not useful.
- **5. Failure**
  - When i first hit Fail EU-A, in the 'Drasi' section of the dashboard multiple things are poorly expressed and are meanignless:
    - 'Recovery' says Nothing to verify, which makes no sense, it is meaningless.
    - Action needed says 'Approve proposed plan'. it is a call to action with no way to take action. Wouldnt the dashboard say the proposal or give a way to get to the proposal.
    - 'Safe if one fails?' is a very stupid and meaningless title and when it says 'already one down' it makes even less sense.
- **7. Recovery**
  - what does this screen even represent? it is a poorly organized hodge podge of made up panels. Is there a way to present this in a way that a real infra support person might have access to? Its really a dashboard for the demo process, not a real operational demo
  - in the presenter screen 2 of section 7, there is a button to "Select proposed plan" at this point i am just confused. What plan is it talking about? And when i click it, the presenter button shows a green tick, but i have no idea what just happened and i dont see anything on the presentation screen.


