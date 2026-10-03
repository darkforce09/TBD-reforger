# Map engine integration suites

The map engine's crate-root integration test folder. It holds no suite: the three headless document
suites it held drive only the mission crates and run as integration suites of `mission_operations`
([its suites](/crates/mission/mission_operations/tests/README.md)). The folder stays until the map
engine's remaining tiers leave the crate.

## Contents

```text
legacy/map_engine/tests/
└── README.md  this file
```

## Boundaries

- Depends on: nothing; the folder holds no test.
- Used by: nobody; `cargo test -p map_engine --all-features` runs the crate's unit tests only.
- Rules: an integration suite that drives only crates under `crates/` lives with the crate it
  drives, not here. The camera suites run in their own crate (`crates/geometry/camera_math/tests/`).
