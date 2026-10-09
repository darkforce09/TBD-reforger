# Change-scoped gate helpers

What a slice or a wave changed, and which crates own those changes: the helpers behind the
change-scoped steps of the platform wave gate (`wasm32 (frontend)`, `fmt (changed)`, the changed
frontend tests) and behind fingerprint invalidation.

## Contents

```text
tools/commands/platform_execution/src/wave_execution/changed/
├── changed_rs.rs                     changed `.rs` files, per-file rustfmt, the wasm scope and its steps
└── include_consumer_package_dirs.rs  workspace members, `include!` consumers, `include_str!` inputs and test-support repository reads
```

## How it works

`tools/commands/platform_execution/src/wave_execution/changed.rs` holds `DEFAULT_BASE`
(`main...HEAD`) and `FRONTEND_DIR` (`crates/frontend/shell/frontend_application`) and re-exports both files.

- `changed_rs(base)` is the union of the committed diff against the base and the working tree's
  changes. A listed path may be a deletion, so each caller decides what absence means.
- `fmt_changed` runs `rustfmt --check` on each changed file that exists, with the edition of the
  crate that owns it (read up to the root manifest when the crate inherits it); a range of
  deletions only is a named skip.
- `wasm_scope_prefixes` starts at the app and at every crate of the frontend family (the members
  under crates/frontend) and walks their dependency edges, `path` ones and `workspace = true`
  ones that name a workspace member (the `crates/` members the frontend depends on), instead of a
  fixed list; the wave gate's trunk build runs only when the range touches that scope.
  `frontend_tests_changed` also counts the scope's
  `include_str!` inputs and the repository files its tests read through the frontend test support
  (`repository_reads_under`: `golden!("<file>")` as `contracts/fixtures/api_goldens/<file>`, a
  forwarding `golden!` as the whole folder, the path argument of `repository_text` and
  `repository_path`), and runs the family's tests into a per-slice private target folder.
- `workspace_members` reads the root manifest's members through `verification_core`, globs such as
  `crates/*/*` expanded and `exclude` entries removed; an unreadable workspace is an error.
- `include_consumer_package_dirs` finds the workspace crates that `include!` a changed fragment
  with no package of its own. `compiled_include_input_paths` lists the JSON, WGSL and SQL files
  that `include_str!` and `include_bytes!` pull in, and every file under a folder prefix a macro
  completes per call site (the golden responses), so touching them invalidates the owning crate.

The slice gate passes no base and gets `main...HEAD`, the slice's own diff inside its worktree.
The wave gate passes `<base>..HEAD` from `super::base`, because `main...HEAD` is empty on merged
`main`.

## Boundaries

- Depends on: `super::host` (host runs), `super::ledger`, the manifest and workspace member readers
  of `repository_laws`, the workspace and crate `Cargo.toml` manifests, `git`,
  `cargo` and `rustfmt`.
- Used by: `super::gate` (both gate drivers) and `super::touch` (`touch_changed`,
  `touch_workspace`).
- Rules: the wasm scope is derived, never hard-coded, and always holds the frontend itself; a
  manifest that cannot be read adds nothing to it, and the gate's `cargo check` step fails on
  such a workspace first; the frontend scope holds the frontend's include inputs and not the
  API's.

## Related documentation

- [Known traps](/documentation/runbooks/factory_waves/known_traps.md) — the editions, the shared cache and the
  vacuous-check traps these helpers guard against.
