# Repository status

The [ticketboard](/documentation_v2/glossary.md#ticketboard) feature that tells the viewer whether
the [ticket](/documentation_v2/glossary.md#ticket) registry on disk can be trusted: the verdict of
`cargo xtask ticket check --strict`, a `git status` chip over the files the ticket commands write,
and the file watch that reloads the board when those files change.

## Contents

```text
apps/ticketboard/src/repository_status/
├── events.rs  `StatusEvent`: re-check, cancel the check, toggle the output, toggle the changed-file list
├── mod.rs     the module tree
├── models/    the strict check's phases and verdict, the coalescer, the `git status` chip, `StatusView`
├── services/  the `notify` watches, the path filter and the debouncer with its suppression rules
└── ui/        the status banner: verdict, output, watch state and chip
```

## How it works

The feature holds the rules; the application holds the processes and the timing. The application
spawns the strict check at launch, on "Re-check", on a debounced watch fire and after a failed
ticket command, and spawns `git status` after every reload and every check exit, each through its
own `Coalescer` so bursts yield one run and one follow-up. It feeds the output line by line into
`CheckModel` and the finished `git status` into `chip_from_exit`.

```text
.ai/tickets/**, CLAUDE.md, the roadmap ──notify──▶ Debouncer (600 ms quiet)
                                                     │ fire
                                                     ├──▶ reload the corpus, wave lock, receipts, estimates
                                                     └──▶ strict check, unless a ticket command runs
strict check exit, reload ──▶ git status ──▶ GitChip
strict check output ──▶ CheckModel ──▶ StatusView ──▶ ui::trust_banner_ui ──▶ StatusEvent
```

While a ticket command runs, the application suppresses the watch's checks and keeps its reloads,
and one debounce span after the command the trailing window stays silent too. Only an observed
exit 0 turns the banner green. A failure of the `.ai/tickets/` watch shows "watch unavailable"; a
failure of the root or roadmap watch shows "watch degraded". The watch does not cover the Eden gap
analysis, which the `git status` chip does.

## Public surface

- `models::check_status`: `CheckModel`, `Coalescer`, `CHECK_ARGS` and `utc_hms`, which
  `crate::application` holds and uses to spawn and time the check.
- `models::git_status`: `GitChip`, `GIT_ARGS` and `chip_from_exit`, for the application's
  `git status` run.
- `services::file_watch`: `spawn`, `WatchHandle` and `Debouncer`, which the application arms,
  keeps and polls.
- `models::view::StatusView`, `ui::trust_banner_ui` and `events::StatusEvent`: the banner, which
  the application lends the view, paints and turns into actions.

## Boundaries

- Depends on: `crate::core` (`process::BoundedLog`, `time::utc_hms`, the `ui` colours and row
  height); `ticket_engine::repository` (`TICKETS_DIR`, `documentation::ROADMAP` and
  `documentation::GAP_ANALYSIS`); the `notify` crate; `eframe::egui` in `ui/` only; at run time,
  `cargo xtask ticket check --strict` and `git`, which the application spawns.
- Used by: `crate::application` (`mod.rs`, `background_events.rs`, `feature_views.rs` and
  `events.rs` in `apps/ticketboard/src/application/`).
- Rules:
  - the viewer never reimplements the check: it runs the command and reports its exit, and only
    exit 0 is green (`apps/ticketboard/src/repository_status/models/tests/check_status.rs`);
  - a ticket command's own writes never start a strict check
    (`apps/ticketboard/src/repository_status/services/tests/file_watch.rs`);
  - `models/` and `services/` name no egui type, and no other feature imports `ui/`
    (`dependency_boundaries_and_external_test_placement_are_enforced` in
    `apps/ticketboard/src/tests/architecture_rules.rs`).
