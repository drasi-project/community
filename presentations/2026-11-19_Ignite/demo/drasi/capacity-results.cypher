MATCH (a:CapacityAssessment)
RETURN a.id AS id, a.region AS region, a.model AS model,
       a.budget_tpm AS budget_tpm, a.reserved_tpm AS reserved_tpm,
       a.spare_tpm AS spare_tpm, a.status AS status,
       a.input_revision AS input_revision, a.run_epoch AS run_epoch
