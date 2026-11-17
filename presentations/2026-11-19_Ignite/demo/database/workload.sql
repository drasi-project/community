\set ON_ERROR_STOP on
BEGIN;
-- Superseded by the scenario tables below (plan/contracts.md section 3). Grafana alerts now
-- reach Drasi through its HTTP source; decisions, annotations and scores are Drasi's.
DROP TABLE IF EXISTS ignite_model, ignite_endpoints, ignite_commands, ignite_publications,
    ignite_audit, ignite_alerts;

-- Scenario: what the company has configured. Every change raises ignite_scenario.revision by one.
CREATE TABLE IF NOT EXISTS ignite_workloads (
    id text PRIMARY KEY,
    run_epoch text NOT NULL,
    data_class text NOT NULL CHECK (data_class IN ('restricted','public')),
    region_requirement text NOT NULL CHECK (region_requirement IN ('EU','US','any')),
    model text NOT NULL,
    demand_tpm bigint NOT NULL CHECK (demand_tpm >= 0)
);
CREATE TABLE IF NOT EXISTS ignite_deployments (
    id text PRIMARY KEY,
    run_epoch text NOT NULL,
    region text NOT NULL,
    model text NOT NULL,
    capacity_tpm bigint NOT NULL CHECK (capacity_tpm >= 0)
);
-- Written only by reset and the gateway configuration API (POST /routes/apply).
CREATE TABLE IF NOT EXISTS ignite_routes (
    workload text PRIMARY KEY REFERENCES ignite_workloads(id),
    run_epoch text NOT NULL,
    deployment text NOT NULL REFERENCES ignite_deployments(id),
    approval text,
    applied_ms bigint NOT NULL
);
-- Policy parameters (the talk's policy edit); the rules are the Rego plugin's configuration.
CREATE TABLE IF NOT EXISTS ignite_policy_destinations (
    id text PRIMARY KEY,
    run_epoch text NOT NULL,
    workload text NOT NULL REFERENCES ignite_workloads(id),
    deployment text NOT NULL REFERENCES ignite_deployments(id),
    UNIQUE (workload, deployment),
    CHECK (id = workload || '/' || deployment)
);
-- The change-approval tool's records; status is changed only by calls from Drasi's reactions.
CREATE TABLE IF NOT EXISTS ignite_approvals (
    id text PRIMARY KEY,
    run_epoch text NOT NULL,
    candidate_id text NOT NULL,
    basis text NOT NULL,
    revision bigint NOT NULL,
    status text NOT NULL CHECK (status IN ('pending','applied','rejected','verified')),
    reason text,
    latest boolean NOT NULL,
    requested_ms bigint NOT NULL,
    decided_ms bigint,
    applied_revision bigint,
    verified_ms bigint
);
CREATE UNIQUE INDEX IF NOT EXISTS ignite_approvals_latest ON ignite_approvals(run_epoch) WHERE latest;
-- Gateway configuration API idempotency (x-drasi-delivery-id). Not published.
CREATE TABLE IF NOT EXISTS ignite_deliveries (
    id text PRIMARY KEY,
    received_ms bigint NOT NULL,
    result jsonb NOT NULL
);
-- Fault modes of the simulated deployments. Not published: Drasi sees health only through reports.
CREATE TABLE IF NOT EXISTS ignite_simulation (
    deployment text PRIMARY KEY,
    mode text NOT NULL CHECK (mode IN ('ok','failed','throttled')),
    reporting boolean NOT NULL
);

-- The application's own records.
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
ALTER TABLE ignite_receipts ADD COLUMN IF NOT EXISTS model_revision bigint;
CREATE TABLE IF NOT EXISTS ignite_reports (
    id text PRIMARY KEY,
    run_epoch text NOT NULL,
    observed_ms bigint NOT NULL,
    expires_ms bigint NOT NULL,
    status text NOT NULL CHECK (status IN ('ok','failed','throttled','unknown')),
    sequence bigint NOT NULL
);
CREATE TABLE IF NOT EXISTS ignite_breaches (
    id text PRIMARY KEY REFERENCES ignite_tasks(id),
    run_epoch text NOT NULL,
    deadline_ms bigint NOT NULL,
    observed_ms bigint NOT NULL,
    trace_id text NOT NULL
);

-- Rows must belong to the current run; the scenario revision rises by exactly one per change
-- and restarts at 1 with a new run; reports and tasks only move forward.
CREATE OR REPLACE FUNCTION ignite_monotonic() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_TABLE_NAME = 'ignite_scenario' THEN
        IF TG_OP = 'UPDATE' AND NEW.run_epoch = OLD.run_epoch AND NEW.revision <> OLD.revision + 1 THEN
            RAISE EXCEPTION 'non-monotonic scenario revision';
        END IF;
        IF (TG_OP = 'INSERT' OR NEW.run_epoch <> OLD.run_epoch) AND NEW.revision <> 1 THEN
            RAISE EXCEPTION 'a new run starts at revision 1';
        END IF;
        RETURN NEW;
    END IF;
    IF NEW.run_epoch <> (SELECT run_epoch FROM ignite_scenario WHERE singleton) THEN
        RAISE EXCEPTION 'stale run epoch';
    END IF;
    IF TG_OP = 'UPDATE' AND NEW.run_epoch = OLD.run_epoch THEN
        IF TG_TABLE_NAME = 'ignite_reports' THEN
            IF NEW.sequence <= OLD.sequence OR NEW.observed_ms <= OLD.observed_ms THEN
                RAISE EXCEPTION 'non-monotonic report';
            END IF;
        ELSIF TG_TABLE_NAME = 'ignite_tasks' THEN
            IF NEW.revision <> OLD.revision + 1 THEN
                RAISE EXCEPTION 'non-monotonic revision';
            END IF;
            IF OLD.completed_ms IS NOT NULL OR OLD.cancelled_ms IS NOT NULL OR NEW.content <> OLD.content OR NEW.deadline_ms <> OLD.deadline_ms
                OR NEW.workload <> OLD.workload OR NEW.trace_id <> OLD.trace_id OR NEW.root_span_id <> OLD.root_span_id THEN
                RAISE EXCEPTION 'immutable task identity or completed outcome';
            END IF;
        END IF;
    END IF;
    RETURN NEW;
END $$;
DROP TRIGGER IF EXISTS monotonic ON ignite_scenario;
CREATE TRIGGER monotonic BEFORE INSERT OR UPDATE ON ignite_scenario FOR EACH ROW EXECUTE FUNCTION ignite_monotonic();
DROP TRIGGER IF EXISTS monotonic ON ignite_tasks;
CREATE TRIGGER monotonic BEFORE INSERT OR UPDATE ON ignite_tasks FOR EACH ROW EXECUTE FUNCTION ignite_monotonic();
DROP TRIGGER IF EXISTS monotonic ON ignite_reports;
CREATE TRIGGER monotonic BEFORE INSERT OR UPDATE ON ignite_reports FOR EACH ROW EXECUTE FUNCTION ignite_monotonic();
DROP TRIGGER IF EXISTS monotonic ON ignite_workloads;
CREATE TRIGGER monotonic BEFORE INSERT OR UPDATE ON ignite_workloads FOR EACH ROW EXECUTE FUNCTION ignite_monotonic();
DROP TRIGGER IF EXISTS monotonic ON ignite_deployments;
CREATE TRIGGER monotonic BEFORE INSERT OR UPDATE ON ignite_deployments FOR EACH ROW EXECUTE FUNCTION ignite_monotonic();
DROP TRIGGER IF EXISTS monotonic ON ignite_routes;
CREATE TRIGGER monotonic BEFORE INSERT OR UPDATE ON ignite_routes FOR EACH ROW EXECUTE FUNCTION ignite_monotonic();
DROP TRIGGER IF EXISTS monotonic ON ignite_policy_destinations;
CREATE TRIGGER monotonic BEFORE INSERT OR UPDATE ON ignite_policy_destinations FOR EACH ROW EXECUTE FUNCTION ignite_monotonic();
-- Approvals of an earlier run may still be closed (rejected) after a reset.
DROP TRIGGER IF EXISTS monotonic ON ignite_approvals;
CREATE TRIGGER monotonic BEFORE INSERT ON ignite_approvals FOR EACH ROW EXECUTE FUNCTION ignite_monotonic();

ALTER TABLE ignite_scenario REPLICA IDENTITY FULL;
ALTER TABLE ignite_workloads REPLICA IDENTITY FULL;
ALTER TABLE ignite_deployments REPLICA IDENTITY FULL;
ALTER TABLE ignite_routes REPLICA IDENTITY FULL;
ALTER TABLE ignite_policy_destinations REPLICA IDENTITY FULL;
ALTER TABLE ignite_approvals REPLICA IDENTITY FULL;
ALTER TABLE ignite_tasks REPLICA IDENTITY FULL;
ALTER TABLE ignite_attempts REPLICA IDENTITY FULL;
ALTER TABLE ignite_reports REPLICA IDENTITY FULL;
ALTER TABLE ignite_breaches REPLICA IDENTITY FULL;
-- :publication comes from ports.env (ops/native-runtime.py passes it to psql).
-- ignite_simulation, ignite_deliveries and ignite_receipts are deliberately not published.
ALTER PUBLICATION :"publication" SET TABLE ignite_scenario, ignite_workloads, ignite_deployments,
    ignite_routes, ignite_policy_destinations, ignite_approvals, ignite_tasks, ignite_attempts,
    ignite_reports, ignite_breaches;
-- The native PostgreSQL source refuses publications that replicate TRUNCATE.
ALTER PUBLICATION :"publication" SET (publish = 'insert, update, delete');
COMMIT;
