# T-176 — Phase 0 inventory

Start: `main` @ `b90deac8` (T-175). Evidence: `.ai/artifacts/t176_operator_screens/01–03`.
All findings verified against code (file:line). This is the pre-edit trace the fixes act on.

## Forest render — two independent layers on the screens

| | (1) Landcover **wash** — the loose one | (2) TBDD **mass** — the tight one |
|---|---|---|
| Host | `world_assets/world_host.rs::push_landcover` (:304) | `world_assets/forest_mass.rs::push_composite` (:162) |
| Lane role / draw order | 1 `Landcover` / order 4 (low, over sat) | 5 `ForestFill` + 6 `ForestOutline` / order 12–13 |
| Data | `objects/forest-regions.json.gz` (Path B hulls) | per-chunk `objects/density/{cx}_{cy}.bin` (625) |
| Geometry | `compose_landcover_mesh` (vector_compose.rs:155) fill of connected-component outlines | `forest_mass_from_corners` (geometry/forest_mass.rs:197), marching-squares @ `DENSITY_ISO=2` |
| Cell size | 32 m (`forest.rs REGION_CELL_M`), merged mega-regions, ≥8-cell components | 32 m (`density.rs:9 DENSITY_CELL_M = 32`), per-cell |
| Colour / α | `[46,90,50,38]` → flat green α≈0.15 | `[34,120,60]` × `forest_fill_alpha` (0.45/0.35/0.12/0) |
| Visibility gate | `residency.forest_fill_effective()` (residency.rs:1578) | `class_visible("forestFill", zoom)` = `zoom<0` (lod_gates.rs:58) |
| Loads | one static mesh, instant | streams per-chunk (progressive) |

Both draw when zoomed out (`zoom<0`); the mass draws **over** the wash. Screens show the wash
dominating (island-wide, clearings filled). Zoomed in (`zoom≥0`) the mass gates off → tree glyphs +
satellite. **Operator's "darker green ≈ real trees" = satellite canopy + mass; "light wash" = the
loose landcover hull.** `forest-regions.json.gz` and the density grid derive from the **same** tree
data + same 32 m / ≥2-tree threshold — the wash just adds a connected-component fill + mega-merge, so
it inflates coverage and fills interior clearings.

## A1 — forest highlight stale until zoom-in→out

- `world_host.rs::run_viewport`: `push_landcover` (:169) runs **before** `set_viewport` (:172).
  `push_landcover` reads `forest_fill_effective()`, which depends on residency `deck_zoom` +
  `tree_glyph_buf` — both updated only by `set_viewport`. → a **one-pass lag**: the wash is evaluated
  against the previous pass's zoom/tree-pack state.
- `forest_fill_effective` (residency.rs:1578) = `class_visible("forestFill",z) || (toggle_trees &&
  (heatmap_trees || tree_glyph_buf.is_empty()))` — a **persistence clause** that keeps the wash shown
  at `z≥0` while trees haven't packed. The TBDD mass uses only `class_visible("forestFill")` → the two
  gates disagree during load.
- Settle loops break the instant both hosts idle (`mod.rs:306`, boot drain), so the corrective pass
  that would hide a stranded wash after a zoom-in can be denied → wash stranded visible until a later
  camera change ("zoom all the way in then out resets it").
- The TBDD mass renders correctly on a **stable** cold open (default zoom −2.0 → fill on; final
  `push_composite` at forest_mass.rs:109 reflects the fully-resolved chunk set). The stale symptom is
  the **wash**, not the mass.

## A2 — wash too loose / fills clearings

- Loose because the wash = 32 m connected-component filled outline + mega-merge (`tools/tbd-tools/
  src/forest.rs` `REGION_CELL_M=32`, `MIN_COMPONENT_CELLS=8`, `DOMINANT_SHARE=0.66`, `trace_rings`).
- Tightness is capped by 32 m. The density-accurate mass is already the tighter representation but is
  also baked at `DENSITY_CELL_M=32` (`tools/tbd-tools/src/density.rs:9`).
