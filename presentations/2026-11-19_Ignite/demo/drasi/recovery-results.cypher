MATCH (o:RecoveryObservation)-[:RECOVERY_SUMMARY]->(t:RecoveryTasks)
WHERE o.run_epoch = t.run_epoch AND o.applied_revision = t.revision
RETURN o.id AS id, o.id AS approval, o.run_epoch AS run_epoch, o.applied_revision AS applied_revision,
       o.applied_ms AS applied_ms, o.expires_ms AS expires_ms,
       t.outstanding AS outstanding, t.last_success_ms AS last_success_ms, t.successes AS successes,
       CASE WHEN drasi.trueLater(true, o.expires_ms) THEN 'unknown'
            WHEN o.endpoint_status <> 'ok' THEN 'unconfirmed'
            WHEN t.oldest_pending_ms > o.applied_ms AND t.last_success_ms >= o.applied_ms
              AND t.successes >= 3
              AND NOT drasi.trueLater(true, t.last_success_ms + 15000)
              AND drasi.trueLater(true, o.applied_ms + 5000)
              THEN 'confirmed' ELSE 'unconfirmed' END AS state
