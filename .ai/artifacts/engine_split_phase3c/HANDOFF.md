Continue the TBD Reforger engine-split program. Phases 1, 2, 3A and 3B have shipped. Phase 3C has
NOT started. Read before doing anything.

READ FIRST, in this order:
  CLAUDE.md                                            Core Project Laws 1-9
  .ai/artifacts/engine_split_phase3b/00_rules_every_agent_obeys.md   rules + traps 3A and 3B paid for
  docs/platform/engine_split_phase3_baseline.md        what was already red before Phase 3 began
  docs/platform/ENGINE_SPLIT_PROGRAM.md                §4 = Phase 3; 3C is its second half
  /home/Samuel/.claude/plans/read-ai-artifacts-engine-split-phase3b-h-mutable-stream.md  the 3B plan

The rules file in `engine_split_phase3b/` is current and applies unchanged to 3C — the six lessons,
the class_r_scrub trap, the repo-wide cross-crate sweep, the disk footgun, the wasm32 rule. Copy it
into `.ai/artifacts/engine_split_phase3c/` rather than rewriting it from the program document.

WHERE THINGS STAND — HEAD is 0f4509186, do not re-derive this:
  Phase 3B is COMPLETE. 15 commits, thirteen subagents, closed with:
      cargo xtask verify engine-layers        PASS, all 8 rules
      cargo test -p website-map-engine        1436 passed; 0 failed; 2 ignored
      cargo test -p website-frontend          1334 passed; 0 failed
      cargo check --target wasm32-unknown-unknown -p website-frontend   8 warnings
      cargo xtask mk ci-local-leptos          success
      cargo xtask mk leptos-gates             editor-suite 21/21 pass; v-suite red at baseline
      cargo xtask verify file-length          exit 1, the baseline's 9 names
      cargo test -p xtask                     826 passed; 8 failed (the known-red eight, by name)

  `frontend/src/` is now `app_routes.rs`, `main.rs`, `router.rs`, `v2/` — nothing else. The editor
  is CLAUDE.md's atlas exactly: `v2/apps/editor/{arsenal,bridge,input,shell,ui}/` + `tests/` +
  `mission_editor.rs`. `editor/`, `canvas/`, `state/`, `panels/` and the legacy `pages/` tree are
  gone. `render_sync.rs` dissolved into `map-engine/editing/{lanes,routing,selection_universe}`.

PHASE 3C — CLAUDE.md Law 7, the 500-line ceiling, enforced repo-wide

  Two thresholds. Conflating them inflates the work by a third:
      production files   under  500 LOC
      test files         under 1000 LOC
  A file is a test file when its path contains `/tests/` or its name ends `_tests.rs`.

  Scope, per the program document's §3C:
    - make `SIZE-3`'s threshold file-kind aware — 500 for production, 1000 for test
    - retire `SIZE-1` (its >600 warn is dead once 500 hard-fails)
    - generate dated grandfather rows for the current offenders, staggered by tree
    - delete the obsolete rows for files 3C splits — per file, never by path prefix
    - split the frontend's 42 oversized production files; after 3C the frontend is at ZERO and
      gets no grandfather rows at all

