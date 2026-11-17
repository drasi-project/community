MATCH (p:PlanResult)-[:PLAN_RESILIENCE]->(r:ResilienceResult)
WHERE p.run_epoch = r.run_epoch
RETURN CASE WHEN p.state = 'unknown' OR r.state = 'unknown' THEN -1
            WHEN p.state = 'infeasible' THEN 2
            WHEN p.moves > 0 THEN 1
            WHEN r.state = 'at_risk' AND r.unhealthy_bins = 0 THEN 3
            ELSE 0 END AS value
