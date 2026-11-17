# Work remaining

Open items for the rework (`agentofreality-ignite-native`), most important first.
Remove an item when it is done.

## Before the go/no-go decision (Nov 18)

- **Phase 8a verification.** Integration tests are ported and passing; still to
  do: lifecycle tests, the full walkthrough, new fallback recordings, release
  cycles and a go/no-go assessment.
- **Timings in the script are unverified.** "Within about 8 seconds", "plan before
  the alert", "Verified within about 30 seconds" and "about 20 seconds to start
  after a reset" must be measured on the rework and the script corrected.

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
- **Grafana annotations** can be duplicated if a response is lost or Server
  restarts within 30 seconds.

## After the talk

- **Push and merge.** The rework branches in community, drasi-core and
  drasi-server are local only. Push them, and open pull requests for the
  drasi-core and drasi-server fixes.
- **Drasi Server UI issues:** component details requested by ID; 14 px text in
  the expanded query view.
- **Podman memory.** The Podman VM has 7.45 GiB; running both stacks at once ran
  out of memory. Give it more memory (the Mac has 32 GB) before running both again.
