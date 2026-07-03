# T-090.1.2.6 — Satellite + hillshade blend control

**Ticket:** T-090 · **Slice:** T-090.1.2.6  
**Status:** **SHIPPED** @ `b958e3b4` — Mission Settings hillshade strength slider (0–100%, default 40%)  
**Executor:** claude-code  
**Depends on:** **T-091.2** shipped @ `dde589e` (hillshade overlay); **T-090.1.2.1** shipped @ `19bc785` (Satellite pyramid)  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md) · [`t090_basemap_dual_view.md`](t090_basemap_dual_view.md)

---

## In one sentence

Let the operator **tune how strongly hillshade blends over the Satellite basemap** — a Mission Settings control (slider or stepped input) instead of the hard-coded ~40% opacity that reads poorly on photo ortho.

---

## Problem

With **Satellite** basemap and **Show hillshade** both enabled, relief is composited as a semi-transparent grey `BitmapLayer` on top of color ortho tiles (`useDemLayer.ts`, `OPACITY = 0.4`). Operator feedback: the **blend / transparency is not great** — terrain reads flat or muddy; no way to dial relief in for taste or lighting without turning hillshade off entirely.

T-091.2 locked a single global opacity for the minimum bar (on/off). This slice adds **user-adjustable strength** while keeping the same Horn/NW hillshade algorithm and 1024 px downsample.

---

## Goal

1. **Mission Settings** — when hillshade is enabled (or always visible but disabled when off), expose **Hillshade strength** control: **0–100%** (maps to Deck `BitmapLayer` opacity).
2. **Default 40%** when unset — preserves current behavior for existing missions (`showHillshade` true with no stored opacity).
3. **Live preview** — slider updates overlay without reload; no pyramid or DEM rebuild.
4. **Persist** on mission via `meta.environment.hillshadeOpacity` (0–1 float or 0–100 int — pick one, document in `schema.ts`).
5. Works on **Satellite** today; automatically applies when **Map** view ships (**T-090.1.1**) since hillshade sits above either basemap.

---

## UX (normative)

```text
Mission Settings
  [x] Show hillshade
  Hillshade strength   [====●====]  40%     ← disabled/grey when Show hillshade off
```

- Control type: slider + numeric readout (Aegis tokens; match existing `ToggleField` / `Field` patterns).
- Step: 5% or continuous — implementer choice; clamp 0–100%.
- At **0%** with hillshade on: effectively invisible relief (layer may skip render at 0).
- At **100%**: full hillshade RGBA over basemap (may look harsh — allowed).

---

## Implementation notes

| Area | Direction |
|------|-----------|
| **Schema** | `environment.hillshadeOpacity?: number` in `state/schema.ts` — optional, default `0.4` when `showHillshade` |
| **Y.Doc** | `updateEnvironment(md, { hillshadeOpacity })` alongside existing hillshade toggle |
| **Layer** | `useDemLayer.ts` — replace `const OPACITY = 0.4` with prop from `MissionCreatorPage` / store |
| **Settings UI** | `MissionSettingsDialog.tsx` — slider under Show hillshade toggle |
| **Types** | `MissionMeta['environment']` + frontend `types/` if mirrored |
| **Save** | Included in compiled `editor` block via existing environment serialize — no backend model change unless mission row stores environment (verify existing path) |

**Out of scope:** blend modes (multiply vs normal), per-terrain defaults, separate Satellite vs Map opacity, light-azimuth control, re-tuning Horn algorithm.

---

## Manual acceptance

| ID | Pass |
|----|------|
| **B1** | Satellite + hillshade on — slider 0% → relief invisible; 100% → strong grey relief |
| **B2** | Default mission (no opacity field) — matches prior ~40% look |
| **B3** | Adjust slider → Save Version → reload editor → strength restored |
| **B4** | Hillshade off — strength control disabled; toggling on uses last saved strength |
| **B5** | Pan/zoom perf unchanged (no extra layers) |

Log: `.ai/artifacts/t090_1_2_6_verify_log.md`

---

## Verification gate

```bash
cd apps/website/frontend && npm run build && npm run lint
```

Manual B1–B5 in Mission Creator @ Satellite view with hillshade enabled.

