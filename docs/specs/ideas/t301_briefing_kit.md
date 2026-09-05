# T-301 — Briefing kit lists 7 of 13 gear fields

Owner: command center. Operator authorization 2026-09-04: "agents may edit the Enfusion mod scripts; gate =
`cargo xtask mod compile`; in-game behaviour goes on a human checklist."

## Claude Code prompt — T-301

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-301 && pwd && git branch --show-current   # must be slice/T-301
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
apps/mod/tbd-framework/Scripts/Game/TBD/UI/TBD_BriefingData.c:380-560, the gear fields of TBD_MissionSlotStruct, docs/plans/t-301_plan.md
═══ PROBLEM ═══
BuildKit renders seven gear rows; launcher, handgun and throwable (T-182) never reach the briefing screen.
═══ SHIPPED ═══
T-182 thirteen-field gear vocabulary; keep the deliberate pants/boots/handwear omission.
═══ LANGUAGE GATE ═══
Enforce script only; no new files.
═══ LOCKED ═══
- Same empty-skip pattern as the existing rows; no new payload fields.
- Row order documented in the BuildKit header comment.
═══ DO ═══
1. Confirm on main the three fields are absent from BuildKit; paste the grep.
2. Add the three rows. 3. cargo xtask mod compile. 4. cargo xtask platform wave gate --slice T-301
═══ DO NOT ═══
No git add -A, no git stash, no ci-local; touch only TBD_BriefingData.c.
═══ VERIFY ═══
cargo xtask mod compile ; cargo xtask platform wave gate --slice T-301
═══ MANUAL ═══
Human checklist: boot a mission whose slot carries rifle + launcher + handgun + throwable; the briefing kit list shows all four.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```
