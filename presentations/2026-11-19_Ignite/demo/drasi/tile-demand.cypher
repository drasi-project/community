MATCH (w:ai_workloads)-[:WORKLOAD_CONFIG]->(s:routing_config)
WHERE w.run_epoch = s.run_epoch
RETURN sum(w.demand_tpm) AS needed_tpm
