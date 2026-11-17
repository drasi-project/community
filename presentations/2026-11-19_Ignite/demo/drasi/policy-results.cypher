MATCH (d:PolicyDecision)
RETURN d.id AS id, d.subject AS subject, d.target AS target, d.state AS state,
       d.allowed AS allowed, d.reason AS reason, d.policy_hash AS policy_hash,
       d.basis AS basis, d.run_epoch AS run_epoch
