# Document viewer

The [ticketboard](/documentation_v2/glossary/n_to_z.md#ticketboard) feature that opens a repository
Markdown document, such as a [ticket](/documentation_v2/glossary/n_to_z.md#ticket)'s spec, plan or a
citation, in a read-only column beside the ticket details, and shows it as Markdown or, when it
cannot, as raw text with a note saying why.

## Contents

```text
apps/ticketboard/src/document_viewer/
├── events.rs  `DocumentEvent`: close the viewer, or open a path with the operating system's handler
├── mod.rs     the module tree
├── models/    `ViewerState` and the read outcomes it lands
├── services/  the viewer click predicate, the repository fence and the bounded read on a worker thread
└── ui/        the document column: header, progress, Markdown or raw-text fallback
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

The column paints from that state through `ui::viewer_pane_ui`, and its Back and "open externally"
buttons come back as `DocumentEvent`s, which the application turns into `CloseViewer` and
`OpenPath` actions. The application owns the Markdown render cache and the saved column width
(280 to 1600 points, 560 by default).

## Public surface

- `services::document_loading`: `wants_viewer`, which the ticket details of `crate::ticket_browser`
  call to choose the viewer or the external handler; `spawn_read`, which `crate::application`
  starts; and the re-exported `ViewerState` and `LoadedDocument`, which the application holds.
- `ui::viewer_pane_ui`: the column, which `crate::application` paints.
- `events::DocumentEvent`: `CloseViewer` and `OpenPath`, converted into the application's actions.

## Boundaries

- Depends on: `crate::core::ui` (`OUTPUT_ROW_H`); `std` for files, threads and channels;
  `eframe::egui` and `egui_commonmark` in `ui/` only. It is the one feature besides `core` that uses
  nothing from `ticket_engine`.
- Used by: `crate::application` (`mod.rs`, `lifecycle.rs`, `background_events.rs`,
  `action_dispatch.rs`, `events.rs` and `feature_views.rs` in `apps/ticketboard/src/application/`);
  `crate::ticket_browser`'s detail panel, through `wants_viewer`.
- Rules:
  - no document outside the repository root is read, by `..` or by a symbolic link, and no read
    passes 512 KB (`apps/ticketboard/src/document_viewer/services/tests/document_loading.rs`);
  - a stale read never replaces the open document, and closing the viewer never changes the ticket
    selection;
  - `models/` and `services/` name no egui type, and no other feature imports `ui/`
    (`dependency_boundaries_and_external_test_placement_are_enforced` in
    `apps/ticketboard/src/tests/architecture_rules.rs`).
