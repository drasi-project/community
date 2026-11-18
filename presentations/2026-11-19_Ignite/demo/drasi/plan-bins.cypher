MATCH (d:ignite_deployments)-[:DEPLOYMENT_REPORT]->(r:ignite_reports),
      (d)-[:DEPLOYMENT_SCENARIO]->(s:ignite_scenario)
WHERE d.run_epoch = s.run_epoch AND r.run_epoch = s.run_epoch
RETURN d.id AS id, d.run_epoch AS run_epoch, d.capacity_tpm AS capacity_tpm,
       CASE WHEN drasi.trueLater(true, r.expires_ms) THEN 'unknown'
            WHEN r.status = 'ok' THEN 'healthy'
            WHEN r.status IN ['failed', 'throttled'] THEN 'unhealthy'
            ELSE 'unknown' END AS health
