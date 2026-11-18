MATCH (d:model_deployments)-[:DEPLOYMENT_HEALTH]->(r:deployment_health_reports),
      (d)-[:DEPLOYMENT_CONFIG]->(s:routing_config)
WHERE d.run_epoch = s.run_epoch AND r.run_epoch = s.run_epoch
RETURN sum(d.capacity_tpm) AS total_tpm,
       sum(CASE WHEN r.status = 'ok' AND NOT drasi.trueLater(true, r.expires_ms)
                THEN d.capacity_tpm ELSE 0 END) AS healthy_tpm
