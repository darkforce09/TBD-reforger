# Map editing tools test helpers

The test-only helper the crate's source guards share: the source scrub that blanks comments and
string literals, so a guard that reads the crate's own source sees only the tokens a build compiles.

## Contents

```text
crates/mission_editing/map_editing_tools/src/tests/
└── source_scrub.rs  `strip_rust_lexical_noise`: comments and literals blanked character for character
```

## Boundaries

- Depends on: nothing; a pure function of the text handed to it.
- Used by: `ruler/tests/session_local.rs` and `line_of_sight/tests/session_local.rs`, through the
  `source_scrub` module `lib.rs` mounts under `cfg(test)`.
- Rules: line and column positions survive the scrub, so a token found in the result is a token the
  compiler sees.
