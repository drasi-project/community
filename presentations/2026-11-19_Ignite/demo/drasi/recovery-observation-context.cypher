MATCH (r:ignite_reports)-[:REPORT_MODEL]-(m:ignite_model)-[:MODEL_COMMAND]-(c:ignite_commands)
WHERE r.run_epoch = m.run_epoch AND c.run_epoch = m.run_epoch
  AND r.id = c.support_endpoint AND c.applied_revision = m.revision AND c.applied_ms IS NOT NULL
RETURN c.id AS id, c.run_epoch AS run_epoch, c.applied_revision AS applied_revision,
       c.applied_ms AS applied_ms, r.expires_ms AS expires_ms, r.status AS endpoint_status
