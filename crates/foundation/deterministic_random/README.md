# Deterministic random

The `deterministic_random` crate: SplitMix64, the seeded pseudo-random generator behind every
draw in the workspace that has to come out the same on every run, machine and target.

## Contents

```text
crates/foundation/deterministic_random/
├── Cargo.toml  the package: no dependencies, layout tier 0
└── src/        the generator, its prelude and its tests
```

## How it works

`SplitMix64::new(seed)` holds one 64-bit counter. Each `next_u64` adds the golden-ratio increment
`0x9E37_79B9_7F4A_7C15` and mixes the counter through the published SplitMix64 finaliser, so seed
0 yields the reference stream `0xE220_A839_7B1D_CDAF`, `0x6E78_9E6A_A1B9_65F4`,
`0x06C4_5D18_8009_454F`. The derived draws are:

- `next_unit`: the top 53 bits scaled by `2^-53`, a value in `[0, 1)`. Scaling an integer below
  `2^53` by a power of two is exact, so the same seed gives the same bits on every target.
- `next_in(low, span)`: `low + span * next_unit()`, a value in `[low, low + span)`.
- `next_index(bound)`: `next_u64() % bound`, with a zero bound drawing `0`.

The generator is small, fast and well distributed, and it is not cryptographic.

## Getting started

Run from the repository root:

```bash
cargo test -p deterministic_random   # the reference stream, reproducibility and the draw ranges
```

## Configuration

No features and no environment variables.

## Public surface

- `SplitMix64` (`Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`), also in `prelude`: `new(seed)`,
  `next_u64`, `next_unit`, `next_in(low, span)`, `next_index(bound)`.

## Boundaries

- Depends on: nothing.
- Used by: nothing yet. It replaces the two copies of the generator in the map engine: the
  ballistics agreement lattice and the scatter of the mission editing placement patterns.
- Rules: the seed-0 reference stream is pinned (`split_mix_64_matches_the_reference_stream`), and
  so is the bit equality of the multiply and divide spellings of the unit draw
  (`both_unit_spellings_give_identical_bits`); foundation tier, so the crate depends on no
  workspace crate (`cargo xtask verify crate-tiers`).

## Related documentation

- [Engine boundary rules](/documentation/standards/engine_boundary_rules.md) — the dependency
  directions between the workspace crates.
