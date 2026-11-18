MATCH (d:PolicyDecision)
RETURN d.id AS id, d.subject AS subject, d.target AS target,
       CASE WHEN d.policy_status = 'decided' AND d.allowed = true THEN 'allowed'
            WHEN d.policy_status = 'decided' AND d.allowed = false THEN 'denied'
            ELSE 'unknown' END AS state,
       CASE WHEN d.policy_status = 'decided' AND d.allowed = true THEN true ELSE false END AS allowed,
       CASE WHEN d.policy_status = 'decided' AND (d.allowed = true OR d.allowed = false) THEN d.reason
            WHEN d.policy_status = 'error' THEN 'policy error: ' + d.policy_error
            WHEN d.policy_status = 'undefined' THEN 'the policy rule produced no decision'
            ELSE 'the policy rule returned no allowed flag' END AS reason,
       d.policy_hash AS policy_hash, d.policy_input_fingerprint AS basis, d.run_epoch AS run_epoch
