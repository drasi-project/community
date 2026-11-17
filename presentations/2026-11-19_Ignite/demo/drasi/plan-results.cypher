MATCH (p:Plan)
RETURN 'plan' AS id, p.state AS state, p.candidate_id AS candidate_id, p.basis AS basis,
       p.revision AS revision, p.assignments AS assignments, p.moves AS moves,
       p.peak_utilisation AS peak_utilisation, p.reason AS reason, p.run_epoch AS run_epoch,
       p.solved_ms AS solved_ms
