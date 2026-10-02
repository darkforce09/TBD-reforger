# Deterministic random source

The source of `deterministic_random`: the SplitMix64 generator and the crate root that exports
it.

## Contents

```text
crates/foundation/deterministic_random/src/
├── lib.rs           the crate root: module header, `mod` lines and the re-export
├── prelude.rs       `SplitMix64` for glob import
├── split_mix_64.rs  `SplitMix64`: the seeded counter, its finaliser and the derived draws
└── tests/           unit tests of the generator
```

## How it works

`lib.rs` re-exports `split_mix_64::SplitMix64` at the crate root and in `prelude`.
`tests/split_mix_64.rs` pins the seed-0 reference stream, checks that a seed reproduces its whole
stream, proves the multiply and divide spellings of the unit draw bit-identical, and keeps every
derived draw inside its range.

## Boundaries

- Depends on: nothing.
- Used by: callers through the crate root.
- Rules: integer arithmetic and one exact float conversion only, so a stream never depends on the
  target.
