MATCH (r:ResilienceResult)-[:RESILIENCE_SCENARIO]->(s:ignite_scenario)<-[:PLAN_SCENARIO]-(p:PlanResult)
WHERE r.run_epoch = s.run_epoch AND p.run_epoch = s.run_epoch
WITH r, s, p,
     split(coalesce(drasi.previousDistinctValue(r.run_epoch + '|'
       + CASE WHEN r.state = 'unknown' THEN 'unknown' WHEN r.unhealthy_bins > 0 THEN 'degraded'
              WHEN r.state = 'safe' THEN 'safe' ELSE 'at_risk' END), '') + '|', '|') AS was,
     drasi.trueFor(r.state = 'at_risk' AND r.unhealthy_bins = 0, 3000) AS held,
     drasi.trueFor(r.state = 'at_risk' AND r.unhealthy_bins = 0, 63000) AS stale
WITH r, s, p, held, stale, was[0] AS was_run, was[1] AS was_state
WHERE held = true AND NOT (stale = true) AND was_run = r.run_epoch AND (was_state = 'safe' OR was_state = 'degraded')
WITH r, s, p, replace(r.reason, 'batch', 'summaries') AS reason,
     CASE WHEN r.reason CONTAINS 'support' THEN 'support' WHEN r.reason CONTAINS 'batch' THEN 'summaries' ELSE '' END AS workload
RETURN 'at-risk-' + toString(p.solved_ms) AS id, s.run_epoch AS run_epoch, p.solved_ms AS at_ms, '' AS root_id,
       CASE WHEN r.first_failure = '' THEN 'Failover at risk: no valid plan even now'
            WHEN workload = '' THEN 'Failover at risk: losing ' + r.first_failure + ' leaves no valid plan'
            ELSE 'Failover at risk: losing ' + r.first_failure + ' leaves ' + workload + ' nowhere to run' END AS title,
       CASE WHEN r.first_failure = '' THEN 'Everything is healthy, but no plan fits policy and capacity. '
            ELSE 'Everything is healthy, but if ' + r.first_failure + ' failed, '
                 + CASE WHEN workload = '' THEN 'no plan would fit policy and capacity. '
                        ELSE workload + ' would have nowhere allowed to run. ' END END
         + reason + '.' AS text,
       CASE WHEN r.first_failure = '' THEN 'none' ELSE r.first_failure END AS lost,
       CASE WHEN workload = '' THEN 'every workload' ELSE workload END AS affects
