"""Incident cards in Mattermost (plan/contracts.md section 15), on the running demo.

Drasi's `incident-channel` reaction posts each card once: the failure, the overdue requests, the
proposal with its Approve and Reject buttons, Out of date when the facts change, the outcome of an
approval in the approved card's thread (applied, verified, or rejected by Drasi), failover at risk
and failover restored. The approval goes through Mattermost exactly as a press of the card's
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
    d.reset()
    epoch = d.health()["run_epoch"]
    cards = lambda: [c for c in d.incident_cards() if c["run_epoch"] == epoch]  # noqa: E731
    def until(predicate, what, timeout=45):
        return d.wait(lambda: predicate(cards()) and cards(), timeout, what)
    def card(kind, n=1):
        return until(lambda cs: len([c for c in cs if c["card"] == kind]) >= n, f"{n} '{kind}' card(s)")
    try:
        assert d.incident_cards() == [], "the reset clears the channel"
        d.api("/traffic", {"run_epoch": epoch, "running": True})
        time.sleep(12)
        assert cards() == [], "a healthy service posts nothing"

        d.control("fail", "EU-A")
        posted = card("overdue")
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
                      "(policy and capacity from Drasi's queries), then support overdue")

        d.configure(destinations={"batch": ["EU-A", "EU-B"]})
        superseded = card("superseded")[-1]
        assert field(superseded, "Now") == "No valid plan", superseded
        stale = d.api("/mattermost/approve-latest", {"run_epoch": epoch})
        assert stale["post_id"] == proposal["id"], stale
        rejected = until(lambda cs: any(c["card"] == "outcome" for c in cs), "the rejection")[-1]
        assert rejected["root_id"] == proposal["id"], rejected
        assert rejected["attachment"]["title"] == "Rejected by Drasi: nothing was changed", rejected
        assert field(rejected, "Approved by") == "@oncall", rejected
        checks.append("policy edit: Out of date (no valid plan); approving the stale card is rejected by "
                      "Drasi, in that card's thread")

        d.configure(destinations={"batch": ["EU-A", "EU-B", "US-C"]})
        second = card("proposal", 2)
        second = [c for c in second if c["card"] == "proposal"][-1]
        approved = d.api("/mattermost/approve-latest", {"run_epoch": epoch})
        assert approved["post_id"] == second["id"], approved
        verified = until(lambda cs: any(c["card"] == "outcome" and c["root_id"] == second["id"]
                                        and c["ignite_id"].endswith("-verified") for c in cs),
                         "applied and verified in the thread", 60)
        replies = [c for c in verified if c["root_id"] == second["id"]]
        assert [c["attachment"]["title"] for c in replies] == [
            "Applied: Move summaries to US-C and support to EU-B", "Verified: support has recovered"], replies
        assert field(replies[0], "Routes") == "support → EU-B · summaries → US-C", replies
        assert d.routes() == {"support": "EU-B", "batch": "US-C"}
        checks.append("policy restored: a new proposal; Approve applies it (Applied, then Verified, in its "
                      "thread) and posts no Out of date card")

        d.control("restore", "EU-A")
        time.sleep(5)
        d.configure(capacity={"EU-A": 50000})
        risk = card("at-risk")[-1]
        assert risk["attachment"]["title"] == "Failover at risk: losing EU-B leaves support nowhere to run", risk
        d.configure(capacity={"EU-A": 100000})
        restored = card("restored")[-1]
        assert restored["attachment"]["title"] == "Failover restored", restored
        checks.append("EU-A at 50k: failover at risk (losing EU-B leaves support nowhere to run); back to "
                      "100k: failover restored")

        time.sleep(8)
        final = cards()
        ids = [c["ignite_id"] for c in final]
        assert len(ids) == len(set(ids)), f"each card is posted once: {ids}"
        assert [c["card"] for c in final if c["root_id"]] == ["outcome"] * 3
        checks.append("every card posted once; only outcomes are thread replies")
        path = d.save("incident-channel.json", {"checks": checks, "cards": final})
        print(json.dumps({"passed": checks, "evidence": str(path)}, indent=2))
    finally:
        d.reset()


if __name__ == "__main__":
    main()
