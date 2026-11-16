MATCH (b:ignite_breaches)
RETURN b.id AS id, b.run_epoch AS run_epoch, b.deadline_ms AS deadline_ms,
       b.observed_ms AS observed_ms, b.trace_id AS trace_id
