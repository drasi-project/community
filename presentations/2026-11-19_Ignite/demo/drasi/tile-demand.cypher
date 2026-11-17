MATCH (w:ignite_workloads)-[:WORKLOAD_SCENARIO]->(s:ignite_scenario)
WHERE w.run_epoch = s.run_epoch
RETURN sum(w.demand_tpm) AS needed_tpm
