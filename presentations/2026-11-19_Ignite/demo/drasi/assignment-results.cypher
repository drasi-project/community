MATCH (a:Assignment)
RETURN a.subject AS id, a.subject AS item, a.target AS bin, a.current AS current,
       a.target <> a.current AS moved, a.demand_tpm AS demand_tpm, a.run_epoch AS run_epoch
