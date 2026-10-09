**Status:** live

# Ticketboard documentation

The documents on the [ticketboard](/documentation/glossary/n_to_z.md#ticketboard), the native desktop
viewer of the ticket registry in `tools/tickets/ticketboard_desktop/`. Developers and operators read them below the
crate's code READMEs, for the flows, the reasons and the open work.

## Contents

```text
documentation/tools/tickets/ticketboard_desktop/
└── ticketboard_viewer.md  opening a repository, browsing, changing a ticket, and the rules behind them
```

## How it works

The [ticketboard viewer](/documentation/tools/tickets/ticketboard_desktop/ticketboard_viewer.md) document follows the
[feature doc template](/documentation/standards/templates/feature_doc.md). The code READMEs are
exact about the modules: the [crate README](/tools/tickets/ticketboard_desktop/README.md) for running and checking
the viewer, the [source README](/tools/tickets/ticketboard_desktop/src/README.md) for the module layout and its
architecture tests, and one README per feature module (`ticket_actions`, `ticket_browser`,
`wave_plan`, `execution_metrics`, `document_viewer`, `repository_status`), each holding that
feature's egui views. The feature models, services and application state are documented in the
[`ticketboard_model` README](/tools/tickets/ticketboard_model/README.md) and its module READMEs.
The folder mirrors `tools/tickets/ticketboard_desktop/`.

## Code

- [Ticketboard](/tools/tickets/ticketboard_desktop/) — the crate the viewer document covers.
- [Ticketboard model](/tools/tickets/ticketboard_model/) — the headless models, services and
  application state the crate paints.
- [Ticket actions](/tools/tickets/ticketboard_model/src/ticket_actions/) — the command dispatch and
  file-change guard the document's "Changing a ticket" flow describes, with its views in
  [`tools/tickets/ticketboard_desktop/src/ticket_actions/`](/tools/tickets/ticketboard_desktop/src/ticket_actions/).

## Boundaries

- Depends on: the ticketboard code (`tools/tickets/ticketboard_desktop/` and `tools/tickets/ticketboard_model/`),
  the `ticket_model` crate and the `cargo xtask ticket`
  commands it runs, which every claim is checked against; the feature doc template; the ticket
  registry for open work.
- Used by: the `tools/tickets/ticketboard_desktop/` README, which links this folder under Related documentation;
  the [tooling documentation](/documentation/tools/README.md) index.
- Rules: the document describes the committed code, and a disagreement goes under Known
  discrepancies with both places.

## Related documentation

- [Tooling architecture](/documentation/tools/tooling_architecture.md) — the dependency rule
  that the ticketboard reads tickets through `ticket_model`.
- [Ticket registry](/.ai/tickets/README.md) — the files, statuses and commands the viewer shows
  and runs.
