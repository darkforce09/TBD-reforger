# TBD Reforger Platform Documentation Master Hub

The canonical documentation architecture for the TBD Reforger Platform monorepo. This hierarchy mirrors the codebase directly, providing an unambiguous 1:1 mapping between production code and architectural specifications.

> **Status:** a blueprint. `documentation_v2/` is still the authoritative tree, and every code pin and ticket citation points there, until [`ARCHITECTURE_PLAN.md`](/documentation_v2/archive/documentation_v2_refactor/architecture_plan.md) Phase 3 executes. The hub documents here were derived from `documentation_v2/` on 2026-09-16; [`ANALYSIS_AND_INVENTORY.md`](/documentation_v2/archive/documentation_v2_refactor/analysis_and_inventory.md) §6 lists what changed underneath them since.

## 1. Monorepo Architectural Atlas

```text
documentation_v2/
├── tickets/                                  <-- Living ticket specifications & implementation plans
│   ├── specs/                                <-- Flat repository of all ticket specifications (no subfolders)
│   └── plans/                                <-- Flat repository of all 4-section implementation plans (TEMPLATE.md)
│
├── website/                                  <-- Mirrors apps/website/ (Web platform suite)
│   ├── api/                                  <-- Axum REST & SSE backend, authentication, database
│   ├── graphics_engine/                      <-- Pure WebGPU renderer (wgpu), pipelines, shaders
│   ├── map_engine/                           <-- World spatial computation, 512m chunks, CRDT store
│   └── frontend/                             <-- Leptos 0.8 CSR single-page application (apps, core, pages)
│
├── mod/                                      <-- Mirrors apps/mod/ (Enfusion engine mod suite)
│   ├── tbd_framework/                        <-- Core gameplay mod (game modes, phase manager, radios, loadouts)
│   ├── tbd_export/                           <-- Workbench export plugins (terrain DEM, roads, objects, registry)
│   └── tbd_emcp/                             <-- Enfusion MCP automation bridge (19 NetAPI handlers)
│
├── tools/                                    <-- Mirrors tools_v2/ (Platform tooling suite)
│   ├── xtask/                                <-- Central workspace task runner (`cargo xtask`) & verifications
│   ├── developer_tools/                      <-- Heavy async CLI suite (`developer-tools`: gate, enf, mcpd, world, map, capture)
│   ├── ticket_engine/                        <-- Ticket domain database & TOML serialization (`ticket-engine`)
│   ├── verification_core/                    <-- Fail-closed static verification library (`verification-core`)
│   └── ticketboard/                          <-- Native egui/eframe desktop ticket viewer (`apps/ticketboard/`)
│
├── platform/                                 <-- Mirrors docs/platform/ minus its ticket specs: standards, factory briefs, runbooks, audits, known-bugs
│
├── runbooks/                                 <-- Standard operational runbooks
│   ├── local_development.md                  <-- Host vs container execution, dev-login, cargo wrappers
│   ├── deployment.md                         <-- Staging & production deployment (Docker, Caddy, Systemd)
│   ├── database_operations.md                <-- Sqlx migrations, backups, restore safety, drills
│   └── testing_and_ci.md                     <-- Local CI, headless Chrome CDP test runner, V-suite
│
└── design_system/                            <-- Visual & tactical design specifications
    ├── design_tokens.md                      <-- Dark-only theme, Aegis colorways, typography scale
    └── military_symbology.md                 <-- NATO MIL-STD-2525 symbology and in-game marker palette
```

## 2. Core Architectural Laws

1. **Hard Gate — No Silent Deferrals**: Never invent "out of scope" or defer without explicit operator approval.
2. **Git Discipline — Direct to Main**: All commits land on `main`; git branches are strictly prohibited.
3. **Fundamentals & Clean Architecture**: Simplicity, understandability, and long-term maintainability supersede temporary patches.
4. **Zero Context Needed**: Every directory, file, module, and symbol is self-describing without historical context.
5. **Categorize Variants & Primitives**: Avoid flat dumping of disparate domains; group related tools, pages, and components into dedicated, well-named domains.
6. **Flat Symmetry in Ticket Artifacts**: Both `tickets/specs/` and `tickets/plans/` operate as flat, deterministic namespaces without arbitrary subdirectories, ensuring 100% predictable path resolution for tickets.
7. **Strict Boundary Layers**:
   - `website-graphics-engine`: Pure GPU rendering primitives (knows zero map concepts).
   - `website-map-engine`: Map graphics, spatial BVH, terrain streaming, CRDT scenario store.
   - `website-frontend`: Presentation, navigation, and CAD workspace shells.
   - `website-api`: Axum REST API and SSE real-time broadcast hub.
