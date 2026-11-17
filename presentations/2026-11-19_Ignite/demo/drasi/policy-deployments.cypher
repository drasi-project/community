MATCH (d:ignite_deployments)-[:DEPLOYMENT_SCENARIO]->(s:ignite_scenario)
WHERE d.run_epoch = s.run_epoch
RETURN d.id AS id, d.run_epoch AS run_epoch, d.region AS region, d.model AS model,
       d.capacity_tpm AS capacity_tpm
