MATCH (m:ignite_model)
RETURN m.id AS id, m.run_epoch AS run_epoch, m.revision AS revision, m.payload AS payload
