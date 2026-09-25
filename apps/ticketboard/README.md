# Ticketboard

The `ticketboard` crate: the native egui/eframe desktop viewer of the
[ticket](/documentation_v2/glossary.md#ticket) registry in `.ai/tickets/`. It shows every parent
and child ticket on a status board, the recorded [wave](/documentation_v2/glossary.md#wave) lanes,
the program tree, measured run receipts and historical estimates, with each ticket's details and
the repository's Markdown documents beside the board. Developers and operators run it on a
desktop.

## Contents

```text
apps/ticketboard/
├── Cargo.toml  the `ticketboard` package: one binary, the `glow` feature, the `ticket-engine` path dependency
└── src/        the entry point, the application, the shared core and the seven feature modules
```

## How it works

The viewer reads the registry only through the `ticket-engine` crate in `tools_v2/ticket-engine/`
and changes it only by running `cargo xtask ticket <verb>` as a subprocess, one command at a time,
behind a check that the ticket file has not changed since the action was offered. It writes no
file under the repository: its one direct write is its preferences, kept in eframe storage in the
user's configuration directory.

On start it finds the repository root, loads the ticket corpus, the wave lock, the run receipts,
the estimates and the scope vocabulary on a worker thread, and runs `cargo xtask ticket check
--strict` for the status banner beside a `git status` chip. A file watch on `.ai/tickets/` (and,
best effort, the repository root and the roadmap folder) reloads the board after 600 ms of quiet.
A malformed ticket refuses the whole corpus with the file named and the error verbatim, while a
missing or broken wave lock, receipt or estimate stays local to its own tab. Measured receipts and
estimated tokens never share a total; wave lanes show the lock exactly as stored; and a document
opens only when its path, symbolic links resolved, stays inside the repository, with a named
raw-text fallback when it cannot render as Markdown. `src/README.md` maps the modules.

## Getting started

Run these from the repository root; the viewer needs a desktop display and a graphics driver.

```bash
cargo run -p ticketboard                          # opens the window; stays in the foreground
cargo run -p ticketboard -- /path/to/checkout     # a named repository root instead of discovery
cargo run -p ticketboard --features glow          # the OpenGL renderer, for drivers wgpu fails on
cargo run -p ticketboard -- --help                # the usage
```

Check the crate with:

```bash
cargo fmt -p ticketboard --check
cargo clippy -p ticketboard --locked --all-targets --all-features -- -D warnings
cargo test -p ticketboard --locked                # temporary fixtures; no window, no ticket command
cargo test -p ticketboard --locked -- --ignored   # the three tests that read the live corpus, estimates and wave lock
cargo build -p ticketboard --locked --features glow
cargo xtask verify file-length
```

## Configuration

- The positional argument: the repository root, which must contain `.ai/tickets/`. It wins over
  discovery even when it lacks that folder, and the window then says so. Without it, the viewer
  walks up from the working directory to the first folder holding `.ai/tickets/`, and then tries
  the root saved in the preferences.
- The `glow` feature: builds eframe's glow backend and selects it; the default build uses wgpu.
- Preferences, in eframe storage under the user's configuration directory: `repo_root`, the last
  adopted repository root, revalidated at the next start; `viewer_w`, the document column's
  width, clamped to 280 to 1600 on load and 560 by default.
- The environment: `CARGO`, `PATH` and `HOME` locate the `cargo` binary the ticket commands run
  (`$CARGO`, then `cargo` on `PATH`, then `$HOME/.cargo/bin/cargo`), since a desktop session's
  `PATH` can miss it.

## Public surface

- The `ticketboard` binary: `ticketboard [REPO_ROOT]`, or `--help` and `-h` for the usage. It
  exits when the window closes. There is no library target.

## Boundaries

- Depends on: `tools_v2/ticket-engine/` for the ticket model, validation, repository paths and the
  wave lock format; `cargo xtask ticket` and `git`, run as subprocesses; the files under
  `.ai/tickets/`; the `eframe`, `egui_commonmark`, `egui_extras`, `notify`, `rfd`, `serde`,
  `serde_json`, `time` and `toml` crates.
- Used by: people at a desktop; no crate or command in the repository runs it.
- Rules: every ticket change goes through a `cargo xtask ticket` command, and the viewer never
  runs `wave repack` itself; the module layout, the dependency directions and the file-size limits
  hold under `src/tests/architecture_rules.rs`, whose `source_inspection.rs` reads grouped imports
  and aliases and ignores comments and string literals; `cargo xtask verify file-length` covers
  `src/` too.

## Related documentation

- [Ticket registry](/.ai/tickets/README.md) — the ticket files, statuses and commands the viewer
  shows and runs.
- [Factory waves](/documentation_v2/runbooks/factory_waves/README.md) — how waves are packed and
  run.
