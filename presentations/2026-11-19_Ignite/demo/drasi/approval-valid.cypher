MATCH (a:ignite_approvals)-[:APPROVAL_PLAN]->(p:PlanResult)
WHERE a.status = 'pending' AND p.state = 'feasible' AND a.candidate_id = p.candidate_id
  AND a.basis = p.basis AND a.revision = p.revision
RETURN a.id AS approval, a.candidate_id AS candidate_id, a.basis AS basis,
       a.revision AS expected_revision, p.assignments AS assignments
