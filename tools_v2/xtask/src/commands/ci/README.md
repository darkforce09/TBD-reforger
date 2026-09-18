# Ci

Build and CI composites use the ordered task definitions in `task_definitions.rs`. `task_runner.rs` resolves and runs tasks; leaf adapters, shell execution, and environment setup have separate modules.

Source modules: `chromium_install.rs`, `editor_api.rs`, `mod.rs`, `task_definitions.rs`, `task_runner.rs`.
