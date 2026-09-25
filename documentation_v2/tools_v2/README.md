**Status:** live

# Tooling documentation

The documents on the developer tooling in `tools_v2/`: how the crates fit together, and the
subsystems whose flows and reasons go deeper than their code READMEs. Developers and AI agents
start here before changing a command, a check or a tooling crate.

## Contents

```text
documentation_v2/tools_v2/
├── developer-tools/          the Enfusion script oracle and the map raster pipeline
├── ticket-engine/            the token estimate factor behind the ticket registry's estimates
└── tooling_architecture.md   the crates, their dependency direction, invariants and verification surface
```

## How it works

Start with the [tooling architecture](/documentation_v2/tools_v2/tooling_architecture.md): it
lays out the four crates and the npm package, the rules that keep them apart and the tests that
hold those rules. The subfolders mirror the crate folders that have documents of their own:

| Tooling unit | Code README | Documents |
|---|---|---|
| `xtask`, the `cargo xtask` command router and every repository verification | [`tools_v2/xtask/`](/tools_v2/xtask/README.md), with the [command line](/tools_v2/xtask/src/cli/README.md) and [verifications](/tools_v2/xtask/src/verifications/README.md) READMEs | [Tooling architecture](/documentation_v2/tools_v2/tooling_architecture.md) |
| `ticket-engine`, the ticket registry library | [`tools_v2/ticket-engine/`](/tools_v2/ticket-engine/README.md) | [`ticket-engine/`](/documentation_v2/tools_v2/ticket-engine/README.md) |
| `verification-core`, the fail-closed verdict and lock primitives | [`tools_v2/verification-core/`](/tools_v2/verification-core/README.md) | [Tooling architecture](/documentation_v2/tools_v2/tooling_architecture.md) |
| `developer-tools`, the six heavy executables | [`tools_v2/developer-tools/`](/tools_v2/developer-tools/README.md) | [`developer-tools/`](/documentation_v2/tools_v2/developer-tools/README.md) |
| `enfusion_mcp_node_package`, the pinned MCP server | [`tools_v2/enfusion_mcp_node_package/`](/tools_v2/enfusion_mcp_node_package/README.md) | [Enfusion MCP tooling runbook](/documentation_v2/runbooks/enfusion_mcp_tooling.md) |

The [ticketboard](/documentation_v2/ticketboard/README.md), the desktop viewer that links
`ticket-engine`, has its own top-level folder because its code lives in `apps/ticketboard/`.
Procedures that run the tooling are runbooks, not documents here: the
[factory waves](/documentation_v2/runbooks/factory_waves/README.md), the
[editor gates](/documentation_v2/runbooks/editor_gates.md) and
[local development](/documentation_v2/runbooks/local_development.md).

## Code

- [Developer tooling](/tools_v2/) — the four crates and the npm package the architecture
  document describes.
- [Ticket engine](/tools_v2/ticket-engine/) — covered in `ticket-engine/`.
- [Developer tools](/tools_v2/developer-tools/) — covered in `developer-tools/`.

## Boundaries

- Depends on: the code under `tools_v2/` and the tests in `tools_v2/xtask/src/tests/`, which every
  claim is checked against; the feature doc template; the ticket registry for open work.
- Used by: the READMEs of `tools_v2/` and its four crates, which link these documents under
  Related documentation; the [documentation root](/documentation_v2/README.md).
- Rules: a subfolder mirrors a crate folder's spelling; a document describes the committed code
  and records a disagreement under Known discrepancies; the archived tooling plans stay frozen and
  the live architecture is this folder's.

## Related documentation

- [Tooling architecture plan, archived](/documentation_v2/archive/tools_v2_refactor/architecture_plan.md)
  — the frozen plan the live architecture document carries forward.
- [Coding standards](/documentation_v2/standards/coding_standards/README.md) — the repository-wide
  rules the tooling's structural tests enforce.
