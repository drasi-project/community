MATCH (a:change_approvals)-[:APPROVAL_PLAN]->(p:PlanResult),
      (a)-[:APPROVAL_CONFIG]->(s:routing_config)
WHERE a.run_epoch = s.run_epoch AND p.run_epoch = s.run_epoch AND a.latest = true
RETURN CASE WHEN p.state = 'unknown' THEN -1
            WHEN a.status = 'applied' THEN 3
            WHEN a.status = 'pending' THEN 2
            WHEN p.state = 'feasible' AND p.moves > 0 THEN 1
            WHEN a.status = 'rejected' THEN 5
            WHEN a.status = 'verified' THEN 4
            ELSE 0 END AS value,
       coalesce(a.change, '') AS text
