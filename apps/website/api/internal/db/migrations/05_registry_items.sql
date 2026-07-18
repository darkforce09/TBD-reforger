-- 05_registry_items.sql
-- Runs AFTER GORM AutoMigrate. AutoMigrate creates the registry_items table from
-- the model; this asserts the composite unique index (the ON CONFLICT target used
-- by the dev seed and cmd/import-registry-items) plus a browse-order index. All
-- idempotent.

-- Unique per modpack+ResourceName (upsert key). Matches the GORM uniqueIndex tag,
-- so this is a no-op if AutoMigrate already created it.
CREATE UNIQUE INDEX IF NOT EXISTS idx_registry_items_modpack_resource
    ON registry_items (modpack_id, resource_name);

-- Catalog list/browse order within a modpack.
CREATE INDEX IF NOT EXISTS idx_registry_items_modpack_sort
    ON registry_items (modpack_id, sort_order);
