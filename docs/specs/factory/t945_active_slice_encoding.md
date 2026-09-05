# T-945 — Ticket encoding: `active_slice` on disk

## Claude Code prompt — T-945
```text
═══ PREFLIGHT ═══ worktree .ai/artifacts/worktrees/T-945, branch slice/T-945, CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══ .ai/tickets/T-945.toml, docs/plans/t-945_plan.md, xtask/src/tickets_store.rs (ALLOWED_NEW, TicketFile, the on-disk key test), xtask/src/schema_gates.rs:1548-1568
═══ PROBLEM ═══ active_slice is read by gate 10 but not encodable on disk; the gate falls back to a hard-coded id.
═══ SHIPPED ═══ T-917.2 (encoding C), T-917.6 (plan gate).
═══ LANGUAGE GATE ═══ Rust + JSON schema only.
═══ LOCKED ═══ Existing keys and their mapping unchanged; one commit for the widening.
═══ DO ═══ Widen; refuse instead of fallback; set T-090's field (the command center re-words the hub header); perturb; gate.
═══ DO NOT ═══ Touch other tickets or docs.
═══ VERIFY ═══ cargo test -p xtask -- tickets_store schema_gates; cargo xtask ticket check; cargo xtask platform wave gate --slice T-945
═══ MANUAL ═══ none
═══ RETURN ═══ Report schema per brief. Ready for Cursor doc sync.
```
