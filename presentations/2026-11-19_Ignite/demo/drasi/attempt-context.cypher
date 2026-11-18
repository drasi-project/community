MATCH (a:model_call_attempts)-[:ATTEMPT_CONFIG]->(s:routing_config)
WHERE a.run_epoch = s.run_epoch
RETURN a.id AS id, a.request_id AS request_id, a.run_epoch AS run_epoch, a.workload AS workload,
       a.endpoint AS endpoint, a.status AS status, a.trace_id AS trace_id,
       a.started_ms AS started_ms, a.finished_ms AS finished_ms, a.model_revision AS model_revision
