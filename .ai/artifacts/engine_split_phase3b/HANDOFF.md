Continue the TBD Reforger engine-split program. Phases 1, 2 and 3A have shipped. Phase 3B has NOT
started. Read before doing anything.

READ FIRST, in this order:
  CLAUDE.md                                            Core Project Laws 1-9
  .ai/artifacts/engine_split_phase3a/00_rules_every_agent_obeys.md   rules + traps 3A learned
  docs/platform/engine_split_phase3_baseline.md        what was already red before Phase 3 began
  docs/platform/ENGINE_SPLIT_PROGRAM.md                §4 = Phase 3 (contains errors, see below)
  /home/Samuel/.claude/plans/phase-2-is-done-silly-iverson.md   the approved program plan

The plan file OVERRIDES the program document wherever they disagree. Both predate 3A, so BOTH are
now stale in the specific ways listed under "CORRECTIONS" below.

WHERE THINGS STAND — HEAD is 9ae1eedf0, do not re-derive this:
  Phase 3A is COMPLETE. 21 commits, nine subagents, closed with the full gate green:
      cargo xtask mk ci-local-leptos          success (fmt + wasm32 clippy + test + trunk release)
      cargo xtask verify engine-layers        PASS, all 8 rules
      cargo test -p website-map-engine        1414 passed; 0 failed; 2 ignored
      cargo test -p website-frontend          1317 passed; 0 failed
      cargo check --target wasm32-unknown-unknown -p website-frontend   8 warnings
      cargo test -p xtask                     826 passed; 8 failed (the known-red eight, by name)

  `apps/website/frontend/src/editor/state/operations/` and `state/operations.rs` NO LONGER EXIST.
  Every document mutation reaches the engine directly through
  `map-engine/src/editing/hosted_commands/` (17 files). 124 new native tests now cover logic that
  previously needed a browser.

  `target-container/` was cleaned after 3A — 157.7 GiB reclaimed, 195G free. The FIRST cargo
  invocation of the new session rebuilds from scratch and will take a while. That is expected.

PHASE 3B — reshape the frontend into `v2/apps/editor/`

  `apps/website/frontend/src/editor/` is 110 files / 86,968 LOC. `v2/apps/` is 42 READMEs and
  ZERO lines of Rust, and is not even compiled — `v2/mod.rs` declares only `core` and `pages`.

  Scope, per the plan's §3B:
    - add `pub mod apps;` to `v2/mod.rs` and make `v2/apps/` real
    - reshape into `v2/apps/editor/{ui,input,bridge,shell,arsenal}`
    - split `canvas/render_sync.rs` (998 LOC)
    - repoint `mission_editor.rs`'s re-export blocks — do NOT delete them
    - `pages/operations/{orbat_manager,faction_manager}.rs` -> `v2/apps/editor/ui/modals/`
    - `pages/debug/*` (6 files) -> `v2/apps/debug/`
    - move `router.rs` / `app_routes.rs` / `fixtures/t159/manifests/routes.csv` IN LOCKSTEP
    - rewrite the 42 stale `v2/apps/` READMEs to the landed tree

  Current subtree sizes:
    panels/  43,734 LOC (22 files)    arsenal/ 11,895 (7)    state/ 9,006 (22)
    canvas/   6,794 LOC (10 files)    tools/    1,886 (6)    mission_editor_tests/ 8,019 (36)
    root: mission_editor.rs 3,748 · layout.rs 1,323 · world_layer_prefs.rs 353

