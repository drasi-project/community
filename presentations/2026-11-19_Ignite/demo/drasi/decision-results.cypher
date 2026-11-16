MATCH (d:Decision)
RETURN d.id AS id, d.run_epoch AS run_epoch, d.revision AS revision,
       d.policy_revision AS policy_revision, d.basis AS basis, d.state AS state,
       d.reason AS reason, d.candidate_id AS candidate_id, d.valid AS valid, d.payload AS payload
