MATCH (r:ResilienceResult)-[:RESILIENCE_CONFIG]->(s:routing_config)<-[:PLAN_CONFIG]-(p:PlanResult)
WHERE r.run_epoch = s.run_epoch AND p.run_epoch = s.run_epoch
WITH r, s, p,
     split(coalesce(drasi.previousDistinctValue(r.run_epoch + '|'
       + CASE WHEN r.state = 'unknown' THEN 'unknown' WHEN r.unhealthy_bins > 0 THEN 'degraded'
              WHEN r.state = 'safe' THEN 'safe' ELSE 'at_risk' END), '') + '|', '|') AS was,
     drasi.trueFor(r.state = 'safe' AND r.unhealthy_bins = 0, 3000) AS held,
     drasi.trueFor(r.state = 'safe' AND r.unhealthy_bins = 0, 63000) AS stale
WITH r, s, p, held, stale, was[0] AS was_run, was[1] AS was_state
WHERE held = true AND NOT (stale = true) AND was_run = r.run_epoch AND was_state = 'at_risk'
RETURN 'restored-' + toString(p.solved_ms) AS id, s.run_epoch AS run_epoch, p.solved_ms AS at_ms, '' AS root_id,
       'Failover restored' AS title,
       'Losing any one deployment now still leaves a valid plan for every workload.' AS text
