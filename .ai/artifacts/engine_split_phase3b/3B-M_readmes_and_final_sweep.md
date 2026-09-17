# 3B-M — the READMEs describe the tree that exists, and Phase 3B closes

Read `00_rules_every_agent_obeys.md` first. Every rule there applies to this brief.

Every move has landed. This brief makes the documentation true and runs the phase's acceptance.

## Part 1 — the `v2/apps/` READMEs

The 42 READMEs that were under `v2/apps/` described a `features/` + `ui/{top,left,right,bottom,
canvas,modals}` + `state/` shape that contradicts CLAUDE.md's atlas, and referenced a
`src/v2/map_engine` that no longer exists. Briefs 3B-C2 and 3B-L deleted the stale ones under
`editor/` and `debug/`. By operator decision **CLAUDE.md wins**, and this brief writes the
replacements against the tree as it now stands.

Write one README per directory that exists under `v2/apps/`, and none for a directory that does
not. Each says, in a few lines: what the directory holds, what depends on it, and the boundary it
must not cross. Walk the landed tree to derive the list — do not work from the old one.

`v2/apps/planner/` and `v2/apps/aar/` stay empty scaffolds. Their READMEs say so plainly and say
what they are for; the engine's `editing/` must not acquire editor-only assumptions that would
block them, and that constraint belongs in writing.

`src/v2/README.md` still describes the old domain layout. Reconcile it: `apps/{editor,planner,aar,
debug}` is now real and compiled, and its `map_engine/` section is false. Keep it consistent with
`docs/platform/ENGINE_SPLIT_PROGRAM.md`.

CLAUDE.md itself is **not yours** — it is modified in the working tree by someone else, and the
landed shape already matches its atlas. Do not edit it.

## Part 2 — the final sweep

- `rg -n 'src/editor|pages/debug|pages/operations/(orbat|faction)_manager' xtask tools` → empty.
  A stale row in `xtask/src/gate_t180.rs` panics every test in that gate, because its tests
  `fs::copy(...).unwrap()` every path they name.
- `rg -n 'crate::editor|crate::pages::debug' apps/website/frontend/src` → empty.
- **`.coding-standards-allowlist.yaml` carries rows naming paths Phase 3B moved** — at least
  `frontend/src/pages/debug/building_viewer.rs`, `frontend/src/pages/operations/orbat_manager.rs`
  and `frontend/src/editor/state/operations/entity.rs` (the last stale since 3A). A row whose path
  no longer exists exempts nothing, so it is dead weight rather than a failure — but the file whose
  debt it was written for still exists under its new path, and its exemption silently stopped
  applying when the file moved. **Repoint every row whose subject this phase relocated**, and
  report any row whose subject Phase 3A deleted outright rather than deleting it yourself — the
  obsolete-row sweep is 3C's, the rows 3B invalidated are 3B's.
- Every dated allowlist row in `v2/tests/doc_audit/allowlist.rs` names a file that exists, carries
  a real reason, and expires `2026-12-31`. No row uses a never-expires spelling.
- No production file under `v2/apps/` lacks a `//!` header or leaves a `pub` item undocumented —
  those two rules were never exemptible.

- **Comments that still name a file's old home.** The reshape briefs repointed code and pins, but
  prose lagged in a few places. Known at the time of writing: `mission_editor.rs`, `shell/` and
  several files under `tests/` carry comments naming `canvas/boot.rs`, `canvas/viewport.rs`,
  `canvas/overlays.rs`, `canvas/pointer_hover.rs` and `canvas/tactical_graphics.rs` — a directory
  that no longer exists. Re-derive the full list rather than trusting this one, by grepping the
  frontend for every directory name Phase 3B retired (`editor/canvas`, `editor/panels`,
  `editor/state`, `editor/tools`, `editor/world_assets`, `pages/debug`, `mission_editor_tests`).
  Rewrite each to name where the code is now, present tense, per Law 8.
