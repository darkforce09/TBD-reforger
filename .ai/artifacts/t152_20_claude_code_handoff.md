# T-152.20 — Claude Code handoff (settings toggles)

**Slice:** T-152.20 · **Branch:** `ticket/T-152` · **Worktree:** `.ai/artifacts/worktrees/TBD-T-152`
**Spec:** [`t152_20_settings_completeness.md`](../docs/specs/Mission_Creator_Architecture/t152_20_settings_completeness.md)
**Executor:** claude-code · **Tag:** `T-152.20`

## Context

Remediation ladder after `.17` shipped; `.18` (icon extract) and `.19` (Workbench label export) **deferred** by operator — no Workbench needed for this slice.

Audit **A15 / O10:** `WorldClassToggles` has **12** keys; Mission Settings exposes **5**. Seven are engine-wired but UI-hidden (localStorage-only).

## Scope (TS only)

| Add UI for | Already wired |
|------------|----------------|
| roads, buildings, forest, trees, props, contours, sea | `worldLayerPrefs.ts` + `wgpuWorldLoader.ts` |

**Do not** change `DEFAULT_TOGGLES`, Rust/wasm, or engine visibility policy.

## Touch files (expected)

1. `apps/website/frontend/src/features/mission-creator/layout/MissionSettingsDialog.tsx` — 7 `ToggleField` rows **before** existing five; order per spec L3.
2. Tests — keyof `WorldClassToggles` completeness (12/12) + prefs persistence for new keys.
3. `.ai/artifacts/t152_20_verify_log.md`

## Acceptance

| Gate | Bar |
|------|-----|
| G1 | 12/12 controls (keyof-driven test) |
| G2 | Flip → `getClassToggles()` + localStorage round-trip |
| G3 | `git diff --stat crates/` empty |
| G4 | `npm test` + build + lint |

## Manual

M1: flip all 12 in Settings → map reacts; prefs survive reload.

## Deferred (do not reopen)

- T-152.18 icon extract
- T-152.19 Workbench Path A export
