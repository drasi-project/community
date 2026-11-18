MATCH (w:ai_workloads)-[:WORKLOAD_ROUTE]->(r:workload_routes),
      (w)-[:WORKLOAD_CONFIG]->(s:routing_config),
      (d:model_deployments)
WHERE w.run_epoch = s.run_epoch AND r.run_epoch = s.run_epoch AND d.run_epoch = s.run_epoch
RETURN w.id AS subject, d.id AS target, s.run_epoch AS run_epoch,
       w.demand_tpm AS demand_tpm, r.deployment AS current, d.capacity_tpm AS capacity_tpm
