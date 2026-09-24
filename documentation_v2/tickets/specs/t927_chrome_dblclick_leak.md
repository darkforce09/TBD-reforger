# T-927 — Editor chrome dblclick leaks to the map

## Claude Code prompt — T-927
```text
═══ PREFLIGHT ═══ worktree .ai/artifacts/worktrees/T-927, branch slice/T-927, CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══ .ai/tickets/T-927.toml, docs/plans/t-927_plan.md, mission_editor.rs dblclick handler
═══ PROBLEM ═══ dblclick on docks / Attributes overlay / top strip reaches the canvas dblclick handler and opens the place picker or Attributes.
═══ SHIPPED ═══ T-822 (outliner dblclick) — same family; reuse its guard pattern.
═══ LANGUAGE GATE ═══ Rust/Leptos only.
═══ LOCKED ═══ Canvas empty-ground and entity dblclick keep working.
═══ DO ═══ Gate on event target; stop propagation at chrome roots; smoke both directions; perturb.
═══ DO NOT ═══ Touch files outside owns; no docs edits.
═══ VERIFY ═══ cargo xtask mk ci-local-leptos; cargo xtask mk leptos-gates; cargo xtask platform wave gate --slice T-927
═══ MANUAL ═══ dblclick each chrome surface → nothing; dblclick empty map → picker.
═══ RETURN ═══ Report schema per brief. Ready for Cursor doc sync.
```
