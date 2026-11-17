\set ON_ERROR_STOP on
CREATE TABLE IF NOT EXISTS ignite_scenario (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    run_epoch text NOT NULL
);
CREATE TABLE IF NOT EXISTS ignite_endpoints (
    id text PRIMARY KEY,
    region text NOT NULL,
    model text NOT NULL,
    budget_tpm bigint NOT NULL CHECK (budget_tpm >= 0),
    reserved_tpm bigint NOT NULL CHECK (reserved_tpm >= 0),
    available boolean NOT NULL,
    revision bigint NOT NULL CHECK (revision > 0),
    run_epoch text NOT NULL
);
ALTER TABLE ignite_endpoints REPLICA IDENTITY FULL;
-- :publication and :slot come from ports.env (ops/native-runtime.py passes them to psql).
SELECT format('CREATE PUBLICATION %I FOR TABLE ignite_endpoints WITH (publish = ''insert, update, delete'')', :'publication')
WHERE NOT EXISTS (SELECT FROM pg_publication WHERE pubname = :'publication') \gexec
SELECT pg_create_logical_replication_slot(:'slot', 'pgoutput')
WHERE NOT EXISTS (SELECT FROM pg_replication_slots WHERE slot_name = :'slot');
