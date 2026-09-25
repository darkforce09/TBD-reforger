**Status:** live

# Ticketboard documentation

The documents on the [ticketboard](/documentation_v2/glossary.md#ticketboard), the native desktop
viewer of the ticket registry in `apps/ticketboard/`. Developers and operators read them below the
crate's code READMEs, for the flows, the reasons and the open work.

## Contents

```text
documentation_v2/ticketboard/
└── ticketboard_viewer.md  opening a repository, browsing, changing a ticket, and the rules behind them
```

## How it works

The [ticketboard viewer](/documentation_v2/ticketboard/ticketboard_viewer.md) document follows the
[feature doc template](/documentation_v2/standards/templates/feature_doc.md). The code READMEs are
exact about the modules: the [crate README](/apps/ticketboard/README.md) for running and checking
the viewer, the [source README](/apps/ticketboard/src/README.md) for the module layout and its
architecture tests, and one README per feature module (`ticket_actions`, `ticket_browser`,
`wave_plan`, `execution_metrics`, `document_viewer`, `repository_status`). The folder mirrors
`apps/ticketboard/` with `apps/` left out.

## Code

- [Ticketboard](/apps/ticketboard/) — the crate the viewer document covers.
- [Ticket actions](/apps/ticketboard/src/ticket_actions/) — the command dispatch and file-change
  guard the document's "Changing a ticket" flow describes.

## Boundaries

- Depends on: the ticketboard code, the `ticket-engine` crate and the `cargo xtask ticket`
  commands it runs, which every claim is checked against; the feature doc template; the ticket
  registry for open work.
- Used by: the `apps/ticketboard/` README, which links this folder under Related documentation;
  the [tooling documentation](/documentation_v2/tools_v2/README.md) index.
- Rules: the document describes the committed code, and a disagreement goes under Known
  discrepancies with both places.

## Related documentation

- [Tooling architecture](/documentation_v2/tools_v2/tooling_architecture.md) — the dependency rule
  that the ticketboard reads tickets through `ticket-engine`.
- [Ticket registry](/.ai/tickets/README.md) — the files, statuses and commands the viewer shows
  and runs.
