MATCH (a:ignite_attempts)-[:ATTEMPT_MODEL]-(m:ignite_model)
WHERE a.run_epoch = m.run_epoch
RETURN a.id AS id, a.task_id AS task_id, a.run_epoch AS run_epoch, a.workload AS workload,
       a.endpoint AS endpoint, a.status AS status, a.trace_id AS trace_id,
       a.started_ms AS started_ms, a.finished_ms AS finished_ms, a.model_revision AS model_revision
