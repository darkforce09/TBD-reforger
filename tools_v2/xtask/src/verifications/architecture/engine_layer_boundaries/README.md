# Engine layer gate runner

The runner of `cargo xtask verify engine-layers`: it proves each matcher on known subjects, walks
the three crates the rules read, and judges the eight engine-layer rules whose matchers, pins and
report text sit in `tools_v2/xtask/src/verifications/architecture/engine_layer_rules.rs`.

## Contents

```text
tools_v2/xtask/src/verifications/architecture/engine_layer_boundaries/
├── boundary_evaluation.rs   judges rules 1 to 7 over the walked files and writes the PASS or FAIL line
└── verify_engine_layers.rs  the entry point: matcher self-probes, the crate walks, the scanned counts
```

## Boundaries

- Depends on: the parent module `engine_layer_boundaries.rs` with its two sibling files
  (`engine_layer_rules.rs` for the matchers and pins, `engine_layer_scan.rs` for walking,
  pin comparison and refusals); `verification_core` (`scan::walk_files`,
  `scan::matching_lines`, `Pattern`, `gate::probe_str`, `NotRun`).
- Used by: the parent module, which re-exports `verify_engine_layers` for
  `tools_v2/xtask/src/commands/verify/dispatch.rs` and for the `verify-engine-layers` row of
  `tools_v2/xtask/src/commands/ci/task_definitions.rs` (through
  `task_definitions/verification_dispatch.rs`); the parent's tests call `run`.
- Rules:
  - output goes into a line buffer that `verify_engine_layers` prints, so the tests assert on the
    exact lines;
  - every matcher is probed on subjects with known answers before it judges source, and a probe
    that answers wrongly fails the run (exit 1);
  - a missing root or manifest (`apps/website/graphics-engine`, `apps/website/map-engine/src`,
    `apps/website/frontend`) is "did not run" (exit 2), and an empty walk or an empty
    `data/` or `world/` tree is a failure (`inputs_that_were_never_read_do_not_pass`,
    `an_absent_half_of_the_wall_is_not_a_clean_wall` in
    `tools_v2/xtask/src/verifications/architecture/tests/engine_layer_boundaries.rs`).
