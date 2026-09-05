# T-158 — Editor shell UX consolidation

Owner: command center. UI code ticket (executor claude-code), packs after T-142 and T-939.x.

## Claude Code prompt — T-158

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-158 && pwd && git branch --show-current   # must be slice/T-158
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
apps/website/frontend/src/editor/panels/top_strip.rs:1-200, dock_left.rs:880-920 and its tab builder, state/commands_hotkeys.rs, docs/plans/t-158_plan.md
═══ PROBLEM ═══
Two Settings entries, an inert History button, a duplicate Assets tab.
═══ SHIPPED ═══
T-818 right-dock asset surface, T-142 toolbelt/attributes polish, T-193 settings gate — keep them.
═══ LANGUAGE GATE ═══
Rust/Leptos only.
═══ LOCKED ═══
- No control disappears without its function surviving elsewhere (list them in the report).
- Hotkeys keep working; inventory test pins the final control set.
═══ DO ═══
1. Inventory test (paste today's set). 2. Consolidate Settings. 3. Wire or remove History. 4. Remove the Assets tab.
5. Perturb (re-add Assets) → red → restore → touch → green. 6. Gates.
═══ DO NOT ═══
No git add -A, no git stash, no ci-local; no edits outside the two owned files.
═══ VERIFY ═══
cargo xtask mk ci-local-leptos ; cargo xtask mk leptos-gates ; cargo xtask platform wave gate --slice T-158
═══ MANUAL ═══
Operator: open the editor; count Settings entries (1), click every top-bar button (none inert), left dock has no Assets tab.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```
