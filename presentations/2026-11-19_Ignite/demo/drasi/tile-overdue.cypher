MATCH (t:ignite_tasks)-[:TASK_SCENARIO]->(s:ignite_scenario)
WHERE t.run_epoch = s.run_epoch AND t.completed_ms IS NULL AND t.cancelled_ms IS NULL
  AND drasi.trueLater(true, t.deadline_ms)
RETURN t.workload AS workload, count(t) AS value
