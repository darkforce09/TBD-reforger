# Ballistics agreement cases source

The source of `ballistics_agreement_cases`: the seeded case lattice with its mapping, lead summary
and bit walk, the case identifier, and the crate root that exports them.

## Contents

```text
crates/ballistics/ballistics_agreement_cases/src/
├── case_lattice.rs  `agreement_cases`, `fire_mission_inputs`, `lead_summary`, `case_bit_patterns`
├── ids.rs           `AgreementCaseId`: `<seed as 16 hex digits>_<index as 4 digits>`
├── lib.rs           the crate root: module header, `mod` lines and the re-exports
├── prelude.rs       the case, its id and the four functions for glob import
└── tests/           unit tests of the reference stream, the lattice, the mapping and the bit walk
```

## How it works

`case_lattice.rs` draws from `deterministic_random::SplitMix64` and maps each case through
`fire_mission_planning`; nothing here keeps state between calls.

## Boundaries

- Depends on: `ballistics_model`, `ballistics_solver`, `fire_mission_planning`,
  `deterministic_random` and `newtype_ids`; `serde_json` and `libm`.
- Used by: the developer tools' agreement gate and the single-page app's agreement bench, through
  the crate root.
- Rules: the drawn cases keep their bit patterns across builds; `tests/case_lattice.rs` pins the
  published SplitMix64 reference stream.
