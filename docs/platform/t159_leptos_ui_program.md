# T-159 — Leptos UI rewrite program

**Status:** program hub · **ACTIVE:** **T-159.20** (save / export) · **Latest:**
**T-159.19** @ `f444b878` (tag **T-159.19**) · **Worktree:**
`.ai/artifacts/worktrees/TBD-T-159/` @ `t-159-leptos-ui`

## Progress (tip `f444b878`)

| Milestone | Status |
|-----------|--------|
| Through **T-159.18** click-select | shipped |
| **T-159.19** marquee + drag-move + edit-driven persist | `f444b878` |
| **T-159.20** Save Version + Export compile | **ACTIVE** |
| **T-159.21+** Eden chrome / undo / cutover | queued |

### Verify logs (recent)

- [`.ai/artifacts/t159_18_verify_log.md`](../../.ai/artifacts/t159_18_verify_log.md)
- [`.ai/artifacts/t159_19_verify_log.md`](../../.ai/artifacts/t159_19_verify_log.md) — marquee/move; edit persist;
  **CI note:** `upload_marquee` + headless WebGPU `mappedAtCreation` panic → smoke uses `?force=webgl`

### Next

Save/export before full Eden chrome — proves compile + auth POST path from Leptos doc.

## Slice index

| Slice | Status |
|-------|--------|
| **T-159.19** | shipped `f444b878` |
| **T-159.20** | **ready** — `t159_20_save_export.md` |
| **T-159.21+** | queued |
