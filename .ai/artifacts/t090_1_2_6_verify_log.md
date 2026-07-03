# T-090.1.2.6 — verify log (hillshade blend control)

**Slice:** T-090.1.2.6 · **Executor:** claude-code · **Date:** 2026-07-03
**Branch:** `ticket/T-090` (worktree `.ai/artifacts/worktrees/TBD-T-090`, parallel stream B)
**Spec:** `docs/specs/Mission_Creator_Architecture/t090_1_2_6_hillshade_blend_control.md`

## Change summary

- `tactical-map/state/schema.ts` — `environment.hillshadeOpacity?: number` (0–1 float; default 0.4 when undefined, documented inline).
- `tactical-map/layers/useDemLayer.ts` — `const OPACITY = 0.4` removed; hook takes `opacity` (clamped 0–1). Memo split: Horn image on `[terrain, show, version]`, BitmapLayer on `[image, opacity, terrain]` — slider drag swaps the cheap layer only, never re-runs the Horn build. Layer skipped (`null`) at clamped opacity ≤ 0.
- `tactical-map/types.ts` + `TacticalMap.tsx` — new `hillshadeOpacity` prop (default 0.4) threaded to `useDemLayer`.
- `mission-creator/MissionCreatorPage.tsx` — passes `env?.hillshadeOpacity ?? 0.4`.
- `mission-creator/layout/RightInspector/fields.tsx` — new `SliderField` (native range, step 5, mono `%` readout, disabled = dimmed row).
- `mission-creator/layout/MissionSettingsDialog.tsx` — "Hillshade strength" slider under Show hillshade; writes `updateEnvironment(md, { hillshadeOpacity: pct / 100 })`; disabled when hillshade off. `hillshadePercent()` helper keeps eslint complexity ≤ 15.

No ydoc code change needed (`updateEnvironment` already takes `Partial<environment>`). No Horn algorithm / blend-mode change. No map-assets / pyramid / DEM change. No backend / JSON-schema change (`environment` is an open object in `mission-editor-payload.schema.json`).

## Automated gates (all exit 0)

```
cd apps/website/frontend
npm run build   → tsc -b && vite build … ✓ built in 640ms          EXIT 0
npm run lint    → eslint .                                          EXIT 0
npm test -- --run → Test Files 5 passed (5) · Tests 43 passed (43)  EXIT 0
```

(First lint pass flagged `MissionSettingsDialog` complexity 18 > 15 from the added `?.`/`??`
branches — resolved via `hillshadeOn` const + module-level `hillshadePercent()`; re-run clean.)

## Manual acceptance B1–B5

| ID | Status | Notes |
|----|--------|-------|
| **B1** | OPERATOR | Satellite + hillshade on → slider 0% returns `null` layer (relief invisible, code-guaranteed: `clamped <= 0` skip); 100% → `opacity: 1` full grey relief. In-browser look check pending operator. |
| **B2** | CODE-VERIFIED + OPERATOR | Missions without the field render at exactly the prior 0.4: `MissionCreatorPage` passes `env?.hillshadeOpacity ?? 0.4`; `TacticalMap` prop default 0.4; same BitmapLayer params as pre-change. |
| **B3** | CODE-VERIFIED + OPERATOR | Save Version serializes `environment: { ...meta.environment }` (`compiler/compile.ts:172,254`) so `hillshadeOpacity` rides the compiled `editor` block; hydrate sets `environment` wholesale (`state/ydoc.ts:553,664`) → strength restored on reload. Operator: adjust → Save → reload. |
| **B4** | CODE-VERIFIED + OPERATOR | Slider `disabled={!hillshadeOn}` (dimmed row); the opacity field persists independently of the toggle, so re-enabling hillshade uses the last saved strength. |
| **B5** | CODE-VERIFIED + OPERATOR | No extra layers; opacity is excluded from the Horn-image memo deps, so slider drag never rebuilds the 1024px raster — only the BitmapLayer instance swaps. Pan/zoom memo behavior unchanged from T-091.2. |

Operator pass in Mission Creator @ Satellite view with hillshade enabled still required to close B1–B5 visually.
