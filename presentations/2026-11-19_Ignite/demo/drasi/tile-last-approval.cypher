MATCH (a:change_approvals)-[:APPROVAL_CONFIG]->(s:routing_config)
WHERE a.run_epoch = s.run_epoch AND a.latest = true
RETURN a.id AS approval, a.status AS status, a.reason AS reason,
       CASE WHEN a.status = 'pending' THEN 1 WHEN a.status = 'applied' THEN 2
            WHEN a.status = 'verified' THEN 3 WHEN a.status = 'rejected' THEN 4 ELSE -1 END AS value
