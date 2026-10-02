**Status:** live

# Tooling documentation

The documents on the developer tooling in `tools/`: how the crates fit together, and the
subsystems whose flows and reasons go deeper than their code READMEs. Developers and AI agents
start here before changing a command, a check or a tooling crate.

## Contents

```text
documentation/tools/
├── developer_tools/          the Enfusion script oracle and the map raster pipeline
├── ticket_engine/            the token estimate factor behind the ticket registry's estimates
└── tooling_architecture.md   the crates, their dependency direction, invariants and verification surface
```

## How it works

Start with the [tooling architecture](/documentation/tools/tooling_architecture.md): it
lays out the four crates and the npm package, the rules that keep them apart and the tests that
hold those rules. The subfolders mirror the crate folders that have documents of their own:

| Tooling unit | Code README | Documents |
|---|---|---|
| `xtask`, the `cargo xtask` command router and every repository verification | [`tools/xtask/`](/tools/xtask/README.md), with the [command line](/tools/xtask/src/cli/README.md) and [verifications](/tools/xtask/src/verifications/README.md) READMEs | [Tooling architecture](/documentation/tools/tooling_architecture.md) |
| `ticket_engine`, the ticket registry library | [`tools/ticket_engine/`](/tools/ticket_engine/README.md) | [`ticket_engine/`](/documentation/tools/ticket_engine/README.md) |
| `verification_core`, the fail-closed verdict and lock primitives | [`tools/verification_core/`](/tools/verification_core/README.md) | [Tooling architecture](/documentation/tools/tooling_architecture.md) |
| `developer_tools`, the six heavy executables | [`tools/developer_tools/`](/tools/developer_tools/README.md) | [`developer_tools/`](/documentation/tools/developer_tools/README.md) |
| `enfusion_mcp_node_package`, the pinned MCP server | [`tools/enfusion_mcp_node_package/`](/tools/enfusion_mcp_node_package/README.md) | [Enfusion MCP tooling runbook](/documentation/runbooks/enfusion_mcp_tooling.md) |

The [ticketboard](/documentation/ticketboard/README.md), the desktop viewer that links
`ticket_engine`, has its own top-level folder because its code lives in `apps/ticketboard/`.
Procedures that run the tooling are runbooks, not documents here: the
[factory waves](/documentation/runbooks/factory_waves/README.md), the
[editor gates](/documentation/runbooks/editor_gates.md) and
[local development](/documentation/runbooks/local_development.md).

## Code

- [Developer tooling](/tools/) — the four crates and the npm package the architecture
  document describes.
- [Ticket engine](/tools/ticket_engine/) — covered in `ticket_engine/`.
- [Developer tools](/tools/developer_tools/) — covered in `developer_tools/`.

## Boundaries

- Depends on: the code under `tools/` and the tests in `tools/xtask/src/tests/`, which every
  claim is checked against; the feature doc template; the ticket registry for open work.
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
