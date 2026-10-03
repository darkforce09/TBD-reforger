# Document viewer

The state and reads behind the [ticketboard](/documentation/glossary/n_to_z.md#ticketboard) feature
that opens a repository Markdown document, such as a
[ticket](/documentation/glossary/n_to_z.md#ticket)'s spec, plan or a citation, in a read-only column
beside the ticket details. The desktop application paints the column from
`apps/ticketboard/src/document_viewer/ui/`, as Markdown or, when it cannot, as raw text with a note
saying why.

## Contents

```text
tools/tickets/ticketboard_model/src/document_viewer/
├── events.rs  `DocumentEvent`: close the viewer, or open a path with the operating system's handler
├── mod.rs     the module tree
├── models/    `ViewerState` and the read outcomes it lands
└── services/  the viewer click predicate, the repository fence and the bounded read on a worker thread
```

## How it works

A click on a `.md` spec, plan or citation path in the ticket details emits the application's
`OpenDoc` action. The application's `open_doc` puts the `ViewerState` into `Loading` for that
repository-relative path and starts `spawn_read`, replacing any read in flight. The worker resolves
the path inside the repository, reads at most 512 KB and sends back a `LoadedDocument`; the
application polls it each frame and `land`s it, which applies it only when the viewer still waits
for that same path.

```text
Closed ──open(rel)──▶ Loading(rel) ──land(rel, outcome)──▶ Rendered(rel) or Fallback(rel)
   ▲                     │  a later open(rel2) restarts at Loading(rel2); the old read is dropped
   └───── close (Back) ──┴──────────────────────────────────────────────────┘
```

The application's column (`apps/ticketboard/src/document_viewer/ui/document_column.rs`) paints from
that state, and its Back and "open externally" buttons come back as `DocumentEvent`s, which
`crate::application_state::events` turns into `CloseViewer` and `OpenPath` actions. The application owns the Markdown render cache and the saved column width
(280 to 1600 points, 560 by default).

## Public surface

- `services::document_loading`: `wants_viewer`, which the ticket details of `crate::ticket_browser`
  call to choose the viewer or the external handler; `spawn_read`, which the desktop application
  starts; and the re-exported `ViewerState` and `LoadedDocument`, which the application holds.
- `events::DocumentEvent`: `CloseViewer` and `OpenPath`, converted into `Action`s by
  `crate::application_state::events`.

## Boundaries

- Depends on: the crate's `Error` (`DocumentRefused`); `std` for files, threads and channels. It is
  the one feature besides `core` that uses nothing from `ticket_model`.
- Used by: `crate::application_state::events`; the desktop application:
  `apps/ticketboard/src/document_viewer/ui/`, the files of `apps/ticketboard/src/application/` that
  open, poll, close and lend the viewer, and the ticket details in
  `apps/ticketboard/src/ticket_browser/ui/detail_panel/` (`cells.rs`, `body_sections.rs`), through
  `wants_viewer`.
- Rules:
  - no document outside the repository root is read, by `..` or by a symbolic link, and no read
    passes 512 KB (`tools/tickets/ticketboard_model/src/document_viewer/services/tests/document_loading.rs`);
  - a stale read never replaces the open document, and closing the viewer never changes the ticket
    selection;
  - nothing here names egui
    (`model_dependency_boundaries_and_external_test_placement_are_enforced` in
    `tools/tickets/ticketboard_model/src/tests/architecture_rules.rs`).
