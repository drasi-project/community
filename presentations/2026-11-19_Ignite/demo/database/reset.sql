\set ON_ERROR_STOP on
BEGIN;
SELECT pg_advisory_xact_lock(190619);
UPDATE ignite_tasks SET cancelled_ms=(extract(epoch FROM clock_timestamp())*1000)::bigint,
    revision=revision+1
WHERE completed_ms IS NULL AND cancelled_ms IS NULL
  AND run_epoch=(SELECT run_epoch FROM ignite_scenario WHERE singleton);
INSERT INTO ignite_scenario(singleton, run_epoch) VALUES (true, gen_random_uuid()::text)
ON CONFLICT(singleton) DO UPDATE SET run_epoch = excluded.run_epoch;
DELETE FROM ignite_endpoints;
INSERT INTO ignite_endpoints
SELECT fixture.*, 1, scenario.run_epoch
FROM (VALUES
    ('EU-A', 'EU', 'support-model', 100000::bigint, 60000::bigint, true),
    ('EU-B', 'EU', 'support-model', 100000::bigint, 80000::bigint, true),
    ('US-C', 'US', 'support-model', 100000::bigint, 0::bigint, true)
) AS fixture(id, region, model, budget_tpm, reserved_tpm, available)
CROSS JOIN ignite_scenario scenario;
INSERT INTO ignite_reports
SELECT fixture.id, scenario.run_epoch, 0, 0, 'unknown', 0
FROM (VALUES ('EU-A'), ('EU-B'), ('US-C')) fixture(id) CROSS JOIN ignite_scenario scenario
ON CONFLICT(id) DO UPDATE SET run_epoch=excluded.run_epoch, observed_ms=0, expires_ms=0,
    status='unknown', sequence=0;
INSERT INTO ignite_model
SELECT 'active', run_epoch, 1, '{
  "policy": {"revision":1,"destinations":{"support":["EU-A","EU-B"],"batch":["EU-A","EU-B","US-C"]}},
  "endpoints": [
    {"id":"EU-A","region":"EU","model":"support-model","budget_tpm":100000,"mode":"ok","reporting":true},
    {"id":"EU-B","region":"EU","model":"support-model","budget_tpm":100000,"mode":"ok","reporting":true},
    {"id":"US-C","region":"US","model":"support-model","budget_tpm":100000,"mode":"ok","reporting":true}
  ],
  "workloads": [
    {"id":"support","region":"EU","model":"support-model","reservation_tpm":60000,"destination":"EU-A"},
    {"id":"batch","region":"any","model":"support-model","reservation_tpm":80000,"destination":"EU-B"}
  ]
}'::jsonb FROM ignite_scenario
ON CONFLICT(id) DO UPDATE SET run_epoch=excluded.run_epoch, revision=1, payload=excluded.payload;
COMMIT;
