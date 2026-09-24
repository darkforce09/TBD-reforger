# T-705 — Plan

## Context

Eden's General States decide which player gadgets exist; five flags (map, compass, watch, GPS, radio) are on the wire since T-706 with no reader.

## Approach

1. Verify on main: no gadget flag binding in `TBD_MissionLoader.c`.
2. `TBD_MissionLoader.c`: bind the grouped flag object; new `Backend/TBD_GadgetFlags.c`: on player spawn remove or withhold the disabled gadgets.
3. Compile; unset flags keep today's loadout.

## Risks

- Gadgets arrive via loadout after spawn; hook late enough (post-loadout) or the removal is undone.

## Verification

- `cargo xtask mod compile` · `cargo xtask platform wave gate --slice T-705`
