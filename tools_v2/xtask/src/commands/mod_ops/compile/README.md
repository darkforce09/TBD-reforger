# Headless compile gate internals

The two halves of `cargo xtask mod compile`: the run that boots the native dedicated server over a
throwaway addon directory and reads its verdict from the logs, and the triage that turns those
logs into an exit code.

## Contents

```text
tools_v2/xtask/src/commands/mod_ops/compile/
├── execution.rs              entry points, the server run and its log poll, the Workbench-tooling guard
└── report_compile_errors.rs  argument parsing, error triage, the load-count guard and the log readers
```

## How it works

`tools_v2/xtask/src/commands/mod_ops/compile.rs` holds the options, the help text and the addon
templates, and declares both files with `mod`.

```text
run / run_selftest / run_preflight            (execution.rs)
  └─ run_with_root
       ├─ env checks: host bridge, server binary, tbd-framework addon.gproj  ── exit 3
       └─ compile_inner
            ├─ run dir under $TMPDIR: addons/tbd-framework -> checkout, plus the
            │  selftest and --probe addons when asked
            ├─ ArmaReforgerServer -addonsDir … -addons TBD_Framework[,…] (setsid, timeout)
            ├─ poll console.log / error.log every 300 ms up to TBD_COMPILE_TIMEOUT (180 s)
            ├─ "SCRIPT    (E):" ─▶ report_compile_errors ─────────────────── exit 1
            ├─ load_count_guard: loaded ≥ vanilla baseline + Scripts/Game .c ─ else exit 3
            ├─ workbench_tooling_guard: no tbd-framework/Scripts/WorkbenchGame ─ else exit 1
            └─ "OK: compiled clean" ──────────────────────────────────────── exit 0
```

- The engine process exits 0 even when compilation fails, so the verdict comes only from the log
  lines `Game successfully created` and `SCRIPT    (E):`; a timeout with neither exits 2.
- `report_compile_errors` prints each `file:line: message` whose file exists in the mod or a
  throwaway addon, then up to ten cascaded vanilla errors.
- `load_count_guard` calibrates the vanilla-only file count once, into `.compile-vanilla-baseline`
  at the repository root, by booting the server with no addons.
- `run_selftest` passes only when the deliberately broken selftest addon makes the gate exit 1;
  `run_preflight` checks only that the server binary and a non-empty
  `apps/mod/tbd-framework/resourceDatabase.rdb` exist.

## Boundaries

- Depends on: `crate::commands::mod_ops::compile_host` (host bridge, process-group kill, run-dir
  cleanup on a signal), `crate::core::repository_root`, the `regex` crate, and the Arma Reforger
  dedicated server under `$HOME/.local/share/Steam/steamapps/common/Arma Reforger Server`.
- Used by: `tools_v2/xtask/src/commands/mod_ops/compile.rs`, which re-exports `run`,
  `run_selftest` and `run_preflight` to the `mod` dispatch.
- Rules: an environment fault exits 3 and never 1, so a machine problem cannot read as broken mod
  code (`no_server_is_rc3` and `no_addon_is_rc3` in
  `tools_v2/xtask/src/commands/mod_ops/tests/compile/tests.rs`); the Game-module count takes only
  `.c` files and refuses a missing `Scripts/Game`
  (`count_game_scripts_counts_only_c_and_refuses_a_missing_tree`); the Workbench-tooling guard
  names the folder and what it holds (`workbench_tooling_guard_reports_the_dir_and_what_is_in_it`).
