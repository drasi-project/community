//! Contract tests against this copy's PostgreSQL. They reset the scenario, so they are ignored by
//! default; run with `DATABASE_URL` from demo/.env:
//! `cargo test --locked -p ignite-workloads -- --ignored --test-threads=1`
use crate::{db, external, scenario, Payload};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

fn body<T: DeserializeOwned>(value: Value) -> Payload<T> {
    Payload(serde_json::from_value(value).expect("request shape"))
}

fn delivery(id: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("x-drasi-delivery-id", HeaderValue::from_str(id).unwrap());
    headers
}

async fn ok(result: crate::Api) -> Value {
    match result {
        Ok(axum::Json(value)) => value,
        Err(error) => panic!("{}: {:#}", error.status, error.error),
    }
}

fn status(result: crate::Api) -> StatusCode {
    result.err().map_or(StatusCode::OK, |e| e.status)
}

async fn reset() -> (String, i64) {
    let mut client = db().await.unwrap();
    let tx = client.transaction().await.unwrap();
    tx.batch_execute(&scenario::reset_sql()).await.unwrap();
    let current = scenario::current(&tx).await.unwrap();
    tx.commit().await.unwrap();
    current
}

async fn approval(id: &str) -> Value {
    db().await
        .unwrap()
        .query_one(
            "SELECT to_jsonb(a) FROM ignite_approvals a WHERE id=$1",
            &[&id],
        )
        .await
        .unwrap()
        .get(0)
}

