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
---

# Addendum — slot-body materialization: possess-route deploy + vanilla stand-down

**Date:** 2026-07-25 · **Executor:** Fable 5 / Claude Code · **Branch:** `main` ·
**Baseline:** `f4b25440` (slot-body materialization, compile-pending) ·
**Tickets/tags:** for Cursor to mint

Closes the three defects the materialization handoff left open, plus one found during
verification. Architecture unchanged (bodies materialized from compiled JSON at mission
load, one per slot, dressed before the lobby); what changed is how the player is handed
onto a body, and how much of vanilla is allowed to run.

## Compile receipt (the set→map conversion in f4b25440 had never been compiled)

Fresh Workbench boot, `Compiling Game scripts` section: **zero errors**
(`Module: Game; loaded 5651x files; 11055x classes`, engine 1.7.0.54, "Game successfully
created"). The `set<int>` → `map<int,bool>` conversion is confirmed good: the two
`Index out of bounds` VM exceptions at `TBD_SpawnManager.c:461` ← `:381` that fired every
run before are **gone** in every session since.

## Defect 1 — `set.Remove()` by index · FIXED (compile-confirmed)

Nothing further; the conversion shipped in `f4b25440`, this program only proved it.

## Defect 2 — automatic faction cycling · ROOT-CAUSED + FIXED

**Root cause (measured, not inferred).** The churn is emitted by vanilla
`SCR_PlayerFactionAffiliationComponent`, not by any TBD code — the only TBD faction write
is one `SetAffiliatedFactionByKey` per bind. Session `logs_2026-07-24_18-48-37` and a
reproduction on 2026-07-25 both show it starting ~9 ms after slot materialization,
**before** the first VM exception and **before** the bind, then continuing unbroken for
minutes after a successful bind — which **falsifies handoff Lead 1** (the VM exceptions
were not the cause; with them fixed the churn was unchanged at 13 switches / 9 after
census). Each newly-visited faction drags one `PlayableGroup.et` spawn plus a
`Formation of group … not found in SCR_AIWorld` warning behind it — 4 distinct factions,
4 groups, so the groups are downstream, not the driver.

The driver is handoff **Lead 2**: with slot bodies replacing spawn points there are zero
`SCR_SpawnPoint` entities, so once vanilla registers the player it hunts for a faction it
can spawn him on and never finds one — re-rolling roughly once a second, forever.
`TBD_SCR_MenuSpawnLogic.OnPlayerAuditSuccess_S` was feeding that hunt directly: its
materialized-guard is still false at audit time (audit ~1.1 s before materialization), so
it fell through to `super` every run.

**Fix.** New `TBD_SCR_RespawnSystemComponent.c` (`modded class SCR_RespawnSystemComponent`)
swallows `OnPlayerRegistered_S` / `OnPlayerAuditSuccess_S`, and `TBD_SCR_MenuSpawnLogic`
stops calling `super` on those two hooks. Vanilla never learns the player exists, so the
hunt never starts. **Measured: 13 switch lines → 0, and 0 after the census line.**

Every override is guarded on the new `TBD_FrameworkManager.IsFrameworkWorld()` (resolved
off the live game mode, not a static, because statics outlive a world inside one Workbench
process). PlayableSelector's equivalents are unconditional — they are a total conversion;
this mod loads world-globally and must stay inert on plain vanilla worlds.

**Deliberately NOT overridden:** `IsRespawnEnabled()` / `IsFactionChangeAllowed()`.
Reporting respawn "off" reads well but makes the authority reject our own possess request
(`CanRequestSpawn_S` consults it). They were in the first working build and were removed
once the possess route landed; the churn stayed at 0 without them, which also confirms
registration suppression — not the policy getters — is what fixes it.

## Defect 3 — `EngineFactionKey` gaps · FIXED

`indfor` → `FIA` and `civ` → `CIV` added (compiled keys are `blufor/opfor/indfor/civ` from
the flatten `slug_key`; all four engine keys are proven present by the churn log itself).
Added a fallback: when a mission key maps to nothing, the body's own
`FactionAffiliationComponent.GetDefaultAffiliatedFaction()` key is used, and the write is
skipped entirely (with a WARNING) rather than registering the player under `""`. Also
added `SCR_FactionManager.UpdatePlayerFaction_S` after the affiliation write — without it
the player is faction-correct locally but invisible to faction-keyed vanilla systems.

## Defect 4 (found during verification) — client pinned on the loading screen · FIXED

**Not in the handoff; found because the operator was watching.** After the churn fix the
player was bound, possessed, and had a live `PlayerCamera` — server-side state was
perfect, census `characters=1 bodies=1 players=1` — yet the client sat on the vanilla
loading screen. Instrumented runs showed `pmControlled=set localPc=set localControlled=set
localMain=set camera=PlayerCamera topMenu=none`: possession genuinely worked and **no menu
was open**, so nothing script-side was holding an overlay.

**Root cause.** `SCR_PlayerController.SetInitialMainEntity` possesses the body and gives it
a camera, but it is not a spawn: it never runs the vanilla spawn finalize, so the
client-side "player spawned locally" notification (`SCR_RespawnComponent`'s chain,
`SGetOnLocalPlayerSpawned`) never fires and the loading screen is never released. This is
why the earlier architecture — which used the vanilla spawn pipeline — never showed it.

Two cheaper levers were tried first and are recorded as **negative results**: forcing
`GetWaitForSpawnPoints()` to false on framework worlds (correct on its own merits — vanilla
would otherwise wait forever for spawn points that no longer exist — and kept), and
retiming our own `GetOnPlayerSpawned()` invoke to fire only once possession lands (also
kept, on the fallback path). Neither released the screen.

**Fix.** Deploy now goes through vanilla's own **possess** spawn request —
`SCR_PossessSpawnData.FromEntity(body)` + `SCR_PossessSpawnRequestComponent.RequestRespawn`
— the engine's designed "this player takes over an entity that already exists" path. It
creates no entity, so the double-spawn class stays fixed (census remains
`characters == bodies`), while running the full finalize the client waits on.
`SetInitialMainEntity` remains as the fallback when the request component is missing or
refuses, and each outcome logs. Because the possess pipeline fires the spawn invoker
itself, our self-announce now runs **only** on the fallback route — self-announcing on both
notified every listener twice (measured: two `deployed player=` lines per bind).

**Operator receipt:** in-world, first person, dressed, at the slot transform, no loading
screen (screenshot, 2026-07-25 01:00).

## Player controller prefab (new)

`Prefabs/Systems/TBD_PlayerController.et` (+`.meta`), wired via `PlayerControllerPrefab` on
`TBD_GameMode.et` together with `m_bAutoPlayerRespawn 0` / `m_bAllowFactionChange 0`.
It inherits vanilla `DefaultPlayerControllerMP.et` and disables exactly the two
**body-creating** request components — `SCR_FreeSpawnRequestComponent` and
`SCR_SpawnPointRespawnRequestComponent` — while leaving `SCR_RespawnComponent` and
`SCR_PossessSpawnRequestComponent` enabled, because the possess route is now the deploy
path. Vanilla therefore cannot spawn a body on a framework world, but can still hand a
player to one that exists. Proof it is live: the session spawns
`{1A1ABD939E1E8423}Prefabs/Systems/TBD_PlayerController.et` where it previously spawned
vanilla `{225E51284CC95CFA}…DefaultPlayerControllerMP.et`.

Component-instance GUIDs are vanilla facts; PlayableSelector was used as a design
reference only, never as a source of code.

## Death → redeploy (new)

With the vanilla deploy menu stood down, nothing re-deployed a killed player: `OnPlayerKilled`
re-armed the guard and then nobody asked again. It now schedules `RedeployAfterDeath` after
`m_iRedeployDelayMs` (new attribute, default 5000), gated on `m_bAutoDeploy`, guarded on
still-connected and not-already-deployed. `DeployPlayerEx` finds the slot body dead and
rematerializes a fresh dressed one — re-equip every spawn, operator-locked; the corpse stays.

## Harness

`scripts/mod/tbd-spawn-determinism.sh` gains two per-run assertions: total
`has switched from faction` lines > 3 fails the run, and **any** such line after the
`[TBD][Audit]` census line fails it (the sharp signal — a switch after census means the hunt
loop is alive). A healthy run emits 0.

## Gate — INCOMPLETE (2/5), not a pass

The 5-run gate was **stopped after run 2 at the operator's request** (the harness restarts
Workbench every run and they needed the machine). What the two completed runs show:

