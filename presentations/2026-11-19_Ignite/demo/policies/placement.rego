package ignite
import rego.v1

default allow := false
allow if {
    input.endpoint.id in input.policy.destinations[input.workload.id]
    input.workload.model == input.endpoint.model
    input.workload.region == "any"
}
allow if {
    input.endpoint.id in input.policy.destinations[input.workload.id]
    input.workload.model == input.endpoint.model
    input.workload.region == input.endpoint.region
}

reasons contains "destination excluded by current policy" if {
    not input.endpoint.id in input.policy.destinations[input.workload.id]
}
reasons contains "model incompatible" if {
    input.workload.model != input.endpoint.model
}
reasons contains "immutable restricted classification requires matching region" if {
    input.workload.region != "any"
    input.workload.region != input.endpoint.region
}
reason := "Rego permits classification, destination and model" if allow
reason := concat("; ", sort(reasons)) if not allow
assessment := {"allowed": allow, "reason": reason}
