\set ON_ERROR_STOP on
BEGIN;
CREATE TABLE IF NOT EXISTS ignite_model (
    id text PRIMARY KEY CHECK (id = 'active'),
    run_epoch text NOT NULL,
    revision bigint NOT NULL CHECK (revision > 0),
    payload jsonb NOT NULL
);
CREATE TABLE IF NOT EXISTS ignite_tasks (
    id text PRIMARY KEY,
    run_epoch text NOT NULL,
    workload text NOT NULL CHECK (workload IN ('support','batch')),
    content text NOT NULL,
    trace_id text NOT NULL,
    root_span_id text NOT NULL,
    created_ms bigint NOT NULL,
    deadline_ms bigint NOT NULL,
    completed_ms bigint,
    cancelled_ms bigint,
    endpoint text,
    outcome text,
    revision bigint NOT NULL DEFAULT 1
);
ALTER TABLE ignite_tasks ADD COLUMN IF NOT EXISTS cancelled_ms bigint;
ALTER TABLE ignite_tasks ADD COLUMN IF NOT EXISTS root_span_id text NOT NULL DEFAULT '0000000000000001';
ALTER TABLE ignite_tasks ADD COLUMN IF NOT EXISTS completed_revision bigint;
CREATE INDEX IF NOT EXISTS ignite_tasks_pending ON ignite_tasks(run_epoch, created_ms) WHERE completed_ms IS NULL;
CREATE TABLE IF NOT EXISTS ignite_attempts (
    id text PRIMARY KEY,
    task_id text NOT NULL REFERENCES ignite_tasks(id),
    run_epoch text NOT NULL,
    workload text NOT NULL,
    endpoint text NOT NULL,
    model_revision bigint NOT NULL,
    started_ms bigint NOT NULL,
    finished_ms bigint,
    status integer,
    detail text,
    trace_id text NOT NULL,
    tokens bigint NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS ignite_attempts_task ON ignite_attempts(task_id, started_ms);
CREATE TABLE IF NOT EXISTS ignite_receipts (
    id text PRIMARY KEY,
    task_id text NOT NULL REFERENCES ignite_tasks(id),
    run_epoch text NOT NULL,
    endpoint text NOT NULL,
    workload text NOT NULL,
    content text NOT NULL,
    response text NOT NULL,
    tokens bigint NOT NULL,
    received_ms bigint NOT NULL
);
CREATE TABLE IF NOT EXISTS ignite_reports (
    id text PRIMARY KEY,
    run_epoch text NOT NULL,
    observed_ms bigint NOT NULL,
    expires_ms bigint NOT NULL,
    status text NOT NULL CHECK (status IN ('ok','failed','throttled','unknown')),
    sequence bigint NOT NULL
);
ALTER TABLE ignite_receipts ADD COLUMN IF NOT EXISTS model_revision bigint;
CREATE TABLE IF NOT EXISTS ignite_commands (
    id text PRIMARY KEY,
    run_epoch text NOT NULL,
    candidate_id text NOT NULL,
    basis text NOT NULL,
    input_revision bigint NOT NULL,
    policy_revision bigint NOT NULL,
    decision jsonb NOT NULL,
    status text NOT NULL CHECK(status IN ('committed','gateway_applied','recovery_confirmed','rejected')),
    committed_ms bigint NOT NULL,
    applied_ms bigint,
    applied_revision bigint,
    support_endpoint text NOT NULL,
    recovered_ms bigint,
    error text,
    UNIQUE(run_epoch,candidate_id)
);
CREATE TABLE IF NOT EXISTS ignite_audit (
    id bigserial PRIMARY KEY,
    command_id text,
    recorded_ms bigint NOT NULL,
    action text NOT NULL,
    detail jsonb NOT NULL
);
CREATE TABLE IF NOT EXISTS ignite_publications (
    id text PRIMARY KEY,
    target text NOT NULL CHECK(target IN ('grafana','langfuse')),
    payload jsonb NOT NULL,
    created_ms bigint NOT NULL,
    delivered_ms bigint,
    attempts bigint NOT NULL DEFAULT 0,
    next_attempt_ms bigint NOT NULL DEFAULT 0,
    last_error text
);
ALTER TABLE ignite_audit ADD COLUMN IF NOT EXISTS run_epoch text;
CREATE TABLE IF NOT EXISTS ignite_breaches (
    id text PRIMARY KEY REFERENCES ignite_tasks(id),
    run_epoch text NOT NULL,
    deadline_ms bigint NOT NULL,
    observed_ms bigint NOT NULL,
    trace_id text NOT NULL
);
CREATE TABLE IF NOT EXISTS ignite_alerts (
    id text PRIMARY KEY,
    status text NOT NULL,
    title text NOT NULL,
    message text NOT NULL,
    event_ms bigint NOT NULL,
    received_ms bigint NOT NULL,
    payload jsonb NOT NULL
);
CREATE OR REPLACE FUNCTION ignite_monotonic() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.run_epoch <> (SELECT run_epoch FROM ignite_scenario WHERE singleton) THEN
        RAISE EXCEPTION 'stale run epoch';
    END IF;
    IF TG_OP = 'UPDATE' AND NEW.run_epoch = OLD.run_epoch THEN
        IF TG_TABLE_NAME = 'ignite_reports' THEN
            IF NEW.sequence <= OLD.sequence OR NEW.observed_ms <= OLD.observed_ms THEN
                RAISE EXCEPTION 'non-monotonic report';
            END IF;
        ELSE
            IF NEW.revision <> OLD.revision + 1 THEN
                RAISE EXCEPTION 'non-monotonic revision';
            END IF;
            IF TG_TABLE_NAME = 'ignite_tasks' THEN
                IF OLD.completed_ms IS NOT NULL OR OLD.cancelled_ms IS NOT NULL OR NEW.content <> OLD.content OR NEW.deadline_ms <> OLD.deadline_ms
                    OR NEW.workload <> OLD.workload OR NEW.trace_id <> OLD.trace_id OR NEW.root_span_id <> OLD.root_span_id THEN
                    RAISE EXCEPTION 'immutable task identity or completed outcome';
                END IF;
            END IF;
        END IF;
    END IF;
    RETURN NEW;
END $$;
DROP TRIGGER IF EXISTS monotonic ON ignite_model;
CREATE TRIGGER monotonic BEFORE INSERT OR UPDATE ON ignite_model FOR EACH ROW EXECUTE FUNCTION ignite_monotonic();
DROP TRIGGER IF EXISTS monotonic ON ignite_tasks;
CREATE TRIGGER monotonic BEFORE INSERT OR UPDATE ON ignite_tasks FOR EACH ROW EXECUTE FUNCTION ignite_monotonic();
DROP TRIGGER IF EXISTS monotonic ON ignite_reports;
CREATE TRIGGER monotonic BEFORE INSERT OR UPDATE ON ignite_reports FOR EACH ROW EXECUTE FUNCTION ignite_monotonic();
ALTER TABLE ignite_model REPLICA IDENTITY FULL;
ALTER TABLE ignite_tasks REPLICA IDENTITY FULL;
ALTER TABLE ignite_attempts REPLICA IDENTITY FULL;
ALTER TABLE ignite_reports REPLICA IDENTITY FULL;
ALTER TABLE ignite_breaches REPLICA IDENTITY FULL;
ALTER TABLE ignite_alerts REPLICA IDENTITY FULL;
ALTER TABLE ignite_commands REPLICA IDENTITY FULL;
UPDATE ignite_model SET revision=revision+1,
    payload=payload || '{"policy":{"revision":1,"destinations":{"support":["EU-A","EU-B"],"batch":["EU-A","EU-B","US-C"]}}}'::jsonb
WHERE NOT payload ? 'policy';
-- :publication comes from ports.env (ops/native-runtime.py passes it to psql).
ALTER PUBLICATION :"publication" SET TABLE ignite_endpoints, ignite_model, ignite_tasks,
    ignite_attempts, ignite_reports, ignite_breaches, ignite_alerts, ignite_commands;
-- The native PostgreSQL source refuses publications that replicate TRUNCATE.
ALTER PUBLICATION :"publication" SET (publish = 'insert, update, delete');
COMMIT;
