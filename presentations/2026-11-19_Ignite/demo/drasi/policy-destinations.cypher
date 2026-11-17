MATCH (p:ignite_policy_destinations)-[:DESTINATION_SCENARIO]->(s:ignite_scenario)
WHERE p.run_epoch = s.run_epoch
RETURN p.id AS id, p.run_epoch AS run_epoch, p.workload AS workload, p.deployment AS deployment