#[tokio::test]
#[ignore]
async fn stand_in_apis_follow_the_contract() {
    let (epoch, revision) = reset().await;
    assert_eq!(revision, 1);
    let s = scenario::load(&db().await.unwrap()).await.unwrap();
    assert_eq!(s.route("support").unwrap(), "EU-A");
    assert_eq!(s.route("batch").unwrap(), "EU-B");
    assert_eq!(s.destinations["support"], ["EU-A", "EU-B"]);
    assert!(s.deployments.iter().all(|d| d.mode == "ok" && d.capacity_tpm == 100_000));
    let tag = crate::identity();
    let (a1, a2) = (format!("{tag}-1"), format!("{tag}-2"));

    // Approval tool: records, never validates; a repeated ID is a no-op; one latest.
    let request = json!({"id":a1,"candidate_id":"c1","basis":"b1","revision":1});
    assert_eq!(ok(external::approve(body(request.clone())).await).await["id"], a1);
    assert_eq!(ok(external::approve(body(request)).await).await["id"], a1);
    ok(external::approve(body(json!({"id":a2,"candidate_id":"c2","basis":"b2","revision":1}))).await).await;
    assert_eq!(approval(&a1).await["latest"], false);
    assert_eq!(approval(&a2).await["latest"], true);
    assert_eq!(approval(&a2).await["status"], "pending");

    // Gateway configuration API: compare-and-set on revision, idempotent on delivery and state.
    let apply = |approval: &str, candidate: &str, basis: &str, expected: i64| {
        json!({"approval":approval,"candidate_id":candidate,"basis":basis,"expected_revision":expected,
               "assignments":{"support":"EU-B","batch":"US-C"}})
    };
    let first = ok(external::apply(delivery(&format!("{tag}/d1")), body(apply(&a1, "c1", "b1", 1))).await).await;
    assert_eq!(first, json!({"result":"applied","revision":2}));
    let replay = ok(external::apply(delivery(&format!("{tag}/d1")), body(apply(&a1, "c1", "b1", 1))).await).await;
    assert_eq!(replay, first);
    let again = ok(external::apply(delivery(&format!("{tag}/d2")), body(apply(&a1, "c1", "b1", 1))).await).await;
    assert_eq!(again["result"], "duplicate");
    let s = scenario::load(&db().await.unwrap()).await.unwrap();
    assert_eq!((s.revision, s.route("support").unwrap(), s.route("batch").unwrap()), (2, "EU-B", "US-C"));
    let applied = approval(&a1).await;
    assert_eq!((applied["status"].as_str(), applied["applied_revision"].as_i64()), (Some("applied"), Some(2)));
    let route_approval: Option<String> = db().await.unwrap()
        .query_one("SELECT approval FROM ignite_routes WHERE workload='support'", &[]).await.unwrap().get(0);
    assert_eq!(route_approval.as_deref(), Some(a1.as_str()));

    let conflict = ok(external::apply(delivery(&format!("{tag}/d3")), body(apply(&a2, "c2", "b2", 1))).await).await;
    assert_eq!(conflict["result"], "conflict");
    let rejected = approval(&a2).await;
    assert_eq!(rejected["status"], "rejected");
    assert_eq!(rejected["reason"], "Scenario changed since the plan (revision 2, plan revision 1)");

    // Permanent failures are 4xx; nothing changes.
    assert_eq!(status(external::apply(HeaderMap::new(), body(apply(&a1, "c1", "b1", 2))).await), StatusCode::BAD_REQUEST);
    assert_eq!(status(external::apply(delivery(&format!("{tag}/d4")), body(apply("missing", "c", "b", 2))).await), StatusCode::NOT_FOUND);
    let a3 = format!("{tag}-3");
    ok(external::approve(body(json!({"id":a3,"candidate_id":"c3","basis":"b3","revision":2}))).await).await;
    assert_eq!(status(external::apply(delivery(&format!("{tag}/d5")), body(apply(&a3, "other", "b3", 2))).await), StatusCode::BAD_REQUEST);
    let partial = json!({"approval":a3,"candidate_id":"c3","basis":"b3","expected_revision":2,"assignments":{"support":"EU-A"}});
    assert_eq!(status(external::apply(delivery(&format!("{tag}/d6")), body(partial)).await), StatusCode::BAD_REQUEST);
    assert_eq!(approval(&a3).await["status"], "pending");

    // Reject and verified: one transition each, otherwise no-ops.
    let path = |id: &str| axum::extract::Path(id.to_owned());
    assert_eq!(ok(external::reject(path(&a3), body(json!({"reason":"The plan changed since it was approved"}))).await).await["result"], "rejected");
    assert_eq!(ok(external::reject(path(&a3), body(json!({"reason":"again"}))).await).await["result"], "no-op");
    assert_eq!(approval(&a3).await["reason"], "The plan changed since it was approved");
    assert_eq!(ok(external::verified(path(&a3), body(json!({"verified_ms":1}))).await).await["result"], "no-op");
    assert_eq!(ok(external::verified(path(&a1), body(json!({"verified_ms":1234}))).await).await["result"], "verified");
    assert_eq!(ok(external::verified(path(&a1), body(json!({"verified_ms":5678}))).await).await["result"], "no-op");
    assert_eq!(approval(&a1).await["verified_ms"], 1234);
    assert_eq!(ok(external::reject(path(&a1), body(json!({"reason":"late"}))).await).await["result"], "no-op");
    assert_eq!(status(external::verified(path("missing"), body(json!({"verified_ms":1}))).await), StatusCode::NOT_FOUND);

    // Desk controls: configuration raises the revision once per change; simulation never does.
    let stale = json!({"run_epoch":epoch,"revision":1,"capacity":{"EU-A":50000}});
    assert_eq!(status(scenario::configuration(body(stale)).await), StatusCode::CONFLICT);
    let edit = json!({"run_epoch":epoch,"revision":2,"capacity":{"EU-A":50000},"destinations":{"batch":["EU-A","EU-B"]}});
    assert_eq!(ok(scenario::configuration(body(edit)).await).await["revision"], 3);
    let same = json!({"run_epoch":epoch,"revision":3,"capacity":{"EU-A":50000},"destinations":{"batch":["EU-B","EU-A"]}});
    assert_eq!(ok(scenario::configuration(body(same)).await).await["revision"], 3);
    let control = json!({"run_epoch":epoch,"deployment":"EU-A","action":"fail"});
    assert_eq!(ok(scenario::control(body(control)).await).await["revision"], 3);
    let s = scenario::load(&db().await.unwrap()).await.unwrap();
    assert_eq!(s.destinations["batch"], ["EU-A", "EU-B"]);
    assert_eq!(s.deployment("EU-A").unwrap().capacity_tpm, 50_000);
    assert_eq!(s.deployment("EU-A").unwrap().mode, "failed");
    let bad = json!({"run_epoch":epoch,"deployment":"EU-Z","action":"fail"});
    assert_eq!(status(scenario::control(body(bad)).await), StatusCode::BAD_REQUEST);

    // A new run: revision 1, seed restored, old approvals cannot be applied.
    let a5 = format!("{tag}-5");
    ok(external::approve(body(json!({"id":a5,"candidate_id":"c5","basis":"b5","revision":3}))).await).await;
    let (new_epoch, revision) = reset().await;
    assert!(new_epoch != epoch && revision == 1);
    let a4 = format!("{tag}-4");
    ok(external::approve(body(json!({"id":a4,"candidate_id":"c4","basis":"b4","revision":1}))).await).await;
    assert_eq!(approval(&a4).await["latest"], true);
    let old = ok(external::apply(delivery(&format!("{tag}/d7")), body(apply(&a5, "c5", "b5", 1))).await).await;
    assert_eq!(old["result"], "conflict");
    assert_eq!(approval(&a5).await["reason"], "The demo was reset after this approval");
    assert_eq!(scenario::current(&db().await.unwrap()).await.unwrap().1, 1);
    reset().await;
}
