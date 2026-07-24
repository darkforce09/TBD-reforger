# Spawn/Equip Determinism + Stable Slot Identity · verify log

**Date:** 2026-07-24 · **Executor:** Fable 5 / Claude Code · **Branch:** `main` ·
**Plan:** operator-approved (this session) — Phase A (A1 authority / A2 equip / A3 gear) + Phase B (B1 uid / B2 mutate-apply) ·
**Baseline:** `0be53e16` (T-068.12) · **Tickets:** for Cursor to mint (registry untouched per agent split)

## Operator decisions honored

- Slot model: dynamic authored slots + permanent `uid`; **no fixed pool** (128 = server
  *player* cap, not an authored-slot limit; 40/84, 80/80, FIA all legal). Cap indicator
  is informational only.
- Respawn: **re-equip every spawn**.

## Root causes fixed (all measured before fixing)

1. **Dual spawn ownership** — vanilla `DoSpawn_S` fell through to a second body whenever
   `DeployPlayer` returned false (incl. "already deployed"): the transfer feeling, the
   orphaned kit-body "AI", the vanilla-kit player. → `DeployPlayerEx` tri-state
   (`DEPLOYED/ALREADY/RETRY/FAILED/NOT_MINE`); vanilla may spawn ONLY on `NOT_MINE`
   (client / no framework mission); `RETRY` gets a bounded 500 ms scheduler (cap 20);
   the push wave fires once (LOBBY only) behind `m_bAutoDeploy` (PIE/dev wave —
   T-068.13 slot picker will default it off); build-side wave deleted.
2. **Roster race** — fetch fired the same tick as LOBBY; first-caller-wins assignment
   flipped roster/round-robin run-to-run. → the stage machine now waits for roster
   settle (500 ms ticks, 2 s deadline → `ForceSettle`) BEFORE LOBBY; late REST responses
   are ignored after settle; breadcrumb `[TBD][Spawn] roster settled=… assignments=…`.
3. **Equip aimed at a timer, not the spawn** — `GetPlayerControlledEntity` at +3000 ms.
   → equips fire from the vanilla `SCR_BaseGameMode.GetOnPlayerSpawned()` invoker with
   the DELIVERED entity; per-spawn idempotency (`map<playerId, EntityID>`); completed
   applications pruned. (1.7 fact, measured via compile error: `OnPlayerSpawned` /
   4-arg `OnPlayerKilled` are NOT component virtuals — invoker + the
   `SCR_InstigatorContextData` kill shape [CRF Rally precedent] are.)
4. **Verify-then-delete at +1000 ms** deleted slow-settling garments (measured live:
   the same pants lookup succeeded at 15:56 and failed at 16:55). → poll-until-worn
   (500 ms × 6) before any delete; cargo strictly after wear verify.
