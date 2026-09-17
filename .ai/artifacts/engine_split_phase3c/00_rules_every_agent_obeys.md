## Non-negotiable rules (identical for every Phase 3B subagent)

Read these first, in this order:
  /run/media/system/Disk_2/Projects/TBD-Reforger/CLAUDE.md          (Core Project Laws 1-9)
  /run/media/system/Disk_2/Projects/TBD-Reforger/.cursor/rules/no-silent-deferrals.mdc
  /run/media/system/Disk_2/Projects/TBD-Reforger/docs/platform/ENGINE_SPLIT_PROGRAM.md  (Phase 3 = section 4)
  /home/Samuel/.claude/plans/read-ai-artifacts-engine-split-phase3b-h-mutable-stream.md
      (the approved 3B plan; its measurements OVERRIDE the program document wherever they disagree)

LAW 1 - HARD GATE, NO SILENT DEFERRALS. Do the whole ask. You may not invent "out of scope",
"deferred", "follow-up", "MVP", or a DEFERRED section in a verify log. The ONLY deferral
authorized anywhere in Phase 3 is T-986 (v-suite oracle re-freeze), by explicit operator word.
If you are genuinely blocked, STOP and report the blocker - do not route around it.

LAW 2 - GIT. Commit directly to `main`. NEVER create a branch; `git checkout -b` is forbidden.
Commit-subject suffix for this phase: `(3B)`. End every commit message with:
    Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>

LAW 3 - No ad-hoc hacks. If a clean solution needs a structural refactor, do the refactor.
        In particular: never reach a module through `#[path = "..."]` to dodge a move.

LAW 4 - Every folder/file/symbol name must be unmistakable with ZERO project context.
LAW 5 - Group variants into named subfolders. Never flat-dump `thing_1.rs .. thing_13.rs`.
LAW 7 - Production files < 500 LOC, test files < 1000 LOC. No inline `#[cfg(test)] mod tests`
        blocks: tests live in sibling files declared
        `#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`
        Phase 3B does NOT split oversized files - that is 3C. See "the doc_audit ratchet" below.
LAW 8 - Comments describe what the code does NOW and WHY. Absolutely no "moved from",
        "was previously", "split out of", "renamed at", no phase numbers in code comments.
        Commit history owns history. This applies to comments you WRITE and to headers you
        carry along on a move - rewrite them.

ZERO BEHAVIOR CHANGE is the contract for all of Phase 3. Moves, splits, dedupes. If the editor
looks or behaves differently, that is a bug in your work.

THE WORKING TREE IS DIRTY AND THAT IS DELIBERATE. ~70 files are modified/untracked that are
NOT yours: `apps/mod/**` (explicitly out of scope per spec section 7), `CLAUDE.md`,
`xtask/src/{check,constants,gate_mod_compile,sync}.rs`, untracked `documentation_v2/`,
`tools_v2/`, `docs/platform/ENGINE_SPLIT_PROGRAM.md`, and the `v2/**` READMEs.
**Stage ONLY files you yourself authored or edited.** Use explicit `git add <path>`; never
`git add -A`, never `git add .`, never `git commit -a`.

BUILD. You are in a container. Every cargo invocation MUST be prefixed:
    CARGO_TARGET_DIR=target-container cargo ...
Host and container glibcs must not share `target/`. Never run bare `cargo`.
Never run two cargo commands concurrently - they deadlock on the target-dir lock.

REPORTING. Return every verification command's output VERBATIM. Do not summarize, do not
paraphrase, do not write "all green". Paste the actual text. The orchestrator reads your paste
instead of re-running the suite, so a paraphrase destroys the only evidence there is.

## The doc_audit ratchet - the thing that makes 3B different from 3A

`apps/website/frontend/src/v2/doc_audit_tests.rs` audits EVERY production `.rs` under `src/v2`
(it skips directories literally named `tests`) and enforces five rules:

```
1. the file opens with a `//!` header
2. at most 500 lines
3. every `pub` item carries a doc comment
4. no inline `#[cfg(test)] mod name { ... }` block
5. no comment names a ticket (`T-123`) or a wave (`wave129`, `w12`)
```

Phase 3B moves 81 files that violate rules 2, 4 and 5 in bulk (42 oversized, 45 inline test
modules, 2,775 ticket/wave comment lines). Those three are 3C's job, and 3B carries them on a
**dated grandfather allowlist** inside the audit - a `(path, reason, expires)` row per file,
expiring `2026-12-31`. An expired row stops exempting; a row naming a path that does not exist
FAILS. Never write a never-expiring row.

**Rules 1 and 3 are NOT exempted.** The brief that moves a file writes the `//!` header and the
missing doc comments it owes. 148 undocumented items across 25 files, and one missing header,
are distributed across the move briefs.

When your brief moves a file that holds a grandfather row, **update the row's path in the same
commit**. A stale row is a hard failure, by design.

## Disk - this footgun has bitten repeatedly

**Always run cargo from the repo root.** FOUR separate 3A agents leaked stray `target-container/`
directories into the source tree by running cargo after `cd`-ing into a subdirectory, where the
relative `CARGO_TARGET_DIR` resolves below the repo root - 6.2, 2.1, 3.2 and 6.6 GB. If a command
needs another cwd, use a subshell so it cannot leak: `( cd sub && ... )`. Check `df -h .`
occasionally; under ~10G free, stop and say so.

## The class_r_scrub truncation trap - this WILL bite you

