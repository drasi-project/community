# The incident, told by the on-call engineer

The demo's story as the person on call would tell it to a colleague the next
morning. Everything they see and do here is something the live demo shows.

---

So you missed the fun yesterday. Quick rundown.

**The setup, if you haven't looked at it in a while.** We run two things on the
model platform: the customer support assistant and the document summaries job.
Three deployments, 100k tokens a minute each. Support runs on EU-A and needs about
60k. Summaries run on EU-B and need about 80k. US-C is idle headroom. The catch is
that support handles restricted customer data, so it's EU only. Summaries can run
anywhere. And we promise support an answer in 15 seconds.

**What I saw first.** I had the Operations dashboard up. Support errors went from
0 to 100% almost straight away, answer time went to "No answers", and the support
queue started climbing. About 20 seconds later the "Model calls failing" alert
fired. Normal stuff, tells you something's broken.

**Then the bottom row.** That's the Drasi row, and it was already ahead of the
alert. Deployment status had EU-A as Failed, from its health checks. Action needed
said "Approve proposed plan", so there was already a fix waiting, before the alert
had even fired.

Support overdue was the one I watched. It counts requests that have blown their
15-second deadline, and it ticks up the moment each one crosses the line, even
though nothing happens to the request itself. On the waiting chart the red
overdue line just trails the orange queue line by 15 seconds. The second alert,
"Support requests overdue", is on that number, which is the one customers actually
feel.

**Who was hit.** I opened the overdue list behind that tile. It's an actual list
of the late requests, not a count, with the oldest at the top. I picked the oldest
one: the customer's question, every retry to EU-A failing with a 503, and the
point its deadline passed. Same request is a trace in Langfuse if anyone needs the
detail.

**Where to move it.** First instinct was "fail support over to US-C, it's idle."
Can't. The Recovery plan dashboard had support marked Not allowed on US-C, because
of the data rule. EU-B only had 20k spare, and support needs 60k. On paper we had
plenty, 200k healthy against 140k needed, but not anywhere support is allowed to
go.

The proposed plan had already worked it out: move summaries to US-C, which frees
EU-B, then put support on EU-B. Two moves, both within policy. I raised the change
request with that plan.

**The bit that caught me out.** While the change was waiting, compliance tightened
the rule so summaries had to stay in Europe as well. The plan went to "No valid
plan" straight away, which is right: with EU-A down there's no way to fit both
workloads in EU-B. When I approved the change I'd already raised, it got rejected,
with the reason, because the facts it was based on had changed. Good thing too, or
I'd have pushed summaries to the US against the new rule. Compliance backed it out
after a few minutes, the plan came straight back, and I approved the current one.

**The fix.** The approved plan was applied through the gateway's config API.
Running now showed support on EU-B and summaries on US-C, and Last approval said
Applied. Recovery sat on "Awaiting evidence" for a bit. It won't call it fixed
just because the routes changed. It waits until the backlog from before the change
has drained and new support requests are being answered on the new route. Then it
went to Verified. Errors dropped, the queue drained, overdue went back to 0.
EU-A was still down at that point; we'd just routed around it.

**The one I'd have missed.** Later, EU-A came back, but only at 50k. Every health
signal was green, no errors, nothing alerting. But "Safe if one fails?" said No:
not if EU-B fails. Support was now on EU-B, and if EU-B went, support would need
60k in Europe and EU-A only had 50k. Action needed said At risk. Nobody would have
spotted that until EU-B fell over. We got EU-A back to full capacity and it flipped
back to Yes, no outage.

**Bottom line.** Grafana told me something was broken. The Drasi row told me who
was waiting, where the work was allowed to go, what would fit, whether my approval
was still valid, whether the fix actually worked, and that we were exposed
afterwards. It all just updated as things changed, without me running queries or
re-checking anything.

---

## Where each part appears in the talk

| In the story | In the talk | Screen |
|---|---|---|
| The setup | The scenario | Slide |
| What I saw first, the bottom row | Failure | Operations dashboard |
| Who was hit | Who is affected | Query slide, then the late request |
| Where to move it, raising the change | Recovery | Recovery plan dashboard |
| The bit that caught me out | Recovery, optional policy beats | Recovery plan dashboard |
| The fix | Recovery | Recovery plan, then Operations |
| The one I'd have missed | Healthy but fragile | Recovery plan, then Operations |

Two things in the story are simulated for the demo: the three deployments
(in-process stubs that fail or slow down on command) and the change-approval tool
(the presenter desk). Everything the engineer reads on the dashboards, and every
decision about lateness, policy, the plan, the approval and recovery, is computed
by Drasi from real data changes.
