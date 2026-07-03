# T-090.1.2.6 — Claude Code handoff (hillshade blend control)

**Slice:** T-090.1.2.6 · **Executor:** claude-code  
**Branch:** `ticket/T-090` · **Worktree:** `.ai/artifacts/worktrees/TBD-T-090`  
**Depends on:** T-091.2 @ `dde589e` (hillshade overlay) · T-090.1.2.8 @ `db9057ef` (unified satellite)  
**Spec:** [`docs/specs/Mission_Creator_Architecture/t090_1_2_6_hillshade_blend_control.md`](../../docs/specs/Mission_Creator_Architecture/t090_1_2_6_hillshade_blend_control.md)

## Problem

Hard-coded `OPACITY = 0.4` in `useDemLayer.ts` makes Satellite + hillshade look muddy. Operator needs a **Hillshade strength** slider (0–100%) with default 40% for backward compatibility.

## Parallel note

Stream **B** — safe alongside **T-090.1.2.5** on `main` (no shared files). Do **not** edit `scripts/map-assets/**` or `packages/map-assets/**`.

## Implementation

| Step | File / action |
|------|----------------|
| Schema | `state/schema.ts` — `environment.hillshadeOpacity?: number` (0–1) |
| Y.Doc | `updateEnvironment(md, { hillshadeOpacity })` |
| Layer | `useDemLayer.ts` — accept opacity prop; skip layer at 0 |
| Settings | `MissionSettingsDialog.tsx` — slider under Show hillshade |
| Wire | `TacticalMap.tsx` / `MissionCreatorPage.tsx` from store |
| Save | Confirm opacity in compiled `editor` environment block |

## Verify

```bash
cd apps/website/frontend && npm run build && npm run lint
```

Manual **B1–B5** in spec @ Satellite + hillshade on.

Log: `.ai/artifacts/t090_1_2_6_verify_log.md`

Tag **`T-090.1.2.6`**. Return **"Ready for Cursor doc sync."**

## Do not

- Edit docs/registry
- Touch map-assets pipeline (stream A owns that)
