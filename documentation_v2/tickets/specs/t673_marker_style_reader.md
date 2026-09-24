# T-673 — Marker style and Area markers: the Enfusion reader

Ticket: .ai/tickets/T-673.toml · Schema half shipped in T-706 · Base fields shipped in T-069.

## Claude Code prompt — T-673

```
Read CLAUDE.md first. Implement **T-673** — marker style + Area marker reader.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-673
═══ READ ═══  docs/plans/t-673_plan.md; apps/mod/tbd-framework/Scripts/Game/TBD/Markers/TBD_MarkerData.c; TBD_MarkerClient.c
═══ PROBLEM ═══  Six MRK keys (size, rotation, shape, brush, color, alpha) are on the wire but nothing in the mod reads them, so Area markers never render in game.
═══ SHIPPED ═══  T-069 marker base fields; T-706 $defs/marker widening.
═══ LANGUAGE GATE ═══  Enfusion script (.c) under apps/mod/tbd-framework only; no schema JSON; no TypeScript.
═══ LOCKED ═══
  - Verify the gap on main first (rg the keys in apps/mod: zero readers)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; new sibling .c files are named in the ticket
  - No packages/tbd-schema edits (T-706 owns widening)
  - In-game behaviour goes on the human checklist; the gate is cargo xtask mod compile
  - Icon-only markers must render exactly as today
═══ DO ═══
  1. Confirm zero readers for the six keys in Markers/
  2. Add the six fields + defaults to TBD_MarkerData and bind them from the payload
  3. Render Area markers (shape × brush × color × alpha) and rotated/sized icons in TBD_MarkerClient
  4. Diff against salvage/t853-dropped/T-673 and take only matching reader hunks
  5. Tag T-673 · commit prefix T-673:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mod compile; cargo xtask platform wave gate --slice T-673
═══ MANUAL ═══  Human checklist: an Area marker authored in the editor appears with its shape/colour/alpha in game; icon markers unchanged.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```