CORRECTIONS — measured at HEAD, both source documents are wrong about these:

  1. **`include_str!` exposure is ~25x what the plan says.** The plan's R2 reports "21 include_str!
     sites" in `editor/`. MEASURED: **537 occurrences across 62 files, 89 distinct paths, and 96
     of those occurrences reach OUTSIDE `editor/` via `../../`.** Worst offenders:
     `attributes_modal.rs` 56, `dock_right.rs` 53, `t628_boot_progress.rs` 45,
     `settings_modal.rs` 38, `arsenal/mod.rs` 30, `top_strip.rs` 28.
     **Every one of these breaks the moment a file moves.** This is the dominant risk in 3B and it
     must drive the decomposition — not the folder layout.

  2. **The hard pins moved.** The plan cites `help_modal.rs:679/684`, `ui.rs:471`,
     `eden_chrome.rs:39`, `mission_editor.rs:2848`. At HEAD they are `help_modal.rs:679/684`,
     `v2/core/ui/tests/ui.rs:471`, `dock_right.rs:5594`, `eden_chrome.rs:39` (a `pub use` that
     keeps the mount path stable) and `mission_editor.rs:2857` (the `FactionManagerDialog` mount).
     Re-derive line numbers; do not trust any cited here after the first commit lands.

  3. **`state/` is not what either document describes.** 3A left it as `armed_placement/` (4 files),
     `editor_context/` (5 files), `entity_selection.rs`, `undo_grouped_gestures.rs`,
     `commands_hotkeys.rs`, `doc_host.rs`, `history.rs`, `hydrate.rs` (821), `persist.rs` (1,548),
     `save_status.rs`, `tab_lock.rs`, `title_prefer.rs`. The plan's 3B table predates all of it.

  4. Three plan corrections that still hold: `canvas/commands.rs` -> `input/` (not `bridge/`);
     `state/commands_hotkeys.rs`'s wasm half -> `shell/` (not `input/`); `state/history.rs`'s
     window-level keydown -> `input/` (its undo drive already crossed in 3A).

HOW TO EXECUTE:
  Decompose 3B into agent-sized briefs and write each one to
  `.ai/artifacts/engine_split_phase3b/` BEFORE launching its agent. ~10 briefs is the right order
  of magnitude; split by destination folder, but let the `include_str!` blast radius override that
  where it has to. One general-purpose subagent per brief, ONE AT A TIME, never in parallel.

  The orchestrator gates between agents: the agent runs its brief's verification and pastes it
  VERBATIM, and you READ that rather than re-running it. Spot-check only a specific risky claim,
  and only with cheap git/grep — never a second cargo run. Re-running a green suite costs minutes
  and proves nothing.

  Keep each agent to exactly one brief. The first 3A agent was stopped for running too long on too
  wide a scope, and that constraint held for all nine that followed.

LESSONS 3A PAID FOR — put every one of these in every brief:

  1. **The class_r_scrub truncation trap.** `class_r_scrub::live_code()` blanks a frontend file
     from its FIRST `#[cfg(test)]` to EOF. A test-module declaration near the top of a scrubbed
     file silently blanks the whole production body and reds every pin that reads it — it cost one
     agent a debugging cycle and reddened seven unrelated pins. **Declare test modules at the
     BOTTOM.** `mission_editor.rs:3096` depends on this directly: its `#[cfg(test)]` block must
     stay after every production item.
  2. **Scope discipline.** Sweeps cover the files the brief names, NOT the repo. A defect found
     outside the brief is REPORTED in the final message and left alone. A whole-repo comment sweep
     in 3A reached into `api`, `xtask` and `tbd-tickets`, none of which the engine split touches.
  3. **Run only the brief's verification, once, at the end.** No self-initiated clippy/fmt/
     cross-crate passes... EXCEPT where a brief's artifact is `cfg(target_arch = "wasm32")` and
     therefore invisible to the native suite. Check whether what you are moving is wasm-gated; if
     it is, the wasm32 check and `cargo fmt --all -- --check` are mandatory, not extras. 3A shipped
     three unformatted files because two agents skipped that.
  4. **Never `cd` before cargo.** FOUR separate 3A agents leaked stray `target-container/` dirs
     into the source tree this way — 6.2, 2.1, 3.2 and 6.6 GB. Run cargo from the repo root; use
     `( cd sub && ... )` if another cwd is genuinely needed.
  5. **Repoint pins, never weaken them.** When a pin's subject moves, point the pin at where it
     went. Never delete or loosen one to make it pass.
  6. **Watch for gates that name paths you delete.** 3A's last agent found `xtask/src/gate_t180.rs`
     naming 24 deleted files — and because its tests `fs::copy(...).unwrap()` every row, EVERY
     test in that gate panicked and `verify_t180` failed outright. A stale path list is not
     cosmetic. Grep `xtask/` for any path under `editor/` before you finish.

