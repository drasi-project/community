package placement

import rego.v1

# Placement policy for the AI platform, evaluated by drasi.standard/policy
# once for every workload (input.subject) and deployment (input.target) pair.
# input.destinations lists the deployments the operator currently allows this
# workload to use (rows of data_residency_policy). Changing those rows
# is a data change; changing this module is a configuration change.

allowed_destination if input.target.id in input.destinations

region_permitted if input.subject.region_requirement == "any"

region_permitted if input.subject.region_requirement == input.target.region

default allow := false

allow if {
	allowed_destination
	region_permitted
	input.subject.model == input.target.model
}

reasons contains "destination excluded by current policy" if not allowed_destination

reasons contains "model incompatible" if input.subject.model != input.target.model

reasons contains "restricted data must stay in its region" if not region_permitted

reason := "Rego permits classification, destination and model" if allow

reason := concat("; ", sort(reasons)) if not allow

assessment := {"allowed": allow, "reason": reason}
