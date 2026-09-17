# CI Workflow Verifications (`verifications/ci`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Validates GitHub Actions workflow files and task integrity.

---

## Verifications

- **`ci_workflow_shell.rs`** (formerly `verify_ci_shell.rs` & `verify_ci_shell_rules.rs`): Enforces that GitHub Actions workflow YAML files contain zero inline shell scripting and delegate exclusively to `cargo xtask ...`.
- **`ci_schema_parity.rs`** (formerly `gate_t468.rs`): Pin verifying that CI test workflows and wave runners execute all schema gates and cannot be hollowed out.