OPERATOR DECISIONS ALREADY MADE — do not re-ask:
  1. T-986 (v-suite oracle re-freeze) is explicitly DEFERRED out of Phase 3. Acceptance is a diff
     against the committed baseline: set equality on failing route names, and the four clean
     routes (notfound, eventmgr, callback, login) must still pass. Pass -> fail is a hard stop.
  2. Leave the dirty working tree alone. ~70 files are modified/untracked that are NOT ours
     (apps/mod/** is out of scope per §7, plus CLAUDE.md, xtask/src/*, documentation_v2/,
     tools_v2/, v2/** READMEs). Stage ONLY files you authored. Never `git add -A`.
  3. `pages/operations/{orbat_manager,faction_manager}.rs` -> `v2/apps/editor/ui/modals/` (unrouted
     editor modals, not pages). Keep distinct from `v2/pages/operations/orbat_selection/`.
  4. `v2/apps/`'s 42 existing READMEs describe a `features/` + `ui/{top,left,right,bottom,canvas,
     modals}` + `state/` shape that CONTRADICTS CLAUDE.md's atlas (`ui/ input/ bridge/ shell/
     arsenal/`) and reference a deleted `src/v2/map_engine`. **CLAUDE.md wins**; rewrite them.
  5. STOP AFTER 3B AND NOTIFY THE OPERATOR — this means the END OF PHASE 3B, not the end of the
     first brief. Do not launch 3C or 3D. They remain owed work, paused at operator instruction:
     not deferred, not descoped.

KNOWN RED BEFORE 3B TOUCHES ANYTHING — not yours, not regressions:
  gate v-suite verify            21 of 25 routes fail (T-986; the program doc's "22" is wrong)
  cargo xtask verify file-length exit 1, 9 unallowlisted SIZE-3 — Phase 3C clears them
  cargo test -p xtask            8 failures, all pinning a missing apps/mod/** script:
                                   gate_t437::tests::{collapsed_returns_fail_registry_pins,
                                     live_tree_holds, paraphrase_injection_is_caught}
                                   schema_gates::t212_objective_spine_tests::{objective_spine_is_
                                     read_in_the_objectives_lane, the_lane_scan_can_still_report_zero}
                                   schema_gates::t212_side_fallback_tests::invalid_side_is_neutral_
                                     but_absent_and_valid_sides_keep_their_roles
                                   schema_gates::t212_staged_golden_tests::the_staged_1_3_golden_
                                     objectives_row_binds_to_the_reader
                                   schema_gates::unread_wire_field_tests::all_1_3_fields_are_
                                     unread_on_the_live_tree
  cargo test -p tbd-tickets      1 failure, store::tests::corpus_roundtrip_real_tree_byte_identical
                                   (pre-existing, confirmed unrelated to Phase 3)
  file-length sits inside verify-coding-standards inside ci-local, so ci-local is red until 3C.
  Judge the xtask suite by NAME SET, never count — it drifts run to run.

PHASE 3B ACCEPTANCE:
  CARGO_TARGET_DIR=target-container cargo xtask verify engine-layers      PASS, all 8 rules
  CARGO_TARGET_DIR=target-container cargo test -p website-map-engine --all-features   >= 1414
  CARGO_TARGET_DIR=target-container cargo test -p website-frontend                    >= 1317
  CARGO_TARGET_DIR=target-container cargo check --target wasm32-unknown-unknown -p website-frontend
  CARGO_TARGET_DIR=target-container cargo fmt --all -- --check
  CARGO_TARGET_DIR=target-container cargo xtask mk ci-local-leptos
  CARGO_TARGET_DIR=target-container cargo xtask mk leptos-gates
    -> v-suite diffed against the committed baseline per decision 1 above.

BUILD AND DISK:
  Every cargo invocation: CARGO_TARGET_DIR=target-container cargo ...   (never bare cargo)
  Never run two cargo commands concurrently — they deadlock on the target-dir lock.
  ALWAYS run cargo from the repo root. Check `df -h .`; under ~10G free, stop.
  Currently 195G free; target-container was cleaned, so the first build is a full one.

COMMITS: directly to main, never branch. Subject suffix (3B). End with
  Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>

Start by reading the files above and measuring the tree yourself, then decompose 3B into briefs,
write them to .ai/artifacts/engine_split_phase3b/, and launch the first agent.
