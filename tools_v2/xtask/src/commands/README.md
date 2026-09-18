# Domain command adapters

Command domains own their Clap declarations, dispatch, and operational handlers. The top-level CLI composes these domains; shared repository discovery, host execution, target-directory handling, and test utilities live in `../core/`.

## Repository operations

- `db/`: local database lifecycle, seeds, integration tests, and milestone announcements.
- `deploy/`: website and staging deployment, database backup, restore, and restore drills.
- `setup/`: Workbench, client addons, server profiles, staging servers, and MCP game-root setup.
- `fetch/`: vanilla source and API reference extraction.
- `mod_ops/`: mod compilation, development and playtest servers, mission tests, world boot checks, and mod wave execution.
- `mcp/`: daemon lifecycle, JSON-RPC calls, NetAPI access, smoke checks, and Workbench logs.
- `debug/`: diagnostic probes, direct joins, and remote logs.
- `reproduction/`: reproduction fixtures and mission-version upload workflows.

## Build and platform orchestration

- `build/`: build and development-server recipes.
- `ci/`: task definitions and execution, Chromium installation, and editor API support.
- `platform/`: platform preflight, slice execution and worktrees, and wave execution with gate and merge handling.
- `agent_context/`: agent-context command dispatch and guards.

## Contracts and domain services

- `generate/`: schema-driven code generation.
- `schema/`: mission flattening and schema command dispatch.
- `verify/`: verification CLI adapters; checks themselves live under `../verifications/`.
- `ticket/`: ticket-engine adapters plus host-side agent execution and cleanup.
- `wave/`: ticket-engine adapters for lock compilation, checking, and collision selection.
- `map/`: terrain-export orchestration and adapters to developer-tools asset processing.

Ticket persistence and wave-lock services belong to ticket-engine. Engine-backed map verification and blueprint compilation belong to developer-tools. Command adapters pass repository paths to these libraries rather than duplicating their implementations. Domain tests live in sibling `tests/` directories.
