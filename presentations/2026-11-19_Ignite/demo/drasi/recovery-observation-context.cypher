MATCH (a:change_approvals)-[:APPROVAL_ROUTE]->(rt:workload_routes)-[:ROUTE_HEALTH]->(r:deployment_health_reports),
      (a)-[:APPROVAL_CONFIG]->(s:routing_config)
WHERE rt.workload = 'support' AND a.run_epoch = s.run_epoch AND rt.run_epoch = s.run_epoch
  AND r.run_epoch = s.run_epoch AND a.applied_revision = s.revision
  AND (a.status = 'applied' OR a.status = 'verified')
RETURN a.id AS id, a.run_epoch AS run_epoch, a.applied_revision AS applied_revision,
       rt.applied_ms AS applied_ms, r.expires_ms AS expires_ms, r.status AS endpoint_status
