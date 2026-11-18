MATCH (p:PlanResult)-[:PLAN_CONFIG]->(s:routing_config),
      (f:DeploymentFacts)-[:DEPLOYMENT_FACTS_CONFIG]->(s),
      (g:PolicyFacts)-[:POLICY_FACTS_CONFIG]->(s)
WHERE p.run_epoch = s.run_epoch AND f.run_epoch = s.run_epoch AND g.run_epoch = s.run_epoch
WITH p, s, f, g,
     split(coalesce(drasi.previousDistinctValue(p.run_epoch + '|' + p.state + '|' + toString(p.moves) + '|' + p.candidate_id), '') + '|||', '|') AS was
WITH p, s, f, g, was[0] AS was_run, was[1] AS was_state, was[2] AS was_moves
WHERE p.state = 'feasible' AND p.moves > 0
  AND was_run = p.run_epoch AND was_state <> 'unknown' AND was_state <> ''
  AND drasi.trueLater(true, p.solved_ms + 5000)
  AND NOT drasi.trueLater(true, p.solved_ms + 60000)
WITH p, s, f, g, replace(split(p.reason, ' (')[0], 'batch', 'summaries') AS change
RETURN 'proposal-' + p.candidate_id AS id, s.run_epoch AS run_epoch, p.solved_ms AS at_ms, '' AS root_id,
       'Proposed change: ' + change AS title,
       'Drasi’s plan for the current facts: every workload on a healthy deployment its policy allows, within capacity, with the fewest moves. Approve to apply it through the gateway’s configuration API.' AS text,
       CASE WHEN f.problems = '' THEN 'Capacity changed' ELSE f.problems END AS why,
       toString(toInteger(round(p.peak_utilisation * 100))) + '%' AS peak,
       g.policy AS policy,
       'healthy: ' + f.healthy + ' · needed: ' + g.needed AS capacity,
       'revision ' + toString(p.revision) + ' · plan ' + left(p.candidate_id, 8) AS based_on,
       change, p.candidate_id AS candidate_id, p.basis AS basis, p.revision AS revision
