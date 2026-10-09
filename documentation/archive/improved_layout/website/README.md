**Status:** live

# Website platform reorganization specifications

Design proposals, architectural blueprints, and execution roadmaps for restructuring the
`apps/website` subsystem in the TBD Reforger monorepo.

## Contents

```text
documentation/archive/improved_layout/website/
├── 01_master_overview.md        the primary combined synthesis of all website platform changes
├── 02_website_root_structure.md  subsystem root layout, peer crates, and new directories
├── 03_deploy_layout.md          unified deploy/ layout, compose topologies, and xtask updates
├── 04_frontend_layout.md        frontend src/ cleanup, apps vs pages, and map_engine removal
├── 05_transport_layout.md       frontend transport layer, client, refresh, sse, and DTO elimination
├── 06_api_v2_layout.md          API v2 architecture, website-api-types crate, codegen, and database
├── 07_engine_boundaries.md      pure renderer vs map engine vs frontend layer invariants & gates
└── 08_migration_roadmap.md      step-by-step phased execution roadmap ensuring zero CI breakage
```

## How it works

This folder contains the complete, systematic specification for reorganizing `apps/website`. Each
document addresses a distinct architectural boundary or subsystem layer. The proposals are strictly
designed to satisfy all monorepo verification gates (`markdown-placement`, `readme-coverage`,
`engine-layers`, `file-length`, and `staging-compose-paths`).

The plan preserves working code and avoids disruptive import churn by focusing on clear directory
responsibilities, unified deployment configurations, shared typed wire contracts, and clean module naming.

## Documents

1. **[Master overview](01_master_overview.md)**: Combined synthesis of all changes, high-level Before
   vs. After directory matrix, and core design principles.
2. **[Website root structure](02_website_root_structure.md)**: Top-level layout of `apps/website/`,
   peer crate relationships, and elimination of root configuration clutter.
3. **[Deploy layout](03_deploy_layout.md)**: Unification of dev and staging compose files, Dockerfile,
   and necessary downstream xtask gate updates.
4. **[Frontend layout](04_frontend_layout.md)**: Frontend architecture (`apps/`, `pages/`, `core/`),
   removing empty `map_engine` directories, and clarifying session auth boundaries.
5. **[Transport layout](05_transport_layout.md)**: Dedicated frontend transport specification:
   renaming `core/api` to `core/transport`, browser fetch, tab session coordination, SSE telemetry,
   and eliminating duplicate DTOs.
6. **[API v2 layout](06_api_v2_layout.md)**: Dedicated API specification: establishing the shared
   `website-api-types` crate, single source of truth for contracts, database migrations SHA-384
   immutability, and domain services.
7. **[Engine boundaries](07_engine_boundaries.md)**: Pure GPU rendering primitives vs. domain logic vs.
   presentation layer rules (`cargo xtask verify engine-layers`).
8. **[Migration roadmap](08_migration_roadmap.md)**: Phased rollout steps for future execution sessions
   to prevent broken builds or CI failures.

## Related documentation

- [Website documentation](/documentation/mod/README.md) — index of platform documents.
- [Code-tree anchor](/documentation/archive/improved_layout/website_code_tree_anchor.md) — code-tree index file.
