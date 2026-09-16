# TASK: Phase 3A — pull editing into the engine

Create `apps/website/map-engine/src/editing/` and move the editor's engine logic into it.
Everything below is measured fact, already verified. Do NOT re-derive it; DO verify anything
you are about to depend on.

## A. What moves into `map-engine/src/editing/`

    history/    <- frontend/src/editor/state/{history.rs (862), doc_host.rs (170)}
                   The undo drive over data/store/crdt/undo_groups, plus `after_local_edit`
                   (rebind + persist + one undo step). history.rs ALSO installs a window-level
                   Ctrl+Z/Y keydown -- that keydown is DOM and STAYS in the frontend (3B puts
                   it in input/). Only the undo drive crosses.
    commands/   <- frontend/src/editor/state/commands_hotkeys.rs LINES 23-551 ONLY.
                   Those are pure: compiled_export_text, compile_diagnostics_summary,
                   row_meta_missing_message, export_gesture_is_duplicate,
                   apply_row_metadata_to_export, live_doc_title, format_merge_report,
                   SelectedEntity, resolve_selected_entities, prefab_leaf,
                   entity_display_name, count_noun, format_grid_ref, grid_position_text,
                   classnames_text, selection_summary_text, duplicate_slot_id_report.
                   `mod imp` (lines 552-1653, cfg wasm32) is clipboard/download/toast/save/
                   export -- a BROWSER surface. It stays in the frontend; 3B moves it to shell/.
    batch.rs    <- frontend/src/editor/state/operations/batch.rs (75) -- `with_batch` undo grouping.
                   It contains one DOM call; strip it (the confirm belongs frontend-side).
    host.rs     <- the doc-handle half of frontend/src/editor/state/operations/context/ (5 files,
                   567 LOC). OPS_CTX holds BOTH a live DocHandle and leptos RwSignals. Split it:
                   the handle + borrow chain cross, the signals and every `open_*`/`close_*`
                   signal writer stay frontend.
    picking.rs  <- frontend/src/editor/state/picking.rs (124) AND its sibling test file
                   frontend/src/editor/state/picking/tests/selection.rs (200).
                   picking.rs is wired by `#[cfg(test)] #[path = "picking/tests/selection.rs"] mod tests;`
                   at picking.rs:120-123. It is a pure camera-unproject adapter over
                   spatial::indexing::picking + data::store::selection. Zero DOM. Moves whole.
                   Delete the leftover husk dirs (README.md files) afterwards.
    persist/    <- the SERIALIZE half of frontend/src/editor/state/{persist.rs (1719),
                   hydrate.rs (1159)}. persist.rs drives IndexedDB via the `idb` crate and
                   hydrate.rs does an authed HTTP GET -- BOTH of those transports STAY frontend.
                   Only the serialize/deserialize/merge-policy halves cross.
    tools/      <- frontend/src/editor/tools/{los_tool.rs (2079), ruler_tool.rs (1497),
                   select_tool.rs (920), viewshed_scheduler.rs (455), place_helpers.rs (80),
                   los_world.rs (526)} plus los_world's sibling test file los_world_tests.rs (455)
                   and tools/tests/place_helpers_tests.rs (353).
    belt/       <- the engine-side half of frontend/src/editor/canvas/render_sync.rs.
                   3B owns the actual split of that file. For 3A, leave render_sync.rs alone.

## B. What merges into `data/store/operations/` (NOT into editing/)

