# T-152.20 — Mission Settings world-layer toggle completeness

**Ticket:** T-152 · **Slice:** T-152.20 (remediation ladder #9)
**Status:** `queued`
**Executor:** **claude-code** (Claude Code)
**Authority:** T-152 program hub · audit [`t152_11_fidelity_audit_report.md`](../../../.ai/artifacts/t152_11_fidelity_audit_report.md) §10 A15 (O10)
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152` · branch `ticket/T-152` · tag **`T-152.20`**
**Depends on:** none (parallel-safe; O10 in T-152.22 depends on this)

## In one sentence

Expose the 7 missing world-layer toggles (roads, buildings, forest, trees, props, contours, sea) in the Mission Settings dialog so every `WorldClassToggles` pref is operator-controllable and O10 becomes executable — pure dumb-UI wiring over plumbing that already exists.

---

## Problem

Audit A15: `WorldClassToggles` defines **12** classes (`worldLayerPrefs.ts:20-38`) but `MissionSettingsDialog.tsx:147-175` exposes only **5** (fences, airfield, heights, townLabels, roadNames). The other 7 are consumed by the loader/residency (`wgpuWorldLoader.ts:131` `set_glyph_toggles`, etc.) yet reachable only by editing localStorage. `props` defaults **false** with no UI to enable. Operator gate O10 ("each pref off works") is untestable for 7 of 12 classes.

---

## Goal

1. Seven new Switch rows in the "World layers" section of `MissionSettingsDialog.tsx`, same pattern as the existing five (`setClassToggle` calls) — order: Roads, Buildings, Forest, Trees, Props, Contours, Sea, then the existing five.
2. Every toggle round-trips: flip → engine visibility reacts (existing subscription paths) → persists (`tbd-mc-world-layers`).
3. Vitest coverage: a dialog test asserting all 12 class keys have a control, and a prefs test for persistence of the new keys.
4. Verify log `.ai/artifacts/t152_20_verify_log.md`.

---

## Out of scope

- Changing any default value (incl. `props: false` — it stays; it just becomes flippable).
- New engine visibility semantics (T-152.14/.15 own gate behavior).
- Debug toggles (`worldmapDebug` stays Ctrl+Alt+D).

---

## Locked decisions

| # | Decision | Rationale |
|---|----------|-----------|
| L1 | Dumb UI only: Switch rows + existing `setClassToggle`/`subscribeWorldLayerPrefs` plumbing — zero Rust/wasm changes | LANGUAGE GATE (this is the one TS-only slice) |
| L2 | Defaults untouched (`DEFAULT_TOGGLES` as-is) | Behavior change belongs to engine slices |
| L3 | Labels: sentence-case cartographic names ("Roads", "Buildings", "Forest mass", "Trees", "Props", "Contours", "Sea") | Aegis dialog copy style |
| L4 | Test asserts key-completeness against `WorldClassToggles` keyof — a 13th class added later fails the test until exposed | Future-proof O10 |
| L5 | Commit `T-152.20:` · tag `T-152.20` · verify log | House convention |

---

## Tasks

1. `MissionSettingsDialog.tsx`: 7 Switch rows.
2. Tests: dialog completeness (L4) + prefs persistence.
3. Manual toggle sweep note for M1.
4. Verify suite + verify log + commit + tag.

---

## Mathematical acceptance matrix

| Gate | Predicate | Class |
|------|-----------|-------|
| **G1** | Dialog exposes a control for **every** key of `WorldClassToggles` (12/12) — keyof-driven test | Completeness |
| **G2** | Flipping each new toggle updates `getClassToggles()` + persists across reload (unit) | Round-trip |
| **G3** | No engine-crate diffs; no default changes (`DEFAULT_TOGGLES` byte-identical) | Scope |
| **G4** | FE `npm test` + `npm run build` + `npm run lint` exit 0 | Regression |

---

## Verify

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152/apps/website/frontend
npm test && npm run build && npm run lint
cd ../../.. && git diff --stat crates/   # expect empty
```

---

## Manual acceptance

- **M1:** Settings → World layers: flip each of the 12 — map reacts (trees/buildings/roads/forest/contours/sea/props visibly toggle); prefs survive reload.

---

## Documentation sync (Cursor, after merge)

Registry `T-152.20 → shipped`; hub row; `./scripts/ticket sync`.

---

## Claude Code prompt — T-152.20 (copy-paste)

Authority: this spec. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the T-152 worktree:
  /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152

Implement **T-152.20** — Mission Settings world-layer toggle completeness.

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152/apps/website/frontend
  npm ci --no-audit 2>/dev/null || npm install

═══ READ (in order — spec wins) ═══
  1. docs/specs/Mission_Creator_Architecture/t152_20_settings_completeness.md
  2. apps/website/frontend/src/features/mission-creator/layout/MissionSettingsDialog.tsx (:147-175)
  3. apps/website/frontend/src/features/tactical-map/state/worldLayerPrefs.ts
  4. apps/website/frontend/src/features/tactical-map/wgpu/wgpuWorldLoader.ts (toggle consumption — READ ONLY)

═══ PROBLEM ═══
  5 of 12 world-layer prefs exposed; trees/buildings/props/roads/forest/contours/sea unreachable
  without localStorage surgery; O10 untestable.

═══ SHIPPED (do not reopen) ═══
  Engine gate semantics (.14/.15); existing 5 toggles.

═══ LANGUAGE GATE ═══
  TS dumb UI ONLY this slice. Zero crates/** changes (G3 asserts).

═══ LOCKED ═══
  - 7 Switch rows, existing setClassToggle pattern; defaults untouched
  - keyof-completeness test (12/12, future-proof)

═══ DO ═══
  1. Dialog rows
  2. Completeness + persistence tests
  3. Verify; .ai/artifacts/t152_20_verify_log.md; commit "T-152.20: ..."; tag T-152.20

═══ DO NOT ═══
  - Change DEFAULT_TOGGLES or engine behavior
  - Edit docs/**, .ai/tickets/**

═══ VERIFY (all exit 0) ═══
  (bash block from spec §Verify)

═══ MANUAL ═══
  M1 toggle sweep

═══ RETURN ═══
  - Commit SHA + tag; verify log path
  - 12/12 completeness test name
```
