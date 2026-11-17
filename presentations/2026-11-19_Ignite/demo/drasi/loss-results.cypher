MATCH (l:LossScenario)
RETURN l.lost AS id, l.lost AS lost, l.state AS state, l.assignments AS assignments,
       l.reason AS reason, l.basis AS basis, l.run_epoch AS run_epoch
