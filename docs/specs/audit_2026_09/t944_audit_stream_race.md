# T-944 — Audit stream: id-order race and half-open socket

Follow-on to T-940.6 (wave 248). Both defects were found and disclosed by the slice, with repros.

## Claude Code prompt — T-944
```text
═══ PREFLIGHT ═══ worktree .ai/artifacts/worktrees/T-944, branch slice/T-944, CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══ .ai/tickets/T-944.toml, docs/plans/t-944_plan.md, services/audit_notify.rs, handlers/admin/audit.rs:150-260, tests/audit_notify.rs
═══ PROBLEM ═══ id > last_id watermark skips late-committing rows; half-open socket defeats the poll fallback.
═══ SHIPPED ═══ T-940.6 (triggers, NOTIFY stream, listener with backoff).
═══ LANGUAGE GATE ═══ Rust only.
═══ LOCKED ═══ Keep trigger + migration 0025 untouched; keep the poll fallback; no new deps.
═══ DO ═══ Reproduce both with integration tests (red pasted); dedupe ring + payload-id delivery; heartbeat with deadline; perturb; gate.
═══ DO NOT ═══ Touch events.rs / missions.rs; no docs edits.
═══ VERIFY ═══ TBD_IT_BASE_DB=tbd_slice_t944_it cargo xtask db test-it; cargo xtask platform wave gate --slice T-944
═══ MANUAL ═══ Open the admin audit console; pg_terminate the LISTEN backend; rows keep arriving ≤ 2 s.
═══ RETURN ═══ Report schema per brief. Ready for Cursor doc sync.
```
