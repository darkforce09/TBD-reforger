# Mission validation tests

Unit tests of the validation rules: every rule's firing and silence, the context gates, the
registry's order and its self-check.

## Contents

```text
crates/mission/mission_validation/src/tests/
├── cases_1.rs  the V1 to V4 and ORBAT rules, the self-check and the evaluation order
├── cases_2.rs  the context rules: asset resolution, the loadout policy, vehicle cargo, garment capacity
└── mod.rs      the shared payload fixtures, contexts and cargo catalogue
```

## Boundaries

- Depends on: the crate root through `use super::*`, the rule modules, and `mission_wire_safety`
  for the capacity catalogue.
- Rules: a test builds every payload it reads inline; nothing here reads a file.
