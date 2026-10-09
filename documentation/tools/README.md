**Status:** live

# Tooling documentation

The documents on the developer tooling in `tools/`: how the crates fit together, and the
subsystems whose flows and reasons go deeper than their code READMEs. Developers and AI agents
start here before changing a command, a check or a tooling crate.

## Contents

```text
documentation/tools/
├── developer_tools/          the executables' index
├── enfusion/                 the Enfusion script oracle behind `enf`
├── map_assets/               the map raster pipeline behind `map`
├── staging/                  the member load and the acknowledgement-dropping relay, end to end
├── tickets/                  the token estimate factor behind the ticket registry's estimates, the ticketboard's documents
└── tooling_architecture.md   the crates, their dependency direction, invariants and verification surface
```

## How it works

Start with the [tooling architecture](/documentation/tools/tooling_architecture.md): it
lays out the crates and the npm package, the rules that keep them apart and the tests that
hold those rules. The subfolders mirror the crate folders that have documents of their own:

| Tooling unit | Code README | Documents |
|---|---|---|
| `xtask`, the `cargo xtask` command router and every repository verification | [`tools/xtask/`](/tools/xtask/README.md), with the [command line](/tools/xtask/src/cli/README.md) and [verify group](/tools/xtask/src/commands/verify/README.md) READMEs | [Tooling architecture](/documentation/tools/tooling_architecture.md) |
| `ticket_model`, `ticket_metrics`, `ticket_wave_lock`, `ticket_registry`, `ticketboard_model` and `ticketboard_desktop`, the ticket crates and the ticketboard | [`tools/tickets/`](/tools/tickets/README.md) | [`tickets/`](/documentation/tools/tickets/README.md) |
| `verification_core`, `process_runner` and `repository_laws`, the foundation crates: verdicts and the lock, child processes, the repository laws | [`tools/foundation/`](/tools/foundation/README.md) | [Tooling architecture](/documentation/tools/tooling_architecture.md) |
| `enfusion_pak`, `enfusion_script_index` and `enfusion_mcp_broker`, the Enfusion crates: the pak reader, the script oracle, the MCP broker | [`tools/enfusion/`](/tools/enfusion/README.md) | [`enfusion/`](/documentation/tools/enfusion/README.md) |
| `staging_load_plan`, `staging_load_generator` and `acknowledgement_dropping_relay`, the staging crates: the member load's plan and its generator, the fault-injecting relay | [`tools/staging/`](/tools/staging/README.md) | [`staging/`](/documentation/tools/staging/README.md) |
| `map_raster_pipeline`, the map asset crate behind `map`: satellite, Map view, labels, water archives and the glyph atlas | [`tools/map_assets/`](/tools/map_assets/README.md) | [`map_assets/`](/documentation/tools/map_assets/README.md) |
| `developer_tools`, the eight tool executables | [`tools/developer_tools/`](/tools/developer_tools/README.md) | [`developer_tools/`](/documentation/tools/developer_tools/README.md) |
| `enfusion_mcp_node_package`, the pinned MCP server | [`tools/enfusion_mcp_node_package/`](/tools/enfusion_mcp_node_package/README.md) | [Enfusion MCP tooling runbook](/documentation/runbooks/enfusion_mcp_tooling.md) |

The [ticketboard](/documentation/tools/tickets/ticketboard_desktop/README.md), the desktop viewer that links
`ticket_model`, has its documents in `tickets/ticketboard_desktop/`, beside its code in
`tools/tickets/ticketboard_desktop/`.
Procedures that run the tooling are runbooks, not documents here: the
[factory waves](/documentation/runbooks/factory_waves/README.md), the
[editor gates](/documentation/runbooks/editor_gates.md) and
[local development](/documentation/runbooks/local_development.md).

## Code

- [Developer tooling](/tools/) — the four crates and the npm package the architecture
  document describes.
- [Ticket crates](/tools/tickets/) — covered in `tickets/`.
- [Developer tools](/tools/developer_tools/) — covered in `developer_tools/`.

## Boundaries

- Depends on: the code under `tools/` and the tooling tests in
  `tools/checks/repository_checks/src/tests/`, which every claim is checked against; the feature doc template; the ticket registry for open work.
- Used by: the READMEs of `tools/` and its four crates, which link these documents under
  Related documentation; the [documentation root](/documentation/README.md).
- Rules: a subfolder mirrors a crate folder's spelling; a document describes the committed code
  and records a disagreement under Known discrepancies; the archived tooling plans stay frozen and
  the live architecture is this folder's.

## Related documentation

- [Tooling architecture plan, archived](/documentation/archive/tools_v2_refactor/architecture_plan.md)
  — the frozen plan the live architecture document carries forward.
- [Coding standards](/documentation/standards/coding_standards/README.md) — the repository-wide
  rules the tooling's structural tests enforce.
