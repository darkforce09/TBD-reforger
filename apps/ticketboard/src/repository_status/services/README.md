# File watch

The [ticketboard](/documentation_v2/glossary/n_to_z.md#ticketboard)'s watch on the registry files: the
`notify` watches that report a relevant change, and the debouncer that turns a burst of changes into
one reload and, when no ticket command is running, one strict check.

## Contents

```text
apps/ticketboard/src/repository_status/services/
├── file_watch.rs  `spawn` and `WatchHandle`, the `relevant` path filter, and the `Debouncer`
├── mod.rs         the module tree
└── tests/         unit tests for bursts, suppression, the trailing window and the path filter
```

## How it works

`spawn(root, tx, on_event)` arms three watches, on `.ai/tickets/`, on the repository root and on
the folder of the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) roadmap, and
returns a `WatchHandle` that keeps them alive:

| Watch | Mode | Relevant paths | On failure |
|---|---|---|---|
| `.ai/tickets/` | recursive | every file: tickets, `wave.lock`, receipts | `Err`: "watch unavailable" |
| the repository root | non-recursive | `CLAUDE.md` only | listed in `degraded`: "watch degraded" |
| the roadmap's folder | non-recursive | the roadmap file only | listed in `degraded` |

The folder watches let a file replaced by rename still be seen, and `relevant` drops their other
files so build output and sibling documents cause no reloads. Read events are ignored. A relevant
event sends `()` on `tx` and calls `on_event`, a repaint request.

`Debouncer` works on milliseconds the caller passes in, with no clock or thread of its own. A burst
fires once, `DEBOUNCE_MS` (600 ms) after its last event, and `due_in` tells the application when to
wake. Every fire reloads; it also runs the strict check only when an event of the burst arrived
unsuppressed and past the trailing window, and suppression is off at fire time. The application
sets suppression while a ticket command runs; clearing it opens a trailing window of one debounce
span, so the command's last writes start no check.

## Boundaries

- Depends on: `ticket_engine::repository` (`TICKETS_DIR` and `documentation::ROADMAP`); the
  `notify` crate; `std::sync::mpsc`.
- Used by: `crate::application`: `arm_watch`, `poll_watch` and `set_verb_in_flight` in
  `apps/ticketboard/src/application/background_events.rs`, and the `Debouncer` held in
  `apps/ticketboard/src/application/mod.rs`.
- Rules:
  - only a failure of the `.ai/tickets/` watch is an error; the other two degrade visibly;
  - a fire during a ticket command reloads without a check, and a real edit mixed into command
    residue keeps its check (`suppressed_fires_reload_only`,
    `mixed_burst_across_the_trailing_edge_keeps_its_check`,
    `suppression_at_fire_time_beats_a_checkworthy_event` in `tests/file_watch.rs`);
  - the path filter matches the watched files only
    (`relevance_filter_matches_the_watched_surfaces_only`); the `notify` shell in `spawn` has no
    test of its own.
