# Mod script checks

Checks over the [EnfScript](/documentation/glossary/a_to_f.md#enfscript) sources and `.layout` files of
the `tbd-framework` [mod](/documentation/glossary/g_to_m.md#mod): the structural gate for UI
layouts, and the two [Workbench](/documentation/glossary/n_to_z.md#workbench) spawn checks behind `cargo xtask mod`.

## Contents

```text
tools/checks/mod_script_checks/src/
├── error.rs                               `Error` and `Result`: a check that could not find the checkout or read a file
├── lib.rs                                 the crate root: the checks' shared contract, `mod` lines and the re-exports
├── prelude.rs                             the `verify` check's entry point for glob import
├── spawn_determinism.rs                   `mod spawn-determinism`: preflight, offline selftest, log normalising
├── spawn_determinism_live.rs              the live Workbench runs of spawn-determinism and their comparison
├── spawn_verification.rs                  `mod spawn-verify`: a 25 s Workbench play, judged by `mcp wb-logs`
├── tests/                                 unit tests of the layout parser, the layout names and spawn determinism
├── ui_layout_parser/                      the parser's record, keyword and conversion helpers
├── ui_layout_parser.rs                    `Analyzer`: arms C1 to C4 and C6 over one `.layout` text
└── ui_layouts.rs                          `verify ui-layouts`: walks the layouts, runs the parser and arm C5
```

## How it works

The layout check reads committed files under `mod/tbd-framework/` from the checkout root, prints
its own report and writes no file.

| Check | Reads | Holds |
|---|---|---|
| `ui-layouts` | the `.layout` files directly in `mod/tbd-framework/UI/layouts/` (not its subfolders), and every file under `mod/tbd-framework/Scripts/Game/TBD/UI/` | C1 brace balance, C2 attested slot classes, C3 frame slot geometry, C4 container children declare a slot, C5 every widget name a script looks up exists, C6 a container child's slot sets its alignment |

The `ui-layouts` walk does not descend: every committed layout sits in a subfolder of
`mod/tbd-framework/UI/layouts/` (`Common/`, `Hud/` or `Session/`), so the check finds
no file and exits 1 with `FAIL: no .layout files under …`.

`spawn_determinism.rs` and `spawn_verification.rs` are not `verify` verbs: `cargo xtask mod`
dispatches `spawn-determinism` and `spawn-verify` to them, and both drive a running Workbench.
spawn-determinism first checks that the Workbench Net API listens on `ENFUSION_WORKBENCH_PORT`
(5775 by default), then compares the normalised spawn and equip log lines of several plays of one
world; spawn-verify plays the open world through `cargo xtask mcp call` and returns the verdict of
`cargo xtask mcp wb-logs`. `--selftest` runs either offline.

Exit codes of `verify ui-layouts`: 0 held; 1 a violation or a missing file; 2 a check that did
not run (a missing layout folder).

## Public surface

- `ui_layouts::verify_ui_layouts`: the `cargo xtask verify ui-layouts` entry, taking the checkout
  root.
- `spawn_determinism::run` and `spawn_verification::run`: the bodies of
  `cargo xtask mod spawn-determinism` and `cargo xtask mod spawn-verify`.
- At the crate root: `Error` and `Result`; `prelude`: the `verify` entry.
- Crate-private: the `ui_layout_parser`.

## Boundaries

- Depends on: `verification_core` (`Pattern`, `Verdict`, `NotRun`, `gate`, `scan`);
  `process_runner` (`Run`, `Merged`); `content_digest` (the run digests); `regex`;
  `repository_root` (the checkout root); `repository_layout` (the spawn-determinism runbook);
  `thiserror`; `cargo xtask mcp` subprocesses and `ss` for the spawn checks. `spawn_verification`
  runs its `cargo xtask` children through `process_runner` too: the selftest replaces this process
  (`Run::replace_process`), and the live arm's children share the terminal (`Run::terminal`) so
  their report streams live.
- Used by:
  - `tools/xtask/src/commands/verify/dispatch.rs` and
    `tools/commands/mod_operations/src/mod_dispatch.rs`.
- Rules:
  - A missing layout folder never reads as a pass.
  - The printed report is part of each check's contract.

## Related documentation

- [Spawn determinism](/documentation/runbooks/spawn_determinism.md) — running the Workbench
  spawn checks.
- [Mod command group](/tools/commands/mod_operations/src/README.md) — the `mod spawn-determinism`
  and `mod spawn-verify` commands.