- Density is a **corner-count** grid: each corner counts trees in a ±16 m window; `iso=2`
  marching-squares fills cells with ≥2 trees (`geometry/forest_mass.rs`). Derived from tree instance
  positions (`build.rs:445-495`) — re-derivable **without Workbench** from committed data
  (`objects/chunks/*.json.gz` instances `[prefabId,x,y,z,yaw]` + `objects/prefabs.json.gz` `kind`).
  Staging is gitignored/absent (`staged-meta.json` missing) → the existing `build_world_objects`
  staging path can't run; a new committed-chunk path is needed.
- **Operator decision:** re-bake to **8 m** now (32 m too coarse). 8 m ≈ 11 MB (16× compose), needs a
  decoupled ~12–16 m canopy kernel + retuned iso to avoid speckle (Everon tree spacing ≈ 11 m). 4 m
  declined (≈42 MB, 64×, fights B2).

## B1 — palette place ghost invisible during drag

- Event path is correct: palette leaf `pointerdown` → `editor_ops::begin_place` (pointer-drag, **not**
  HTML5 DnD; eden_chrome.rs:983); the map `pointermove` listener is on the **outer `container` div**
  (mission_editor.rs:1123-1126), an ancestor of both canvas and palette, and calls `set_place_preview`
  while `has_pending()` (mission_editor.rs:761-768). `atlas_ready` is set at engine mount.
- Root cause is render-side: `engine.rs::draw_batches` icon-atlas bind selection (`:958-962`) —
  ```
  let atlas_bg = match batch.role {
      LaneRole::SlotDrag => slot_drag_bind,
      LaneRole::Slots | LaneRole::Clusters => slot_base_bind,
      _ => glyph_atlas_bind,          // ← SlotPlacePreview lands here
  };
  let Some(atlas_bg) = atlas_bg else { continue };   // ← skipped if glyph atlas None
  ```
  `set_place_preview` (engine.rs:3871) uploads a **slot-atlas** ring to `LaneRole::SlotPlacePreview`,
  which is **absent** from the match → bound to the wrong (glyph) atlas / skipped when it's `None`
  (cold map) → invisible ghost. Drop works because it routes through `LaneRole::Slots` (correctly
  bound). `SlotDrag` (post-place drag preview) works for the same reason.

## B2 — zoom+pan stutter (zoom alone fine)

- `schedule_camera_settle` (mod.rs:321-357): `SETTLE_DEBOUNCE_MS=120`, `SETTLE_MAX_LATENCY_MS=250`;
  single timer (no stacking). Under a continuous input stream the 120 ms debounce never expires, so
  the 250 ms deadline **forces a settle ~4×/s for the whole gesture**.
- Every pan `pointermove` re-arms it (mission_editor.rs:779); every wheel tick mutates `zoom`
  (mission_editor.rs:628) and also re-arms (`:646`). So during zoom+pan each forced settle sees a
  changed zoom → the heavy synchronous zoom-band recompute runs:
  - `dem.sync` (mod.rs:289 → dem_vectors.rs:100-123) — **contour marching-squares rebuild** on interval
    change (`reduce_grid_2x` + `contour_segments` + upload); the heaviest main-thread item. Plus
    `push_sea` on α-band change.
  - `world.run_viewport` `drain`+`push_to_engine` (world_host.rs:182-183); `forest.push_composite`
    retint+upload on α-band change (forest_mass.rs:226-239).
- Zoom-alone avoids it: with no pan stream, the re-arm stops the instant the wheel pauses → the
  cadence goes quiet; each heavy settle is a one-off, imperceptible.
- Seam: `flush_viewport` (mod.rs:276-311) is where a gesture-active flag can skip `dem.sync` (and, if
  the bench needs, forest/world recompose) during an active pan, deferring the full recompute to the
  gesture-end settle (mission_editor.rs:953).

## Fix map (→ approach in the plan)
- **B1**: engine.rs:960 add `LaneRole::SlotPlacePreview` to the `slot_base_bind` arm.
- **A1**: world_host.rs — `push_landcover` after `set_viewport`; A2 removes the forest wash → symptom source gone.
- **A2**: re-bake density @ 8 m (committed-chunk tool + kernel) + retune `DENSITY_ISO` + drop forest-kind wash + measured α.
- **B2**: `gesture_active` flag on `MapHost`; `flush_viewport` defers `dem.sync` (escalate to forest/world if bench shows) during active pan.
