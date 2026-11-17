MATCH (r:RecoveryResult)
RETURN CASE WHEN r.state = 'confirmed' THEN 1 WHEN r.state = 'unknown' THEN -1 ELSE 0 END AS value
