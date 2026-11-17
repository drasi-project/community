MATCH (a:ignite_approvals)-[:APPROVAL_SCENARIO]->(s:ignite_scenario)
WHERE a.run_epoch = s.run_epoch
  AND NOT drasi.trueLater(true, coalesce(a.decided_ms, a.requested_ms) + 30000)
RETURN a.id + '-' + a.status AS id, coalesce(a.decided_ms, a.requested_ms) AS at_ms,
       CASE WHEN a.status = 'pending' THEN 'Approved: ' + a.id
            WHEN a.status = 'applied' THEN 'Applied: ' + a.id + ' (revision ' + a.applied_revision + ')'
            WHEN a.status = 'verified' THEN 'Recovery verified: ' + a.id
            ELSE 'Rejected: ' + a.id + ': ' + coalesce(a.reason, '') END AS text
