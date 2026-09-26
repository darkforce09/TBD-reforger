# Ticketboard core

The reusable foundations of the [ticketboard](/documentation_v2/glossary/n_to_z.md#ticketboard) that
know nothing about [tickets](/documentation_v2/glossary/n_to_z.md#ticket): subprocess execution and
bounded logs, wall-clock labels, opening a path externally, and the small interface primitives the
features share.

## Contents

```text
apps/ticketboard/src/core/
├── mod.rs    the module tree: `process`, `time` and `ui`
├── process/  streamed subprocesses, the bounded output log, cargo discovery and opening a path
├── time.rs   `epoch_secs` and `utc_hms`, the wall clock as seconds and as an explicit UTC label
└── ui/       the shared accent colours, the output row height and the identifier link
```

## How it works

Everything that waits runs off the UI thread. `process::spawn_streaming` starts a program on
worker threads and hands back a `ProcessHandle` whose channel the application drains once per
frame without blocking; the workers wake the UI through the callback they are given, never
through application state. `process::BoundedLog` keeps the last 500 output lines and counts the
rest, so a pane can say how many earlier lines it dropped. `time::utc_hms` formats seconds since
the Unix epoch as `HH:MM:SS UTC` with no timezone dependency, since the registry's own timestamps
are UTC too. `ui` is the one module here that depends on egui.

## Public surface

- `process::spawn_streaming`, `ProcessHandle`, `ProcessEvent`, `BoundedLog`, `LOG_CAP` and
  `resolve_cargo`: the strict check, `git status` and ticket command runs in `crate::application`,
  the command state in `crate::ticket_actions::models`, and the logs of
  `crate::repository_status`.
- `process::external_open::open_path`: `crate::application`, for the open-path action.
- `time::epoch_secs` and `time::utc_hms`: `crate::application` and
  `crate::repository_status::models::check_status`, which re-exports `utc_hms`.
- `ui`: the colours, `OUTPUT_ROW_H` and `identifier_link`, used by the `ui` modules of every
  feature.

## Boundaries

- Depends on: `std`, and `eframe::egui` in `ui` alone.
- Used by: `crate::application` and the feature modules listed above.
- Rules: core imports no feature module and nothing from `ticket_engine`, and a feature may import
  `core::ui` although it never imports another feature's `ui` (the test
  `dependency_boundaries_and_external_test_placement_are_enforced` in
  `apps/ticketboard/src/tests/architecture_rules.rs`); `process` and `time` stay free of egui.
