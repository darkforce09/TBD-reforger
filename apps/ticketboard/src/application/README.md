# `application/`

## Responsibility

Composes the native window, owns session state and preferences, coordinates background results, and applies feature events after painting.

## Public surface

`TicketboardApp::new` constructs the eframe application. Feature-view adapters assemble borrowed data and translate feature events into application actions. Workspace reload resolves selected ticket identities against the refreshed corpus.

## Dependency rules

This is the composition layer: it may import every feature. Features never import it. Combined loading belongs here so the registry loader does not depend on waves, metrics, or browsing. Subprocess results are polled without blocking the paint path.

## Files

- [action_dispatch.rs](action_dispatch.rs) — Action dispatch.
- [background_events.rs](background_events.rs) — Background events.
- [background_loading.rs](background_loading.rs) — Background loading.
- [command_execution.rs](command_execution.rs) — Command execution.
- [events.rs](events.rs) — Events.
- [feature_views.rs](feature_views.rs) — Feature views.
- [lifecycle.rs](lifecycle.rs) — Lifecycle.
- [mod.rs](mod.rs) — Module interface and composition.
- [preferences.rs](preferences.rs) — Preferences.
- [shell_screens.rs](shell_screens.rs) — Shell screens.
- [tests/rendering.rs](tests/rendering.rs) — Tests for rendering.
- [tests/window_layout.rs](tests/window_layout.rs) — Tests for window layout.
- [tests/workspace_reload.rs](tests/workspace_reload.rs) — Tests for workspace reload.
- [ticket_command_views.rs](ticket_command_views.rs) — Ticket command views.
- [window.rs](window.rs) — Window.
- [window_layout.rs](window_layout.rs) — Window layout.
- [workspace_reload.rs](workspace_reload.rs) — Workspace reload.
- [workspace_state.rs](workspace_state.rs) — Workspace state.

Unit tests live in sibling `tests/` files declared with `#[cfg(test)]` and an explicit `#[path = "tests/…"]`. Production files contain fewer than 500 raw lines; test files contain at most 1,000.
