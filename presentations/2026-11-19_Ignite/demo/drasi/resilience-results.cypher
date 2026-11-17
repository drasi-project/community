MATCH (r:Resilience)
RETURN 'resilience' AS id, r.state AS state, r.first_failure AS first_failure, r.baseline AS baseline,
       r.unhealthy_bins AS unhealthy_bins, r.reason AS reason, r.basis AS basis,
       r.revision AS revision, r.run_epoch AS run_epoch
