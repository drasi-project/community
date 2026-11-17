MATCH (a:Assignment)
RETURN a.item AS id, a.item AS item, a.bin AS bin, a.current AS current, a.moved AS moved,
       a.demand_tpm AS demand_tpm, a.candidate_id AS candidate_id, a.run_epoch AS run_epoch
