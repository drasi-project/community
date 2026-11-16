MATCH (e:ignite_endpoints)
RETURN e.id AS id, e.region AS region, e.model AS model,
       e.budget_tpm AS budget_tpm, e.reserved_tpm AS reserved_tpm,
       e.available AS available, e.revision AS revision, e.run_epoch AS run_epoch
