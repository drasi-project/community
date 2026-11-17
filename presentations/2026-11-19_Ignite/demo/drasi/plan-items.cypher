MATCH (w:ignite_workloads)-[:WORKLOAD_ROUTE]->(r:ignite_routes),
      (w)-[:WORKLOAD_SCENARIO]->(s:ignite_scenario)
WHERE w.run_epoch = s.run_epoch AND r.run_epoch = s.run_epoch
RETURN w.id AS id, s.run_epoch AS run_epoch, s.revision AS revision,
       w.demand_tpm AS demand_tpm, r.deployment AS deployment
