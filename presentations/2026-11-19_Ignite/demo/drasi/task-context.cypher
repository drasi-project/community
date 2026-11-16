MATCH (t:ignite_tasks)-[:TASK_MODEL]-(m:ignite_model)
WHERE t.run_epoch = m.run_epoch
RETURN t.id AS id, t.run_epoch AS run_epoch, t.workload AS workload,
       t.trace_id AS trace_id, t.created_ms AS created_ms, t.deadline_ms AS deadline_ms,
       t.completed_ms AS completed_ms, t.cancelled_ms AS cancelled_ms,
       t.endpoint AS endpoint, t.revision AS revision
