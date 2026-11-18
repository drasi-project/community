MATCH (d:model_deployments)-[:DEPLOYMENT_HEALTH]->(r:deployment_health_reports),
      (d)-[:DEPLOYMENT_CONFIG]->(s:routing_config)
WHERE d.run_epoch = s.run_epoch AND r.run_epoch = s.run_epoch
RETURN d.id AS deployment, d.capacity_tpm AS capacity_tpm,
       CASE WHEN drasi.trueLater(true, r.expires_ms) THEN -1
            WHEN r.status = 'ok' THEN 0
            WHEN r.status = 'throttled' THEN 1
            WHEN r.status = 'failed' THEN 2
            ELSE -1 END AS status
