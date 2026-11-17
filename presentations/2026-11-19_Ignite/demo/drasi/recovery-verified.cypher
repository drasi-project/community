MATCH (r:RecoveryResult)
WHERE r.state = 'confirmed'
RETURN r.approval AS approval, r.last_success_ms AS verified_ms
