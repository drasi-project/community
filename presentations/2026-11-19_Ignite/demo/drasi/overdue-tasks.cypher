MATCH (t:ignite_tasks)-[:TASK_MODEL]-(m:ignite_model)
WHERE t.run_epoch = m.run_epoch AND t.completed_ms IS NULL AND t.cancelled_ms IS NULL
  AND drasi.trueLater(true, t.deadline_ms)
RETURN t.id AS id, t.run_epoch AS run_epoch, t.workload AS workload,
       t.trace_id AS trace_id, t.deadline_ms AS deadline_ms, 'overdue' AS impact
