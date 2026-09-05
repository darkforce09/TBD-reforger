# T-704 — Command palette over every editor command

Owner: command center. 3den E2 (3DEN-TOOL-013/014); promoted from idea 2026-09-04.

## Claude Code prompt — T-704

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-704 && pwd && git branch --show-current   # must be slice/T-704
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
apps/website/frontend/src/editor/panels/{top_strip.rs:80-200, mod.rs}, the toolbelt panel, state/commands_hotkeys.rs, mission_editor.rs hotkey arm, docs/plans/t-704_plan.md
═══ PROBLEM ═══
No way to reach a command by name; chrome navigation is the T-668 complaint.
═══ SHIPPED ═══
Cmd enums and their handlers; modal_stack z_class contract (O-3); T-785 focus rules.
═══ LANGUAGE GATE ═══
Rust/Leptos only; no JS.
═══ LOCKED ═══
- Palette dispatches the existing handlers; no duplicated command logic.
- mission_editor.rs: call sites only (allowlisted SIZE-3); new code in command_palette.rs.
- localStorage reads/writes wrapped in try/catch equivalents; page works with none.
═══ DO ═══
1. Confirm on main no palette exists (grep). 2. Write command_palette.rs + registration. 3. Wire Alt+Space (and Ctrl+K).
4. wasm tests (ranking, toggle, coverage). 5. Perturb (drop one entry) → red → restore → touch → green. 6. Gates.
═══ DO NOT ═══
No git add -A, no git stash, no ci-local; no allowlist edits; no new hotkeys that shadow existing ones.
═══ VERIFY ═══
cargo xtask mk ci-local-leptos ; cargo xtask mk leptos-gates ; cargo xtask platform wave gate --slice T-704
═══ MANUAL ═══
Operator: Alt+Space, type "exp", Enter → Export runs; reopen → Export ranked first.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```
