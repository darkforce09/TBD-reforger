# CI schema parity pins

The parts of `cargo xtask verify ci-schema-parity` below its constants: the entry point, the pins
it applies to `.github/workflows/ci.yml`, to the CI task table and to the wave gate sources, and
the comment stripper that runs before any pin. The parent file
`tools_v2/xtask/src/verifications/ci/schema_parity.rs` holds the pinned spellings.

## Contents

```text
tools_v2/xtask/src/verifications/ci/schema_parity/
├── source_audit.rs              the entry point and the workflow, task-table and wave-gate pins
└── strip_yaml_hash_comments.rs  removes `#` comments outside quotes, keeping the line structure
```

## Boundaries

- Depends on: the parent's constants; the CI task table `TASKS`
  (`tools_v2/xtask/src/commands/ci/task_definitions.rs`, reached through `task_runner`), read in
  process;
  `tools_v2/xtask/src/verifications/architecture/wave_gate_sources.rs` for the facade linkage
  check; the `regex` crate.
- Used by: the parent module, which re-exports `verify_ci_schema_parity`; its tests call
  `run_pins`, `task_pins` and `ci_run_is_good`.
- Rules:
  - comments are stripped before every pin (`#` in `ci.yml`, `#` and `//` in the wave sources),
    so a commented-out step or row satisfies nothing (`wave_commented_run_does_not_satisfy`);
  - `gate.rs` must declare and publicly export `checkrun` and `gate_dispatch`, and each linked
    file is read on its own, so a missing or disconnected implementation fails
    (`each_missing_wave_child_fails_the_runtime_gate`,
    `facade_exports_cannot_replace_implementation_bodies`);
  - a missing or unreadable input prints `FAIL:` and exits 1, like every other failure; the gate
    never exits 2.
