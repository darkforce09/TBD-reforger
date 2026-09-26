**Status:** live

# Ticketboard viewer

The desktop window onto the [ticket](/documentation_v2/glossary/n_to_z.md#ticket) registry: the
[ticketboard](/documentation_v2/glossary/n_to_z.md#ticketboard) shows every ticket on a status board, the
program tree, the recorded [wave](/documentation_v2/glossary/n_to_z.md#wave) lanes and the token metrics,
opens the repository's documents beside them, and changes a ticket only by running the same
`cargo xtask ticket` command a developer would type. Developers and operators use it to see the
state of the work and to move tickets through their statuses.

## Where it lives

- Code: [`apps/ticketboard/`](/apps/ticketboard/README.md), with one README per module under
  [`src/`](/apps/ticketboard/src/README.md); it links
  [`tools_v2/ticket-engine/`](/tools_v2/ticket-engine/README.md) for the ticket model.
- Entry: `cargo run -p ticketboard [-- <repository root>]`; `src/main.rs` opens the window and
  `application::TicketboardApp::new` resolves the root.
- Related features: the [ticket registry](/.ai/tickets/README.md) and its commands; the
  [token estimate factor](/documentation_v2/tools_v2/ticket-engine/token_estimate_factor.md) behind
  the estimated totals; the [factory waves runbook](/documentation_v2/runbooks/factory_waves/README.md)
  that writes the wave lock the Waves tab shows.

## Behaviour

### Opening a repository

1. The root is the positional argument, else the first folder holding `.ai/tickets/` found
   walking up from the working directory, else the root saved in the preferences. A root without
   `.ai/tickets/` shows the no-repository screen with a native folder picker.
2. A worker thread loads the corpus, the wave lock, the run receipts, the estimates and the scope
   vocabulary; the window runs `cargo xtask ticket check --strict` for its status banner and
   `git status` for a chip beside it.
3. A malformed ticket refuses the whole corpus, naming the file with the error verbatim; a broken
   wave lock, receipt, estimate or vocabulary stays local to its own display, so one bad file never
   blanks the board.
4. A file watch on `.ai/tickets/` (and, best effort, the repository root and the roadmap folder)
   reloads after 600 ms of quiet. The board stays on screen until the new data lands, filters and
   the selection carry over by id, and an open document stays open.

### Browsing

- Four tabs, "Board", "Waves", "Tree" and "Metrics": the status columns of every parent and child
  ticket with facet filters; the wave lock's lanes exactly as stored, with ownership collisions;
  the program tree; and the measured receipts beside the estimated tokens.
- A detail column shows the selected ticket, and a resizable document column opens any Markdown
  file of the repository it links, as Markdown or, when it cannot render, as named raw text. A
  document opens only when its path, symbolic links resolved, stays inside the repository.
- Measured and estimated tokens never share a total: they have distinct row and total types, so
  they cannot be added by accident, and an absent folder shows an explicit empty state rather than
  zeros.

### Changing a ticket

1. A card's menu or the detail column's action strip offers only the transitions the ticket's
   status allows: ship, set a status, mark ready, reorder, add a ticket or a child, remove, and
   advance a program's slice.
2. The dialog shows the exact command line and holds a fingerprint of the ticket file taken when
   the action was offered.
3. "Run" re-hashes the file; if it changed on disk, the action is refused with a toast and the
   board reloads. Otherwise `cargo run --package xtask -- ticket <verb> …` runs in the repository
   root, one command at a time from a queue, with every mutating control disabled meanwhile.
4. On exit the corpus and `git status` always reload. A success toasts the last output line that
   is not cargo's own; a failure keeps the full output open, drops the queued commands, requests
   one strict check, and shows `cargo xtask wave repack` as text when the output names a stale
   wave. The viewer never runs `wave repack` itself.

### Known discrepancies

- The Remove dialog lists only dotted-id descendants
  (`apps/ticketboard/src/ticket_actions/services/commands/transitions.rs:195-208`) — `remove
  --force` cascades over `children[]` and parent links (`tools_v2/ticket-engine/src/ops/ordering.rs:5-12`),
  so it can delete a listed child the dialog does not show.
- The Mark ready form asks only for a spec path and mentions only the dependency gate
  (`apps/ticketboard/src/ticket_actions/ui/dialogs/mark_ready.rs:64-106`) — `mark-ready` also
  refuses without a plan document on disk and while a ready-tier body field is empty
  (`tools_v2/ticket-engine/src/ops/readiness.rs:17-31`).
- The `git status` chip reports the gap analysis
  (`apps/ticketboard/src/repository_status/models/git_status.rs:13`) — the file watch does not
  reload on a change to it alone (`apps/ticketboard/src/repository_status/services/file_watch.rs:121-132`).
- The crate README says the viewer reads the registry only through `ticket-engine` — it keeps its
  own copies of the wave-lock types, the scope vocabulary parsing and the receipt and estimate
  checks, the latter without the `TOKENS_PER_LOC` factor rule
  (`apps/ticketboard/src/execution_metrics/estimated/validation.rs`).

## Data

- Reads, through `ticket-engine` where it can: `.ai/tickets/T-*.toml`, the wave lock and its
  history, `.ai/tickets/metrics/<id>/` (run receipts), `.ai/tickets/estimates/` and the scope
  vocabulary; `git status`; any Markdown file inside the repository.
- Runs: `cargo xtask ticket <verb>` for every change and `cargo xtask ticket check --strict` for
  the banner, through the `cargo` that `$CARGO`, `PATH` or `$HOME/.cargo/bin/cargo` names.
- Writes: nothing under the repository itself; its preferences (the last root and the document
  column's width) live in eframe storage in the user's configuration directory.

## Design

- One window, 1500 by 950 at start and 720 by 480 at least: a top bar, a filter bar, the active
  tab, a 420-point detail column and the document column (280 to 1600 points, 560 by default);
  below a 1100-point width only the document column shows, and Back closes it alone.
- Every feature keeps its data free of egui and draws from a view the application lends it, so
  the architecture tests in `apps/ticketboard/src/tests/architecture_rules.rs` can hold the module
  boundaries.
- No design reference set exists; the viewer is a developer tool.

## Open work

- [T-1143 — Fix ticketboard Remove dialog missing descendants remove --force deletes](/.ai/tickets/T-1143.toml)
  (idea, no plan): the Remove dialog lists every ticket the forced removal deletes.
- [T-1144 — Add plan path and ready-tier gates to ticketboard ready form](/.ai/tickets/T-1144.toml)
  (idea, no plan): the Mark ready form asks for the plan path and states every gate `mark-ready`
  applies.
- [T-1141 — Check whether the ticketboard should watch the gap analysis](/.ai/tickets/T-1141.toml)
  (idea, no plan): the chip stays current when only the gap analysis changes, or stops reporting
  it.
- [T-1142 — Decide whether the ticketboard imports the ticket-engine logic it copies](/.ai/tickets/T-1142.toml)
  (idea, no plan): the copies of engine logic go, or are recorded as intended.
- [T-1145 — Cleanup ticketboard comment residue, stale names and misplaced tests](/.ai/tickets/T-1145.toml)
  (idea, no plan): comments, test names and test placement match the code.
- [T-1137 — Gate ticket-engine, verification-core, ticketboard and fleet agent tests and clippy](/.ai/tickets/T-1137.toml)
  (idea, no plan): CI runs the ticketboard's tests and clippy.

## Decisions

- Every change is a `cargo xtask ticket` subprocess: the viewer can never write a ticket file the
  command line would not, and it inherits every validation the commands apply.
- A change is refused when the file moved under it: a second writer (an agent, an editor) wins,
  and the viewer reloads rather than overwrite.
- One malformed ticket refuses the corpus, while other files fail locally: a partial board would
  hide the broken ticket, but a broken metrics file should not hide the board.
- Measured and estimated tokens stay apart in the types: the estimates are derived, and summing
  them with measurements would make both meaningless.
