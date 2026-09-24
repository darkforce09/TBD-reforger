# T-157 — Plan

## Context
CreateMissionDialog (editor/library/create_dialog.rs) is a plain form with fields that duplicate editor settings.
The backlog asks for a visual terrain picker and a modset choice (presets from T-135).

## Approach
1. New `editor/library/map_picker.rs` (register in `library/mod.rs`): cards from terrain-registry.json entries,
   thumbnail from each manifest (fallback glyph), arrow-key selection; wasm coverage test.
2. `editor/library/create_dialog.rs`: replace the terrain dropdown with the picker, add the modset select, remove
   time/weather/max players; `mission_library.rs`: pass presets and keep Ctrl+N.
3. Document digest test: create for everon before/after → identical document.
4. Perturbation: picker skips one terrain → coverage red; restore, `touch`, green.

## Risks
- Thumbnails need an asset per terrain under packages/map-assets (not owned): use the satellite mip if present,
  else a glyph, and report the gap.

## Verification
- `cargo xtask mk ci-local-leptos` · `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-157`
