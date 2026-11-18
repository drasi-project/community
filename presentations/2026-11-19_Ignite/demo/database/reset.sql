\set ON_ERROR_STOP on
BEGIN;
SELECT pg_advisory_xact_lock(190619);
UPDATE ai_requests SET cancelled_ms=(extract(epoch FROM clock_timestamp())*1000)::bigint,
    revision=revision+1
WHERE completed_ms IS NULL AND cancelled_ms IS NULL
  AND run_epoch=(SELECT run_epoch FROM routing_config WHERE singleton);
INSERT INTO routing_config(singleton, run_epoch, revision) VALUES (true, gen_random_uuid()::text, 1)
ON CONFLICT(singleton) DO UPDATE SET run_epoch = excluded.run_epoch, revision = 1;
DELETE FROM workload_routes;
DELETE FROM data_residency_policy;
DELETE FROM ai_workloads;
DELETE FROM model_deployments;
INSERT INTO ai_workloads(id, run_epoch, data_class, region_requirement, model, demand_tpm)
SELECT fixture.id, scenario.run_epoch, fixture.data_class, fixture.region_requirement, fixture.model, fixture.demand_tpm
FROM (VALUES
    ('support', 'restricted', 'EU', 'support-model', 60000::bigint),
    ('batch', 'public', 'any', 'support-model', 80000::bigint)
) AS fixture(id, data_class, region_requirement, model, demand_tpm)
CROSS JOIN routing_config scenario;
INSERT INTO model_deployments(id, run_epoch, region, model, capacity_tpm)
SELECT fixture.id, scenario.run_epoch, fixture.region, 'support-model', 100000
FROM (VALUES ('EU-A', 'EU'), ('EU-B', 'EU'), ('US-C', 'US')) AS fixture(id, region)
CROSS JOIN routing_config scenario;
INSERT INTO workload_routes(workload, run_epoch, deployment, approval, applied_ms)
SELECT fixture.workload, scenario.run_epoch, fixture.deployment, NULL,
    (extract(epoch FROM clock_timestamp())*1000)::bigint
FROM (VALUES ('support', 'EU-A'), ('batch', 'EU-B')) AS fixture(workload, deployment)
CROSS JOIN routing_config scenario;
-- Both workloads start Europe-only. Losing either EU deployment then leaves no plan (failover at
-- risk) until summaries are allowed on US-C as well (the desk's Summaries: Europe + US).
INSERT INTO data_residency_policy(id, run_epoch, workload, deployment)
SELECT fixture.workload || '/' || fixture.deployment, scenario.run_epoch, fixture.workload, fixture.deployment
FROM (VALUES ('support', 'EU-A'), ('support', 'EU-B'),
             ('batch', 'EU-A'), ('batch', 'EU-B')) AS fixture(workload, deployment)
CROSS JOIN routing_config scenario;
INSERT INTO deployment_health_reports
SELECT fixture.id, scenario.run_epoch, 0, 0, 'unknown', 0
FROM (VALUES ('EU-A'), ('EU-B'), ('US-C')) fixture(id) CROSS JOIN routing_config scenario
ON CONFLICT(id) DO UPDATE SET run_epoch=excluded.run_epoch, observed_ms=0, expires_ms=0,
    status='unknown', sequence=0;
DELETE FROM simulated_deployment_faults WHERE deployment NOT IN ('EU-A', 'EU-B', 'US-C');
INSERT INTO simulated_deployment_faults(deployment, mode, reporting)
VALUES ('EU-A', 'ok', true), ('EU-B', 'ok', true), ('US-C', 'ok', true)
ON CONFLICT(deployment) DO UPDATE SET mode='ok', reporting=true;
COMMIT;