`class_r_scrub::live_code()` (`v2/core/test_support/class_r_scrub/scrub.rs`, `cut_test_module`)
blanks a file from its **first** `#[cfg(test)]` to EOF. So in any frontend file that a Class-R
scrub or the keymap census reads, a test-module declaration placed near the top blanks the entire
production body below it, and every pin that scrubs that file goes red at once.

**Put `#[cfg(test)] #[path = "tests/<file>.rs"] mod ...;` declarations at the BOTTOM of the file,
below any existing test modules.** `mission_editor.rs`'s `#[cfg(test)]` block depends on this
directly: it must stay after every production item.

## Source pins

This codebase pins behaviour by **source inspection** routinely - `include_str!` over another
file's text, then a scrub and an assertion. Phase 3B's first brief anchors every cross-file pin
to `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/..."))` so that moving a file no longer
breaks the pins that read it. After that brief:

- **Never** write a new `include_str!("../..")` relative pin across directories. Use the anchored
  form. A pin to the file's own text (`include_str!("my_own_file.rs")`) stays as it is.
- **Grep for pin suffixes across line breaks.** rustfmt wraps a long `include_str!(concat!(...))`
  over several lines, so a line-oriented grep for `include_str!("/src/v2/apps/editor/panels/...")`
  misses it. Search for the path suffix alone, not the macro call around it — 57 sites hid from a
  line-based regex in brief 3B-A.
- **Repoint pins, never weaken them.** When a pin's subject moves, point the pin at where it went.
  Never delete a pin, loosen an assertion, or drop a census row to make a suite pass.

## Other crates pin frontend source — sweep the WHOLE repo, not just `xtask/`

`website-map-engine`'s own tests `include_str!` frontend files. Three of them named the pre-move
`frontend/src/editor/` tree and were missed by a sweep scoped to `xtask/` and `tools/`, so
`cargo test -p website-map-engine` stopped compiling and nobody knew until the next brief tried to
run it.

**After any move, grep every crate for paths under the tree you touched** — at minimum
`apps/website/map-engine`, `apps/website/api`, `xtask/` and `tools/`:

```
rg -n 'frontend/src/(editor|pages)' apps xtask tools crates
```

If a hit is inside `website-map-engine`, repoint it AND add
`CARGO_TARGET_DIR=target-container cargo test -p website-map-engine --all-features` to your
verification — a crate whose test build you broke is not green just because your own suite passed.
Convert any bare relative pin you find to the anchored `CARGO_MANIFEST_DIR` form while you are
there.

## Gates outside the frontend name frontend paths

`xtask/src/gate_t180.rs` holds a table of `apps/website/frontend/src/editor/...` paths and
`fs::copy(...).unwrap()`s every row, so ONE stale path panics EVERY test in that gate. `ai.rs`
and `migrate_v2.rs` also carry `src/editor/` string literals. **Grep `xtask/` and `tools/` for
any path under the tree you touch before you finish.**

## Scope discipline

Sweeps, audits and fix-ups are scoped to **the files your brief names**, not the repo. If you
find a real defect outside your brief - a broken doc link, a stale pin, a dead citation in
another crate - **report it in your final message and leave it alone**. Someone owns it; it is
not you. A whole-repo comment sweep in 3A reached into `api`, `xtask` and `tbd-tickets`, none of
which the engine split touches.

Run **only** the verification your brief lists, **once**, at the end. Do not add clippy, fmt, or
cross-crate `cargo check` passes on your own initiative - with ONE exception below.

## wasm32-gated code is invisible to the native suite

Large parts of the editor carry `#[cfg(target_arch = "wasm32")]`, so `cargo test -p website-frontend`
never compiles them. If anything your brief moves or edits is wasm-gated, these two are part of
your verification, not extras:

```
CARGO_TARGET_DIR=target-container cargo check --target wasm32-unknown-unknown -p website-frontend
CARGO_TARGET_DIR=target-container cargo fmt --all -- --check
```

3A shipped three unformatted files because two agents skipped that. `fmt` matters because
`verify-coding-standards` runs it inside `ci-local`.

## Known red before you start - NOT yours, NOT a regression if still red

```
gate v-suite verify          21 of 25 routes fail (T-986, deferred by operator word; the program
                             document's "22" is wrong). The four clean routes - notfound, eventmgr,
                             callback, login - must still PASS. Pass -> fail is a hard stop.
cargo xtask verify file-length   exit 1, 9 unallowlisted SIZE-3   (Phase 3C clears these)
cargo test -p xtask          8 failures, all pinning one missing apps/mod/** script:
  gate_t437::tests::collapsed_returns_fail_registry_pins
  gate_t437::tests::live_tree_holds
  gate_t437::tests::paraphrase_injection_is_caught
  schema_gates::t212_objective_spine_tests::objective_spine_is_read_in_the_objectives_lane
  schema_gates::t212_objective_spine_tests::the_lane_scan_can_still_report_zero
  schema_gates::t212_side_fallback_tests::invalid_side_is_neutral_but_absent_and_valid_sides_keep_their_roles
  schema_gates::t212_staged_golden_tests::the_staged_1_3_golden_objectives_row_binds_to_the_reader
  schema_gates::unread_wire_field_tests::all_1_3_fields_are_unread_on_the_live_tree
cargo test -p tbd-tickets    1 failure, store::tests::corpus_roundtrip_real_tree_byte_identical
```

Counts drift between runs, so judge by **name set**, never by count. A failure outside those
names is yours. `verify file-length` runs inside `verify-coding-standards`, which runs inside
`cargo xtask ci ci-local`, so **`ci-local` is red until Phase 3C lands**, and that is expected.

Full detail: `docs/platform/engine_split_phase3_baseline.md`.
