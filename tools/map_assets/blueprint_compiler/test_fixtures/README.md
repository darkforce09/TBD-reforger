# Blueprint compiler test fixtures

Committed inputs and blessed outputs that the `blueprint_compiler` crate's unit tests, and the
world line-of-sight tests of `map_asset_verification`, load from disk. They pin the
building blueprint compiler and the world line-of-sight model against
[Workbench](/documentation/glossary/n_to_z.md#workbench) recordings of the engine.

## Contents

```text
tools/map_assets/blueprint_compiler/test_fixtures/
└── blueprint/  Workbench recordings, golden compiler outputs and a synthetic prefab tree
```

## How it works

Nothing here is compiled into the crate. The tests resolve each file from the checkout root at run
time, through `repository_root::find_repository_root` and the crate's `test_fixtures::fixture(name)`
helper, and only read it. `cargo test -p blueprint_compiler` and `cargo test -p map_asset_verification`
run every test that reads this tree, and none needs a game install, a browser or a database. The
[Enfusion](/documentation/glossary/a_to_f.md#enfusion) prefab text here is synthetic; every other file
is recorded from the engine or emitted by the compiler.

## Format

- Encoding: JSON, gzip-compressed JSON lines and Enfusion prefab text; the [blueprint fixtures README](/tools/map_assets/blueprint_compiler/test_fixtures/blueprint/README.md)
  gives each file's schema.
- Schema: the loaders and contracts the tests read the files with, named per file in the child
  README.
- Adding a file: put it in the fixture set of the subsystem that reads it, and add the test that
  loads it; no file here is in Git LFS, so every checkout holds the real bytes.

## Producers and consumers

- Producers: the `tbd-export` Workbench handler `EMCP_WB_TbdBlueprint` for the recordings,
  `cargo xtask map` for the golden outputs, and people for the synthetic prefabs.
- Consumers: the unit tests of `tools/map_assets/blueprint_compiler/src/` and
  `tools/map_assets/map_asset_verification/src/tests/`, and
  `tools/foundation/tool_test_support/src/tests/test_checkout_root_tests.rs`, which checks that a fixture here resolves
  from inside `tools/xtask/`.

## Boundaries

- Depends on: the committed Everon assets in `assets/terrains/everon/` that the pins pair with.
- Used by: the tests above; `cargo xtask ci workspace-member-tests` runs them in CI, through this
  crate's and the map asset verification crate's tests.
- Rules: tests address the tree by its full path, so a move updates
  `tools/map_assets/blueprint_compiler/src/test_fixtures.rs` and every other `git grep test_fixtures` hit in the same change.

## Related documentation

- [Blueprint compiler](/tools/map_assets/blueprint_compiler/README.md) — the crate these fixtures test.
- [Building blueprint compiler](/tools/map_assets/blueprint_compiler/src/README.md) — the compiler
  the blueprint fixtures pin.
