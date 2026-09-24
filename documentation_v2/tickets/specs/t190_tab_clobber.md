# T-190 — Two tabs on one mission clobber each other

Owner: command center. F-32 verified repro (editor_hostile_ux_review.md). Packs after T-937.4 (persist.rs) and
before T-295 (realtime), which builds on the same merge path.

## Claude Code prompt — T-190

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-190 && pwd && git branch --show-current   # must be slice/T-190
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
apps/website/frontend/src/editor/state/{persist.rs, hydrate.rs:200-260, mod.rs}, mission_editor.rs conflict arm only, docs/plans/t-190_plan.md
═══ PROBLEM ═══
Blind whole-blob save; no second-tab detection; conflict prompt without counts, timestamps or a destructive marker.
═══ SHIPPED ═══
T-222 (dep, shipped), T-937.4 SaveStatus signal (packs earlier — rebase on it), T-191 recovery bridge.
═══ LANGUAGE GATE ═══
Rust/Leptos only; no JS files.
═══ LOCKED ═══
- Merge = yrs apply_update, never a JSON diff.
- mission_editor.rs: call sites only (allowlisted SIZE-3); new logic goes in tab_lock.rs / hydrate.rs.
- Conflict copy names both counts and timestamps; destructive option marked.
═══ DO ═══
1. wasm last-write-wins test; paste the red. 2. tab_lock.rs + mod.rs registration. 3. read-merge-write in persist.rs.
4. ConflictInfo fields + modal arm. 5. Perturb (skip merge) → red → restore → touch → green. 6. Gates.
═══ DO NOT ═══
No git add -A, no git stash, no ci-local; no file-length allowlist edits; no server changes (that is T-295).
═══ VERIFY ═══
cargo xtask mk ci-local-leptos ; cargo xtask mk leptos-gates ; cargo xtask platform wave gate --slice T-190
═══ MANUAL ═══
Operator: open one mission in two tabs, delete all in B, add in A, reload both — both sets of edits present, banner seen in B.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```
