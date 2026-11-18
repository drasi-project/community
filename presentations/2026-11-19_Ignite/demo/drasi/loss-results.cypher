MATCH (l:LossScenario)
RETURN l.value AS id, l.value AS lost,
       CASE WHEN l.feasible = true THEN 'feasible' WHEN l.feasible = false THEN 'infeasible' ELSE 'unknown' END AS state,
       l.detail AS reason, l.signature AS basis, l.provenance.run_epoch AS run_epoch
