-- T-068.10.2: registry-items v3 — per-item classification metadata + mod provenance.
-- All columns nullable: v2 envelopes (no fields) import unchanged; the v3 exporter
-- fills them. kind stays untyped text (v3 kinds need no DDL).
ALTER TABLE public.registry_items
    ADD COLUMN IF NOT EXISTS abstract boolean,
    ADD COLUMN IF NOT EXISTS arsenal_type text,
    ADD COLUMN IF NOT EXISTS weight_kg double precision,
    ADD COLUMN IF NOT EXISTS volume_cm3 double precision,
    ADD COLUMN IF NOT EXISTS max_weight_kg double precision,
    ADD COLUMN IF NOT EXISTS max_volume_cm3 double precision,
    ADD COLUMN IF NOT EXISTS addon text;

-- Forge pickers filter per modpack by kind with abstracts excluded; mod-filter UI sorts by addon.
CREATE INDEX IF NOT EXISTS idx_registry_items_modpack_kind
    ON public.registry_items USING btree (modpack_id, kind)
    WHERE abstract IS NOT TRUE;
CREATE INDEX IF NOT EXISTS idx_registry_items_modpack_addon
    ON public.registry_items USING btree (modpack_id, addon);