**Shipped:** @ `b958e3b4` (tag **T-090.1.2.6**) · verify [`.ai/artifacts/t090_1_2_6_verify_log.md`](../../../.ai/artifacts/t090_1_2_6_verify_log.md) · operator visual pass on B1 still open.

---

## File touch list (expected)

- `apps/website/frontend/src/features/tactical-map/state/schema.ts`
- `apps/website/frontend/src/features/tactical-map/layers/useDemLayer.ts`
- `apps/website/frontend/src/features/tactical-map/TacticalMap.tsx` (wire opacity prop)
- `apps/website/frontend/src/features/mission-creator/layout/MissionSettingsDialog.tsx`
- `apps/website/frontend/src/features/mission-creator/MissionCreatorPage.tsx` (if not via store selector)

No map-assets / pyramid / DEM export changes.

Handoff: [`.ai/artifacts/t090_1_2_6_claude_code_handoff.md`](../../../.ai/artifacts/t090_1_2_6_claude_code_handoff.md) · send-off [`.ai/artifacts/t090_1_2_6_SEND_TO_CLAUDE.md`](../../../.ai/artifacts/t090_1_2_6_SEND_TO_CLAUDE.md)

---

## Claude Code prompt — T-090.1.2.6 (copy-paste)

Extract: `./scripts/ticket prompt T-090 --slice T-090.1.2.6`

```
Read CLAUDE.md first.

Implement **T-090.1.2.6** — hillshade blend strength slider for Satellite basemap.

═══ PREFLIGHT ═══
  cd /path/to/TBD-T-090 worktree (branch ticket/T-090)
  ./scripts/ticket brief T-090

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t090_1_2_6_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t090_1_2_6_hillshade_blend_control.md
  3. apps/website/frontend/src/features/tactical-map/layers/useDemLayer.ts
  4. apps/website/frontend/src/features/mission-creator/layout/MissionSettingsDialog.tsx

═══ PROBLEM ═══
  Satellite + hillshade uses hard-coded OPACITY=0.4 in useDemLayer.ts — relief reads muddy
  on photo ortho with no operator control. Add Mission Settings slider (0–100%) persisted
  on meta.environment. Work in **worktree** `.ai/artifacts/worktrees/TBD-T-090` (parallel stream B).

═══ SHIPPED (do not reopen) ═══
  - T-091.2 @ dde589e — hillshade overlay layer
  - T-090.1.2.8 @ db9057ef — unified satellite basemap

═══ LOCKED ═══
  - Default 0.4 when showHillshade on and field unset (backward compatible)
  - Store as meta.environment.hillshadeOpacity (0–1 float, document in schema.ts)
  - Slider disabled when Show hillshade off; live preview without reload
  - Same Horn/NW algorithm — no blend-mode changes
  - No map-assets / pyramid / DEM rebuild
  - No docs/registry edits

═══ DO ═══
  1. schema.ts — optional environment.hillshadeOpacity; ydoc updateEnvironment path
  2. useDemLayer.ts — replace const OPACITY with prop from store/page
  3. MissionSettingsDialog.tsx — slider + numeric readout under Show hillshade
  4. Wire TacticalMap / MissionCreatorPage as needed
  5. Ensure Save Version round-trips opacity in compiled editor block
  6. .ai/artifacts/t090_1_2_6_verify_log.md — B1–B5
  7. Tag **T-090.1.2.6** · prefix **T-090.1.2.6:**

═══ DO NOT ═══
  - Edit docs/**, `.ai/tickets/registry.json`, CLAUDE status markers
  - Touch scripts/map-assets/** or packages/map-assets/**
  - Change Horn algorithm or add multiply blend modes

═══ VERIFY (all exit 0) ═══
  cd apps/website/frontend && npm run build && npm run lint

═══ MANUAL ═══
  B1: slider 0% → invisible relief; 100% → strong grey relief
  B2: default mission matches prior ~40% look
  B3: Save Version → reload → strength restored
  B4: hillshade off disables slider; on uses last saved strength
  B5: pan/zoom perf unchanged

═══ RETURN ═══
  - Commit SHA + tag T-090.1.2.6
  - Automated verify output (PASS)
  - Manual notes for B1–B5
  - **Ready for Cursor doc sync.**
```
