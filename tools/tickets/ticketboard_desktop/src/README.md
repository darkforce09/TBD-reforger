# Ticketboard source

The source tree of the [ticketboard](/documentation/glossary/n_to_z.md#ticketboard) binary: the entry
point, one composition module, the shared interface primitives, and the egui views of six features.
The models, services, events and application state the views paint are in
[`ticketboard_model`](/tools/tickets/ticketboard_model/README.md).

## Contents

```text
tools/tickets/ticketboard_desktop/src/
├── application/        the eframe application: session, preferences, background jobs, dispatch
├── core/               the shared interface primitives: accent colours, row height, identifier link
├── document_viewer/    the document column: Markdown or raw text with its note, Back, open externally
├── execution_metrics/  the Metrics tab: the measured and the estimated tables and strips
├── main.rs             the command line and the native window
├── repository_status/  the status banner: strict check, output pane, `git status` chip, watch notes
├── ticket_actions/     the ticket menus, action strip, mutation dialogs, command chip, drawer, toasts
├── ticket_browser/     the filter bar, status board, cards, program tree and ticket details
└── wave_plan/          the Waves tab: the recorded lanes, wave 0 and the pack-last tickets
```

## How it works

`main.rs` answers `--help` or `-h` with the usage, takes the first other argument as the
repository root, opens a 1500 by 950 window (720 by 480 at least) titled "Ticketboard" and hands
it `application::TicketboardApp::new`. The renderer is wgpu, or glow in a `--features glow`
build.

`application` composes everything else. Each feature folder here holds only `ui/`: the egui views
that paint a narrow borrowed view of that feature's models from `ticketboard_model`. What the
viewer does there comes back as the feature's events (`ticketboard_model::<feature>::events`),
which `ticketboard_model::application_state::events` turns into actions and the application
applies after the frame. The application also hands the browser the ticket menus and the detail
panel's action strip of `ticket_actions` as callbacks, so neither feature imports the other's
`ui`. `core` holds the interface primitives any module may use.

```text
main.rs ──▶ application ──▶ the six feature views: ticket_browser, ticket_actions, wave_plan,
                │            execution_metrics, document_viewer, repository_status
                │                          │ paint models, emit events
                ▼                          ▼
     ticketboard_model (application_state, feature models, services and events, core)

any module ──▶ core (ui)
```

## Public surface

None: the crate is a binary, and no module is visible outside it. The `ticketboard_desktop`
executable and its argument are described in the crate README.

## Boundaries

- Depends on: `ticketboard_model` for every model, service, event and the application state;
  `ticket_model` for the status vocabulary; `ticket_wave_lock` for the collision verdict;
  `repository_layout` for the `.ai/tickets` folder name; `eframe`, `egui_commonmark`,
  `egui_extras` and `rfd`; at run time `cargo xtask ticket` and `git`, run as subprocesses.
- Used by: nothing in the repository links it; people run the binary.
- Rules:
  - the top level holds only `main.rs`, this README and the eight module folders, each with a
    `mod.rs`, every folder but `application` holds only `mod.rs`, its README and `ui/`, and the
    crate README exists;
  - `core` imports no feature and nothing from `ticket_model`; no feature imports `application`
    or `ticketboard_model::application_state`; a feature imports no other feature's `ui`,
    `core::ui` excepted; no source declares an inline module or unit test;
  - a production file stays under 500 lines (`cargo xtask verify file-length` reports it).
