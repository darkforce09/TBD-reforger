# T-135 — Plan

## Context
Modpacks exist globally (handlers/content/modpacks.rs, T-271 workshop fields) but no mission names one and export
never checks that the mods a mission's registry aliases need are in the chosen modset.

## Approach
1. New `apps/website/api/src/services/workshop_sync.rs` (register in `services/mod.rs`): resolve workshop ids via
   http_retry, cache into modpack_mods; unit tests with a stubbed client.
2. `handlers/content/modpacks.rs`: `PUT/DELETE /missions/{id}/modpack` attach/detach; list shows mission counts;
   export handler path calls `validate_coverage(mission_aliases, preset)` and 422s naming the alias.
3. `apps/website/frontend/src/pages/public/modpacks.rs`: preset editor + attached missions column.
4. Perturbation: drop one mod from the preset fixture → coverage test red; restore, `touch`, green.

## Risks
- Workshop API rate limits; the sync is on-demand with cache, never on page load.
- Migration for the mission→modpack column goes through the existing migrate path (report the file in changes).

## Verification
- `cargo test -p website-api modpacks` · `cargo xtask mk ci-local-leptos` · `cargo xtask platform wave gate --slice T-135`
