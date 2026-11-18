MATCH (a:change_approvals)-[:APPROVAL_CONFIG]->(s:routing_config)
WHERE a.run_epoch = s.run_epoch
  AND NOT drasi.trueLater(true, coalesce(a.decided_ms, a.requested_ms) + 30000)
RETURN a.id + '-' + a.status AS id, coalesce(a.decided_ms, a.requested_ms) AS at_ms,
       CASE WHEN a.status = 'pending' THEN 'Approved: the proposed plan'
            WHEN a.status = 'applied' THEN 'Applied by Drasi: routes changed'
            WHEN a.status = 'verified' THEN 'Recovery verified by Drasi'
            WHEN a.status = 'declined' THEN 'Rejected by ' + coalesce(a.requested_by, '@oncall')
            ELSE 'Rejected by Drasi: ' + coalesce(a.reason, '') END AS text
