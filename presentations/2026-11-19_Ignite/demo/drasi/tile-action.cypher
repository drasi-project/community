MATCH (p:PlanResult)-[:PLAN_RESILIENCE]->(r:ResilienceResult)
WHERE p.run_epoch = r.run_epoch
RETURN CASE WHEN p.state = 'unknown' OR r.state = 'unknown' THEN -1
            WHEN p.state = 'infeasible' THEN 2
            WHEN p.moves > 0 THEN 1
            WHEN r.state = 'at_risk' AND r.unhealthy_bins = 0 THEN 3
            ELSE 0 END AS value,
       CASE WHEN p.state = 'unknown' OR r.state = 'unknown' THEN 'Unknown'
            WHEN p.state = 'infeasible' THEN 'No change fits policy and capacity'
            WHEN p.moves > 0 THEN replace(head(split(p.reason, ' (')), 'batch', 'summaries')
            WHEN r.state = 'at_risk' AND r.unhealthy_bins = 0 AND r.first_failure <> ''
              THEN 'Add capacity: losing ' + r.first_failure + ' leaves '
                   + CASE WHEN r.reason CONTAINS 'support' THEN 'support'
                          WHEN r.reason CONTAINS 'batch' THEN 'summaries'
                          ELSE '' END
                   + CASE WHEN r.reason CONTAINS 'support' OR r.reason CONTAINS 'batch' THEN ' nowhere to run' ELSE 'no valid plan' END
            WHEN r.state = 'at_risk' AND r.unhealthy_bins = 0 THEN 'Add capacity: no valid plan even now'
            ELSE 'None' END AS text
