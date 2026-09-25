# Process helpers

The subprocess plumbing of the [ticketboard](/documentation_v2/glossary.md#ticketboard): starting a
program off the UI thread and streaming its output, keeping a bounded copy of that output, finding
the `cargo` binary a desktop session can run, and opening a path with the operating system's
handler.

## Contents

```text
apps/ticketboard/src/core/process/
├── bounded_log.rs      `BoundedLog`, the last `LOG_CAP` (500) output lines and a count of those dropped
├── cargo_discovery.rs  `resolve_cargo`, which finds the cargo binary a bare desktop PATH can miss
├── external_open.rs    `open_path`, which opens a path with `xdg-open`, or `start` on Windows, and never waits
├── mod.rs              the module tree; re-exports the streaming, cargo and log items
├── streaming.rs        `spawn_streaming`, `ProcessHandle` and `ProcessEvent`
└── tests/              unit tests for cargo resolution, the bounded log, streaming and kill
```

## How it works

`spawn_streaming(program, args, cwd, on_event)` returns a `ProcessHandle` at once; the spawn, the
two pipe readers and the exit watcher all run on worker threads. The handle's channel delivers
`ProcessEvent::Line` for each line of standard output or error in arrival order, and one
terminal event: `Exited { code }` (`None` when a signal ended the process), sent 25 ms after the
exit so the readers can flush their last lines first (a best effort for display order), or
`SpawnFailed` with the operating system's error, after which nothing follows. Standard input is
null. The watcher polls `try_wait` every 25 ms instead of waiting for end of file, because a
killed `cargo run` leaves its child holding the pipes open. `ProcessHandle::kill` sends SIGKILL
from any thread, before the spawn lands or after the exit alike. `on_event` fires after every
send; the application passes `egui::Context::request_repaint`.

`resolve_cargo` tries, in order, `$CARGO` when it names a file, the first `cargo` on `$PATH`,
`$HOME/.cargo/bin/cargo`, and finally the bare name `cargo`, whose spawn failure then shows
verbatim in the status banner.

## Boundaries

- Depends on: `std` only (threads, channels and `std::process`).
- Used by: `crate::application`, which runs the strict check, `git status` and every ticket
  command through `spawn_streaming` and opens paths with `open_path`;
  `crate::ticket_actions::models`, which holds the `ProcessHandle` of a running command; and
  `crate::repository_status`, whose models and banner keep a `BoundedLog`.
- Rules: the UI thread never blocks on a child, its pipes or its spawn; each process ends in
  exactly one terminal event (`tests/process.rs`).
