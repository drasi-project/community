MATCH (p:PlanStatus)
WITH p,
     CASE p.status WHEN 'optimal' THEN 'feasible' WHEN 'infeasible' THEN 'infeasible' ELSE 'unknown' END AS state,
     coalesce(p.summaries.moved, []) AS moved,
     toString(toInteger(round(coalesce(p.objectives.peak, 0.0) * 100))) AS percent
WITH p, state, percent, size(moved) AS n,
     [m IN moved | m.subject + ' to ' + m.target] AS parts
WITH p, state, percent, n, parts, parts[0..size(parts) - 1] AS init
RETURN 'plan' AS id, state,
       CASE WHEN state = 'feasible' THEN p.solutionSignature ELSE '' END AS candidate_id,
       p.signature AS basis,
       p.provenance.revision AS revision,
       CASE WHEN state = 'feasible'
            THEN '{' + reduce(s = '', a IN p.summaries.assignments |
                   s + CASE WHEN s = '' THEN '' ELSE ',' END + '"' + a.subject + '":"' + a.target + '"') + '}'
            ELSE '{}' END AS assignments,
       toInteger(coalesce(p.objectives.moves, 0)) AS moves,
       coalesce(p.objectives.peak, 0.0) AS peak_utilisation,
       CASE WHEN state = 'unknown' AND p.precondition = 'health-known'
              THEN 'Health of ' + (split(split(p.detail, ': [')[1], ']')[0]) + ' is unknown'
            WHEN state = 'unknown' AND p.precondition = 'policy-known'
              THEN 'Policy for ' + replace((split(split(p.detail, ': [')[1], ']')[0]), ', ', ' on ') + ' is unknown'
            WHEN state = 'unknown' THEN coalesce(p.detail, 'The planner has not answered')
            WHEN state = 'infeasible' AND p.detail STARTS WITH 'constraint assign-once['
              THEN 'No allowed, healthy destination with room for ' + (split(split(p.detail, '[')[1], ']')[0])
            WHEN state = 'infeasible' THEN 'Allowed, healthy destinations cannot take everything together'
            WHEN n = 0
              THEN 'Everything already runs on an allowed, healthy destination with room (peak use ' + percent + '%)'
            ELSE 'Move ' + CASE WHEN n = 1 THEN head(parts)
                                ELSE reduce(s = '', x IN init | s + CASE WHEN s = '' THEN '' ELSE ', ' END + x)
                                     + ' and ' + last(parts) END
                 + ' (' + toString(n) + CASE WHEN n = 1 THEN ' move' ELSE ' moves' END
                 + ', peak use ' + percent + '%)'
       END AS reason,
       p.provenance.run_epoch AS run_epoch,
       drasi.changeDateTime(p).epochMillis AS solved_ms
