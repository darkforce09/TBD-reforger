# T-159.22 — Claude Code handoff (outliner + asset palette)

**CWD:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-159`  
**Baseline:** `f02fed5a` / tag `T-159.21`  
**Spec:** [`docs/platform/t159_22_outliner_asset_palette.md`](../../docs/platform/t159_22_outliner_asset_palette.md)

## Context

Eden chrome + undo shipped. Docks are placeholders. Next = live **Editor Layers** tree + **Asset
palette** drag-to-place (React T-033 / AssetBrowser), plus .21 deferred **wheel-over-dock** and a
**CUR Class R** gate.

`MissionDocCore` already has `add_slot`, `add_editor_layer`, `move_slot_to_layer`, etc. Seed creates
8 slots under a default layer — enough for Class R without VirtualOutliner.

`dto::RegistryResponse` exists as `Vec<Value>` — finish character-item typing if you wire live
`GET /api/v1/registry`; otherwise embed a fixture catalog for the smoke.

## Notes from T-159.21

- Soft-WebGPU smokes stay `?force=webgl` (marquee, undo, selfcheck as required).
- Probe empty clicks already inset past docks — keep that contract if dock content grows scrollbars.
- Don’t chase pre-existing website-leptos clippy drift outside touched files.
- Prove clippy “zero new lints” with stash-diff if full-crate counts stay red.

## Return

Tag **T-159.22** + verify log → Cursor sets up **T-159.23**.
