MATCH (a:change_approvals)-[:APPROVAL_CONFIG]->(s:routing_config),
      (rs:workload_routes)-[:ROUTE_CONFIG]->(s),
      (rb:workload_routes)-[:ROUTE_CONFIG]->(s)
WHERE a.run_epoch = s.run_epoch AND rs.workload = 'support' AND rb.workload = 'batch'
  AND (a.status = 'applied' OR a.status = 'verified' OR a.status = 'rejected')
  AND drasi.trueLater(true, a.decided_ms + 1000)
  AND NOT drasi.trueLater(true, a.decided_ms + 60000)
WITH a, s, rs, rb, coalesce(a.requested_by, 'desk') AS approved_by,
     coalesce(a.change, 'the approved change') AS change
RETURN a.id + '-' + a.status AS id, s.run_epoch AS run_epoch, a.decided_ms AS at_ms,
       coalesce(a.post_id, '') AS root_id,
       CASE WHEN a.status = 'applied' THEN 'Applied: ' + change
            WHEN a.status = 'verified' THEN 'Verified: support has recovered'
            ELSE 'Rejected by Drasi: nothing was changed' END AS title,
       CASE WHEN a.status = 'applied' THEN '#3d8bd9' WHEN a.status = 'verified' THEN '#2fa66a' ELSE '#f2a33a' END AS color,
       CASE WHEN a.status = 'applied'
              THEN 'Drasi applied the approved change through the gateway’s configuration API. It calls it fixed only when the backlog has cleared and new support requests are answered on the new route.'
            WHEN a.status = 'verified'
              THEN 'The backlog from before the change has cleared and new support requests are answered on the new route.'
            ELSE coalesce(a.reason, 'Drasi rejected the approval') + '.' END AS text,
       approved_by,
       CASE WHEN a.status = 'applied' THEN 'Routes' ELSE 'Change' END AS detail_title,
       CASE WHEN a.status = 'applied' THEN 'support → ' + rs.deployment + ' · summaries → ' + rb.deployment
            ELSE change END AS detail
