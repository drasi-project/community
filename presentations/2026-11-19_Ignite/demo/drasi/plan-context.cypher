MATCH (s:routing_config)
RETURN s.run_epoch AS run_epoch, s.revision AS revision
