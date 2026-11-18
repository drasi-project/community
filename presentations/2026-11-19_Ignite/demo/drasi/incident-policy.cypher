MATCH (d:PolicyResult)-[:POLICY_WORKLOAD]->(w:ignite_workloads),
      (w)-[:WORKLOAD_SCENARIO]->(s:ignite_scenario)
WHERE d.run_epoch = s.run_epoch AND w.run_epoch = s.run_epoch
WITH s.run_epoch AS run_epoch,
     collect(CASE WHEN w.id = 'support' AND d.state = 'allowed' THEN d.target ELSE '' END) AS support_list,
     collect(CASE WHEN w.id = 'batch' AND d.state = 'allowed' THEN d.target ELSE '' END) AS batch_list,
     collect(CASE WHEN w.id = 'support' THEN
               CASE WHEN w.data_class = 'restricted' AND w.region_requirement <> 'any'
                      THEN ' (restricted data, ' + w.region_requirement + ' only)'
                    WHEN w.data_class = 'restricted' THEN ' (restricted data)'
                    WHEN w.region_requirement <> 'any' THEN ' (' + w.region_requirement + ' only)'
                    ELSE '' END
             ELSE '' END) AS support_rules,
     collect(CASE WHEN w.id = 'batch' THEN
               CASE WHEN w.data_class = 'restricted' AND w.region_requirement <> 'any'
                      THEN ' (restricted data, ' + w.region_requirement + ' only)'
                    WHEN w.data_class = 'restricted' THEN ' (restricted data)'
                    WHEN w.region_requirement <> 'any' THEN ' (' + w.region_requirement + ' only)'
                    ELSE '' END
             ELSE '' END) AS batch_rules,
     collect(CASE WHEN w.id = 'support' THEN toString(w.demand_tpm / 1000) + 'k' ELSE '' END) AS support_demand,
     collect(CASE WHEN w.id = 'batch' THEN toString(w.demand_tpm / 1000) + 'k' ELSE '' END) AS batch_demand
WITH run_epoch,
     [x IN support_list WHERE x <> ''] AS support, [x IN batch_list WHERE x <> ''] AS batch,
     CASE WHEN size(support_rules) = 0 THEN '' ELSE drasi.listMax(support_rules) END AS support_rule,
     CASE WHEN size(batch_rules) = 0 THEN '' ELSE drasi.listMax(batch_rules) END AS batch_rule,
     CASE WHEN size(support_demand) = 0 THEN '' ELSE drasi.listMax(support_demand) END AS support_needs,
     CASE WHEN size(batch_demand) = 0 THEN '' ELSE drasi.listMax(batch_demand) END AS batch_needs
WITH run_epoch, support, batch, support_rule, batch_rule, support_needs, batch_needs,
     CASE WHEN size(support) = 0 THEN '' ELSE drasi.listMin(support) END AS s1,
     CASE WHEN size(support) = 0 THEN '' ELSE drasi.listMax(support) END AS s3,
     CASE WHEN size(batch) = 0 THEN '' ELSE drasi.listMin(batch) END AS b1,
     CASE WHEN size(batch) = 0 THEN '' ELSE drasi.listMax(batch) END AS b3
RETURN run_epoch,
       'support: ' + CASE WHEN size(support) = 0 THEN 'nowhere' ELSE s1
         + reduce(m = '', x IN support | CASE WHEN x <> s1 AND x <> s3 THEN ', ' + x ELSE m END)
         + CASE WHEN s3 <> s1 THEN ', ' + s3 ELSE '' END END + support_rule
       + ' · summaries: ' + CASE WHEN size(batch) = 0 THEN 'nowhere' ELSE b1
         + reduce(m = '', x IN batch | CASE WHEN x <> b1 AND x <> b3 THEN ', ' + x ELSE m END)
         + CASE WHEN b3 <> b1 THEN ', ' + b3 ELSE '' END END + batch_rule AS policy,
       'support ' + support_needs + ', summaries ' + batch_needs AS needed
