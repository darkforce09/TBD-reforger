# TBD_MOD_DESIGN — what we are building, and why

**The north star.** One doc that states what the TBD Framework mod *is*. If a slice ever
conflicts with this, this wins. Program hub: [`t181_event_mod_program.md`](t181_event_mod_program.md).

Every factual claim about CRF or vanilla carries an `@idx <lane>#<Symbol>` marker. `make verify-oracle`
resolves each one against a generated index and fails the build if it does not exist. **Line
numbers are never typed by hand** — cite the name, ask the tool for coordinates:

```bash
cargo run -q -p tbd-tools --bin enf -- lookup UpdateSlotPlayerID
```

---

## 1. The thesis

**TBD is rebuilding the Arma 3 mission workflow inside Arma Reforger.**

| Arma 3 | TBD Reforger |
|---|---|
| Eden editor | the website Mission Creator (`apps/website/frontend`) |
| `.pbo` mission file | **compiled mission JSON** — `GET /api/v1/missions/:id/compiled` |
| Engine-native lobby, briefing, slotting, respawn, spectator | **Reforger ships none of it** |
| Mission calls into a framework | mission JSON drives slots, loadouts, objectives, AO, radio, win conditions |

Arma 3 hands you the lobby, briefing, slot selection, respawn and spectator for free. Reforger
hands you nothing. **That gap is the program.**

## 2. Non-negotiables

- **One life.** TBD events are one life. Death is terminal by design. An **admin can respawn**
  a player who died to a glitch — that path must always exist. CRF is wave/ticket respawn; we
  deliberately diverge.
  **OPERATIONAL CONSEQUENCE — run real events on a DEDICATED server only.** One life is not
  durably enforceable on a listen/hosted server, and this is an engine limit, not a bug we can fix:
  with no backend identity, vanilla `SCR_PlayerIdentityUtils` synthesizes a uuid from the player's
  NAME hash. Durable-across-reconnect and collision-free are therefore mutually exclusive there.
  TBD chooses durable (a same-name reconnect keeps its spent life) and shouts the cost once at
  WARNING: a name change buys a fresh life, and two players sharing a name share one life.
  **What a reconnect restores (accepted limit).** The player's SEAT and a fresh dressed body at the
  slot transform — NOT their position or inventory at the moment they dropped. Vanilla's reservation
  path (`SCR_ReconnectComponent` → `SCR_SpawnLogic.ResolveReconnection`) hangs off the join hook TBD
  deliberately swallows, so honouring it means re-implementing that machinery. Out of scope; stated
  so nobody reports it as a bug.
- **JSON is the contract.** Missions, slots, loadouts, objectives, zones and radio plan all come
  from the compiled mission document. No `.conf` gearscript concept — CRF's 520 gearscript files
  are replaced by per-slot loadout data.
- **The UI is ours.** macOS design *methodology* — direct manipulation, one obvious primary
  action, immediate feedback, progressive disclosure, nothing blocking — rendered in the
  website's **Aegis** tokens (`apps/website/frontend/style/aegis.css`). CRF's `.layout` files are
  behavioural reference only; never ship their look.
- **CRF is an oracle, never a dependency.** Arma Public License. Indexed, cited, never vendored.
  `tbd-framework` takes **no workshop dependencies** — its only `addon.gproj` dependency is the
  vanilla data addon `58D0FB3206B6F859`.
- **PlayableSelector is design-mirror only.** No licence; never copy code.

## 3. The event loop

```
LOBBY ──▶ BRIEFING ──▶ SAFESTART ──▶ LIVE ──▶ END
  │           │                        │
  │           │                        └─ death ─▶ SPECTATOR ─(admin only)─▶ LIVE
  │           └─ read mission, plan with your side
  └─ pick side → group → slot   (a claimed slot is exclusive; survives reconnect)
```

CRF's own machine is a plain integer increment over four states — see
`CRF_EGamemodeState` @idx crf#CRF_EGamemodeState and `AdvanceGamemodeState` @idx crf#AdvanceGamemodeState.
Ours adds SAFESTART and replaces AAR with END, because AAR is deferred (§6).

## 4. What the mod must supply

Full triage — 57 capabilities, every CRF file accounted for — lives in
[`capability_verdicts.tsv`](capability_verdicts.tsv), enforced by `make verify-capability`
(a CRF capability with no TBD verdict is a **build error**). The spine:

| # | Capability | Why it is spine |
|---|---|---|
| 0 | UI framework — menu stack, focus, reusable listbox | blocks every screen below |
| 1 | Lobby / slotting authority | the event cannot start without it |
| 2 | Briefing screen | operator named it explicitly |
| 3 | Gamemode state machine + timers | drives everything |
| 4 | Spawn / possess | already largely built (`TBD_SpawnManager`) |
| 5 | One-life death model + admin respawn | TBD-specific; diverges from CRF |
| 6 | JSON-driven loadouts | replaces gearscript |
| 7 | Admin menu + permissions | operator named it explicitly |
| 8 | Spectator | one life makes it mandatory, not optional |
| 9 | Objectives / win conditions from JSON | ends the round |
| 10 | Replication backbone + faction/groups | everything above needs it |

## 5. Load-bearing Enfusion facts

Proven, not remembered. Each resolves through `make verify-oracle`.

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
handler. CRF documents this in its own header comment; 119 replicated properties are catalogued in
`crf_rplprops.tsv`.

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

Recorded here because CLAUDE.md forbids silent deferrals.

- **Full AAR / statistics recording — DEFERRED.** Operator: *"that's also the AAR, which is not
  easy to do… we have to record everything. That's very complex. I don't feel like we have the
  time."* Results POST survives as a **thin** end-of-round summary only.
- **Out of scope:** CRF's 10+ game modes, persistence/save-load, vehicle depot, parachutes,
  airdrop, mortar, battle royale, rally points, third-party mod bridges (ACE/CVON/CSI/…).
- **Radio/VON is wanted but not via CRF's route** — CRF depends on the external CVON workshop
  mod; TBD must not. Reimplement from `radioPlan.nets[]`.

## 7. How to work here

1. **Query before designing** — `enf lookup`, `enf dirs`, `capability_matrix.tsv`.
2. **Compile on the fast lane** — `make mod-compile` is ~1.3 s and needs no Workbench.
   Workbench is only for world/prefab/PIE visual work, and it is serial.
3. **Cite what you claim** — `@idx`, or it is an opinion.
4. **Make it work → right → fast.** Working is not half-assed.
5. **Never widen a process kill** to `ArmaReforgerServer`; the operator runs their own servers.
