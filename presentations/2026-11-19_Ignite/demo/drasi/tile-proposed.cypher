MATCH (a:AssignmentResult)
RETURN a.item AS workload, a.bin AS deployment, a.demand_tpm AS demand_tpm
