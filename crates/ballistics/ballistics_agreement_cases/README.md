# Ballistics agreement cases

The `ballistics_agreement_cases` crate: a seeded, deterministic lattice of battery fire problems
over a catalog and the one mapping both halves of the native/wasm32 agreement use, so the browser
build of the planner can be compared with the native build bit for bit.

## Contents

```text
crates/ballistics/ballistics_agreement_cases/
├── Cargo.toml  the package: the three other ballistics crates, `deterministic_random`, `newtype_ids`; layout tier 4
└── src/        the case lattice, the case id, the prelude and the tests
```

## How it works

`agreement_cases(catalog, seed, count)` draws `count` cases from a `SplitMix64` stream: case `i`
fires shell `i mod n` of the `n` catalog shells some weapon fires, the weapon drawn among those
that fire it; the target distance spans the shell's whole reach, so any charge (or none) may solve
it; every fourth case is calm. `fire_mission_inputs` turns a case into fire-mission inputs (every
height `manual`, the wind always present), `lead_summary` restates the lead gun's recommended
rings and time of flight, and `case_bit_patterns` / `f64_bit_patterns` record every `f64` of
`{"inputs", "solution"}` by JSON pointer as the 16 hexadecimal digits of its IEEE 754 bits.

## Getting started

Run from the repository root:

```bash
cargo test -p ballistics_agreement_cases   # the reference stream, determinism, coverage, mapping, bit walk
```

## Configuration

None: no features and no environment variables.

## Public surface

- `agreement_cases`, `AgreementCase`, `AgreementCaseId`, `MAX_GUNS_PER_CASE`,
  `MAX_WIND_SPEED_M_S`.
- `fire_mission_inputs`, `lead_summary`, `case_bit_patterns`, `f64_bit_patterns`; `prelude`.

## Boundaries

- Depends on: `ballistics_model`, `ballistics_solver`, `fire_mission_planning`,
  `deterministic_random` and `newtype_ids`; `serde_json` and `libm`.
- Used by: the agreement gate in `tools/browser_testing/browser_gate_suites/src/ballistics_agreement/`
  and the agreement bench in `crates/frontend/workspaces/debug_benches/src/ballistics_agreement/`, which draw,
  map and walk the same cases from the same seed and count.
- Rules: the generator reproduces the published SplitMix64 stream; the same seed draws the same
  cases and covers every fired shell; every drawn case is a valid battery request; the mapping,
  lead summary and bit walk are pinned (`src/tests/case_lattice.rs`).

## Related documentation

- [Ballistics crates](/crates/ballistics/README.md) — the category and its tiers.
- [Ballistics agreement bench](/documentation/crates/frontend/workspaces/debug_benches/ballistics_agreement_page.md)
  — the browser half of the agreement.
