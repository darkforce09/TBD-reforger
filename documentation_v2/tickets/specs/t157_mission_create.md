# T-157 — Mission Create visual overhaul

Owner: command center. UI code ticket (executor claude-code). Dialog lives in editor/library/.

## Claude Code prompt — T-157

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-157 && pwd && git branch --show-current   # must be slice/T-157
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
apps/website/frontend/src/editor/library/{create_dialog.rs, mission_library.rs:1-140, mod.rs}, packages/map-assets/terrain-registry.json, docs/plans/t-157_plan.md
═══ PROBLEM ═══
New Mission is a form with a terrain dropdown and fields that belong to editor settings.
═══ SHIPPED ═══
T-286 role gating on New Mission, T-135 modset presets (land first), T-193 settings gate.
═══ LANGUAGE GATE ═══
Rust/Leptos only.
═══ LOCKED ═══
- The created document is byte-identical to today's for the same terrain (digest test).
- Role gating and Ctrl+N unchanged.
═══ DO ═══
1. Digest test + dialog inventory on main. 2. map_picker.rs + registration. 3. Dialog rewire; remove the three fields.
4. Perturb (skip a terrain) → red → restore → touch → green. 5. Gates.
═══ DO NOT ═══
No git add -A, no git stash, no ci-local; no edits under packages/map-assets or pages/.
═══ VERIFY ═══
cargo xtask mk ci-local-leptos ; cargo xtask mk leptos-gates ; cargo xtask platform wave gate --slice T-157
═══ MANUAL ═══
Operator: Ctrl+N → cards with thumbnails, choose a modset, create → editor opens as before.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```