- **run1 / run2 digests byte-identical** — `6abcbdd84e02`, 21 canonical lines each
- census `characters=1 bodies=1 players=1` both runs (materialization model holds:
  characters == bodies, so the possess route created nothing)
- **zero** `SCRIPT (E)` / Virtual Machine Exception lines
- **zero** `has switched from faction` lines — the new churn assertions pass
- `possess request accepted` and `vanilla respawn system suppressed (framework world)`
  present in both

**This is not the required 5/5 and must not be read as one.** To close it:
`bash scripts/mod/tbd-spawn-determinism.sh 5` (~15 min, restarts Workbench per run).

Digest moved from the prior program's `3e31fd8cf7c6` (18 lines) to `6abcbdd84e02`
(21 lines) — expected: `+possess request accepted`, `+vanilla respawn system suppressed`,
`+cargo` lines now under the slot tag, and the old vanilla-pipeline lines gone. The gate
compares runs against each other, not against a stored golden, so it re-baselines itself.

## Named follow-ups (no silent deferrals)

- **Dedicated-server client-side spawn invoke.** Our fallback-path invoke is server-side
  only; PlayableSelector RPCs theirs to the client as well. Not exercised in PIE
  (single process), and the possess route makes it moot on the primary path — but it is
  the fallback's known gap on a real server.
- **Dedicated / JIP pass** (playerId reuse, real audit, reconnect reclaim) — PIE cannot
  exercise these. Pre-existing item, still open.
- **T-068.13 lobby / slot picker** — `ClaimSlot` is the ready backend; `m_bAutoDeploy`
  turns the PIE wave off when the picker lands.
- **Operator manual receipt still outstanding:** death → redeploy (grenade at feet →
  fresh dressed body at the slot after ~5 s, corpse remains). PIE logs cannot prove the
  player-death path.
