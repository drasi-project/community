MATCH (t:ai_requests)-[:REQUEST_CONFIG]->(s:routing_config)
WHERE t.run_epoch = s.run_epoch AND t.completed_ms IS NULL AND t.cancelled_ms IS NULL
  AND drasi.trueLater(true, t.deadline_ms)
RETURN t.workload AS workload, count(t) AS value
