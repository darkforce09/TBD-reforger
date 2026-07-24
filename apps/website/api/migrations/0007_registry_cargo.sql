-- T-068.15.1: cargo capacity export — grid columns + compat edge multiplicity.
--
-- registry_items: inventory UI grid (cells) derived by the Workbench scanner
-- (registry-items.schema.json cargo_grid_w/h; VOLUME_PER_CELL_CM3=50, width 4,
-- min height 3). Nullable: absent when the prefab has no readable capacity.
ALTER TABLE registry_items
    ADD COLUMN cargo_grid_w integer,
    ADD COLUMN cargo_grid_h integer;

-- registry_compat: the scanner emits one character_default_cargo edge per
-- InitialInventoryItems PrefabsToSpawn entry (duplicates = quantity), but the
-- (modpack, from, to, type) unique key collapsed them to one row. qty carries
-- the aggregated count; evidence joins the key (COALESCE keeps the importer's
-- NULL ≡ '' canonical form) so the same item in different storages
-- (TargetStorage=Pants/... vs Vest/...) stays distinct rows.
ALTER TABLE registry_compat
    ADD COLUMN qty integer NOT NULL DEFAULT 1;

DROP INDEX idx_registry_compat_edge;
CREATE UNIQUE INDEX idx_registry_compat_edge
    ON registry_compat (modpack_id, from_node, to_node, edge_type, COALESCE(evidence, ''));
