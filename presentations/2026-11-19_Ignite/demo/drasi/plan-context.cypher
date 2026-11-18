MATCH (s:ignite_scenario)
RETURN s.run_epoch AS run_epoch, s.revision AS revision
