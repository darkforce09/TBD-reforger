# T-290 — Plan

## Context
Nine flatten outputs have no mod reader; each audit re-derives the list. The fix is a ledger that a test enforces,
plus the three readers that are cheap (author, templateId, winConditions.mode). Packs last on flatten.rs.

## Approach
1. `crates/map-engine-core/src/mission/flatten.rs`: `EMIT_LEDGER: &[(field, Consumer)]`; test walks the golden
   output's top-level keys (and the nine named paths) and asserts each is in the ledger — red on main.
2. `Backend/TBD_MissionLoader.c`: parse meta.author, meta.templateId, winConditions.mode into the struct;
   `Backend/TBD_MissionValidator.c`: WARNING on unknown mode. `cargo xtask mod compile`.
3. Annotate environment, factions.tickets, orbat.type, briefingSeconds (advisory), orbat (parity-check) as
   non-consumed with the ticket that would consume them.
4. Perturbation: delete one ledger row → red; restore, `touch`, green.
## Risks
- flatten.rs is heavily shared; rebase after T-291/T-299/T-242/T-310 and re-run the ledger test.

## Verification
- `cargo test -p map-engine-core --all-features mission::flatten` · `cargo xtask mod compile`
- `cargo xtask platform wave gate --slice T-290`
