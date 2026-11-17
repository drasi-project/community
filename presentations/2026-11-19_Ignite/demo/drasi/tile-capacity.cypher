MATCH (d:ignite_deployments)-[:DEPLOYMENT_REPORT]->(r:ignite_reports),
      (d)-[:DEPLOYMENT_SCENARIO]->(s:ignite_scenario)
WHERE d.run_epoch = s.run_epoch AND r.run_epoch = s.run_epoch
RETURN sum(d.capacity_tpm) AS total_tpm,
       sum(CASE WHEN r.status = 'ok' AND NOT drasi.trueLater(true, r.expires_ms)
                THEN d.capacity_tpm ELSE 0 END) AS healthy_tpm
