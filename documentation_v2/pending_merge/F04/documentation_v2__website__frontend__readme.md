# Frontend Leptos Application (`website/frontend/`)

The frontend application is a client-side rendered (CSR) Rust WebAssembly SPA built with Leptos 0.8 and Trunk, running on port 3000.

## Architectural Organization (`src/v2/`)
- `apps/`: Standalone high-performance CAD workspaces (Scenario Editor, Tactical Planner, AAR Telemetry Player, Diagnostics).
- `pages/`: Standard platform navigation, account management, community operations, mission hub, and admin panels.
- `core/`: Shared infrastructure across all frontend surfaces: API client, authentication session, and reusable UI design tokens.

## Visual Design Reference Colocation
All 26 legacy macOS visual design blueprints and Mission Creator mockups are colocated directly within their corresponding feature directories in this hierarchy with explicit historical reference notices.

## Code Mapping
- Source: `apps/website/frontend/src/v2/`
- Styles: `apps/website/frontend/style/aegis.css`
