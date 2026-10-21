-- sdkwork:migration
-- id: 202610210001_timestamptz_normalization
-- engine: postgres
-- module: knowledgebase
-- purpose: Normalize every Knowledgebase business instant to TIMESTAMPTZ.
--   The schema previously mixed TIMESTAMP (naive) and TIMESTAMPTZ in the same
--   tables (for example kb_outbox_event.created_at vs claimed_at). With the
--   session time zone pinned to UTC the values were correct, but the mixed types
--   remained a portability and review hazard. Existing values are interpreted as
--   UTC (the repository always wrote UTC) and converted without loss.
-- reversible: false
-- rollback: forward-fix (reversing would reinterpret absolute instants; keep
--   TIMESTAMPTZ and repair forward)
-- transactional: true
-- lock: table-rewrite
-- lock_timeout: 5s
-- statement_timeout: 120s
-- contract_version: 1.5.0
-- rewrite_expectation: ALTER COLUMN TYPE rebuilds each table once; acceptable for
--   the pre-launch dataset and required for absolute-time correctness
-- cancellation: cancel and rerun; the DO block only touches columns that are
--   still `timestamp without time zone`, so it is idempotent
-- replication_impact: table rewrites are WAL-logged; run in a maintenance window
-- recovery: rerun after lock contention; converted columns are skipped

BEGIN;

DO $$
DECLARE
    target record;
BEGIN
    FOR target IN
        SELECT table_name, column_name
        FROM information_schema.columns
        WHERE table_schema = current_schema()
          AND table_name LIKE 'kb\_%'
          AND data_type = 'timestamp without time zone'
        ORDER BY table_name, ordinal_position
    LOOP
        EXECUTE format(
            'ALTER TABLE %I ALTER COLUMN %I TYPE TIMESTAMPTZ USING %I AT TIME ZONE ''UTC''',
            target.table_name,
            target.column_name,
            target.column_name
        );
    END LOOP;
END $$;

COMMIT;
