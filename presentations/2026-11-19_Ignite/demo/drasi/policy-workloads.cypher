MATCH (w:ignite_workloads)-[:WORKLOAD_SCENARIO]->(s:ignite_scenario)
WHERE w.run_epoch = s.run_epoch
RETURN w.id AS id, w.run_epoch AS run_epoch, w.data_class AS data_class,
       w.region_requirement AS region_requirement, w.model AS model, w.demand_tpm AS demand_tpm
