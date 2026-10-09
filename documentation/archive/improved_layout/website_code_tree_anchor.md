**Status:** archived — see [the restructure program](/documentation/archive/restructure/README.md)

# Website platform reorganization plan

The code-tree planning anchor for the architectural cleanup and directory reorganization of
`apps/website`. It holds no production code, only this index pointing to the detailed design
specifications in `documentation_v2/`.

## Contents

```text
apps/website/improved_layout/
```

## How it works

This directory anchors the reorganization initiative within the `apps/website/` code tree. In
accordance with the monorepo's documentation placement invariants (enforced by
`cargo xtask verify markdown-placement`, Rule 1), code trees contain only `README.md` files. All
in-depth design proposals, architectural rationale, and execution blueprints live under
`documentation_v2/website/improved_layout/`.

## High-level reorganization summary

```text
apps/website/
├── api_v2/              The API: Axum backend & shared contract models
│   ├── tests/           Domain-grouped integration test suites (reduced link overhead)
│   └── types/           [NEW CRATE] website-api-types: shared wire DTOs (WASM & Linux)
├── deploy/              [NEW] Unified deployment configs: Dockerfile.api, compose dev/staging
├── frontend/            Single-page app with strict one-way architectural hierarchy
│   ├── apps/            Full-screen CAD workspaces (editor, debug; stubs: aar, planner)
│   ├── core/            Decoupled foundations: transport/ (HTTP/SSE), auth/ (session), ui/ (tokens)
│   ├── pages/           Strictly routed document pages (operations, account, etc.)
│   └── shell/           [NEW] Dedicated application chrome: AppLayout, TopNav, Sidebar
├── graphics-engine/     Pure GPU rendering primitives (WGPU/WGSL)
├── improved_layout/     [NEW] This planning index
└── map-engine/          Spatial computation, terrain math, mission domain logic
```

## Related documentation

- [Subsystem master overview](/documentation/archive/improved_layout/website/01_master_overview.md) — the
  primary combined synthesis of all changes across the website platform.
- [Subsystem root structure](/documentation/archive/improved_layout/website/02_website_root_structure.md) —
  peer crates, root clutter removal, and absorption of the ad-hoc `shared/` directory.
- [Unified deploy layout](/documentation/archive/improved_layout/website/03_deploy_layout.md) —
  consolidated container configs, compose topologies, and downstream xtask gate updates.
- [Frontend architecture & cleanliness](/documentation/archive/improved_layout/website/04_frontend_layout.md) —
  four-tier hierarchy (`apps/`, `pages/`, `shell/`, `core/`), inverted dependency fixes, and token decoupling.
- [Frontend transport layout](/documentation/archive/improved_layout/website/05_transport_layout.md) —
  dedicated transport specification: browser HTTP, session refresh, SSE, and DTO elimination.
- [API v2 layout](/documentation/archive/improved_layout/website/06_api_v2_layout.md) —
  dedicated API specification: `website-api-types` crate, test suite consolidation, and database migrations.
- [Engine boundary invariants](/documentation/archive/improved_layout/website/07_engine_boundaries.md) —
  rendering primitives, map domain, and presentation layer rules (`cargo xtask verify engine-layers`).
- [Phased migration roadmap](/documentation/archive/improved_layout/website/08_migration_roadmap.md) —
  step-by-step rollout plan ensuring zero broken CI or xtask verification gates.
