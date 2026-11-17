MATCH (r:ignite_routes)<-[:WORKLOAD_ROUTE]-(w:ignite_workloads),
      (r)-[:ROUTE_POLICY]->(p:PolicyResult)
WHERE p.target = r.deployment AND w.run_epoch = r.run_epoch AND p.run_epoch = r.run_epoch
RETURN r.workload AS workload, r.deployment AS deployment,
       CASE WHEN p.state = 'denied' THEN 0 - w.demand_tpm ELSE w.demand_tpm END AS value
