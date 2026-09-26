# Source scrubber

The scrubber the crate's source pins read production code through. A source pin is a test that
asserts on this crate's own source text ("this file calls X"); the scrubber reduces a file to what
a shipping build compiles and runs, so a mention of X in a comment, a string literal or code no
build reaches cannot satisfy the pin. It is test-only support.

## Contents

```text
apps/website/frontend/src/v2/core/test_support/class_r_scrub/
├── cfg.rs     whether a `cfg` predicate holds in every build, in none, or depends on the build
├── consts.rs  harvests the file's `const`, `static` and `let` bindings and folds what it can
├── expr.rs    the small expression grammar that folds an `if` or `while` condition to a value
├── lexer.rs   word boundaries, delimiter matching, and the masked copy of the same length
├── mod.rs     the module tree; re-exports the four entry points and four of the inner helpers
└── scrub.rs   the removal passes, and the four entry points the source pins call
```

## How it works

```text
source ─▶ lexer.rs mask ─▶ scrub.rs passes, in order:
            1 cut from the first `#[cfg(test)]` to the end of the file
            2 remove items whose `cfg` predicate is false in every build   (cfg.rs)
            3 remove blocks whose condition cannot be proved to run         (consts.rs, expr.rs)
            4 remove everything after an unconditional jump
       ─▶ the text that is left
```

`lexer.rs` builds a masked copy with comments blanked, and string and character literals too when
asked; it has exactly the length of the input and keeps every newline, so an index or a line
number taken on the mask addresses the original. Every pass blanks the same range in the mask and
in the working copy, so a later pass never sees what an earlier one removed and braces stay
balanced.

The entry points, all `pub(crate)`:

| Entry point | Returns |
|---|---|
| `live_source` | the production text, comments and unreachable code removed, string literals kept |
| `live_code` | the same with string and character literals blanked, for "a call, not a mention" |
| `only_item` | the one item matching a marker: its signature and balanced body |
| `only_body` | the balanced body of that one item |

An undecided condition fails closed: its block is removed, because a wrongly removed block turns
a pin red while a wrongly kept one leaves it green over code the build never runs. `only_item` and
`only_body` panic on zero matches (a rename) and on two or more (a shadow definition).

## Boundaries

- Depends on: the standard library only.
- Used by: the source pins in the unit tests of `apps/website/frontend/src/v2/apps/editor/`,
  `apps/website/frontend/src/v2/pages/` and `apps/website/frontend/src/v2/core/` (the
  [API](/documentation_v2/glossary/a_to_f.md#api) client, the live status stream, the UI primitives and
  the clipboard write), through `crate::v2::core::test_support::class_r_scrub`.
- Rules: under `live_code`, every decoy shape (comments, literals, `if false`, dead `cfg` items,
  code after a `return;`, constants folded to false) comes out removed, and an unknown condition
  removes its block (`the_scrubber_actually_removes_every_decoy_shape` and
  `the_unknown_condition_fails_closed` in
  `apps/website/frontend/src/v2/core/test_support/tests/class_r_scrub.rs`); a test-only item of a
  production file sits below every production item, since the first pass cuts from the first
  `#[cfg(test)]`.
