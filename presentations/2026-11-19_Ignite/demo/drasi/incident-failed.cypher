MATCH (d:model_deployments)-[:DEPLOYMENT_HEALTH]->(r:deployment_health_reports),
      (d)-[:DEPLOYMENT_CONFIG]->(s:routing_config),
      (rs:workload_routes)-[:ROUTE_CONFIG]->(s),
      (rb:workload_routes)-[:ROUTE_CONFIG]->(s)
WHERE d.run_epoch = s.run_epoch AND r.run_epoch = s.run_epoch
  AND rs.workload = 'support' AND rb.workload = 'batch'
WITH d, s, rs, rb,
     drasi.previousDistinctValue(CASE WHEN r.status = 'failed' THEN 'failed' ELSE toString(r.observed_ms) END) AS healthy_until,
     drasi.trueFor(r.status = 'failed', 3000) AS failed_3s
WHERE failed_3s AND healthy_until IS NOT NULL AND healthy_until <> 'failed'
  AND NOT drasi.trueLater(true, toInteger(healthy_until) + 60000)
WITH d, s, toInteger(healthy_until) AS at_ms,
     CASE WHEN rs.deployment = d.id AND rb.deployment = d.id THEN 'support and summaries'
          WHEN rs.deployment = d.id THEN 'support'
          WHEN rb.deployment = d.id THEN 'summaries'
          ELSE 'nothing' END AS runs
RETURN 'failed-' + d.id + '-' + toString(at_ms) AS id, s.run_epoch AS run_epoch, at_ms, '' AS root_id,
       'Deployment failed: ' + d.id AS title,
       d.id + ' has reported Failed for 3 seconds. ' +
       CASE runs WHEN 'support' THEN 'Support runs there, so its requests are failing.'
                 WHEN 'summaries' THEN 'Summaries run there, so their requests are failing.'
                 WHEN 'support and summaries' THEN 'Support and summaries run there, so their requests are failing.'
                 ELSE 'Nothing runs there now.' END AS text,
       d.region AS region, toString(d.capacity_tpm / 1000) + 'k tokens/min' AS capacity, runs
