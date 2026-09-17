# 3B-B2 — the documentation audit reads code, not the text inside string literals

Read `00_rules_every_agent_obeys.md` first. Every rule there applies to this brief.

## The defect

`apps/website/frontend/src/v2/tests/doc_audit/mod.rs` decides whether a line declares a
documentable item by looking at the line's text. It has no idea whether that line is code or the
inside of a string literal. This codebase pins behaviour by source inspection constantly, so its
tests are full of fixture strings holding Rust source — and every `pub fn` inside one of those
fixtures reads to the audit as an undocumented public item.

It has already forced a hack. `apps/website/frontend/src/editor/state/title_prefer.rs` carries two
`///` lines **inside** fixture string literals, written for no reason other than to quiet this
false positive. They describe decoys in a fixture, they document nothing, and they are the kind of
ad-hoc patch CLAUDE.md Law 3 exists to prevent. As the remaining Phase 3B briefs land 73 more
files under `src/v2`, the same false positive will demand the same hack again.

## The fix

Teach the audit to mask string literals — and the contents of comments — before it applies the
item, inline-module and ticket/wave rules. The crate already does exactly this elsewhere: the
`masked()` helper used by the title-preference extractor blanks comments and string literals
before counting item heads. Read it, and follow the same approach rather than inventing a second
one. Raw strings (`r"…"`, `r#"…"#`), escaped quotes and multi-line strings all occur in this tree
and all must be handled.

The header rule (a file opens with `//!`) is unaffected — it reads the first non-blank line only.

Then **revert the two fixture `///` lines in `title_prefer.rs`**, and confirm the audit still
reports nothing for that file. Reverting them is the proof the fix works; leaving them in place
would hide it.

## Tests

Add them beside the existing mechanism tests, against fixture text, not the live tree:

- a `pub fn` inside a `"…"`, a `r#"…"#` and a multi-line string is not an undocumented item
- a `pub fn` in real code still is
- a string containing `#[cfg(test)] mod tests {` does not trip the inline-module rule
- a comment or string naming a ticket still trips the ticket rule when it is a real comment, and a
  ticket name inside a string literal does not
- a string containing a quote escape (`\"`) does not desynchronise the mask

## Scope

`v2/tests/doc_audit/mod.rs`, its tests, and the two-line revert in
`apps/website/frontend/src/editor/state/title_prefer.rs`. Nothing else. No file moves. If the fix
makes the audit report a file it previously passed, that file owes a real doc comment — report
what you found and write it; do not add a grandfather row for it, because the header and
documented-item rules are not exemptible.

## Verification — run once, at the end, from the repo root

```
CARGO_TARGET_DIR=target-container cargo test -p website-frontend
CARGO_TARGET_DIR=target-container cargo fmt --all -- --check
```

Expected: >= 1327 passed plus your new tests, 0 failed; fmt silent. Paste both verbatim, plus the
audit's finding count over `apps/website/frontend/src/editor/` before and after your change.

Commit directly to `main`:

```
fix(engine-split): the v2 documentation audit ignores source text inside string literals (3B)
```
