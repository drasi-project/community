// Deployment health from telemetry: the AI service's health probes, through the Collector and
// Drasi's OpenTelemetry source. One row per deployment and run, shaped like an deployment_health_reports
// row (id, run_epoch, status, observed_ms, expires_ms) so the health queries read either.
// The source puts the identity attributes (deployment, run_epoch) only in the Metric's id:
// metric:svc:ai-gateway:deployment_health_probe_status:deployment=EU-A,run_epoch=<uuid>.
// A deployment that stops reporting stops being exported; its row keeps its last reading and
// the health queries turn it Unknown once expires_ms passes, as they do for deployment_health_reports.
MATCH (svc:Service)-[:REPORTS]->(m:Metric {name: 'deployment_health_probe_status'})
WHERE svc.name = 'ai-gateway'
RETURN split(split(elementId(m), 'deployment=')[1], ',')[0] AS id,
       split(elementId(m), 'run_epoch=')[1] AS run_epoch,
       CASE toInteger(m.value) WHEN 0 THEN 'ok' WHEN 1 THEN 'throttled' WHEN 2 THEN 'failed'
            ELSE 'unknown' END AS status,
       datetime(m.observedAt).epochMillis AS observed_ms,
       datetime(m.observedAt).epochMillis + 8000 AS expires_ms
