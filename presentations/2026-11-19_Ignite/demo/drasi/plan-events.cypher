MATCH (p:PlanResult)
WHERE drasi.trueFor(p.state = 'feasible' AND p.moves > 0, 2000)
RETURN 'plan-' + p.candidate_id AS id, 'Drasi proposes a plan: ' + replace(p.reason, 'batch', 'summaries') AS text
