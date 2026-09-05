# T-926 — Vehicle Attributes Transform/Position tab

## Claude Code prompt — T-926
```text
═══ PREFLIGHT ═══ worktree .ai/artifacts/worktrees/T-926, branch slice/T-926, CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══ .ai/tickets/T-926.toml, docs/plans/t-926_plan.md, panels/attributes_modal.rs vehicle body (T-818)
═══ PROBLEM ═══ Vehicle Attributes show Heading/Cargo/Crew only; slots have a Transform tab. Add Position X/Y/Z + Heading for vehicles.
═══ SHIPPED ═══ T-818 (Heading/Cargo/Crew), T-819, T-837.
═══ LANGUAGE GATE ═══ Rust/Leptos only.
═══ LOCKED ═══ Existing T-818 fields stay; z from exact f64 rows, never the SoA.
═══ DO ═══ Add the section; commit through the existing vehicle pose op; pin the DOM; perturb.
═══ DO NOT ═══ Touch files outside owns; no docs edits.
═══ VERIFY ═══ cargo xtask mk ci-local-leptos; cargo xtask mk leptos-gates; cargo xtask platform wave gate --slice T-926
═══ MANUAL ═══ Select a placed vehicle → Attributes shows X/Y/Z/Heading; edit X → vehicle moves.
═══ RETURN ═══ Report schema per brief. Ready for Cursor doc sync.
```