`frontend/src/editor/state/operations/` is 29 files / 4367 LOC over a 98-LOC facade
(`state/operations.rs`, which carries a 112-name `pub use entity::{...}`). It has ZERO inline
tests. Audited verdicts:

  13 THIN ADAPTERS -- delete them once host.rs/batch.rs/history land engine-side:
     attrs, compositions, reassign, slot_ids, transform,
     entity/{mod, comments, connections, markers, roster, selection, vehicles, zones}
     Shape: with_batch("label", || OPS_CTX.with(|c| ...borrow chain...
            website_map_engine::data::store::operations::<mod>::<same fn>(core, args)))
            then `if changed { mission_history::after_local_edit(); }`

  3 SECOND IMPL, but ADDITIVE -- they add host/session state the engine does not model.
    Move that state into data/store/operations/, do not discard it:
     cargo.rs (202)              CARGO_DEFAULTS / LOADOUT_BUFFER / APPLY_SEED thread-locals
     tactical_graphics.rs (299)  the TG_STATE draw machine (selected / draft / VertexDrag)
     entity/placement.rs (194)   crew-toggle + active-layer + cargo-seeding placement policy

  7 A-ONLY REAL IMPLEMENTATIONS, zero DOM -- these become NEW files in data/store/operations/:
     entity/layers.rs (264), entity/zone_draw.rs (264), entity/arming.rs (151),
     entity/triggers.rs (120), entity/layer_drag.rs (108), entity/refile.rs (85),
     entity/selection_index.rs (34)

  6 UNIQUE host files -- batch.rs and context/* -- handled in section A above.

`transform.rs` passes a `confirm_bulk` callback (a `window.confirm`) INTO the engine fn. Keep
that inversion: the engine keeps taking a confirm closure, the frontend keeps supplying the DOM one.

`apps/website/map-engine/src/data/store/mod.rs` pins its public surface with
`#[cfg(test)] #[path = "tests/reexports.rs"] mod reexport_pins;` -- update that pin.

## C. The one true dedupe

The program document says `tools/los_world.rs` + `los_world_wasm.rs` "duplicate
spatial/los/world/. Collapse into the engine copy." **That is wrong and following it literally
deletes ~860 LOC of live logic.** los_world.rs contains zero raycasting/BVH/TLAS/DDA/residency;
it holds verdict presentation (combine, format_combined, styling_verdict) and the `ObjectPass`
progressive raster wash over Viewshed, which the ENGINE does not have -- the engine cites
`los_world::ObjectPass` in comments at spatial/los/terrain/scheduler.rs:16,52 and
spatial/los/interior/wash.rs:309 as the pattern it copied.

The ACTUAL duplication is exactly one function, byte-identical:
    frontend .../tools/los_world.rs:200      pub fn map_to_engine(x, y_north, elev) -> [f64;3]
    map-engine .../spatial/los/world/coverage_1.rs:22   the same function
Delete the frontend copy, import the engine's (already re-exported at world/mod.rs:60 and
world/trace/mod.rs:34). Move everything else intact.

`los_world_wasm.rs` (340) is the injected-closure seam onto `streaming::host::with_occluder`.
It is wasm-only glue -- it stays in the frontend.

## D. DOM must not cross. Measured coupling:

    los_tool.rs          3 RwSignal + 3 .get()   -- strip to fn params
    ruler_tool.rs        3 RwSignal + 3 .get()   -- same
    place_helpers.rs     ZERO  (already a 20-line pub-use facade over the engine)
    los_world.rs         ZERO
    select_tool.rs       16 wasm_bindgen hits    -- the real work
    viewshed_scheduler.rs 7 hits, and it OWNS ITS OWN rAF CLOSURE. The rAF belongs to
                          graphics-engine's RafPump; the job FSM belongs in editing/tools/.

An `editing/` type may hold `armed | active | committed` and geometry. It may NOT hold a
PointerEvent, a leptos signal, or an element ref.

## E. Cargo / lib.rs wiring

`editing = ["store", "world"]` ALREADY EXISTS at map-engine/Cargo.toml:38. What does not exist:
  - `map-engine/src/editing/` itself
  - any `pub mod editing;` in map-engine/src/lib.rs (add it `#[cfg(feature = "editing")]`,
    matching the style of the other nine gated modules at lib.rs:14-62)
  - the frontend requesting it. Add `editing` to the SHARED dependency block,
    `apps/website/frontend/Cargo.toml:32` (currently `features = ["world", "io", "store"]`).
    Do not move it to the wasm32 block -- `store` is deliberately in the shared block so
    native `cargo test -p website-frontend` keeps compiling these paths.

## F. Gate rules 5 and 6 -- you write them

`xtask/src/gate_engine_layers.rs` (967 lines, tests in sibling gate_engine_layers_tests.rs)
implements rules 1, 2, 3a, 3b, 4, 7. Rules 5 and 6 are PROSE ONLY at lines 28-30. Write them,
following the existing rule style exactly (a `const` matcher, a self-probe, anti-vacuity file
counting, and `against_pin` if a pin list is needed).

  Rule 5: no `web_sys`, `leptos`, or `wasm_bindgen` under `apps/website/map-engine/src/editing`.
          **SCOPE IT TO editing/, NOT THE WHOLE CRATE.** map-engine carries 266 such hits across
          57 files (streaming/host, diagnostics/readback, doll/renderer, frame/*, ...) and a
          crate-wide rule 5 can never be green. The program's own acceptance line says
          `rg ... apps/website/map-engine/src/editing` -> empty. Hard zero, no allowlist.
  Rule 6: `apps/website/frontend/**` may not import `website_graphics_engine`. Its subject is
          already zero (4 hits exist, all prose in comments/READMEs, zero code imports) -- so
          match on import syntax, not the bare word, or those comments will trip it.

Both must appear in `cargo xtask verify engine-layers` output and be covered by new tests in
gate_engine_layers_tests.rs. Note `xtask/src/mk_ci_tests.rs:64,148` freeze the 13-step ci-local
list -- do not rename or reorder the `verify-engine-layers` step.

## G. Verification -- run ALL of these, paste output VERBATIM

    CARGO_TARGET_DIR=target-container cargo xtask verify engine-layers
    rg 'web_sys|leptos|wasm_bindgen' apps/website/map-engine/src/editing     # must be EMPTY
    CARGO_TARGET_DIR=target-container cargo test -p website-map-engine --all-features
    CARGO_TARGET_DIR=target-container cargo test -p website-frontend
    CARGO_TARGET_DIR=target-container cargo test -p xtask
    CARGO_TARGET_DIR=target-container cargo xtask mk ci-local-leptos

Known-good baselines on the commit you start from (do not regress these):
    verify engine-layers            ENGINE-LAYERS: PASS   (rules 1,2,3a,3b,4,7)
    cargo test -p website-frontend  ok. 1433 passed; 0 failed
Known-RED before you start, NOT yours to fix and NOT a regression if still red:
    cargo xtask verify file-length  exit 1, 9 unallowlisted SIZE-3 (3C fixes this)
    gate v-suite verify             21 of 25 routes fail (T-986, operator-deferred)

Law 7 applies to every file you create: production < 500 lines, tests < 1000, and tests go in
sibling files via `#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`. Several sources you are
moving are far over that (los_tool 2079, ruler_tool 1497, persist 1719, hydrate 1159,
commands_hotkeys 2663) -- split them as you land them so `editing/` is born compliant. The
frontend allowlist rows for those paths become dead; 3C deletes them, not you.
