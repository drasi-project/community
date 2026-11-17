\set ON_ERROR_STOP on
CREATE TABLE IF NOT EXISTS ignite_scenario (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    run_epoch text NOT NULL,
    revision bigint NOT NULL DEFAULT 1 CHECK (revision > 0)
);
ALTER TABLE ignite_scenario ADD COLUMN IF NOT EXISTS revision bigint NOT NULL DEFAULT 1 CHECK (revision > 0);
ALTER TABLE ignite_scenario REPLICA IDENTITY FULL;
-- :publication and :slot come from ports.env (ops/native-runtime.py passes them to psql).
-- workload.sql sets the publication's full table list.
SELECT format('CREATE PUBLICATION %I FOR TABLE ignite_scenario WITH (publish = ''insert, update, delete'')', :'publication')
WHERE NOT EXISTS (SELECT FROM pg_publication WHERE pubname = :'publication') \gexec
SELECT pg_create_logical_replication_slot(:'slot', 'pgoutput')
WHERE NOT EXISTS (SELECT FROM pg_replication_slots WHERE slot_name = :'slot');
