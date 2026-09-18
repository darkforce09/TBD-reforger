-- T-068.10.5: weapon variant collapse — variant_of points a factory attachment/camo
-- configuration at its base weapon (immediate parent). Nullable; only weapon-kind rows
-- carry it. Pickers exclude variants (partial index mirrors the abstract exclusion).
ALTER TABLE public.registry_items
    ADD COLUMN IF NOT EXISTS variant_of text;

CREATE INDEX IF NOT EXISTS idx_registry_items_modpack_variant
    ON public.registry_items USING btree (modpack_id, variant_of)
    WHERE variant_of IS NOT NULL;
