MATCH (t:ai_requests)-[:REQUEST_CONFIG]->(s:routing_config)
WHERE t.run_epoch = s.run_epoch
  AND drasi.trueLater(t.completed_ms IS NULL AND t.cancelled_ms IS NULL, t.deadline_ms)
RETURN t.id AS id, t.run_epoch AS run_epoch, t.workload AS workload,
       t.trace_id AS trace_id, t.deadline_ms AS deadline_ms, 'overdue' AS impact
