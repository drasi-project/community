\set ON_ERROR_STOP on
-- Tables of an earlier design, superseded by the ones in workload.sql.
DROP TABLE IF EXISTS ignite_model, ignite_endpoints, ignite_commands, ignite_publications,
    ignite_audit, ignite_alerts, ignite_breaches;
-- Renames tables from this demo's earlier naming (ignite_*) in place, with their constraints and
-- indexes, so an existing database keeps its data. Safe to repeat; a new database has nothing to rename.
-- ops/native-runtime.py runs it before schema.sql (init) and workload.sql (migrate).
DO $$
DECLARE
    m record;
    c record;
BEGIN
    FOR m IN SELECT * FROM (VALUES
        ('ignite_scenario', 'routing_config'),
        ('ignite_workloads', 'ai_workloads'),
        ('ignite_deployments', 'model_deployments'),
        ('ignite_routes', 'workload_routes'),
        ('ignite_policy_destinations', 'data_residency_policy'),
        ('ignite_approvals', 'change_approvals'),
        ('ignite_tasks', 'ai_requests'),
        ('ignite_attempts', 'model_call_attempts'),
        ('ignite_receipts', 'model_responses'),
        ('ignite_reports', 'deployment_health_reports'),
        ('ignite_deliveries', 'gateway_deliveries'),
        ('ignite_simulation', 'simulated_deployment_faults')
    ) AS v(old, new) LOOP
        IF to_regclass('public.' || m.old) IS NOT NULL AND to_regclass('public.' || m.new) IS NULL THEN
            EXECUTE format('ALTER TABLE %I RENAME TO %I', m.old, m.new);
        END IF;
        CONTINUE WHEN to_regclass('public.' || m.new) IS NULL;
        FOR c IN SELECT conname FROM pg_constraint
                 WHERE conrelid = to_regclass('public.' || m.new) AND conname LIKE m.old || '\_%' LOOP
            EXECUTE format('ALTER TABLE %I RENAME CONSTRAINT %I TO %I',
                           m.new, c.conname, m.new || substr(c.conname, length(m.old) + 1));
        END LOOP;
        FOR c IN SELECT indexname FROM pg_indexes
                 WHERE schemaname = 'public' AND tablename = m.new AND indexname LIKE m.old || '\_%' LOOP
            EXECUTE format('ALTER INDEX %I RENAME TO %I', c.indexname, m.new || substr(c.indexname, length(m.old) + 1));
        END LOOP;
    END LOOP;
    ALTER INDEX IF EXISTS model_call_attempts_task RENAME TO model_call_attempts_request;
    IF EXISTS (SELECT FROM information_schema.columns
               WHERE table_schema = 'public' AND table_name = 'model_call_attempts' AND column_name = 'task_id') THEN
        ALTER TABLE model_call_attempts RENAME COLUMN task_id TO request_id;
    END IF;
    IF EXISTS (SELECT FROM information_schema.columns
               WHERE table_schema = 'public' AND table_name = 'model_responses' AND column_name = 'task_id') THEN
        ALTER TABLE model_responses RENAME COLUMN task_id TO request_id;
    END IF;
    -- Its triggers go with it; schema.sql and workload.sql recreate them on enforce_monotonic.
    DROP FUNCTION IF EXISTS ignite_monotonic() CASCADE;
END $$;
