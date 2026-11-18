MATCH (a:change_approvals)-[:APPROVAL_PLAN]->(p:PlanResult)
WHERE a.status = 'pending' AND p.state <> 'unknown'
  AND NOT (p.state = 'feasible' AND a.candidate_id = p.candidate_id
           AND a.basis = p.basis AND a.revision = p.revision)
RETURN a.id AS approval,
       CASE WHEN p.state = 'infeasible' THEN 'Drasi has no valid plan now: ' + replace(p.reason, 'batch', 'summaries')
            WHEN a.basis <> p.basis THEN 'The facts changed since this plan was proposed'
            ELSE 'The plan changed since it was approved' END AS reason