CORRECTIONS — measured at HEAD, both source documents are wrong about these:

  1. **Extracting the inline test tails does NOT do "most of the work".** The 3B plan says Law 7's
     sibling-file rule alone clears most of the frontend. MEASURED: the tails are real — **140
     inline `#[cfg(test)]` attributes across 44 files, 31,140 LOC, 36% of `v2/apps/`** — but
     extracting every one of them leaves **29 of the 35 oversized files still over 500**:
       dock_right   6535 -> 4139      settings_modal  4524 -> 2433
       top_strip    4787 -> 2848      attributes_modal 4112 -> 2162
       dock_left    3160 -> 1733      asset_catalog   3058 -> 1457
       loadout      2972 -> 1068      building_viewer 2757 ->  927
     Extraction is step one, not the answer. Plan for real decomposition on all 29.

  2. **Extraction creates NEW test-file violations.** Thirteen tails exceed the 1000-line TEST
     ceiling on their own and must land as several sibling files grouped by subject (Law 5), not
     one dump each: dock_right 2396, settings_modal 2091, attributes_modal 1950, top_strip 1939,
     loadout 1904, building_viewer 1830, asset_catalog 1601, help_modal 1483, dock_left 1427,
     validation_panel 1305, toolbelt 1287, rules 1179, tree 1079. The frontend has **zero** test
     files over 1000 today; a careless extraction hands 3C thirteen.

  3. **The counts moved.** The program document says 43 files over 500 under `editor/`, 25 over
     1000, 47 with the §3B four, and 179 repo-wide. MEASURED at HEAD:
       frontend 42 prod>500, 0 test>1000        map-engine 0 / 0        graphics-engine 0 / 0
       api      19 prod>500, 6 test>1000        tools/tbd-tools 21 / 0  ticketboard 8 / 0
       crates    5 prod>500, 0 test>1000        xtask 73 / 0
     **168 production + 6 test = 174 violations; 126 remain once 3C clears the frontend**, and
     `xtask` is 73 of them.

  4. **Two allowlists now exist, with different dates, and they overlap.** 3B could not land 86k
     LOC under `src/v2` without them:
       `v2/tests/doc_audit/allowlist.rs`   57 rows, ALL expiring 2026-12-31. Exempts audit rules
           2, 4 and 5 only — size, inline test module, ticket/wave comments in prose. The header
           rule and the documented-`pub`-item rule are exempted for nothing and are green.
       `.coding-standards-allowlist.yaml`  46 rows — 44 expiring 2026-11-13, one 2026-12-31, one
           2027-08-13. This is `verify file-length`'s allowlist.
     **The coding-standards rows expire six weeks BEFORE the doc-audit rows covering the same
     files.** Decide that deliberately; do not discover it on 2026-11-13.

  5. **A row that names a path which does not exist is a HARD FAILURE in the doc-audit allowlist**
     (that is what forces 3C to delete a row as it splits a file) but is merely dead weight in the
     coding-standards yaml. Three yaml rows already name paths Phase 3A dissolved —
     `editor/state/operations.rs`, `editor/tools/place_helpers.rs`,
     `editor/state/operations/entity.rs`. Every successor in `map-engine` is under 600 lines, so
     no debt rides on them: they are 3C's obsolete-row sweep, and deleting them costs nothing.

  6. **`node_free.rs` has no 500 rule at all** — SIZE-1 warns >600, SIZE-3 fails >1000. Law 7 is
     unenforced today. `exemption_fields_ok` already requires a non-empty `reason` and an unexpired
     `expires`, so the ratchet mechanism exists; the threshold is what has to change.
     `v2/tests/doc_audit/` implements the same ratchet independently and can be read as a worked
     example — including its refusal of any never-expires spelling, which `node_free.rs` still
     permits via the literal `MC-perf`. **Never use `MC-perf` for a grandfather row.**

WHAT 3B HANDED YOU DIRECTLY:
  - `bridge/overlays.rs` at 1542 lines is one of the nine unallowlisted SIZE-3 files. It is the
    only one inside `apps/website/`.
  - `shell/tab_lock.rs` went 993 -> 1009 during 3B's documentation pass and carries a new
    coding-standards row expiring 2026-12-31. It is debt 3B created and 3C clears.
  - **Law-8 history narration in headers the moves carried.** `bridge/overlays.rs`'s header still
    says "split out of `mission_editor.rs` (Phase B)", "bodies are byte-identical to their
    originals". Every such file already carries a doc-audit row whose reason names this.
  - **Stale bare filenames in prose, at volume** — `gestures.rs`, `commands.rs`, `eden_top_strip`,
    `eden_dock_*`, `eden_tree` across roughly 40 files, 21 in `settings_modal.rs` alone. 3B's sweep
    was scoped to retired directory names; the file names are yours.
  - `apps/website/audit.md` describes the pre-v2 tree throughout. Four 3B briefs flagged it and
    none rewrote it. Recommendation on the table: move it to `docs/specs/website_reorg/` with a
    capture date so it reads as the snapshot it is.

HOW TO EXECUTE:
  Decompose 3C into agent-sized briefs and write each one to `.ai/artifacts/engine_split_phase3c/`
  BEFORE launching its agent. One general-purpose subagent per brief, ONE AT A TIME, never in
  parallel. The orchestrator gates between agents: the agent runs its brief's verification and
  pastes it VERBATIM, and you READ that rather than re-running it. Spot-check only a specific risky
  claim, and only with cheap git/grep — never a second cargo run.

  **Gate first, then split.** Make the threshold file-kind aware and generate the grandfather rows
  as brief one, so every later brief's work is measured by the gate it has to satisfy. Then take
  the files in descending size, one brief per file or per tight group.

  **Extract tests, then decompose production, in separate commits.** A 6,535-line file that both
  sheds 2,396 lines of tests and splits its production body in one commit is unreviewable and
  destroys bisect. That ordering is the program document's own rule and 3B proved it works.

  Splitting obeys Law 5 — group variants into named subfolders, never flat-dump `dock_right_1.rs …
  dock_right_13.rs`. And Law 4 — every new filename self-describing with zero project context. The
  program document proposes folder shapes for the nine worst files; they are a starting point, not
  a specification, and they predate the `ui/docks/` layout the files now live in.

  As each file comes under the ceiling, **delete its doc-audit row in the same commit** — a row
  naming a path that no longer exists fails the audit outright, and a row whose file is now clean
  is a lie the next phase would inherit.

