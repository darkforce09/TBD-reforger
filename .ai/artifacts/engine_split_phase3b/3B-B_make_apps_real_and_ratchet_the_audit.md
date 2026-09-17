# 3B-B — make `v2/apps/` real, and give the v2 documentation audit a dated ratchet

Read `.ai/artifacts/engine_split_phase3b/00_rules_every_agent_obeys.md` first. Every rule there
applies to this brief.

Brief 3B-A has landed: every cross-file source pin now addresses its subject from the crate root.
Nothing has moved yet. This brief builds the two pieces of scaffolding the moves need, and moves
nothing either.

## Part 1 — `v2/apps/` becomes a compiled module

`apps/website/frontend/src/v2/mod.rs` declares `core` and `pages` only, so `v2/apps/` — 42 READMEs
and zero lines of Rust — is not part of the crate at all.

- add `pub mod apps;` to `v2/mod.rs`, in the same style as the two lines above it, and extend that
  file's `//!` header so its "Role" sentence names what `apps` holds: the standalone CAD
  workspaces, which consume `core` and are reached from `pages`.
- create `apps/website/frontend/src/v2/apps/mod.rs` with a `//!` header describing the domain
  (per CLAUDE.md's atlas: the editor, planner, aar and debug workspaces). It declares nothing yet —
  brief 3B-C adds `pub mod editor;`.

## Part 2 — the documentation audit gets a dated allowlist

`apps/website/frontend/src/v2/doc_audit_tests.rs` audits every production `.rs` under `src/v2`
(skipping directories named `tests`) for five rules:

```
1. the file opens with a `//!` header
2. at most 500 lines
3. every `pub` item carries a doc comment
4. no inline `#[cfg(test)] mod name { ... }` block
5. no comment names a ticket or a wave
```

Phase 3B moves 81 files into `src/v2` that break rules 2, 4 and 5 in bulk — 42 are over 500 lines,
45 hold inline test modules, and 2,775 comment lines name a ticket or a wave. Splitting and
sweeping them is Phase 3C's entire subject; the program document is explicit that moves come first
and splits second, because a 6,456-line file that both moves and splits in one commit is
unreviewable and destroys bisect.

So the audit gets the same ratchet shape that `xtask/src/node_free.rs` already uses for
`verify file-length`: a dated allowlist, not a scope exclusion.

### Required semantics

A row is `(path, reason, expires)` where `path` is relative to `src/v2`.

- A row exempts its file from **rules 2, 4 and 5 only**. Rules 1 and 3 — the `//!` header and the
  documentation of `pub` items — are never exempted by a row.
- `reason` must be non-empty.
- `expires` must be a real `YYYY-MM-DD` and must not have passed. An expired row stops exempting,
  so the file it covers starts failing on that date. There is **no never-expires spelling here** —
  `node_free.rs` accepts the literal `MC-perf` for that; this allowlist must not.
- A row whose `path` does not exist is a **hard failure**. That is what forces Phase 3C to delete
  a row as it splits the file, and what makes a later move update its rows instead of orphaning
  them.
- Today's date is computed in UTC from the system clock, the same civil-date arithmetic
  `node_free.rs::civil_ymd` uses. The frontend cannot depend on `xtask`, so this is a small local
  implementation, not an import.

### Where it lives

`doc_audit_tests.rs` sits at `src/v2/doc_audit_tests.rs`, which means the audit currently audits
its own source and is bound by the same 500-line ceiling it enforces. Adding ~81 rows to it would
put it over that ceiling.

Move it into the tests subtree the audit already skips, and split the row table from the rules:

```
src/v2/tests/doc_audit/mod.rs         the five rules and their walk, plus the allowlist mechanism
src/v2/tests/doc_audit/allowlist.rs   the row table, empty for now
```

`v2/mod.rs`'s declaration becomes `#[cfg(test)] #[path = "tests/doc_audit/mod.rs"] mod doc_audit;`,
kept at the **bottom** of the file — `class_r_scrub::live_code()` blanks a file from its first
`#[cfg(test)]` to EOF, and `v2/mod.rs` is read by scrubs.

**Add no rows in this brief.** The table ships empty; brief 3B-C fills it.

### Tests for the mechanism itself

The ratchet is only worth having if its failure modes are pinned. Add tests, in the same file as
the rules, covering:

- an exempt file with an `expires` in the future is not reported for rules 2, 4 or 5
- the same file with an `expires` in the past **is** reported
- a row with an empty `reason` does not exempt
- a row naming a path that does not exist fails the audit
- an exempted file is still reported when it lacks a `//!` header or leaves a `pub` item
  undocumented

Write them against the rule functions directly with fixture text, not against the live tree — a
test that depends on which files happen to be oversized today is a test that rots.

## Scope

`v2/mod.rs`, the new `v2/apps/mod.rs`, and the doc-audit files. Nothing else. No production file
moves, no `editor/` file is touched.

## Verification — run once, at the end, from the repo root

```
CARGO_TARGET_DIR=target-container cargo test -p website-frontend
CARGO_TARGET_DIR=target-container cargo fmt --all -- --check
```

Expected: `website-frontend` >= 1317 passed plus your new mechanism tests, 0 failed. `fmt --check`
silent. Paste both verbatim, and paste `git status --porcelain` for the paths you staged.

Commit directly to `main`:

```
feat(engine-split): the v2 documentation audit ratchets on dated rows (3B)
```
