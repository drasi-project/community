MATCH (r:ResilienceResult)
RETURN CASE WHEN r.state = 'unknown' THEN -1
            WHEN r.unhealthy_bins > 0 THEN 2
            WHEN r.state = 'safe' THEN 1
            ELSE 0 END AS value,
       CASE WHEN r.state = 'at_risk' AND r.unhealthy_bins = 0 THEN r.first_failure ELSE '' END AS lost,
       CASE WHEN r.state <> 'at_risk' OR r.unhealthy_bins > 0 THEN ''
            WHEN r.reason CONTAINS 'support' THEN 'support'
            WHEN r.reason CONTAINS 'batch' THEN 'summaries'
            ELSE '' END AS workload,
       CASE WHEN r.state = 'unknown' THEN 'Unknown'
            WHEN r.unhealthy_bins > 0 THEN 'Already degraded'
            WHEN r.state = 'safe' THEN 'Survives losing any one deployment'
            WHEN r.baseline = 'infeasible' OR r.first_failure = '' THEN 'At risk: no valid plan even now'
            ELSE 'At risk: losing ' + r.first_failure + ' leaves '
                 + CASE WHEN r.reason CONTAINS 'support' THEN 'support'
                        WHEN r.reason CONTAINS 'batch' THEN 'summaries'
                        ELSE 'a workload' END + ' nowhere to run' END AS text
