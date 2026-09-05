# T-945 — Plan

## Context
Schema gate 10 wants the T-090 hub header to name the registry's active slice, but the on-disk
ticket encoding has no `active_slice` key: the loader drops it and the gate falls back to a
hard-coded id. Found while closing wave 248.

## Approach
1. Add `active_slice` to `ALLOWED_NEW`, `TicketFile`, and `.ai/tickets/schema.json` in one commit.
2. Replace the gate's hard-coded fallback with a refusal naming the missing field.
3. Set T-090's `active_slice` and re-word the hub header; perturb by removing the key.

## Risks
`ticket sync` / `check` code paths that enumerate keys — run the full xtask test suite.

## Verification
`cargo test -p xtask -- tickets_store schema_gates`; `cargo xtask ticket check`; `cargo xtask platform wave gate --slice T-945`.
