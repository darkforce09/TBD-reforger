# ORBAT coherency runner

The runner of `cargo xtask verify editor-orbat-coherency`: it applies the static bans and pins and
the cargo test pins that the parent file
`tools_v2/xtask/src/verifications/architecture/editor_orbat_coherency.rs` declares as tables, and
turns each outcome into the gate's output and exit status.

## Contents

```text
tools_v2/xtask/src/verifications/architecture/editor_orbat_coherency/
└── source_audit.rs  the entry point, the static checks, the cargo test pins and their classification
```

## Boundaries

- Depends on: the parent's tables (`BANS`, `PINS`, `CARGO_PINS`) and target paths;
  `verification_core` (`gate::ban`, `gate::require`, `Pattern`, `proc::Run::merged_output`); the
  `regex` crate for the `test result: … N passed` parse; `cargo` on `PATH`, with
  `$HOME/.cargo/bin` put first.
- Used by: the parent module, which re-exports `verify_editor_orbat_coherency` for
  `tools_v2/xtask/src/commands/verify/dispatch.rs`, and the parent's tests, which call
  `static_checks`, `classify`, `passed_counts`, `shown` and `not_run_clause`.
- Rules:
  - the run stops at the first failure, static checks before cargo pins;
  - a missing target file, an unreadable one or a pattern that does not compile is a failure,
    never a pass (`a_missing_target_never_reads_as_a_pass`);
  - a cargo pin fails on a non-zero exit, on output with no `test result:` line, and on zero
    passed tests, so a selector that matches nothing cannot pass
    (`every_cargo_pin_arm_can_go_red`);
  - `FAIL` lines go to stderr with stdout flushed first, and every failure exits 1.
