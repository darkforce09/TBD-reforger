**Status:** live

# Mod design

What the TBD Framework [mod](/documentation_v2/glossary.md#mod) is, the rules every slice of it
keeps, and the [Enfusion](/documentation_v2/glossary.md#enfusion) facts it stands on. When a slice
conflicts with this document, this document wins. The program spec is
[t181_event_mod_program.md](/documentation_v2/tickets/specs/t181_event_mod_program.md).

Every factual claim about CRF or vanilla carries an `@idx <lane>#<Symbol>` marker.
`cargo run -q -p developer-tools --bin enf -- citations` resolves each one against the generated
symbol index and fails when one does not exist. Line numbers are never typed by hand: cite the
name, and ask the tool for coordinates:

```bash
cargo run -q -p developer-tools --bin enf -- lookup UpdateSlotPlayerID
```

## 1. The thesis

**TBD rebuilds the Arma 3 mission workflow inside Arma Reforger.**

| Arma 3 | TBD Reforger |
|---|---|
| Eden editor | the website [Mission Creator](/documentation_v2/glossary.md#mission-creator) (`apps/website/frontend`) |
| `.pbo` mission file | the compiled [mission](/documentation_v2/glossary.md#mission) [artifact](/documentation_v2/glossary.md#artifact), which the game server fetches from `GET /api/v1/game-runtime/artifacts/{artifactId}` |
| Engine-native lobby, briefing, slotting, respawn, spectator | **Reforger ships none of it** |
| Mission calls into a framework | the mission document drives slots, loadouts, objectives, AO, radio and win conditions |

Arma 3 hands a community the lobby, briefing, slot selection, respawn and spectator for free.
Reforger hands it nothing. **That gap is the program.**

## 2. Non-negotiables

- **One life.** TBD [events](/documentation_v2/glossary.md#event) are one life. Death is terminal
  by design. An **admin can respawn** a player who died to a glitch, and that path always exists
  (`#tbd respawn <playerId>`, see the
  [admin scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/README.md)). CRF is
  wave/ticket respawn; TBD diverges deliberately.
  **OPERATIONAL CONSEQUENCE — run real events on a DEDICATED server only.** One life is not
  durably enforceable on a listen/hosted server, and this is an engine limit, not a fixable bug:
  with no backend identity, vanilla `SCR_PlayerIdentityUtils` synthesizes a uuid from the player's
  NAME hash. Durable-across-reconnect and collision-free are therefore mutually exclusive there.
  TBD chooses durable (a same-name reconnect keeps its spent life) and states the cost once at
  WARNING: a name change buys a fresh life, and two players sharing a name share one life.
  **What a reconnect restores (accepted limit).** The player's SEAT and a fresh dressed body at the
  slot transform — NOT their position or inventory at the moment they dropped. Vanilla's reservation
  path (`SCR_ReconnectComponent` → `SCR_SpawnLogic.ResolveReconnection`) hangs off the join hook TBD
  deliberately swallows, so honouring it means re-implementing that machinery. It is out of scope,
  and stated here so nobody reports it as a bug.
- **JSON is the contract.** Missions, slots, loadouts, objectives, zones and the radio plan all come
  from the compiled mission document. There is no `.conf` gearscript concept: CRF's 260 gearscript
  configs (520 files with their `.meta`) are replaced by per-slot loadout data.
- **The UI is ours.** macOS design *methodology* — direct manipulation, one obvious primary
  action, immediate feedback, progressive disclosure, nothing blocking — rendered in the
  website's **Aegis** tokens (`apps/website/frontend/style/aegis.css`), which `TBD_UITheme`
  mirrors. CRF's `.layout` files are behavioural reference only; their look never ships. The
  [mod UI index](/documentation_v2/mod/tbd-framework/UI/README.md) says where each screen lives.
- **CRF is an oracle, never a dependency.** Arma Public License: indexed, cited, never vendored.
  `tbd-framework` takes **no workshop dependencies** — its only `addon.gproj` dependency is the
  vanilla data addon `58D0FB3206B6F859` (`apps/mod/tbd-framework/addon.gproj`).
- **PlayableSelector is design-mirror only.** It ships with **no licence file at all**, which is
  worse than APL rather than better: default copyright applies, so there is **no permission to copy,
  adapt or redistribute any of it**. Read it to learn how a lobby or slot picker is *shaped*, then
  write TBD's own; never a line. The `PS_` lane of `cargo xtask verify no-crf-leak` enforces it: the
  lane reads `apps/mod/playable_selector` when a checkout holds that folder, else the folder the
  `TBD_PS_ORACLE` variable names. The full rules are §Oracle lanes of the
  [mod slice workflow](/documentation_v2/runbooks/mod_slice_workflow.md).

## 3. The event loop

`TBD_EGameStage` (`apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Stages/TBD_GameStage.c`) holds
seven stages; `TBD_FrameworkManager` owns the current one.

```text
LOADING ──▶ LOBBY ──▶ BRIEFING ──▶ SAFE_START ──▶ LIVE ──▶ END ──▶ DEBRIEF
              │          │                          │
              │          │                          └─ death ─▶ SPECTATOR ─(admin respawn only)─▶ LIVE
              │          └─ read the mission, plan with your side, Ready & Continue deploys you
              └─ pick side → group → slot   (a claimed slot is exclusive; survives reconnect)
```

CRF's own machine is a plain integer increment over four states — see
`CRF_EGamemodeState` @idx crf#CRF_EGamemodeState and `AdvanceGamemodeState` @idx crf#AdvanceGamemodeState.
TBD adds SAFE_START and closes a round with END (the winner and the reason) and DEBRIEF (a
scoreboard of kills and deaths per player) instead of a full after-action review, which §6
defers. The stage machine is described in the
[Stages README](/apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Stages/README.md) and the
[Orchestrator README](/apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/README.md).

## 4. What the mod must supply

The full triage — 57 capabilities, every CRF file accounted for — lives in
[capability_verdicts.tsv](/documentation_v2/mod/tbd-framework/capability_verdicts.tsv), indexed by
[capability verdicts](/documentation_v2/mod/tbd-framework/capability_verdicts.md) and enforced by
`cargo run -q -p developer-tools --bin enf -- capability` (a CRF capability with no TBD verdict is
a **build error**). The spine:

| # | Capability | Why it is spine | Code |
|---|---|---|---|
| 0 | UI framework — menu stack, focus, reusable listbox | blocks every screen below | [UI/Core](/apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/) |
| 1 | Lobby / slotting authority | the event cannot start without it | [Session/Lobby](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/) |
| 2 | Briefing screen | the operator named it explicitly | [Session/Briefing](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/) |
| 3 | Gamemode state machine + timers | drives everything | [Gamemode](/apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/) |
| 4 | Spawn / possess | the deploy path every stage relies on | [Systems/Spawning](/apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/) |
| 5 | One-life death model + admin respawn | TBD-specific; diverges from CRF | [Systems/Spawning](/apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/), [Session/Admin](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/) |
| 6 | JSON-driven loadouts | replaces gearscript | [Systems/Loadouts](/apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Loadouts/) |
| 7 | Admin menu + permissions | the operator named it explicitly | [Session/Admin](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/) |
| 8 | Spectator | one life makes it mandatory, not optional | [Session/Spectator](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/) |
| 9 | Objectives / win conditions from JSON | ends the round | [Gamemode/Objectives](/apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/) |
| 10 | Replication backbone + faction/groups | everything above needs it | [Gamemode/Orchestrator](/apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/), [Systems/AI](/apps/mod/tbd-framework/Scripts/Game/TBD/Systems/AI/) |

## 5. Load-bearing Enfusion facts

Proven, not remembered. Each resolves through `cargo run -q -p developer-tools --bin enf -- citations`.

**Deploy is a POSSESS request, not a raw takeover.** `SCR_PossessSpawnData` @idx api#SCR_PossessSpawnData
exposes `static SCR_PossessSpawnData FromEntity (notnull IEntity entity)`, handed to
`SCR_PossessSpawnRequestComponent` @idx api#SCR_PossessSpawnRequestComponent. `SetInitialMainEntity`
possesses the body and even gives it a camera, but **it is not a spawn** — vanilla's finalize never
runs, so the client sits on the loading screen forever. CRF found that pipeline by
modding the vanilla handler: `SCR_PossessSpawnHandlerComponent` @idx crf#SCR_PossessSpawnHandlerComponent
(in `VanillaOverrides/CRF_SCR_PossessSpawnHandlerComponent.c` — the file name is not the class
name) hooks `OnFinalizeDone_S` @idx crf#OnFinalizeDone_S, which is the finalize step the client
waits on.

**`[RplProp(onRplName:)]` fires automatically only on the proxy.** Authority must invoke its own
handler. CRF documents this in its own header comment; its 119 replicated properties are catalogued
in `.ai/artifacts/enf-index/crf_rplprops.tsv`.

**The join hook is `OnPlayerAuditSuccess`** @idx crf#OnPlayerAuditSuccess — *not* `OnPlayerConnected`.

**Slot claim/release are not the names you would guess.** Claim is
`UpdateSlotPlayerID` @idx crf#UpdateSlotPlayerID and release is
`CleanupCharacterFromSlot` @idx crf#CleanupCharacterFromSlot. CRF also hand-rolls bit
serialisation via `RplSave` @idx crf#RplSave / `RplLoad` @idx crf#RplLoad rather than relying on
`RplProp` alone. An agent asked to summarise that file invented `RequestSlotChange`,
`ReleaseSlot` and `GetInstance` — none exist. **This is why the index is mechanical.**

**Menus derive from `ChimeraMenuBase`** @idx api#ChimeraMenuBase. **Vanilla respawn lives in
`SCR_RespawnSystemComponent`** @idx api#SCR_RespawnSystemComponent, which TBD stands down on
framework worlds. **`SCR_BaseGameMode`** @idx api#SCR_BaseGameMode is the gamemode base.

**Enfusion is lenient.** `int x = ;` compiles clean; undefined symbols are what actually error.
Do not rely on the compiler to catch sloppiness.

**Enforce `set`/`array.Remove` is by index** — use `map<K,bool>` / `RemoveItem`.

## 6. Deferrals — by operator word

Recorded here because `CLAUDE.md` forbids silent deferrals. The `DEFERRED` verdict in
[capability_verdicts.tsv](/documentation_v2/mod/tbd-framework/capability_verdicts.tsv) points here.

- **Full AAR / statistics recording — DEFERRED.** Operator: *"that's also the AAR, which is not
  easy to do… we have to record everything. That's very complex. I don't feel like we have the
  time."* The results POST (`TBD_ResultsReporter` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/`) stays a **thin** end-of-round summary, and the
  DEBRIEF scoreboard shows only kills and deaths per player.
- **Out of scope:** CRF's 10+ game modes, persistence/save-load, vehicle depot, parachutes,
  airdrop, mortar, battle royale, rally points, third-party mod bridges (ACE/CVON/CSI/…).
- **Radio/VON is wanted but not via CRF's route** — CRF depends on the external CVON workshop
  mod; TBD must not. TBD tunes the engine's own radios from the mission's `radioPlan.nets[]`
  (see the [Radio README](/apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Radio/README.md)).

## 6b. Spectator streaming range

A dead player's spectator camera steers an inert streaming host (`TBD_SpectatorHost`), so the free
camera sees a world that is actually loaded. The side effect is that streaming range is *steerable
by a spectating client*, and `TBD_SpectatorTargets`'s faction filter is discipline whose real
backstop is "the engine's own replication range": a **modified** client could pull the enemy AO
into scope.

The code settles it with a leash: `m_fHostMaxRangeM` on `TBD_SpectatorComponent`
(`apps/mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/TBD_SpectatorComponent.c`) defaults
to 2000 m from the player's own death position; 0 or any non-positive value means that default,
never unlimited, and the authority clamps every host move to it. A shorter leash is a smaller
positive number, a longer one a larger number. Against a longer leash: one life means spectators
are *players who are out*, not neutral observers, and the group with the most motive; for it,
TBD events are a known community, not a public server.

## 7. How to work here

1. **Query before designing** — `enf lookup`, `enf dirs`, `.ai/artifacts/enf-index/capability_matrix.tsv`.
   The [vanilla source coverage](/documentation_v2/mod/tbd-framework/vanilla_source_coverage.md)
   says which lane answers which vanilla question.
2. **Compile on the fast lane** — `cargo xtask mod compile` takes seconds and needs no Workbench.
   [Workbench](/documentation_v2/glossary.md#workbench) is only for world, prefab and play-in-editor
   visual work, and it is serial.
3. **Cite what you claim** — `@idx`, or it is an opinion.
4. **Make it work → right → fast.** Working is not half-assed.
5. **Never widen a process kill** to `ArmaReforgerServer`; the operator runs their own servers.

## Open work

- [T-181 — TBD Framework, Arma-3-parity event mod](/documentation_v2/tickets/specs/t181_event_mod_program.md)
  (deferred, no plan): the program parent this document serves.
- [T-181.16 — Two-client dedicated-server event loop E2E](/.ai/tickets/T-181.16.toml) (queued, no
  plan): a human playtest proving connect, slot, brief, deploy, terminal death and admin respawn
  on a dedicated server with two real clients.
- [T-941 — Enfusion mod lifecycle](/documentation_v2/tickets/specs/t941_mod_lifecycle.md) (queued,
  [plan](/documentation_v2/tickets/plans/t-941_plan.md)): safe start, lobby deploy, END and
  DEBRIEF, the objective HUD and the spectator clamp; every child ticket has shipped.
- [T-1085 — Feed the pre-game screens live catalogs instead of mocks](/.ai/tickets/T-1085.toml)
  (idea, no plan): the Mission Selector, lobby, briefing and players panel read the server's data
  instead of the mock catalogs.
- [T-1086 — Close mission runtime gaps](/.ai/tickets/T-1086.toml) (idea, no plan): mission
  parameters with no reader, group AI defaults, audio for late joiners.
- [T-1092 — Split EnfScript files over 500 lines and gate their length](/.ai/tickets/T-1092.toml)
  (idea, no plan): Law 7 for the mod's scripts.

## Related documentation

- [TBD Framework documentation](/documentation_v2/mod/tbd-framework/README.md) — the index of this
  folder
- [Framework addon](/apps/mod/tbd-framework/README.md) — the addon's code, configuration and
  load order
- [Mod slice workflow](/documentation_v2/runbooks/mod_slice_workflow.md) — how a slice is built,
  compiled and verified against these rules
