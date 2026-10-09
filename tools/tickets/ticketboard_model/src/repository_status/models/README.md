# Repository status models

The pure state behind the [ticketboard](/documentation/glossary/n_to_z.md#ticketboard)'s status banner:
the progress and verdict of the strict registry check, the `git status` chip over the files the
registry commands write, and the borrowed view the banner paints from.

## Contents

```text
tools/tickets/ticketboard_model/src/repository_status/models/
├── check_status.rs  `CheckModel`, `Coalescer`: the strict check's command, phases, errors and verdict
├── git_status.rs    `GitChip` and `GIT_ARGS`: the `git status --porcelain` query and its parse
├── mod.rs           the module tree
└── view.rs          `StatusView`: check, output, watch state and chip, borrowed for a frame
```

## How it works

The strict check is `CHECK_COMMAND`, `cargo xtask ticket check --strict`, spawned as `CHECK_ARGS`,
the argument list the `xtask` alias in `.cargo/config.toml` expands to. `CheckModel` is fed from
the output stream: `on_start` enters the "building xtask…" phase, `on_line` moves to "checking…"
at cargo's `Running` line or at the first line the check itself prints (`check OK` or an `ERROR: `
line) and counts the `ERROR: ` lines, and `on_exit` or `on_spawn_failed` records the `Outcome`.
Only exit code 0 of a check that ran is green ("check OK — strict · exit 0 · HH:MM:SS UTC"); a red
verdict names the exit code, a kill or the spawn error, and a failure with no `ERROR: ` line points
at the output instead of claiming zero errors.

`Coalescer` keeps a run single-flight: a trigger during a run marks it dirty, and the run's exit
starts exactly one follow-up. The application uses one for the check and one for `git status`.

`GIT_ARGS` asks `git status --porcelain` about `.ai/tickets/`, the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) roadmap, the Eden gap analysis
and `CLAUDE.md`. `chip_from_exit` parses the output only on exit code 0, keeping
porcelain `XY path` lines verbatim and dropping any other line, since standard error is merged in;
any other exit is "git unavailable" with the reason.

## Boundaries

- Depends on: `crate::core::process::BoundedLog` and `crate::core::time::utc_hms`, which
  `check_status` re-exports; `repository_layout` (`TICKETS_DIR`, and `documentation`'s
  `ROADMAP` and `GAP_ANALYSIS`).
- Used by: the desktop application: `tools/tickets/ticketboard_desktop/src/application/` (`mod.rs` holds the
  `CheckModel`, the git `Coalescer` and the `GitChip`; `background_events.rs` spawns the check and
  `git status` with `CHECK_ARGS` and `GIT_ARGS`; `feature_views.rs` builds the `StatusView`) and
  `tools/tickets/ticketboard_desktop/src/repository_status/ui/`, which paints it.
- Rules: `CHECK_ARGS` stays the expansion of `CHECK_COMMAND`; only an observed exit 0 is green;
  a burst of triggers yields one run and one follow-up; no egui type appears here.
