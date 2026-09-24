# T-301 — Plan

## Context
T-182 made launcher/handgun/throwable real gear fields; BuildKit (TBD_BriefingData.c:519) still renders the seven
pre-T-182 rows, so the planning screen hides exactly the weapons that change what a squad can attempt.

## Approach
1. Read `TBD_BriefingData.c` BuildKit and the gear fields of `TBD_MissionSlotStruct` (Gamemode/).
2. Add launcher, handgun, throwable rows after the primary weapon using the existing empty-skip pattern; update the
   header comment with the row order and the deliberate pants/boots/handwear omission.
3. `cargo xtask mod compile`; add the checklist item (four-weapon loadout mission) to the report MANUAL block.

## Risks
- Layout overflow with ten rows; the kit list already scrolls (verify in the checklist).
- Field names differ from the wire names; copy them from the struct, not from the schema.

## Verification
- `cargo xtask mod compile`
- `cargo xtask platform wave gate --slice T-301`
- Human checklist: briefing screen shows launcher/handgun/throwable for a four-weapon slot.
