MATCH (r:workload_routes)<-[:WORKLOAD_ROUTE]-(w:ai_workloads),
      (r)-[:ROUTE_POLICY]->(p:PolicyResult),
      (r)-[:ROUTE_HEALTH]->(h:deployment_health_reports)
WHERE p.target = r.deployment AND w.run_epoch = r.run_epoch AND p.run_epoch = r.run_epoch
  AND h.run_epoch = r.run_epoch
RETURN r.workload AS workload,
       CASE r.workload WHEN 'support' THEN 'Support' WHEN 'batch' THEN 'Summaries' ELSE r.workload END AS name,
       r.deployment AS deployment,
       CASE WHEN p.state = 'unknown' OR drasi.trueLater(true, h.expires_ms) THEN -1
            WHEN p.state = 'denied' THEN 3
            WHEN h.status = 'ok' THEN 0
            WHEN h.status = 'throttled' THEN 1
            WHEN h.status = 'failed' THEN 2
            ELSE -1 END AS status
