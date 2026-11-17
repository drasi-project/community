MATCH (a:ignite_approvals)-[:APPROVAL_ROUTE]->(rt:ignite_routes)-[:ROUTE_REPORT]->(r:ignite_reports),
      (a)-[:APPROVAL_SCENARIO]->(s:ignite_scenario)
WHERE rt.workload = 'support' AND a.run_epoch = s.run_epoch AND rt.run_epoch = s.run_epoch
  AND r.run_epoch = s.run_epoch AND a.applied_revision = s.revision
  AND (a.status = 'applied' OR a.status = 'verified')
RETURN a.id AS id, a.run_epoch AS run_epoch, a.applied_revision AS applied_revision,
       rt.applied_ms AS applied_ms, r.expires_ms AS expires_ms, r.status AS endpoint_status
