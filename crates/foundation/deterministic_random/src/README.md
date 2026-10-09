# Deterministic random source

The source of `deterministic_random`: the SplitMix64 generator, the two linear congruential
generators and the crate root that exports them.

## Contents

```text
crates/foundation/deterministic_random/src/
├── lib.rs                  the crate root: module header, `mod` lines and the re-exports
├── linear_congruential.rs  `LinearCongruential64` (MMIX) and `LinearCongruential32` (C `rand`)
├── prelude.rs              the three generators for glob import
├── split_mix_64.rs         `SplitMix64`: the seeded counter, its increment, its finaliser and the derived draws
└── tests/                  unit tests of the generators
```

## How it works

`lib.rs` re-exports the three generators at the crate root and in `prelude`.
`tests/split_mix_64.rs` pins the seed-0 reference stream, checks that a seed reproduces its whole
stream, proves the finaliser equal to the draw one increment above the counter and the multiply
and divide spellings of the unit draw bit-identical, and keeps every derived draw inside its
range. `tests/linear_congruential.rs` pins both seed-0 reference streams, checks every draw
against the wrapping step of the previous state, reproducibility per state, and the 32-bit unit
draw as the top 24 bits over `2^24`.

## Boundaries

- Depends on: nothing.
- Used by: callers through the crate root.
- Rules: integer arithmetic and one exact float conversion only, so a stream never depends on the
  target.
