# T-159.23 — Attributes modal (queued after T-159.22.1)

**Parent:** [`t159_leptos_ui_program.md`](t159_leptos_ui_program.md) · **Executor:** claude-code ·
**Worktree:** `.ai/artifacts/worktrees/TBD-T-159/` · **Baseline:** **T-159.22.1** (when shipped)

## Problem

Placed slots can be selected and moved, but there is no **Attributes** surface to edit role / tag /
stance / transform numerically (React `AttributesModal` on double-click).

## Scope (locked for when ACTIVE)

| # | Decision |
|---|----------|
| A1 | Double-click (or Activate) a slot → Attributes modal (Aegis Dialog). |
| A2 | Tabs: Transform (x/y/z/rotation) + Identity (role/tag/stance stubs as in React Phase 3.5). Wire `MissionDocCore::update_slot` / `update_slot_position`. |
| A3 | One undo step per commit (blur/Enter) — depends on **T-159.22.1** granularity. |
| A4 | **Out of scope:** Arsenal tab / loadout doll (later), ORBAT tree (.24), Faction Manager. |

## Status

**Queued.** Do not implement until **T-159.22.1** ships and Cursor activates this slice + handoff.
