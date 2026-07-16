# T-159.22 — Editor Layers outliner + Asset palette DnD

**Parent:** [`t159_leptos_ui_program.md`](t159_leptos_ui_program.md) · **Executor:** claude-code ·
**Worktree:** `.ai/artifacts/worktrees/TBD-T-159/` · **Baseline:** **T-159.21** @ `f02fed5a`

## Problem

Eden docks are placeholders. React left outliner shows live Editor Layers (+ ORBAT); right palette
lets operators drag assets onto the map (`addSlot` under `activeLayerId`). Leptos needs the same
**seed-scale** wiring before Attributes / Arsenal / 367k virtualization.

Also deferred from **T-159.21:** wheel over a dock still zooms the map (capture-phase listener);
CUR toolbelt math has no committed gate.

## Locked decisions

| # | Decision |
|---|----------|
| O1 | **Left dock — Editor Layers:** build tree from `MissionDocCore` `editorLayers` + slots (seed already creates a default layer). Click folder → set active layer; click slot → selection (match map select); optional Space/flyTo later. ORBAT section may stay a stub header this slice. |
| O2 | **Right dock — Asset palette:** Factions tab with a placeable catalog. Prefer typed `GET /api/v1/registry` (`dto::RegistryResponse` already stubbed — finish item typing) **or** an embedded fixture catalog if the smoke must not depend on a live DB. Vehicles/Markers/Objectives tabs stay stubs. |
| O3 | **Drag-to-place:** leaf drag → drop on canvas → `add_slot` under active layer with `asset_id`; rebind glyphs; `after_local_edit` (undoable + persist). MIME/payload shape may be Leptos-native (HTML5 DnD or pointer-drag) — document the chosen path. |
| O4 | **Wheel over chrome:** do **not** zoom when the wheel target is inside left/right dock (or top/bottom chrome). Map wheel over the free canvas stays. |
| O5 | **CUR Class R:** commit a smoke (or extend an existing editor smoke) that asserts toolbelt CUR at known pointer→world points (use the .21 CDP math: centre → 6400/6400; document zoom/−2 scale). Off-map → em dash. |
| O6 | **Scale:** seed-scale tree is enough (8 slots). **Do not** port full `@tanstack`-style VirtualOutliner @ 367k this slice. Optional threshold stub OK; no 367k gate. |
| O7 | **Out of scope:** Attributes modal, Faction Manager, Arsenal, layer reparent DnD / rename/delete polish (add-folder OK if cheap), ORBAT Manager, cluster, Mission Settings. |
| O8 | Keep all **9** editor smokes green; marquee/undo stay `?force=webgl` where required. |

## Do

1. Fill left dock with live Editor Layers tree; right dock with Factions catalog + place.
2. Wheel-over-dock fix + CUR Class R gate.
3. Smoke: outliner click selects; drag-place bumps OBJ / digest; wheel over dock does not zoom.
4. `.ai/artifacts/t159_22_verify_log.md` · tag **T-159.22**.

## Claude Code prompt — T-159.22 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the WORKTREE at .ai/artifacts/worktrees/TBD-T-159/ (NOT main).

Implement **T-159.22** — Editor Layers outliner + Asset palette DnD.

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-159
  test "$(basename "$(git rev-parse --show-toplevel)")" = "TBD-T-159"
  git status --porcelain
  git branch --show-current
  git rev-parse --short HEAD  # expect f02fed5a (T-159.21) or later after main merge

═══ READ ═══
  1. .ai/artifacts/t159_22_claude_code_handoff.md
  2. docs/platform/t159_22_outliner_asset_palette.md
  3. docs/platform/t159_leptos_ui_program.md
  4. .ai/artifacts/t159_21_verify_log.md  (wheel-over-dock + CUR notes)
  5. apps/website-leptos/src/eden_chrome.rs
  6. apps/website-leptos/src/mission_editor.rs
  7. apps/website-leptos/src/mission_history.rs
  8. apps/website-leptos/src/mission_doc.rs
  9. apps/website-leptos/src/dto.rs  (RegistryResponse stub)
  10. crates/map-engine-core/src/doc/store.rs  (add_slot, editor_layers, materialize)
  11. apps/website/frontend/src/features/mission-creator/layout/LeftOutliner/EditorLayersSection.tsx
  12. apps/website/frontend/src/features/mission-creator/layout/RightInspector/AssetBrowser.tsx
  13. apps/website/frontend/src/features/mission-creator/layout/RightInspector/AssetPalette.tsx

═══ PROBLEM ═══
  Docks are empty placeholders. Need live Editor Layers outliner + Asset palette drag-to-place
  at seed scale, plus .21 deferred wheel-over-dock fix and a CUR Class R gate.

═══ SHIPPED ═══
  T-159.21 @ f02fed5a — Eden chrome + undo/redo; 9 editor smokes
  T-159.20 Save/Export · T-159.19 marquee/move

═══ LOCKED ═══
  - Left: Editor Layers from MissionDoc; click folder=active, click slot=select
  - Right: Factions catalog (registry API or fixture) + drag-to-place via add_slot
  - Wheel over dock must not zoom; canvas wheel unchanged
  - CUR Class R smoke (centre → 6400/6400 at default cam)
  - Seed-scale only — no 367k VirtualOutliner
  - Keep 9 prior editor smokes green

═══ DO ═══
  1. Outliner + palette + place path + after_local_edit
  2. Wheel-over-dock fix + CUR gate
  3. New smoke(s) Class R
  4. .ai/artifacts/t159_22_verify_log.md
  5. Commit T-159.22: · tag T-159.22 (Opus harness trailer)

═══ DO NOT ═══
  - Edit docs/** or registry
  - Port Attributes / Arsenal / Faction Manager / full VirtualOutliner
  - Break any of the 9 prior editor smokes
  - “Fix” pre-existing crate-wide clippy drift outside your files

═══ VERIFY ═══
  Prior 9 smokes pass
  New: outliner select; place → OBJ/digest; wheel over dock no zoom; CUR Class R
  cargo check wasm32 + trunk build --release

═══ RETURN ═══
  - SHA + tag T-159.22
  - verify log
  - Ready for Cursor → T-159.23 (Attributes / ORBAT depth — confirm with Cursor)
```
