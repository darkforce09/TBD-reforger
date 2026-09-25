# Developer tools test fixtures

Committed inputs and blessed outputs that the `developer_tools` library's unit tests load from
disk. They pin the building blueprint compiler and the world line-of-sight model against
[Workbench](/documentation_v2/glossary.md#workbench) recordings of the engine. The reference data
the browser gates compare against lives in `tools_v2/developer-tools/fixtures/`.

## Contents

```text
tools_v2/developer-tools/test_fixtures/
└── blueprint/  Workbench recordings, golden compiler outputs and a synthetic prefab tree
```

## How it works

Nothing here is compiled into the crate. The tests resolve each file from the checkout root at run
time, through `test_repo_root()` or `find_repo_root()` in
`tools_v2/developer-tools/src/repository_paths.rs` and the `fixture(name)` helper of the blueprint
tests, and only read it. `cargo test -p developer-tools` runs every test that reads this tree, and
none needs a game install, a browser or a database. The
[Enfusion](/documentation_v2/glossary.md#enfusion) prefab text here is synthetic; every other file
is recorded from the engine or emitted by the compiler.

## Format

- Encoding: JSON, gzip-compressed JSON lines, one binary occlusion sidecar and Enfusion prefab
  text; the [blueprint fixtures README](/tools_v2/developer-tools/test_fixtures/blueprint/README.md)
  gives each file's schema.
- Schema: the loaders and contracts the tests read the files with, named per file in the child
  README.
- Adding a file: put it in the fixture set of the subsystem that reads it, and add the test that
  loads it; no file here is in Git LFS, so every checkout holds the real bytes.

## Producers and consumers

- Producers: the `tbd-export` Workbench handler `EMCP_WB_TbdBlueprint` for the recordings,
  `cargo xtask map` for the golden outputs, and people for the synthetic prefabs.
- Consumers: the unit tests in `tools_v2/developer-tools/src/blueprint/tests/` and
  `tools_v2/developer-tools/src/map_verification/tests/`, the path-resolution test in
  `tools_v2/developer-tools/src/tests/repository_paths.rs`, and
  `tools_v2/xtask/src/tests/repository_root_tests.rs`, which checks that a fixture here resolves
  from inside `tools_v2/xtask/`.

## Boundaries

- Depends on: the committed Everon assets in `assets_v2/terrains/everon/` that the pins pair with.
- Used by: the tests above; `cargo xtask ci developer-tools-test` runs them in CI.
- Rules: the prose rules of `tools_v2/xtask/src/tests/tooling_prose_rules.rs` exempt this tree from
  the ticket-id and Rust-file-name rules, since the recordings are data; tests address the tree by
  its full path, so a move updates `tools_v2/developer-tools/src/blueprint/tests/module/tests.rs`,
  `tools_v2/developer-tools/src/tests/repository_paths.rs` and every other `git grep test_fixtures`
  hit in the same change.

## Related documentation

- [Developer tools](/tools_v2/developer-tools/README.md) — the crate these fixtures test.
- [Building blueprint compiler](/tools_v2/developer-tools/src/blueprint/README.md) — the compiler
  the blueprint fixtures pin.
