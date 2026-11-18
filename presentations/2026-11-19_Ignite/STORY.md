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

**Then the bottom row.** That's Decision Support, the Drasi row, and it was
already ahead of the alert. Deployment status had EU-A as Failed. Nobody told
Drasi that: it worked it out from EU-A's health-check telemetry. Recommended
action already named a change: "Move summaries to US-C and
support to EU-B". And it was in #incident-ai-support before the alert had even
fired: Drasi had posted "Deployment failed: EU-A", and right under it the proposed
change, with an Approve button.

Support overdue was the one I watched. It counts requests that have blown their
15-second deadline, and it ticks up the moment each one crosses the line, even
though nothing happens to the request itself. On the waiting chart the red
overdue line just trails the orange queue line by 15 seconds. The second alert,
"Support requests overdue", is on that number, which is the one customers actually
feel.

**Who was hit.** Drasi posted "Support requests overdue" in the channel as soon as
the first one passed its deadline, with a link to the tile. I opened the overdue
list behind that tile. It's an actual list
of the late requests, not a count, with the oldest at the top. I picked the oldest
one: the customer's question, every retry to EU-A failing with a 503, and the
point its deadline passed. Same request is a trace in Langfuse if anyone needs the
detail.

**Where to move it.** First instinct was "fail support over to US-C, it's idle."
Can't. The proposal card spelled out the policy: support only on EU-A or EU-B,
because of the data rule. EU-B only had 20k spare, and support needs 60k. On paper
we had plenty, 200k healthy against 140k needed (the card lists both), but not
anywhere support is allowed to go.

The proposed change had already worked it out: move summaries to US-C, which frees
EU-B, then put support on EU-B. Two moves, both within policy, and the card says
which facts it was based on.

**The bit that caught me out.** While I was reading the proposal, compliance
tightened the rule so summaries had to stay in Europe as well. Drasi posted "Out of
date: the facts changed" straight away, with "No valid plan", which is right: with
EU-A down there's no way to fit both workloads in EU-B. I hadn't scrolled down, and
pressed Approve on the proposal I was reading. Drasi rejected it in the card's
thread, with the reason, because the facts it was based on had changed. Good thing
too, or I'd have pushed summaries to the US against the new rule. Compliance backed
it out after a few minutes, Drasi posted a new proposal straight away, and I
approved that one.

**The fix.** The approved change was applied through the gateway's config API.
Drasi replied in the card's thread, "Applied: Move summaries to US-C and support to
EU-B", and Change status said "Applied, verifying" for a bit. It won't call it
fixed just because the routes changed. It waits until the backlog from before the
change has drained and new support requests are being answered on the new route.
Then it went to Verified, and the same thread got "Verified: support has
recovered". Errors dropped, the queue drained, overdue went back to 0. EU-A was
still down at that point; we'd just routed around it.

**The one I'd have missed.** Later, EU-A came back, but only at 50k. Every health
signal was green, no errors, nothing alerting. But Failover readiness said No: not
if EU-B fails, and Drasi posted "Failover at risk: losing EU-B leaves support
nowhere to run". Support was now on EU-B, and if EU-B went, support would need 60k
in Europe and EU-A only had 50k. Nobody would have spotted that until EU-B fell
over. We got EU-A back to full capacity, it flipped back to Yes and Drasi posted
"Failover restored", no outage.

**Bottom line.** Grafana told me something was broken. Drasi told me, on the
dashboard and in the incident channel, who was waiting, where the work was allowed
to go, what would fit, whether my approval was still valid, whether the fix
actually worked, and that we were exposed afterwards. It all just updated as
things changed, without me running queries or re-checking anything.

---

## Where each part appears in the talk

| In the story | In the talk | Screen |
|---|---|---|
| The setup | The scenario | Slide |
| What I saw first, the bottom row | Failure | Operations dashboard, then #incident-ai-support |
| Who was hit | Who is affected | #incident-ai-support, the query slide, then the late request |
| Where to move it | Recovery | #incident-ai-support (the proposal card) |
| The bit that caught me out | Recovery, optional policy beats | #incident-ai-support |
| The fix | Recovery | #incident-ai-support (Approve on the card), Operations, then the card's thread |
| The one I'd have missed | Healthy but fragile | Operations, then #incident-ai-support |

Two things in the story are simulated for the demo: the three deployments
(in-process stubs that fail or slow down on command) and the change-approval tool
(it records the approval pressed on the card). Everything the engineer reads on the
dashboard and in the channel, and every decision about lateness, policy, the plan,
the approval and recovery, is computed by Drasi from real data changes.
