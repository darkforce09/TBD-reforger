# Cargo manifest syntax

The lexical half of the `Cargo.toml` reader in
`tools/verification_core/src/repository_laws/cargo_manifest.rs`: the pieces of the TOML subset
Cargo manifests are written in, read without a TOML dependency.

## Contents

```text
tools/verification_core/src/repository_laws/cargo_manifest/
└── toml_subset.rs   comments, closed entries, strings, arrays, inline-table fields and `workspace = true`
```

## How it works

`parse_manifest` reads a manifest line by line; these helpers strip a `#` comment outside a string,
join an entry whose `{` or `[` has not closed, take the strings of an array, read one field of an
inline table and tell whether a value inherits from the workspace.

## Boundaries

- Depends on: `std` only.
- Used by: `super` (`cargo_manifest.rs`) alone; its tests in
  `tools/verification_core/src/repository_laws/tests/cargo_manifest.rs` and
  `tests/workspace_members.rs` pin the behaviour.
- Rules: a `#` or bracket inside a `"…"` string is text, never syntax.
