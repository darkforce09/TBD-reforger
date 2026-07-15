# T-159 — Leptos UI rewrite program

**Status:** program hub · **ACTIVE:** **T-159.19** (marquee + drag-move) · **Latest:**
**T-159.18** @ `eb30ebea` (tag **T-159.18**) · **Worktree:**
`.ai/artifacts/worktrees/TBD-T-159/` @ `t-159-leptos-ui`

## Progress (tip `eb30ebea`)

| Milestone | Status |
|-----------|--------|
| Pages + camera 15.x + MissionDoc 16 + persist 17 | shipped |
| **T-159.18** LMB click-select (frozen cam + PointIndex) | `eb30ebea` |
| **T-159.19** Marquee + entity drag-move + persist notify | **ACTIVE** |
| **T-159.20+** Save/export / Eden chrome / cutover | queued |

### Verify logs

- [`.ai/artifacts/t159_17_verify_log.md`](../../.ai/artifacts/t159_17_verify_log.md) — semantic digest; edit-driven persist deferred
- [`.ai/artifacts/t159_18_verify_log.md`](../../.ai/artifacts/t159_18_verify_log.md) — select/clear/Ctrl-toggle; selection = `Rc<RefCell>` (not RwSignal); S8 no encode change

### Next rationale

Complete `useSelectTool` LMB path (marquee + move) before save/export — unlocks
`MissionDocCore::move_entities` + first **edit-driven** `yrs_persist` debounce (S8).

## Slice index

| Slice | Status |
|-------|--------|
| **T-159.18** | shipped `eb30ebea` |
| **T-159.19** | **ready** — `t159_19_marquee_drag.md` |
| **T-159.20+** | queued (save/export next after .19) |
