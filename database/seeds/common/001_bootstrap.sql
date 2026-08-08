-- Knowledgebase bootstrap seed
--
-- The Knowledgebase application ships with `seedOnBoot=false` and intentionally carries
-- no locale-aware initialization data yet: all runtime content is created through the
-- application API (spaces, documents, OKF profiles) rather than seeded rows. This file is
-- kept as an explicit no-op so the `standard` seed profile applies deterministically and
-- the manifest stays self-describing. Add real initialization data here only when a
-- product contract defines required bootstrap rows.
SELECT 1;
