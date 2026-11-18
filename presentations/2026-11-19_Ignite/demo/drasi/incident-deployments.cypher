MATCH (d:ignite_deployments)-[:DEPLOYMENT_REPORT]->(r:ignite_reports),
      (d)-[:DEPLOYMENT_SCENARIO]->(s:ignite_scenario)
WHERE d.run_epoch = s.run_epoch AND r.run_epoch = s.run_epoch
WITH s.run_epoch AS run_epoch,
     collect(CASE WHEN drasi.trueLater(true, r.expires_ms) THEN d.id + ' is not reporting'
                  WHEN r.status = 'failed' THEN d.id + ' is down'
                  WHEN r.status = 'throttled' THEN d.id + ' is throttled'
                  ELSE '' END) AS problem_list,
     collect(CASE WHEN r.status = 'ok' AND NOT drasi.trueLater(true, r.expires_ms)
                  THEN d.id + ' ' + toString(d.capacity_tpm / 1000) + 'k' ELSE '' END) AS healthy_list
WITH run_epoch, [x IN problem_list WHERE x <> ''] AS problems, [x IN healthy_list WHERE x <> ''] AS healthy
WITH run_epoch, problems, healthy,
     CASE WHEN size(problems) = 0 THEN '' ELSE drasi.listMin(problems) END AS first_problem,
     CASE WHEN size(problems) = 0 THEN '' ELSE drasi.listMax(problems) END AS last_problem,
     CASE WHEN size(healthy) = 0 THEN '' ELSE drasi.listMin(healthy) END AS first_healthy,
     CASE WHEN size(healthy) = 0 THEN '' ELSE drasi.listMax(healthy) END AS last_healthy
RETURN run_epoch,
       first_problem
         + reduce(m = '', x IN problems | CASE WHEN x <> first_problem AND x <> last_problem THEN ', ' + x ELSE m END)
         + CASE WHEN last_problem <> first_problem THEN ', ' + last_problem ELSE '' END AS problems,
       CASE WHEN size(healthy) = 0 THEN 'none' ELSE first_healthy
         + reduce(m = '', x IN healthy | CASE WHEN x <> first_healthy AND x <> last_healthy THEN ', ' + x ELSE m END)
         + CASE WHEN last_healthy <> first_healthy THEN ', ' + last_healthy ELSE '' END END AS healthy