- **Dead citations to `state/operations`, a module Phase 3A deleted.** `website-map-engine`'s
  `editing/commands/merge_report.rs`, `editing/tools/selection/gesture.rs` and
  `editing/tools/selection/pick.rs` carry intra-doc links to `crate::editor::state::operations::…`
  — a path that names a module the engine crate has never had. The frontend carries the same stale
  prose in `mission_editor.rs`, `bridge/overlays.rs`, `bridge/tactical_graphics.rs`,
  `ui/outliner/*` and `input/pointer_gestures.rs`. Re-derive the list; rewrite each to name the
  engine command that does the work now. A broken intra-doc link in the engine is the one of these
  that can actually fail a build.
- **Stale symbol paths in other crates' prose.** The `arsenal_rules` -> `arsenal::rules` rename left
  doc comments naming the old spelling in `website-map-engine`
  (`data/scenario/validation/wire_safety/scan.rs`, `data/scenario/validation/validator/loadout.rs`)
  and in `website-api` (`handlers/missions/registry.rs`). They are prose, not pins, so nothing
  fails — but they cite a symbol that no longer exists. Re-derive the list and correct them.
- `xtask/src/gate_engine_layers.rs` holds a `crate::editor::tools::ruler_tool::…` string as a
  negative regex sample. It is a fixture, so nothing fails on it, but it names a module path that
  has not existed since the tree moved. Refresh it.

`apps/website/audit.md` has now been flagged by four separate briefs: its migration table, its
diagram and its prose all name `frontend/src/editor/…`, a tree that no longer exists. It is a
point-in-time snapshot document rather than a live spec, and rewriting it is not Phase 3B's job.
**Report it with a one-line recommendation and leave it.** Do not rewrite it, and do not delete it.

Report anything else you find outside this brief's scope; do not fix it.

## Part 3 — Phase 3B acceptance, run from the repo root

One at a time, never two cargo commands at once:

```
CARGO_TARGET_DIR=target-container cargo xtask verify engine-layers
CARGO_TARGET_DIR=target-container cargo test -p website-map-engine --all-features
CARGO_TARGET_DIR=target-container cargo test -p website-frontend
CARGO_TARGET_DIR=target-container cargo check --target wasm32-unknown-unknown -p website-frontend
CARGO_TARGET_DIR=target-container cargo fmt --all -- --check
CARGO_TARGET_DIR=target-container cargo xtask mk ci-local-leptos
CARGO_TARGET_DIR=target-container cargo xtask mk leptos-gates
```

Expected:

- `verify engine-layers` PASS, all 8 rules.
- `website-map-engine` >= 1414 passed, 0 failed (brief 3B-D added tests, so it should be higher).
- `website-frontend` >= 1317 passed, 0 failed.
- wasm32 check clean; `fmt --check` silent.
- `mk ci-local-leptos` success.
- `mk leptos-gates`: its `v-suite verify` is red at baseline by operator decision. Diff it against
  `docs/platform/engine_split_phase3_baseline.md` — **set equality on the 21 failing route names**
  (the set may shrink, never grow), and `notfound`, `eventmgr`, `callback`, `login` must still
  pass. Pass → fail is a hard stop.

Known red and **not** yours: `verify file-length` (9 unallowlisted SIZE-3, cleared by Phase 3C, so
`ci-local` — as opposed to `ci-local-leptos` — stays red), the eight named `cargo test -p xtask`
failures (judge by name set, never count), and
`tbd-tickets::store::tests::corpus_roundtrip_real_tree_byte_identical`.

## Report

Paste every command's output verbatim — this is the phase's closing evidence and the orchestrator
reads it instead of re-running anything. Then state, plainly: what moved, what the allowlist now
holds and when it expires, and what Phase 3C inherits.

**Phase 3B ends here. Do not start 3C or 3D.** They are owed work, paused at operator instruction.

Commit directly to `main`:

```
docs(engine-split): the v2 apps READMEs describe the tree that exists (3B)
```
