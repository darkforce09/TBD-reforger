# Frontend Core Foundations (`website/frontend/core/`)

Shared foundational libraries and primitives powering the entire Leptos frontend application.

## Subsystems
- `api/`: Asynchronous HTTP API client, DTO types mirroring backend models, SSE subscription listener, error handling.
- `auth/`: Session token storage, role-based authorization (`admin`, `mission_maker`, `leader`, `enlisted`), route guards.
- `ui/`: Design system primitives (buttons, inputs, cards, dialogs, badges, icons).
- `utils/`: MGRS coordinate formatting, DOM event helpers, time manipulation.

## Code Mapping
- Source: `apps/website/frontend/src/v2/core/`