OPERATOR DECISIONS ALREADY MADE — do not re-ask:
  1. T-986 (v-suite oracle re-freeze) is DEFERRED out of Phase 3. Acceptance is a diff against the
     committed baseline: set equality on failing route names, and the four clean routes (notfound,
     eventmgr, callback, login) must still pass. Pass -> fail is a hard stop.
  2. Leave the dirty working tree alone. ~70 files are modified/untracked that are NOT ours
     (apps/mod/** is out of scope per §7, plus CLAUDE.md, xtask/src/*, documentation_v2/,
     tools_v2/). Stage ONLY files you authored. Never `git add -A`.
  3. The gate ratchets **repo-wide**, not just over the paths Phase 3 touched. Stagger the expiry
     dates by tree — the program document suggests 2027-01-31 for api/tools/crates/ticketboard and
     2027-06-30 for xtask, the largest holder and not shipped code.
  4. Two task chips are already filed and are NOT 3C's work: unifying the three Rust source-masking
     lexers, and fixing `mk ci-local-leptos` leaking a relative `CARGO_TARGET_DIR` into
     `apps/website/frontend/target-container/` (839 MB, gitignored, structural — the command
     reproduces it, it is not agent error).

KNOWN RED BEFORE 3C TOUCHES ANYTHING — not yours, not regressions:
  gate v-suite verify            21 of 25 routes fail (T-986; the program doc's "22" is wrong).
                                 3B measured exact set equality with the baseline, including
                                 identical per-route diff counts. Hold that line.
  cargo xtask verify file-length exit 1, 9 unallowlisted SIZE-3:
                                   v2/apps/editor/bridge/overlays.rs 1542
                                   crates/tbd-tickets/src/ops.rs 2476
                                   tools/tbd-tools/src/world/forest_smooth.rs 1243
                                   xtask/src/{backfill_stamps 1282, check 2329, estimate_tokens
                                     1647, wave/base 1124, wave/land 1659, wave_lock 2169}
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
  Judge the xtask suite by NAME SET, never count — it drifts run to run. One 3B agent saw
  `gate_mod_compile::tests::no_server_is_rc3` flake once; it passes standalone and on re-run, and
  it lives in an uncommitted not-ours file.

PHASE 3C ACCEPTANCE:
  CARGO_TARGET_DIR=target-container cargo xtask verify file-length   GREEN, with the 500 tier live
  CARGO_TARGET_DIR=target-container cargo xtask verify engine-layers      PASS, all 8 rules
  CARGO_TARGET_DIR=target-container cargo test -p website-map-engine --all-features   >= 1436
  CARGO_TARGET_DIR=target-container cargo test -p website-frontend                    >= 1334
  CARGO_TARGET_DIR=target-container cargo check --target wasm32-unknown-unknown -p website-frontend
  CARGO_TARGET_DIR=target-container cargo fmt --all -- --check
  CARGO_TARGET_DIR=target-container cargo xtask ci ci-local          GREEN — 3C is what unblocks it
  CARGO_TARGET_DIR=target-container cargo xtask mk ci-local-leptos
  CARGO_TARGET_DIR=target-container cargo xtask mk leptos-gates
    -> editor-suite 21/21; v-suite diffed against the committed baseline per decision 1 above.
  Zero production files over 500 LOC under `v2/apps/`, zero test files over 1000, and zero
  doc-audit allowlist rows left for a frontend file that is now clean.

BUILD AND DISK:
  Every cargo invocation: CARGO_TARGET_DIR=target-container cargo ...   (never bare cargo)
  Never run two cargo commands concurrently — they deadlock on the target-dir lock.
  ALWAYS run cargo from the repo root. Check `df -h .`; under ~10G free, stop. 181G free now.
  `mk leptos-gates` needs the dev API on :8080 (`cargo xtask mk rust-api`, which uses its own
  target dir) and Postgres on :5434 (`cargo xtask db up`). A 3B agent lost a run to this.

COMMITS: directly to main, never branch. Subject suffix (3C). End with
  Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>

Start by reading the files above and measuring the tree yourself, then decompose 3C into briefs,
write them to .ai/artifacts/engine_split_phase3c/, and launch the first agent.
