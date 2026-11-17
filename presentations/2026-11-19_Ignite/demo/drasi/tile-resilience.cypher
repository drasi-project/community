MATCH (r:ResilienceResult)
RETURN CASE WHEN r.state = 'unknown' THEN -1
            WHEN r.unhealthy_bins > 0 THEN 2
            WHEN r.state = 'safe' THEN 1
            ELSE 0 END AS value,
       CASE WHEN r.state = 'at_risk' AND r.unhealthy_bins = 0 THEN r.first_failure ELSE '' END AS lost
