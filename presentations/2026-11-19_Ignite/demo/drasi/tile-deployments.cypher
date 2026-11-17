MATCH (d:ignite_deployments)-[:DEPLOYMENT_REPORT]->(r:ignite_reports),
      (d)-[:DEPLOYMENT_SCENARIO]->(s:ignite_scenario)
WHERE d.run_epoch = s.run_epoch AND r.run_epoch = s.run_epoch
RETURN d.id AS deployment, d.capacity_tpm AS capacity_tpm,
       CASE WHEN drasi.trueLater(true, r.expires_ms) THEN -1
            WHEN r.status = 'ok' THEN 0
            WHEN r.status = 'throttled' THEN 1
            WHEN r.status = 'failed' THEN 2
            ELSE -1 END AS status
