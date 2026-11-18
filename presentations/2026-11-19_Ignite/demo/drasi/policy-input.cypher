MATCH (w:ignite_workloads)-[:WORKLOAD_SCENARIO]->(s:ignite_scenario),
      (d:ignite_deployments)-[:DEPLOYMENT_SCENARIO]->(s)
OPTIONAL MATCH (w)-[:WORKLOAD_DESTINATION]->(p:ignite_policy_destinations)
WITH w.id AS workload, w.data_class AS data_class,
     w.region_requirement AS region_requirement, w.model AS workload_model,
     d.id AS deployment, d.region AS region, d.model AS deployment_model,
     s.run_epoch AS run_epoch, count(d) AS matches,
     collect(CASE WHEN p.deployment = d.id AND p.run_epoch = s.run_epoch
                  THEN p.deployment ELSE null END) AS destinations
WHERE matches > 0
RETURN workload + '/' + deployment AS id, run_epoch, workload, data_class,
       region_requirement, workload_model, deployment, region, deployment_model,
       destinations
