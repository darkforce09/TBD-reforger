# Map asset verification adapters

Thin adapters that forward the map asset checks (the prefab BLAS library, the map-object goldens,
the terrain manifest, and the height, location, town and road labels) to
`developer_tools::map_verification`. The checks and their engine-dependent tests live in that
crate; this folder only finds the repository root and routes the call, so xtask never imports a
map engine type.

## Contents

```text
tools/xtask/src/verifications/map_assets/
└── mod.rs  one adapter per map asset check, each passing the repository root and its arguments
```

## Boundaries

- Depends on: `developer_tools::map_verification` (`blas_manifest.rs`, `object_goldens.rs`,
  `terrain_manifest.rs` and `labels.rs` under `tools/developer_tools/src/map_verification/`),
  and `find_repo_root` in `tools/xtask/src/core/repository_root.rs`.
- Used by:
  - `tools/xtask/src/commands/verify/dispatch.rs`, for `cargo xtask verify blas-manifest`;
  - `tools/xtask/src/commands/schema/dispatch.rs`, for `cargo xtask schema map-object-golden`,
    `schema height-labels`, `schema terrain-alignment`, `schema locations`, `schema town-labels`,
    `schema road-names` and `schema terrain-manifest`, each with `--terrain` defaulting to
    `everon`;
  - `tools/xtask/src/commands/ci/task_definitions.rs` and its `verification_dispatch.rs`, whose
    `schema-validate` task runs the map-object goldens and the Everon height labels, and whose
    `verify-terrain` and `verify-terrain-strict` tasks run the Everon terrain manifest and
    alignment.
- Rules: each adapter returns the exit status the developer_tools check returns, unchanged; xtask
  never depends on `website-map-engine` (`tooling_dependency_direction_is_enforced` in
  `tools/xtask/src/tests/tooling_dependency_boundaries.rs`).
