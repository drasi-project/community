MATCH (p:PlanStatus)
WITH p, p.scenarios.loss AS loss, head(coalesce(p.scenarios.loss.failures, [])) AS first,
     CASE p.status WHEN 'optimal' THEN 'feasible' WHEN 'infeasible' THEN 'infeasible' ELSE 'unknown' END AS baseline
WITH p, loss, first, baseline,
     CASE WHEN baseline = 'unknown' OR loss IS NULL THEN 'unknown'
          WHEN first IS NULL AND baseline = 'feasible' THEN 'safe'
          WHEN first IS NOT NULL AND first.status <> 'infeasible' THEN 'unknown'
          ELSE 'at_risk' END AS state
RETURN 'resilience' AS id, state,
       CASE WHEN state = 'at_risk' AND first IS NOT NULL THEN first.value ELSE '' END AS first_failure,
       baseline,
       coalesce(p.setSizes.unhealthy, 0) AS unhealthy_bins,
       CASE WHEN baseline = 'unknown' THEN coalesce(p.detail, 'The planner has not answered')
            WHEN loss IS NULL THEN 'The loss analysis has not finished'
            WHEN state = 'unknown' THEN 'Losing ' + first.value + ' could not be checked: ' + first.detail
            WHEN state = 'safe' THEN 'Losing any one healthy destination still leaves a plan'
            WHEN baseline = 'infeasible' AND p.detail STARTS WITH 'constraint assign-once['
              THEN 'No plan even now: No allowed, healthy destination with room for '
                   + (split(split(p.detail, '[')[1], ']')[0])
            WHEN baseline = 'infeasible'
              THEN 'No plan even now: Allowed, healthy destinations cannot take everything together'
            WHEN first.detail STARTS WITH 'constraint assign-once['
              THEN 'Losing ' + first.value + ' leaves no plan: No allowed, healthy destination with room for '
                   + (split(split(first.detail, '[')[1], ']')[0])
            ELSE 'Losing ' + first.value + ' leaves no plan: Allowed, healthy destinations cannot take everything together'
       END AS reason,
       p.signature AS basis, p.provenance.revision AS revision, p.provenance.run_epoch AS run_epoch
