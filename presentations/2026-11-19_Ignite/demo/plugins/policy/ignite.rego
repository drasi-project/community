package ignite

import rego.v1

# Placement policy for the Ignite graph, evaluated by ignite/rego-policy for
# every workload (input.subject) and deployment (input.target).
# input.parameters holds this workload's rows from ignite_policy_destinations:
# the deployments the operator currently allows it to use. Changing those rows
# is a data change; changing this module is a configuration change.

allowed_destination if {
	some row in input.parameters
	row.deployment == input.target.id
}

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
