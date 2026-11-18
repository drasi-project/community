\set ON_ERROR_STOP on
BEGIN;

-- Scenario: what the company has configured. Every change raises routing_config.revision by one.
CREATE TABLE IF NOT EXISTS ai_workloads (
    id text PRIMARY KEY,
    run_epoch text NOT NULL,
    data_class text NOT NULL CHECK (data_class IN ('restricted','public')),
    region_requirement text NOT NULL CHECK (region_requirement IN ('EU','US','any')),
    model text NOT NULL,
    demand_tpm bigint NOT NULL CHECK (demand_tpm >= 0)
);
CREATE TABLE IF NOT EXISTS model_deployments (
    id text PRIMARY KEY,
    run_epoch text NOT NULL,
    region text NOT NULL,
    model text NOT NULL,
    capacity_tpm bigint NOT NULL CHECK (capacity_tpm >= 0)
);
-- Written only by reset and the gateway configuration API (POST /routes/apply).
CREATE TABLE IF NOT EXISTS workload_routes (
    workload text PRIMARY KEY REFERENCES ai_workloads(id),
    run_epoch text NOT NULL,
    deployment text NOT NULL REFERENCES model_deployments(id),
    approval text,
    applied_ms bigint NOT NULL
);
-- Policy parameters (the talk's policy edit); the rules are the Rego plugin's configuration.
CREATE TABLE IF NOT EXISTS data_residency_policy (
    id text PRIMARY KEY,
    run_epoch text NOT NULL,
    workload text NOT NULL REFERENCES ai_workloads(id),
    deployment text NOT NULL REFERENCES model_deployments(id),
    UNIQUE (workload, deployment),
    CHECK (id = workload || '/' || deployment)
);
-- The change-approval tool's records; status is changed only by calls from Drasi's reactions.
CREATE TABLE IF NOT EXISTS change_approvals (
    id text PRIMARY KEY,
    run_epoch text NOT NULL,
    candidate_id text NOT NULL,
    basis text NOT NULL,
    revision bigint NOT NULL,
    -- declined: a person pressed Reject on the card (nothing else happens).
    status text NOT NULL CHECK (status IN ('pending','applied','rejected','verified','declined')),
    reason text,
    latest boolean NOT NULL,
    requested_ms bigint NOT NULL,
    decided_ms bigint,
    applied_revision bigint,
    verified_ms bigint
);
-- From an approval made on a Mattermost card (POST /mattermost/actions): who pressed it, the
-- card's post (Drasi replies to it with the outcome) and the change the card proposed.
ALTER TABLE change_approvals ADD COLUMN IF NOT EXISTS requested_by text;
ALTER TABLE change_approvals ADD COLUMN IF NOT EXISTS post_id text;
ALTER TABLE change_approvals ADD COLUMN IF NOT EXISTS change text;
ALTER TABLE change_approvals DROP CONSTRAINT IF EXISTS change_approvals_status_check;
ALTER TABLE change_approvals ADD CONSTRAINT change_approvals_status_check
    CHECK (status IN ('pending','applied','rejected','verified','declined'));
CREATE UNIQUE INDEX IF NOT EXISTS change_approvals_latest ON change_approvals(run_epoch) WHERE latest;
-- Gateway configuration API idempotency (x-drasi-delivery-id). Not published.
CREATE TABLE IF NOT EXISTS gateway_deliveries (
    id text PRIMARY KEY,
    received_ms bigint NOT NULL,
    result jsonb NOT NULL
);
-- Fault modes of the simulated deployments. Not published: Drasi sees health only through reports.
CREATE TABLE IF NOT EXISTS simulated_deployment_faults (
    deployment text PRIMARY KEY,
    mode text NOT NULL CHECK (mode IN ('ok','failed','throttled')),
    reporting boolean NOT NULL
);

-- The application's own records.
CREATE TABLE IF NOT EXISTS ai_requests (
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
ALTER TABLE ai_requests ADD COLUMN IF NOT EXISTS cancelled_ms bigint;
ALTER TABLE ai_requests ADD COLUMN IF NOT EXISTS root_span_id text NOT NULL DEFAULT '0000000000000001';
ALTER TABLE ai_requests ADD COLUMN IF NOT EXISTS completed_revision bigint;
CREATE INDEX IF NOT EXISTS ai_requests_pending ON ai_requests(run_epoch, created_ms) WHERE completed_ms IS NULL;
CREATE TABLE IF NOT EXISTS model_call_attempts (
    id text PRIMARY KEY,
    request_id text NOT NULL REFERENCES ai_requests(id),
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
CREATE INDEX IF NOT EXISTS model_call_attempts_request ON model_call_attempts(request_id, started_ms);
CREATE TABLE IF NOT EXISTS model_responses (
    id text PRIMARY KEY,
    request_id text NOT NULL REFERENCES ai_requests(id),
    run_epoch text NOT NULL,
    endpoint text NOT NULL,
    workload text NOT NULL,
    content text NOT NULL,
    response text NOT NULL,
    tokens bigint NOT NULL,
    received_ms bigint NOT NULL
);
ALTER TABLE model_responses ADD COLUMN IF NOT EXISTS model_revision bigint;
CREATE TABLE IF NOT EXISTS deployment_health_reports (
    id text PRIMARY KEY,
    run_epoch text NOT NULL,
    observed_ms bigint NOT NULL,
    expires_ms bigint NOT NULL,
    status text NOT NULL CHECK (status IN ('ok','failed','throttled','unknown')),
    sequence bigint NOT NULL
);

-- Rows must belong to the current run; the scenario revision rises by exactly one per change
-- and restarts at 1 with a new run; reports and tasks only move forward.
CREATE OR REPLACE FUNCTION enforce_monotonic() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_TABLE_NAME = 'routing_config' THEN
        IF TG_OP = 'UPDATE' AND NEW.run_epoch = OLD.run_epoch AND NEW.revision <> OLD.revision + 1 THEN
            RAISE EXCEPTION 'non-monotonic scenario revision';
        END IF;
        IF (TG_OP = 'INSERT' OR NEW.run_epoch <> OLD.run_epoch) AND NEW.revision <> 1 THEN
            RAISE EXCEPTION 'a new run starts at revision 1';
        END IF;
        RETURN NEW;
    END IF;
    IF NEW.run_epoch <> (SELECT run_epoch FROM routing_config WHERE singleton) THEN
        RAISE EXCEPTION 'stale run epoch';
    END IF;
    IF TG_OP = 'UPDATE' AND NEW.run_epoch = OLD.run_epoch THEN
        IF TG_TABLE_NAME = 'deployment_health_reports' THEN
            IF NEW.sequence <= OLD.sequence OR NEW.observed_ms <= OLD.observed_ms THEN
                RAISE EXCEPTION 'non-monotonic report';
            END IF;
        ELSIF TG_TABLE_NAME = 'ai_requests' THEN
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
DROP TRIGGER IF EXISTS monotonic ON routing_config;
CREATE TRIGGER monotonic BEFORE INSERT OR UPDATE ON routing_config FOR EACH ROW EXECUTE FUNCTION enforce_monotonic();
DROP TRIGGER IF EXISTS monotonic ON ai_requests;
CREATE TRIGGER monotonic BEFORE INSERT OR UPDATE ON ai_requests FOR EACH ROW EXECUTE FUNCTION enforce_monotonic();
DROP TRIGGER IF EXISTS monotonic ON deployment_health_reports;
CREATE TRIGGER monotonic BEFORE INSERT OR UPDATE ON deployment_health_reports FOR EACH ROW EXECUTE FUNCTION enforce_monotonic();
DROP TRIGGER IF EXISTS monotonic ON ai_workloads;
CREATE TRIGGER monotonic BEFORE INSERT OR UPDATE ON ai_workloads FOR EACH ROW EXECUTE FUNCTION enforce_monotonic();
DROP TRIGGER IF EXISTS monotonic ON model_deployments;
CREATE TRIGGER monotonic BEFORE INSERT OR UPDATE ON model_deployments FOR EACH ROW EXECUTE FUNCTION enforce_monotonic();
DROP TRIGGER IF EXISTS monotonic ON workload_routes;
CREATE TRIGGER monotonic BEFORE INSERT OR UPDATE ON workload_routes FOR EACH ROW EXECUTE FUNCTION enforce_monotonic();
DROP TRIGGER IF EXISTS monotonic ON data_residency_policy;
CREATE TRIGGER monotonic BEFORE INSERT OR UPDATE ON data_residency_policy FOR EACH ROW EXECUTE FUNCTION enforce_monotonic();
-- Approvals of an earlier run may still be closed (rejected) after a reset.
DROP TRIGGER IF EXISTS monotonic ON change_approvals;
CREATE TRIGGER monotonic BEFORE INSERT ON change_approvals FOR EACH ROW EXECUTE FUNCTION enforce_monotonic();

ALTER TABLE routing_config REPLICA IDENTITY FULL;
ALTER TABLE ai_workloads REPLICA IDENTITY FULL;
ALTER TABLE model_deployments REPLICA IDENTITY FULL;
ALTER TABLE workload_routes REPLICA IDENTITY FULL;
ALTER TABLE data_residency_policy REPLICA IDENTITY FULL;
ALTER TABLE change_approvals REPLICA IDENTITY FULL;
ALTER TABLE ai_requests REPLICA IDENTITY FULL;
ALTER TABLE model_call_attempts REPLICA IDENTITY FULL;
ALTER TABLE deployment_health_reports REPLICA IDENTITY FULL;
-- :publication comes from ports.env (ops/native-runtime.py passes it to psql).
-- simulated_deployment_faults, gateway_deliveries and model_responses are deliberately not published.
ALTER PUBLICATION :"publication" SET TABLE routing_config, ai_workloads, model_deployments,
    workload_routes, data_residency_policy, change_approvals, ai_requests, model_call_attempts,
    deployment_health_reports;
-- The native PostgreSQL source refuses publications that replicate TRUNCATE.
ALTER PUBLICATION :"publication" SET (publish = 'insert, update, delete');
COMMIT;
