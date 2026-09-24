# T-674.2 — Plan
## Context
T-674.1 puts callsign, rank, stance, unitName, tag and `leaderSlotId` on the wire at schemaVersion 1.3. The mod has no fields for them, no `stance` call (word-boundary hits in `apps/mod` are zero), and `TBD_MissionValidator.c:42` lists only the versions it understands — a 1.3 mission is refused outright.

## Approach
1. Verify on main: `cargo xtask mod compile` is green and the validator list lacks 1.3.
2. `TBD_MissionSlotStruct.c`: add the five slot keys and the group `leaderSlotId`; `TBD_MissionLoader.c`: bind them.
3. `TBD_MissionValidator.c`: accept 1.3. `TBD_SpawnManager.c`: apply callsign/rank/unitName to the spawned body, set the stance via the Enfusion character pose call, resolve the squad leader from `leaderSlotId`.
4. Compare with `salvage/t853-dropped/T-674`; reuse only matching hunks.

## Risks
- The stance API may need a controller component rather than a spawn parameter; fallback is a post-spawn call with a note.
- Shares loader/spawn files with T-675.2 (order 4321), which packs after this slice.

## Verification
- `cargo xtask mod compile` · `cargo xtask platform wave gate --slice T-674.2` · in-game identity check on the human checklist
