# Fire mission planning source

The source of `fire_mission_planning`: the battery, the time fuze, the fire-mission assembler, the
client/server comparison rule, the solution wording, and the crate root that exports them.

## Contents

```text
crates/ballistics/fire_mission_planning/src/
├── battery.rs                  `solve_battery`: every gun solved on its own, with its dispersion
├── error.rs                    the crate's `Error` over the fire-mission, battery and fuze errors
├── fire_mission.rs             `solve_fire_mission`: the one assembler of a whole fire-mission solution
├── fire_mission_comparison.rs  `compare_solutions`: the client/server mismatch rule
├── fuze.rs                     `solve_time_fuze_over_charges`: the lowest charge that fuzes, or the precise refusal
├── lib.rs                      the crate root: module header, `mod` lines, the re-exports and the end-to-end mount
├── prelude.rs                  the inputs, solution, battery, comparison and fuze types for glob import
├── solution_wording.rs         `charge_row_words`, `battery_line_words`: a solution in the words an operator reads
└── tests/                      unit tests of each module and the end-to-end sweep over the vanilla catalog
```

## How it works

`fire_mission.rs` calls `battery.rs` for the guns and `fuze.rs` for the burst, and asks
`ballistics_solver` for the lead gun's dispersion and crest clearance; `fire_mission_comparison.rs`
and `solution_wording.rs` read a finished `FireMissionSolution`.

## Boundaries

- Depends on: `ballistics_model` and `ballistics_solver`; `serde`, `thiserror` and `libm`.
- Used by: the API, the mortar calculator, the developer tools' gates and
  `ballistics_agreement_cases`, through the crate root.
- Rules: each module's tests sit in `tests/` under the module's name; `tests/end_to_end.rs` is
  mounted from `lib.rs`.
