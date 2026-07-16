# T-159 — Leptos UI rewrite program

**Status:** program hub · **ACTIVE:** **T-159.21** (Eden chrome + undo) · **Latest:**
**T-159.20** @ `c0e11d54` (tag **T-159.20**) · **Worktree:**
`.ai/artifacts/worktrees/TBD-T-159/` @ `t-159-leptos-ui`

## Progress (tip `c0e11d54`)

| Milestone | Status |
|-----------|--------|
| Through **T-159.19** marquee/move | shipped |
| **T-159.20** Save/Export compile + live POST | `c0e11d54` |
| **T-159.21** Eden chrome (strip/toolbelt stubs) + undo/redo | **ACTIVE** |
| **T-159.22+** Outliner / palette / Arsenal / cutover | queued |

### Verify logs (recent)

- [`.ai/artifacts/t159_19_verify_log.md`](../../.ai/artifacts/t159_19_verify_log.md) — soft-WebGPU marquee → `?force=webgl`
- [`.ai/artifacts/t159_20_verify_log.md`](../../.ai/artifacts/t159_20_verify_log.md) — compile Class R; live Save 201/409/400; crate-wide clippy pre-existing red (document only)

### Next rationale

Minimal Save/Export strip exists; next = **Eden docked chrome scaffold** + **undo/redo** wired to
`MissionDocCore` (React `undo.ts` / TopCommandStrip), without full outliner/palette yet.

## Slice index

| Slice | Status |
|-------|--------|
| **T-159.20** | shipped `c0e11d54` |
| **T-159.21** | **ready** — `t159_21_eden_chrome_undo.md` |
| **T-159.22+** | queued |
