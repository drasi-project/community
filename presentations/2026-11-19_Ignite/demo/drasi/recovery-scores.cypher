MATCH (t:ai_requests)-[:REQUEST_APPROVAL]->(a:change_approvals)
WHERE t.run_epoch = a.run_epoch AND t.workload = 'support' AND a.status = 'verified'
  AND t.completed_revision = a.applied_revision AND t.completed_ms IS NOT NULL
RETURN t.id + '-recovery-' + a.id AS id, t.trace_id AS trace_id, a.id AS approval,
       'Answered on the route of approval ' + a.id + ', which Drasi verified' AS comment
