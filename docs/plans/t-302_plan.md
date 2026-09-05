# T-302 — Plan

## Context
T-182's slot-indexed equip is reasoned from vanilla precedent but never observed; the world-boot lane
(xtask/src/mod_world_boot.rs) only greps existing sentinels, so a silent replace or drop would pass today.

## Approach
1. `Gamemode/TBD_LoadoutEquipHelper.c`: after each weapon-row equip, print `[TBD][Equip] slot=<n> weapon=<res>
   result=<ok|replaced|failed>`; no behaviour change.
2. Boot fixture: a four-weapon loadout row in the mission the world-boot lane loads (documented in mod_world_boot.rs).
3. `xtask/src/mod_world_boot.rs`: assert four `result=ok` lines and zero `result=replaced`; perturbation = expect five.
4. `cargo xtask mod compile`, run the boot lane, gate.

## Risks
- The boot lane needs the server profile (gate_setup_server_profile.rs); if unavailable in the container the assertion
  runs only on the host — say so in the report, keep the human checklist as the second proof.

## Verification
- `cargo xtask mod compile` · `cargo xtask mod world-boot` · `cargo xtask platform wave gate --slice T-302`
- Human checklist: four weapons carried on the live body.
