"""Incident cards in Mattermost (plan/contracts.md section 15), on the running demo, in the talk's order.

Drasi's `incident-channel` reaction posts each card once. The seed provisions only EU-A and EU-B,
with summaries Europe-only, so the run opens with failover at risk. Provisioning US-C changes no
card (summaries still can't run there); allowing summaries in the US restores it. Then the failure,
the overdue requests, the proposal with its Approve and Reject buttons, the outcome of the
approval in that card's thread (applied, verified); restoring EU-A then posts nothing (degraded to
safe is not news), and the routes stay where the plan put them. The approval goes through Mattermost exactly as a press of the card's
Approve button (`POST /mattermost/approve-latest`, the desk's backup). Resets the scenario at the
start and the end, which also clears the channel.
"""
import json
import time

import drasi_outputs as d


def field(card, title):
    return next(f["value"] for f in card["attachment"]["fields"] if f["title"] == title)


def main():
    checks = []
    d.reset(us=False)
    epoch = d.health()["run_epoch"]
    cards = lambda: [c for c in d.incident_cards() if c["run_epoch"] == epoch]  # noqa: E731
    def until(predicate, what, timeout=45):
        return d.wait(lambda: predicate(cards()) and cards(), timeout, what)
    def card(kind, n=1):
        return until(lambda cs: len([c for c in cs if c["card"] == kind]) >= n, f"{n} '{kind}' card(s)")
    try:
        seed = card("at-risk")
        assert [c["card"] for c in d.incident_cards()] == ["at-risk"], "the reset clears the channel; the seed posts at risk"
        assert seed[0]["attachment"]["title"] == "Failover at risk: losing EU-A leaves no valid plan", seed[0]
        assert d.metric("drasi_deployment_health_status", deployment="US-C") is None, "US-C not provisioned yet"
        d.provision()
        d.metric_until("drasi_deployment_health_status", lambda v: v == 0, 20, deployment="US-C")
        d.metric_until("drasi_failover_readiness", lambda v: v == 0, 20, lost="EU-A")
        time.sleep(5)
        assert [c["card"] for c in cards()] == ["at-risk"], "provisioning US-C alone changes no card"
        d.configure(destinations=d.US)
        restored = card("restored")[-1]
        assert restored["attachment"]["title"] == "Failover restored", restored
        checks.append("seed (EU-A and EU-B, summaries Europe-only): failover at risk, losing EU-A leaves no valid "
                      "plan; Provision US-C: US-C Healthy, still at risk, no card; Summaries: Europe + US: "
                      "failover restored")

        d.api("/traffic", {"run_epoch": epoch, "running": True})
        time.sleep(12)
        assert [c["card"] for c in cards()] == ["at-risk", "restored"], "a healthy service posts nothing"

        d.control("fail", "EU-A")
        failed = [c for c in card("failed") if c["card"] == "failed"][0]
        # A repeated delivery of the same card (a retried request, or the same row delivered
        # twice) must not post it again: Drasi's post carries pending_post_id = run/card id.
        post = d.mattermost(f"/posts/{failed['id']}")
        assert post.get("pending_post_id") in (None, "", f"{epoch}/{failed['incident_card_id']}"), post.get("pending_post_id")
        replay = d.mattermost("/posts", token="IGNITE_MATTERMOST_BOT_TOKEN", body={
            "channel_id": post["channel_id"], "root_id": "", "message": "", "props": post["props"],
            "pending_post_id": f"{epoch}/{failed['incident_card_id']}"})
        assert replay["id"] == failed["id"], f"a replayed failure card is the same post: {replay['id']} != {failed['id']}"
        assert [c["card"] for c in cards()].count("failed") == 1, "the replay posted nothing new"
        posted = [c for c in card("overdue") if c["card"] not in ("at-risk", "restored")]
        kinds = [c["card"] for c in posted]
        assert kinds[:3] == ["failed", "proposal", "overdue"], kinds
        failed, proposal, overdue = posted[:3]
        assert failed["attachment"]["title"] == "Deployment failed: EU-A", failed
        assert failed["attachment"]["text"] == ("EU-A has reported Failed for 3 seconds. "
                                                "Support runs there, so its requests are failing."), failed
        assert (field(failed, "Region"), field(failed, "Capacity"), field(failed, "Runs here")) == \
            ("EU", "100k tokens/min", "support"), failed
        assert proposal["attachment"]["title"] == "Proposed change: Move summaries to US-C and support to EU-B"
        assert field(proposal, "Why") == "EU-A is down"
        assert field(proposal, "Policy") == ("support: EU-A, EU-B (restricted data, EU only) · "
                                             "summaries: EU-A, EU-B, US-C")
        assert field(proposal, "Capacity") == "healthy: EU-B 100k, US-C 100k · needed: support 60k, summaries 80k"
        assert [a["id"] for a in proposal["attachment"]["actions"]] == ["approve", "decline"]
        assert overdue["attachment"]["title"] == "Support requests overdue" and field(overdue, "Deadline") == "15 s"
        checks.append("EU-A fails: the failure card (after 3 s), then the proposal with Approve and Reject "
                      "(policy and capacity from Drasi's queries), then support overdue; the failure card once, "
                      "and a replayed delivery of it is the same post")

        approved = d.api("/mattermost/approve-latest", {"run_epoch": epoch})
        assert approved["post_id"] == proposal["id"], approved
        verified = until(lambda cs: any(c["card"] == "outcome" and c["root_id"] == proposal["id"]
                                        and c["incident_card_id"].endswith("-verified") for c in cs),
                         "applied and verified in the thread", 60)
        replies = [c for c in verified if c["root_id"] == proposal["id"]]
        assert [c["attachment"]["title"] for c in replies] == [
            "Applied: Move summaries to US-C and support to EU-B", "Verified: support has recovered"], replies
        assert field(replies[0], "Routes") == "support → EU-B · summaries → US-C", replies
        assert d.routes() == {"support": "EU-B", "batch": "US-C"}
        checks.append("Approve applies the change: Applied, then Verified, in the proposal's thread")

        before = len(cards())
        d.control("restore", "EU-A")
        d.metric_until("drasi_failover_readiness", lambda v: v == 1, 30)
        assert all(v == 0 for _, v in d.series("drasi_deployment_health_status").items()), "every deployment Healthy"
        assert d.routes() == {"support": "EU-B", "batch": "US-C"}, "nothing moves back"
        time.sleep(8)
        final = cards()
        assert len(final) == before, f"restoring EU-A posts nothing: {[c['card'] for c in final[before:]]}"
        checks.append("Restore EU-A: every deployment Healthy, failover survives losing any one deployment, "
                      "the routes stay support → EU-B and summaries → US-C, and no card")
        ids = [c["incident_card_id"] for c in final]
        assert len(ids) == len(set(ids)), f"each card is posted once: {ids}"
        kinds = [c["card"] for c in final]
        assert (kinds.count("at-risk"), kinds.count("restored")) == (1, 1), kinds
        assert [c["card"] for c in final if c["root_id"]] == ["outcome"] * 2
        checks.append("every card posted once; only outcomes are thread replies")
        path = d.save("incident-channel.json", {"checks": checks, "cards": final})
        print(json.dumps({"passed": checks, "evidence": str(path)}, indent=2))
    finally:
        d.reset()


if __name__ == "__main__":
    main()
