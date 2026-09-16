## Non-negotiable rules (identical for every Phase 3 subagent)

Read these first, in this order:
  /run/media/system/Disk_2/Projects/TBD-Reforger/CLAUDE.md          (Core Project Laws 1-9)
  /run/media/system/Disk_2/Projects/TBD-Reforger/.cursor/rules/no-silent-deferrals.mdc
  /run/media/system/Disk_2/Projects/TBD-Reforger/docs/platform/ENGINE_SPLIT_PROGRAM.md  (Phase 3 = section 4)
  /home/Samuel/.claude/plans/phase-2-is-done-silly-iverson.md       (the approved plan; its
      corrections OVERRIDE the program document wherever they disagree)

LAW 1 - HARD GATE, NO SILENT DEFERRALS. Do the whole ask. You may not invent "out of scope",
"deferred", "follow-up", "MVP", or a DEFERRED section in a verify log. The ONLY deferral
authorized anywhere in Phase 3 is T-986 (v-suite oracle re-freeze), by explicit operator word.
If you are genuinely blocked, STOP and report the blocker - do not route around it.

LAW 2 - GIT. Commit directly to `main`. NEVER create a branch; `git checkout -b` is forbidden.
Commit-subject suffix matches the Phase 1/2 precedent: `(3A)`, `(3B)`, `(3C)`, `(3D)`.
End every commit message with:
    Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>

LAW 3 - No ad-hoc hacks. If a clean solution needs a structural refactor, do the refactor.

LAW 4 - Every folder/file/symbol name must be unmistakable with ZERO project context.
LAW 5 - Group variants into named subfolders. Never flat-dump `thing_1.rs .. thing_13.rs`.
LAW 7 - Production files < 500 LOC, test files < 1000 LOC. No inline `#[cfg(test)] mod tests`
        blocks: tests live in sibling files declared
        `#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`
LAW 8 - Comments describe what the code does NOW and WHY. Absolutely no "moved from",
        "was previously", "split out of", "renamed at", no ticket numbers, no phase numbers
        in code comments. Commit history owns history. This applies to comments you WRITE and
        to headers you carry along on a move - rewrite them.

THE WORKING TREE IS DIRTY AND THAT IS DELIBERATE. ~70 files are modified/untracked that are
NOT yours: `apps/mod/**` (42 files, explicitly out of scope), `CLAUDE.md`, `xtask/src/{check,
constants,gate_mod_compile,sync}.rs`, untracked `documentation_v2/`, `tools_v2/`,
`docs/platform/ENGINE_SPLIT_PROGRAM.md`, and already-deleted `v2/map_engine/` READMEs.
**Stage ONLY files you yourself authored or edited.** Use explicit `git add <path>`; never
`git add -A`, never `git add .`, never `git commit -a`.

BUILD. You are in a container. Every cargo invocation MUST be prefixed:
    CARGO_TARGET_DIR=target-container cargo ...
Host and container glibcs must not share `target/`. Never run bare `cargo`.

ZERO BEHAVIOR CHANGE is the contract for all of Phase 3. Moves, splits, dedupes. If the editor
looks or behaves differently, that is a bug in your work.

REPORTING. Return every verification command's output VERBATIM. Do not summarize, do not
paraphrase, do not write "all green". Paste the actual text.

## Known red before you start — NOT yours, NOT a regression if still red

```
cargo xtask verify file-length   exit 1, 9 unallowlisted SIZE-3   (Phase 3C clears these)
cargo test -p xtask              8 flaky failures, all pinning one missing apps/mod/** script:
  gate_t437::tests::collapsed_returns_fail_registry_pins
  gate_t437::tests::live_tree_holds
  gate_t437::tests::paraphrase_injection_is_caught
  schema_gates::t212_objective_spine_tests::objective_spine_is_read_in_the_objectives_lane
  schema_gates::t212_objective_spine_tests::the_lane_scan_can_still_report_zero
  schema_gates::t212_side_fallback_tests::invalid_side_is_neutral_but_absent_and_valid_sides_keep_their_roles
  schema_gates::t212_staged_golden_tests::the_staged_1_3_golden_objectives_row_binds_to_the_reader
  schema_gates::unread_wire_field_tests::all_1_3_fields_are_unread_on_the_live_tree
```

All eight fail on `FAIL: missing apps/mod/tbd-framework/Scripts/Game/TBD/Objectives/TBD_ObjectiveRegistry.c`
— a path absent from the tree and untracked at this commit. `apps/mod/**` is out of scope per the
program document §7. Counts drift between runs (818/10, then 820/8), so judge by **name set**,
never by count. A failure outside those eight names is yours.

`verify file-length` runs inside `verify-coding-standards`, which runs inside `cargo xtask ci
ci-local`. So **`ci-local` is red until Phase 3C lands**, and that is expected.

Full detail: `docs/platform/engine_split_phase3_baseline.md`.

## Disk — this footgun has bitten twice already

The repo lives on a disk that hit 100% full. Twice now, stray `target-container/` directories have
been created **inside the source tree** — 16 from one agent alone, one nested at
`map-engine/src/editing/tools/...`. Cause: running cargo after `cd`-ing into a subdirectory, so the
relative `CARGO_TARGET_DIR` resolves below the repo root.

**Always run cargo from the repo root.** If a command needs another cwd, use a subshell so it
cannot leak: `( cd sub && ... )`. Check `df -h .` occasionally; under ~10G free, stop and say so.

## The class_r_scrub truncation trap — this WILL bite you

`class_r_scrub::live_code()` (`v2/core/test_support/class_r_scrub/scrub.rs`, `cut_test_module`)
blanks a file from its **first** `#[cfg(test)]` to EOF. So in any frontend file that a Class-R
scrub or the keymap census reads, a test-module declaration placed near the top blanks the entire
production body below it, and every pin that scrubs that file goes red at once.

**Put `#[cfg(test)] #[path = "tests/<file>.rs"] mod …;` declarations at the BOTTOM of the file,
below any existing test modules.** Adding one at the top of `state/commands_hotkeys.rs` turned
seven unrelated pins red until it was moved down.

## Scope discipline

Sweeps, audits and fix-ups are scoped to **the files your brief names**, not the repo. If you
find a real defect outside your brief — a broken doc link, a stale pin, a dead citation in
another crate — **report it in your final message and leave it alone**. Someone owns it; it is
not you. Fixing it costs a rerun of suites your brief never needed.

Run **only** the verification your brief lists, **once**, at the end. Do not add clippy, fmt, or
cross-crate `cargo check` passes on your own initiative — the gate above you already covers them,
and every extra cargo invocation is minutes of wall clock on a locked target dir.
