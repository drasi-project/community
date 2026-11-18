MATCH (p:PlanResult)-[:PLAN_CONFIG]->(s:routing_config)
WHERE p.run_epoch = s.run_epoch
WITH p, s,
     split(coalesce(drasi.previousDistinctValue(p.run_epoch + '|' + p.state + '|' + toString(p.moves) + '|'
       + p.candidate_id + '|' + p.assignments + '|' + toString(p.solved_ms) + '|' + p.reason), '') + '|||||||', '|') AS was
WITH p, s, was[0] AS was_run, was[1] AS was_state, was[2] AS was_moves, was[3] AS was_candidate, was[4] AS was_assignments, was[5] AS was_solved, was[6] AS was_reason
WHERE was_run = p.run_epoch AND was_state = 'feasible' AND was_moves <> '0' AND was_moves <> ''
  AND p.state <> 'unknown'
  AND p.solved_ms - toInteger(coalesce(nullIf(was_solved, ''), '0')) >= 5000
  AND NOT (p.state = 'feasible' AND p.moves = 0 AND p.assignments = was_assignments)
  AND drasi.trueLater(true, p.solved_ms + 2000)
  AND NOT drasi.trueLater(true, p.solved_ms + 60000)
WITH p, s, was_candidate, replace(split(was_reason, ' (')[0], 'batch', 'summaries') AS was_change,
     replace(split(p.reason, ' (')[0], 'batch', 'summaries') AS change
RETURN 'superseded-' + was_candidate AS id, s.run_epoch AS run_epoch, p.solved_ms AS at_ms, '' AS root_id,
       'Out of date: the facts changed' AS title,
       'The proposed change “' + was_change + '” no longer matches Drasi’s plan, so Drasi would reject an approval of it. '
         + CASE WHEN p.state = 'feasible' AND p.moves > 0 THEN 'Drasi’s new proposal follows.'
                WHEN p.state = 'feasible' THEN 'No change is needed now: ' + change + '.'
                ELSE 'No change fits policy and capacity now: ' + replace(p.reason, 'batch', 'summaries') + '.' END AS text,
       was_change AS was,
       CASE WHEN p.state = 'feasible' AND p.moves > 0 THEN change
            WHEN p.state = 'feasible' THEN 'No change needed'
            ELSE 'No valid plan' END AS now
