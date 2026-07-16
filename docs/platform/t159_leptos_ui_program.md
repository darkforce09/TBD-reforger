# T-159 — Leptos UI rewrite program

**Status:** program hub · **ACTIVE:** **T-159.22** (outliner + asset palette) · **Latest:**
**T-159.21** @ `f02fed5a` (tag **T-159.21**) · **Worktree:**
`.ai/artifacts/worktrees/TBD-T-159/` @ `t-159-leptos-ui`

## Progress (tip `f02fed5a`)

| Milestone | Status |
|-----------|--------|
| Through **T-159.20** Save/Export | shipped |
| **T-159.21** Eden chrome + undo/redo | `f02fed5a` |
| **T-159.22** Editor Layers outliner + Asset palette DnD | **ACTIVE** |
| **T-159.23+** Attributes / ORBAT depth / Arsenal / cutover | queued |

### Verify logs (recent)

- [`.ai/artifacts/t159_20_verify_log.md`](../../.ai/artifacts/t159_20_verify_log.md) — compile Class R; live Save 201/409/400
- [`.ai/artifacts/t159_21_verify_log.md`](../../.ai/artifacts/t159_21_verify_log.md) — chrome + undo Class R; probe inset under docks; CUR manual CDP only

### Next rationale

Chrome shells are empty placeholders. Next = **live Editor Layers outliner** + **Asset palette
drag-to-place** (React T-033 / AssetBrowser), plus the .21 deferred **wheel-over-dock** fix and a
**CUR Class R gate**. Full VirtualOutliner @ 367k and ORBAT Manager stay later.

## Slice index

| Slice | Status |
|-------|--------|
| **T-159.20** | shipped `c0e11d54` |
| **T-159.21** | shipped `f02fed5a` |
| **T-159.22** | **ready** — `t159_22_outliner_asset_palette.md` |
| **T-159.23+** | queued |
