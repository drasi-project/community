MATCH (d:PolicyResult)
RETURN d.subject AS workload, d.target AS deployment,
       CASE WHEN d.state = 'allowed' THEN 1 WHEN d.state = 'denied' THEN 0 ELSE -1 END AS value
