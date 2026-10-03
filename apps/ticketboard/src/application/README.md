# Ticketboard application

The composition layer of the [ticketboard](/documentation/glossary/n_to_z.md#ticketboard):
`TicketboardApp`, the eframe application that owns the session state and the preferences, starts
and drains every background job, paints the feature views, and applies the actions they emit. The
state it changes (the actions, `WorkspaceState`, reloads, the column layout, the preference keys
and the background load) is `ticketboard_model::application_state`, which paints nothing.

## Contents

```text
apps/ticketboard/src/application/
├── action_dispatch.rs       `apply`, which carries out the actions a frame collected
├── background_events.rs     the strict check, `git status` and file-watch jobs, polled each frame
├── command_execution.rs     ticket commands: change guard, single-flight queue, completion
├── feature_views.rs         adapters that lend each feature its borrowed view
├── lifecycle.rs             `TicketboardApp::new`, root adoption, loads, picker, document opening
├── mod.rs                   the module tree and the `TicketboardApp` state
├── shell_screens.rs         the top bar and the no-repository, loading and refusal screens
├── tests/                   the headless rendering tests
├── ticket_command_views.rs  the renderers of ticket-action dialogs, command chip, drawer, toasts
└── window.rs                the `eframe::App` frame and `save`
```

## How it works

`TicketboardApp::new` resolves the repository root: the positional argument when given, else the
first folder holding `.ai/tickets/ROOT` found walking up from the working directory, else the root
saved in the preferences, revalidated. A root without `.ai/tickets/` leaves the no-repository
screen with a note and a native folder picker, which runs on its own thread. Adopting a root arms
the file watch, starts a load and runs the launch strict check.

```text
NoRepo ──root adopted──▶ Loading ──LoadBundle──▶ Board(WorkspaceState)
                            │                         │ watch fire, command exit, Reload
                            └──corpus refused──▶ Refused ──fix on disk, Reload──▶ Loading
```

Each frame, `window.rs` first polls every job without blocking: the load, the picker, the document
read, the strict check, `git status`, the watch debouncer and the running ticket command. It then
paints the status banner, the top bar, the filter bar, the footer, the command drawer, the right
columns and the active tab, each feature through its adapter in `feature_views.rs`, and the open
dialog and toasts. Features emit their own events, which
`ticketboard_model::application_state::events` turns into `Action`s, and `action_dispatch.rs`
applies them after painting.

- Loading: `ticketboard_model::application_state::background_loading` reads the corpus, the wave lock, the run receipts, the
  estimates and the scope vocabulary on one worker thread. A malformed ticket refuses the whole
  corpus, while wave, receipt, estimate and vocabulary failures stay local to their displays.
  Combined loading lives in the application state so the registry loader depends on no consuming
  feature.
- Reloads: `WorkspaceState::reload` rebuilds the cards, tree rows, facets and aggregates, carries
  the filters and the separate measured and estimated sort choices, and finds the selected and
  compared tickets again by id; a watch-triggered reload keeps the board on screen until the new
  data lands, and never closes an open document.
- Ticket commands: every change runs `cargo xtask ticket <verb>` as a subprocess in the repository
  root. A dispatch first re-hashes the ticket file the action was offered for and refuses, then
  reloads, when it changed on disk since. Commands run one at a time from a queue; while one runs,
  the file watch starts no strict check and every mutation control is disabled. On exit the corpus
  and `git status` always reload. A success closes the drawer and toasts its last output line that
  is not cargo's own; a failure keeps the full output open, drops the queued commands, requests one
  strict check and shows the recovery command when the output carries the stale-wave signature.
  The application never runs `wave repack` itself.
- Layout: the document viewer is a resizable column (280 to 1600 points, 560 by default) to the
  right of the 420-point detail column; below a 1100-point window only the viewer shows, and Back
  closes the viewer alone, keeping the selection.
- Preferences: `save` writes the repository root and the viewer width to eframe storage under the
  keys `repo_root` and `viewer_w`, in the user's configuration directory. It is the application's
  only direct write; it never writes under the repository.

## Boundaries

- Depends on: the feature view modules (`ticket_browser`, `ticket_actions`, `wave_plan`,
  `execution_metrics`, `document_viewer`, `repository_status`) and `crate::core`;
  `ticketboard_model` (`application_state`, every feature's models, services and events, and
  `core`'s process and clock helpers); `ticket_model` (`StatusName`) and `repository_layout`
  (`TICKETS_DIR`); the `eframe`, `egui_commonmark` and `rfd` crates.
- Used by: `apps/ticketboard/src/main.rs`, which passes `TicketboardApp::new` to
  `eframe::run_native`.
- Rules: no feature imports this module (the test
  `dependency_boundaries_and_external_test_placement_are_enforced` in
  `apps/ticketboard/src/tests/architecture_rules.rs`); the UI thread never waits on the disk or a
  subprocess; tickets change only through `cargo xtask ticket` commands; `tests/rendering.rs`
  paints every tab, the ticket details, every mutation dialog and the refusal and document states
  headlessly without running a command.
