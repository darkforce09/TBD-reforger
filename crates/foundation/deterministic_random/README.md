# Deterministic random

The `deterministic_random` crate: SplitMix64, the seeded pseudo-random generator behind every
draw in the workspace that has to come out the same on every run, machine and target, and the two
linear congruential generators whose exact streams seeded outputs are pinned to.

## Contents

```text
crates/foundation/deterministic_random/
├── Cargo.toml  the package: no dependencies, layout tier 0
└── src/        the generators, the prelude and their tests
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

`SplitMix64::INCREMENT` and `SplitMix64::finalise(word)` are public for callers that step a Weyl
sequence of their own (the loadout Apply seed) or mix a word into a key (the loadout Apply draw,
the staging load pacing streams); `finalise(word)` is the draw of a generator whose counter sits
one increment below `word`.

`LinearCongruential64` (Knuth's MMIX multiplier `6364136223846793005` and increment
`1442695040888963407`) and `LinearCongruential32` (the C library's `rand` multiplier
`1103515245` and increment `12345`) step `state × multiplier + increment`, wrapping, and draw the
new state; `LinearCongruential32::next_unit` is the top 24 bits over `2^24`. They exist because
seeded outputs are pinned to their exact streams: the mission document's seeded slot positions,
and the render diagnostics' stress scene (whose first instances are pinned bit for bit) and
compute cull icon field. New code draws from `SplitMix64`.

The generators are small and fast, and none is cryptographic.

## Getting started

Run from the repository root:

```bash
cargo test -p deterministic_random   # the reference stream, reproducibility and the draw ranges
```

## Configuration

No features and no environment variables.

## Public surface

- `SplitMix64` (`Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`), also in `prelude`: `INCREMENT`,
  `new(seed)`, `finalise(word)`, `next_u64`, `next_unit`, `next_in(low, span)`,
  `next_index(bound)`.
- `LinearCongruential64` and `LinearCongruential32` (same derives), also in `prelude`:
  `MULTIPLIER`, `INCREMENT`, `new(state)`, `next_u64` / `next_u32`; the 32-bit one also
  `next_unit`.

## Boundaries

- Depends on: nothing.
- Used by: `formation_geometry` (placement scatter), `ballistics_agreement_cases` (case
  lattice), `ballistics_solver` (tests), `mission_operations` (Apply seed), `mission_document`
  (seeded slots), `mission_creator_arsenal` (Apply draw), `map_render_diagnostics` (stress scene,
  compute cull field), `staging_load_plan` (pacing streams), `map_asset_verification`
  (line-of-sight bench) and `world_export_pipeline` (tests).
- Rules: the seed-0 reference streams are pinned (`split_mix_64_matches_the_reference_stream`,
  `linear_congruential_64_matches_the_mmix_reference_stream`,
  `linear_congruential_32_matches_the_c_library_reference_stream`); foundation tier, so the crate depends on no workspace crate (`cargo xtask verify crate-tiers`).

## Related documentation

- [Crate boundary rules](/documentation/standards/crate_boundary_rules.md) — the dependency
  directions between the workspace crates.
