MATCH (t:ignite_tasks)-[:TASK_SCENARIO]->(s:ignite_scenario)
WHERE t.run_epoch = s.run_epoch AND t.workload = 'support'
WITH s.run_epoch AS run_epoch, s.revision AS revision,
     count(t) AS task_count,
     sum(CASE WHEN t.completed_ms IS NULL AND t.cancelled_ms IS NULL THEN 1 ELSE 0 END) AS pending,
     max(CASE WHEN t.completed_revision = s.revision THEN coalesce(t.completed_ms, 0) ELSE 0 END) AS last_success,
     sum(CASE WHEN t.completed_revision = s.revision THEN 1 ELSE 0 END) AS successes,
     min(CASE WHEN t.completed_ms IS NULL AND t.cancelled_ms IS NULL THEN t.created_ms ELSE 9007199254740991 END) AS oldest_pending
WHERE task_count > 0
RETURN 'support' AS id, run_epoch, revision, toInteger(pending) AS outstanding,
       toInteger(coalesce(last_success, 0)) AS last_success_ms, toInteger(successes) AS successes,
       toInteger(oldest_pending) AS oldest_pending_ms
