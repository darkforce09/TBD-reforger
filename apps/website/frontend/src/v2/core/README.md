# Shared Platform Foundations (`src/v2/core`)

This directory houses zero-business-logic platform primitives shared across all pages and the editor.

---

## Submodules

- **`api/`**: HTTP client with automatic single-flight token refresh, Server-Sent Events (SSE) subscriber, and domain-partitioned DTO models.
- **`auth/`**: Discord OAuth integration, JWT access/refresh token storage, reactive session context, and URL security guards.
- **`ui/`**: The Aegis Design System primitives (< 250 LOC each): Button, TextInput, Select, ModalStack, Tabs, Toast, SplitPane, Icons.
- **`utils/`**: Shared date/time formatting, countdown calculators, and string helpers.

## Invariants
- `core/` must NEVER import from `pages/` or `editor/`.
- UI primitives must be stateless or manage only localized visual state (e.g. dropdown open/closed).
