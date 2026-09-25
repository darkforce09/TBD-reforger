# Change-scoped gate helpers

What a slice or a wave changed, and which crates own those changes: the helpers behind the
change-scoped steps of the platform wave gate (`wasm32 (frontend)`, `fmt (changed)`, the changed
frontend tests) and behind fingerprint invalidation.

## Contents

```text
tools_v2/xtask/src/commands/platform/wave_execution/changed/
├── changed_rs.rs                     changed `.rs` files, per-file rustfmt, the wasm scope and its steps
└── include_consumer_package_dirs.rs  workspace members, `include!` consumers and `include_str!` inputs
```

## How it works

`tools_v2/xtask/src/commands/platform/wave_execution/changed.rs` holds `DEFAULT_BASE`
(`main...HEAD`) and `FRONTEND_DIR` (`apps/website/frontend`) and re-exports both files.

- `changed_rs(base)` is the union of the committed diff against the base and the working tree's
  changes. A listed path may be a deletion, so each caller decides what absence means.
- `fmt_changed` runs `rustfmt --check` on each changed file that exists, with the edition of the
  crate that owns it; a range of deletions only is a named skip.
- `wasm_scope_prefixes` walks the frontend crate's path dependencies (today the map engine and the
  graphics engine) instead of a fixed list. `wasm_changed` runs `cargo check` for
  `wasm32-unknown-unknown` when the range touches that scope, and prints a skip otherwise.
  `frontend_tests_changed` also counts the scope's `include_str!` inputs, and runs the frontend
  tests into a per-slice private target folder.
- `include_consumer_package_dirs` finds the crates that `include!` a changed fragment.
  `compiled_include_input_paths` lists the JSON, WGSL and SQL files that `include_str!` and
  `include_bytes!` pull in, so touching them invalidates the owning crate.

The slice gate passes no base and gets `main...HEAD`, the slice's own diff inside its worktree.
The wave gate passes `<base>..HEAD` from `super::base`, because `main...HEAD` is empty on merged
`main`.

## Boundaries

- Depends on: `super::host` (host runs), `super::ledger`, the workspace and crate `Cargo.toml`
  manifests, `git`, `cargo` and `rustfmt`.
- Used by: `super::gate` (both gate drivers) and `super::touch` (`touch_changed`,
  `touch_workspace`).
- Rules: the wasm scope is derived, never hard-coded, and an unreadable crate widens the check
  instead of skipping it; the frontend scope holds the frontend's include inputs and not the
  API's (`the_frontends_include_str_inputs_are_in_scope_and_the_apis_are_not`); the tests are in
  `tools_v2/xtask/src/commands/platform/wave_execution/tests/changed/tests.rs`.
