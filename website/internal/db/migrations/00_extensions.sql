-- 00_extensions.sql
-- Runs FIRST, before enums and AutoMigrate. pgcrypto provides gen_random_uuid()
-- used as the default for all UUID primary keys (built into core on PG13+, but
-- the extension guarantees availability on older servers).

CREATE EXTENSION IF NOT EXISTS pgcrypto;
