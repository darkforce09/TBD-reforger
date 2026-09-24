# Ticket Engine (`tools_v2/ticket-engine/`)

The typed ticket database and canonical TOML serialization library (`tools_v2/ticket-engine`).

## Features
- Strongly typed ticket domain schema (`Kind`, `StatusName`, `Executor`, `ScopeV2`).
- Canonical field-order TOML parser and emitter with roundtrip property tests.
- Transactional mutation engine (`mark_ready`, `claim_ticket`, `set_status`).
- Ticket checking, view synchronization, metrics, and the dependency graph compiler remain in `tools_v2/xtask` until phase three.

## Code Mapping
- Source: `tools_v2/ticket-engine/`
