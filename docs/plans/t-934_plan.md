# T-934 — Plan

## Context

`apps/website/audit.md` §3/§4 (2026-08-25) proposes the website reorg: flat 73-file frontend (120,240 LOC) into `core/shell/pages/editor`, decomposition of the three monoliths, gesture extraction via `EditorGestureContext`, and backend handler nesting. Operator locked scope to reorg-only, landing ASAP (in-flight conflicts accepted), executor claude-code. Verification against live code produced material corrections (test-code share, 439 `include_str!` guards, backend §3.4 rejected as written) — recorded in the program spec `docs/specs/website_reorg/plan.md` §1.

## Approach

Sixteen children, one commit each, direct to `main`:

- **A (.1–.6) mechanical moves** — batches: core+shell → pages/public → pages/operations+admin → editor library/tools/world → editor/panels → editor/state+arsenal+mission_editor. Per child: `git mv`, folder `mod.rs` reproducing wasm32 cfg gates, crate-wide path rewrite, `include_str!` repoints (self-guards only on rename; cross-file per spec §5 inventory), xtask pin sweep (`gate_t180.rs`, `gate_t439.rs`, `ai.rs`).
- **B (.7–.12) decomposition** — editor_ops `pub use` façade over `state/ops/*`; arsenal split keeping `class_r_scrub` in `arsenal/mod.rs`; mission_editor test evacuation (~7.9k LOC) via `#[cfg(test)] #[path]` mounts; pure helpers → `canvas/render_sync.rs`; overlays → `canvas/overlays.rs`; boot/RAF → `canvas/{boot,viewport}.rs`.
- **B2 (.13–.14) gestures** — `EditorGestureContext` bundling ~35 captured handles; pointer/wheel/dblclick/contextmenu closures → `canvas/gestures.rs`; hotkeys → `state/commands.rs`; final shell ~450–800 LOC.
- **C (.15–.16) backend + close-out** — handlers nested by domain at current file granularity (no new monoliths, `mod` handler → `mod_portal.rs`); close-out remaps stale `owns`, repacks waves, hands doc-link sync to Cursor.

## Risks

`include_str!` source guards break on rename/geometry change — native `cargo test` inside `ci-local-leptos` detects both; spec §5 carries the repoint inventory. Gesture extraction (.13) risks reactive-capture regressions — context-struct pattern, sequenced last, browser gates + manual per-gesture smoke. xtask path pins — per-commit grep sweep. wasm32 cfg fidelity — folder `mod.rs` gates pinned in spec. Fallback for any red child: revert the single commit (each child is standalone).

## Verification

Per spec §6 matrix: `cargo xtask mk ci-local-leptos` per move/split child; `cargo xtask mk leptos-gates` at .6/.11/.12/.13/.14/.16 (+ manual gesture smoke at .13); `cargo xtask ci ci-local` at .15/.16; `cargo xtask ticket check --strict` at close-out.
