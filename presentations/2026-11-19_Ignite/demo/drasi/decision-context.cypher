MATCH (r:ignite_reports)-[:REPORT_MODEL]-(m:ignite_model)
WHERE r.run_epoch = m.run_epoch
RETURN r.id AS id, m.run_epoch AS run_epoch, m.revision AS revision, m.payload AS payload,
       r.observed_ms AS observed_ms, r.expires_ms AS expires_ms, r.sequence AS sequence,
       CASE WHEN drasi.trueLater(true, r.expires_ms) THEN 'unknown' ELSE r.status END AS status
