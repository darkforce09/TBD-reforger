# T-295 — Realtime collaborative editing

Owner: command center. Builds on T-190 (local CRDT merge). ADR-3 deferred multiplayer v1; this is that v1.

## Claude Code prompt — T-295

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-295 && pwd && git branch --show-current   # must be slice/T-295
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target ; cargo xtask db up
═══ READ ═══
apps/website/api/src/{lib.rs, app.rs:700-800}, api/Cargo.toml, frontend state/{persist.rs, mod.rs}, docs/plans/t-295_plan.md
═══ PROBLEM ═══
No live sync between clients; POST /versions cannot tell a stale save from a fresh one.
═══ SHIPPED ═══
T-190 tab lock + read-merge-write, T-937.4 SaveStatus, T-532 rollback tip — keep all three.
═══ LANGUAGE GATE ═══
Rust only (server + Leptos). No JS.
═══ LOCKED ═══
- CRDT merge only; the server never rewrites documents, it relays frames.
- Offline editing keeps working; ws is additive.
- 409 body names the current head version id.
═══ DO ═══
1. Integration test: two clients diverge on main (no relay) — paste. 2. ws feature + realtime module + route.
3. base_version_id check. 4. collab_sync.rs + registration. 5. Perturb (drop base check) → red → restore → touch → green. 6. Gates.
═══ DO NOT ═══
No git add -A, no git stash, no ci-local; no new external crates without naming them in the report.
═══ VERIFY ═══
cargo test -p website-api realtime ; cargo xtask mk ci-local-leptos ; cargo xtask mk leptos-gates ; cargo xtask platform wave gate --slice T-295
═══ MANUAL ═══
Operator: two browsers on one mission; move a slot in A, see it in B within a second; save from a stale tab → 409 toast.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```