5. **Displaced kit garments unhandled** (EquipCloth routes by the item's own AreaType)
   → ground litter. → deterministic swap: incumbents captured pre-equip (both vest
   areas; pre-equip current weapon), same-prefab equips short-circuit (`swap-skipped`),
   displaced incumbents deleted only after the new item verifies worn + a re-check
   (`swapped area=… out=… in=…`); `IsRootedOn`-only verifies log `swap-deferred`,
   never guess-delete. Displaced kit contents are deleted WITH the garment — deliberate,
   Arsenal cargo is the contents SoT.
6. **No death/disconnect handling; one-shot deploy forever** → `OnPlayerKilled`
   (`SCR_InstigatorContextData`) re-arms the guard (slot retained → same slot, fresh
   equip via the spawn hook = re-equip every life); `OnPlayerDisconnected(int,
   KickCauseCode, int)` clears per-player state and records `identity → slot` in a
   reclaim map consulted before roster/round-robin (dedicated servers reuse playerIds);
   10 s spawn watchdog re-arms requests that never materialized.
7. **Lossy gear pipeline** — pants/boots/handwear/backpack dropped at flatten; mod
   equipped only 4 items. → compiled `gear` + `TBD_SlotGearStruct` + `Run()` carry all
   wear areas (`LoadoutBootsArea` / `LoadoutHandwearSlotArea` pinned from the API docs
   index — note the engine's own `LoadoutGooglesArea` typo for goggles); single-vest
   collapse (armoredVest wins) stays, documented in schema + code. Named operator
   follow-up (not this program): optic/magazine mounting + `weapons[1+]`.

## Stable identity (Phase B)

- **B1 `uid`** — compiled `ModSlot.uid` = editor slot id verbatim (`uid`, not `ref` —
  EnforceScript keyword); schema slot def += optional `uid`; mod struct += `uid` +
  `Key()` (uid-else-id); spawn-point map + lookups keyed on `Key()`;
  `GetSlotById` uid-aware; `ValidateMissionSlots` dup-checks uid. Display id
  (`faction:callsign:role:occurrence`) stays the human label. Live route proof:
  `GET /missions/…/compiled` served `"uid":"s1"`.
- **B2 mutate-apply** — `apply_faction_library` REPLACE (delete+recreate = the literal
  "foundation keeps shifting") → MUTATE: overlapping roles write role/tag/character/
  loadout onto the existing slots in place (ids + operator-moved positions survive),
  surplus roles mint (`slot-{side}-apply-{i}`, suffix-uniqued), surplus slots removed;
  first squad reused (renamed), extras removed; vehicles stay replace-semantics via new
  `remove_vehicle` (detach alone orphaned rows). New store op
  `update_slot_role_character` (set/clear tag+assetId; position/stance untouched).
  Headline test `reapply_keeps_overlapping_slot_ids_and_positions` + H1–H4/H9 all green.
- **Cap indicator** — ORBAT Manager header: `N slots · server cap 128 players`
  (error-tinted above 128, never blocking).

## Gates

```bash
cargo test -p map-engine-core --all-features   # flatten 3/3 (uid + wide gear + omission)
                                               # apply_faction 7/7 (incl. B2 headline)
make schema-validate                           # golden missions PASS with uid + wide gear
cargo test --lib services::mission_compile     # G6 validates uid+gear doc  (2/2)
make ci-local-leptos                           # 94 tests + trunk release PASS
# WB compile: fresh boot log shows ZERO script errors (the two override
#   mismatches were caught by the operator's screenshot + boot log and fixed)
# Determinism gate: scripts/mod/tbd-spawn-determinism.sh — see below
```

### Determinism gate (N=5, WB restart per run — loader statics landmine)

**PASS — 5/5 runs byte-identical** (outcome digest `3e31fd8cf7c6`, 18 canonical lines):
census `characters=1 players=1` every run (the reaper working) · zero
`vanilla-fallthrough` · zero `[TBD]` errors · exactly one `spawn requested` ·
every gear item `GEAR-ENSURED` · roster `settled=failed` deterministically pre-LOBBY.

The gate surfaced (and the fixes absorbed) three real engine behaviors along the way:
1. **Vanilla double-spawn** — one `RequestSpawn` fires the spawn invoker TWICE with
   different bodies ~1 s apart; the second is nondeterministically kit-dressed or
   naked. This was the operator's "kitted/unkitted AI + inconsistent player kit +
   ground litter" — now reaped (live superseded bodies deleted, in-flight equips
   cancelled; corpses untouched).
2. **Kit cosmetic RNG** — vanilla randomizes kit garment variants per spawn
   (`Jacket_US_BDU` vs `_rolledup`): the swap correctly replaces the rolled variant
   with the authored jacket; digest treats swap-vs-skip as diagnostic since the worn
   outcome is identical.
3. **Game-dead WB boots** — a restart racing Steam yields "Can't initialize the game"
   (the SAME dialog a script compile error produces); the harness probes world-open
   after connect and cycles (observed recovering: "WARN … cycling again").

## Ops findings recorded

- WB restart racing Steam can boot **game-dead** ("Can't initialize the game" — Net API
  up, World Editor dead). The harness probes `wb_open_resource` after connect and cycles
  up to 3× (`restart_wb_once`). A Game-module **script compile error produces the same
  dialog** — check the boot log's `Compiling Game scripts` section first (the operator's
  instinct was right).
- Background-env PATH lacks `rg` — the harness is pure grep/sed/awk.

## Operator items (explicit)

- In-game eyeball on next session: no ground litter at spawn, kit complete every run,
  `swapped`/`swap-skipped` lines in console, respawn re-equips.
- Dedicated-server pass (JIP / playerId reuse / real audit) via `run-dev-server.sh` +
  `debug-direct-join.sh` when convenient — PIE cannot exercise those paths.

## Ready for Cursor

Mint tickets + tags for: A1+A2+A3+B1 (mod+compiler commit) and B2 (editor commit);
docs sync per this log.
