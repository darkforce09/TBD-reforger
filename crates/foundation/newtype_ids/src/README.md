# Newtype identifiers source

The source of `newtype_ids`: one file per macro, the hidden re-export of the crates their
expansions name, and the crate root.

## Contents

```text
crates/foundation/newtype_ids/src/
├── __private.rs    the `serde` and `uuid` re-exports the expansions name; hidden
├── integer_ids.rs  `integer_id!`: a `Copy` id over the integer type the caller names
├── lib.rs          the crate root: module header and `mod` lines
├── prelude.rs      the three macros for glob import
├── string_ids.rs   `string_id!`: an id over a `String` that borrows as `str`
└── uuid_ids.rs     `uuid_id!`: a `Copy` id over a `Uuid`
```

## How it works

Each macro is `#[macro_export]`, so it lives at the crate root. Its two public arms (with and
without the leading `sqlx,`) forward to one internal `@declare` arm that takes the optional sqlx
attributes as a token list and writes the struct, its derives, the inherent methods and the trait
implementations. Every path in an expansion is absolute (`::core`, `::std`,
`$crate::__private`), so the expansion means the same thing in any calling crate.

## Boundaries

- Depends on: `serde`, `uuid`.
- Used by: callers through the crate root or `prelude`; the expansions themselves through
  `__private`.
- Rules: `lib.rs` holds only the module header and `mod` lines; the macros are tested from outside
  the crate, in `crates/foundation/newtype_ids/tests/`, the way a caller expands them.
