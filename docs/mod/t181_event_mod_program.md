> **T-891/T-892/T-890 (2026-08):** `compile.sh` / `world-boot.sh` / `mod/wave.sh` are deleted — use `cargo run -q -p xtask -- mod compile|world-boot|wave`. Historical bash paths below are archive.

# T-181 — TBD Framework: the Arma-3-parity event mod

**Hub doc.** Open this to know what the program is and what to run next.
Registry: [`.ai/tickets/registry.json`](../../.ai/tickets/registry.json) · North star:
[`TBD_MOD_DESIGN.md`](TBD_MOD_DESIGN.md) (T-181.4) · Capability verdicts:
[`capability_verdicts.tsv`](capability_verdicts.tsv)

## What this program is

**TBD is rebuilding the Arma 3 mission workflow inside Arma Reforger.**

| Arma 3 | TBD Reforger |
|---|---|
| Eden editor | the website Mission Creator (`apps/website/frontend`) |
| `.pbo` mission file | **compiled mission JSON** (`GET /api/v1/missions/:id/compiled`) |
| Engine-native lobby, briefing, slotting, respawn, spectator | **Reforger ships none of it — the mod must build all of it** |

That last row is the whole reason this is a program and not a feature. **CRF** (Arma Public
License, 266 `.c` / 71,606 LOC, gitignored at `apps/mod/crf_framework`) is the proof it can be
done and is the reference oracle — **indexed, never vendored**.

**Locked decisions:** events are **ONE LIFE** (death is terminal; an admin can respawn a glitch
death). Lobby UI follows **macOS methodology** — frictionless, direct-manipulation — rendered in
the website's **Aegis** tokens (`apps/website/frontend/style/aegis.css`). Full AAR/statistics is
**DEFERRED by operator word**.

## Run this

```bash
cargo xtask mod compile             # Enfusion compile gate — ~1.3s, native, no Workbench
cargo xtask mod compile-selftest    # prove the gate still catches a broken .c
cargo run -q -p tbd-tools --bin enf -- capability       # fails if any CRF capability has no TBD verdict
cargo run -q -p tbd-tools --bin enf -- index crf               # rebuild the CRF symbol index

cargo run -q -p tbd-tools --bin enf -- lookup UpdateSlotPlayerID
cargo run -q -p tbd-tools --bin enf -- dirs --depth 5 --min 60
```

**Everything runs on the host.** Agent shells run inside a `debian:12` container with no C
toolchain, so builds and game binaries route through `scripts/lib/hostrun.sh`
(`distrobox-host-exec`). An in-container `cargo build` fails with `linker cc not found`, and a
host-built binary fails in-container with `GLIBC_2.39 not found`. **Neither means the repo is
broken** — a session once "fixed" that misdiagnosis and destroyed 2.6 GB of build artifacts.

## The two compile lanes

| Lane | Runs on | Serial? | Covers |
|---|---|---|---|
| **Fast** | native `ArmaReforgerServer` | **No — parallel** | script compile, script runtime, headless MP, JIP, determinism |
| **Slow** | Workbench (Proton, :5775) | Yes, one instance | world/prefab authoring, resource DB, PIE visual checks |

Default to the fast lane. Measured: compile **780 ms**, whole gate **1.3 s** — versus a
90–120 s Workbench restart that also needed a human to click "Open".

## Stage 1 — FACTORY (shipped)

| Slice | What | Gate |
|---|---|---|
| T-181.0 | `scripts/lib/hostrun.sh` host-aware execution | `ticket check` green from the container |
| T-181.1 | `scripts/mod/compile.sh` + `cargo xtask mod compile` | 0 clean / 1 + `file:line` broken |
| T-181.2 | `enf` bin + CRF index (`.ai/artifacts/enf-index/crf_*.tsv`) | real symbol resolves, invented one does not |
| T-181.2.1 | capability matrix + UNTRIAGED gate | 57 capabilities, zero untriaged |

| T-181.3 / .3.2 | vanilla carve + by-name pak extract | 5 prove-out classes resolve |
| T-181.3.1 | official BI Script API mirror (7,990 classes) | signatures for everything |
| T-181.3.3 | **full vanilla source WITH BODIES** via AR Explorer | `SCR_BaseGameMode.c` resolves |
| T-181.4 | `TBD_MOD_DESIGN.md` + `@idx` citation gate + CRF-leak gate | hallucinated symbol exits 1 |

**The vanilla oracle has four lanes** — see [`vanilla_carve_coverage.md`](vanilla_carve_coverage.md).
Lane 4 (AR Explorer Doxygen source, same game version, method bodies included) supersedes the
others; the pak-compression codec was never cracked and no longer needs to be.

## Stage 2 — the mod spine

**The event loop exists end to end in code:**
`LOBBY (slot picker) → BRIEFING → deploy → ONE LIFE → death → SPECTATOR → admin respawn → JSON win condition → END`

| Shipped | |
|---|---|
| gamemode state machine · one-life death model · admin respawn · JSON win conditions | wave 0 |
| UI framework · JSON loadouts · mission validation | wave 1 |
| one-life hardening (possess route + durable identity) · spectator · briefing | wave 2 |
| lobby slot picker (supersedes T-068.13) · admin menu · mission-doc parser | wave 3 |
| JIP/reconnect hardening · golden fixtures | wave 4 |

**Queued** — waves 5–7: spectator streaming host · AO poly-zones · map markers · results POST ·
briefing ORDERS section · briefing JIP catch-up · lobby raise fix · wire-format sentinel ·
`/compiled` endpoint validation.

**Retired, not built:** `T-181.5` (Workbench `-gproj`) — the native compile lane removed the need.
`T-181.6` (auto-runner) — superseded by the command-center + slice-agent model below.

**Everything above is COMPILE-VERIFIED, not runtime-verified.** No screen has ever opened (see
§The Workbench pass) and no behavioural claim has been observed. That is the honest state.

## Execution model — command center + slice agents

This chat is the **command center**. It owns the registry, integration, and sequencing. Each slice is
delegated to a **full-capability Opus subagent** with a self-contained prompt, so the main context
stays clear and each slice gets the whole context window to reason in.

**Why this is safe now and was not before the factory.** A slice agent can *prove* its work instead
of claiming it:

| Gate | What it proves |
|---|---|
| `bash scripts/mod/compile.sh` | the Enfusion actually compiles (~1.3 s, no Workbench) |
| `enf lookup <Symbol>` | the API being called genuinely exists |
| `cargo xtask verify no-crf-leak` | no oracle code leaked into prod — CRF (Arma Public License) **and** PlayableSelector (no licence at all); see SLICE_WORKFLOW.md §Oracle lanes |
| `cargo run -q -p tbd-tools --bin enf -- capability` | no capability silently forgotten |
| `cargo run -q -p tbd-tools --bin enf -- citations` | no invented `file:line` in docs |

**Rules for slice agents** (put these in every prompt):
1. **Host-aware.** Prefix builds/game binaries with `distrobox-host-exec`. Explain WHY, or the agent
   will "fix" a working toolchain — that already cost 2.6 GB once.
2. **Never self-ship.** Agents implement + compile-verify + report. The command center owns
   `registry.json` and all status transitions.
3. **Batch by file-disjointness, not by count.** The parallelism limit is file collisions, not
   Workbench — the compile gate is parallel-safe (unique temp profile per run, process-group kill
   scoped to that run). `TBD_SpawnManager.c` and `TBD_FrameworkManager.c` are the contended files;
   never give two concurrent agents write access to the same one. Tell each agent which files other
   agents hold, and to REPORT needed hooks rather than edit across the line.
4. **Point at docs, don't inline them.** `TBD_MOD_DESIGN.md` + this hub + `capability_verdicts.tsv`
   carry the binding context.
5. **Demand honesty about verification depth** — compile-verified is not runtime-verified. Anything
   visual needs the operator's eyes; no tool here returns a framebuffer.

## How we work here

- **CRF-first.** Query the index before designing. Don't reinvent the wheel; don't ship its look.
- **Prove Enfusion from source.** Index row, carved vanilla, or `api_search` — never memory.
  An agent summarising `CRF_SlottingManager.c` invented four APIs (`RequestSlotChange`,
  `ReleaseSlot`, `GetInstance`, a wrong base class) and missed `RplSave`/`RplLoad`. That is why
  symbols are extracted mechanically and prose cites index rows.
- **Make it work → right → fast.** Working is not half-assed.
- **No silent deferrals.** The only deferral list is §Deferrals in `TBD_MOD_DESIGN.md`, and it
  carries the operator's own words.

## Absorbed work

**T-068.13** (production LOBBY / slot picker) is **superseded by T-181.9.1**. Scope moved, not
dropped: the picker ships on the new gamemode state machine. `ClaimSlot` in `TBD_SpawnManager`
is already the backend it binds to; `m_bAutoDeploy` turns off the PIE auto-wave when it lands.

## The Workbench pass — what it unblocks and what it settles

One slow-lane pass (open `apps/mod/tbd-framework` in Workbench so it regenerates
`resourceDatabase.rdb`) is now gating FOUR screens and settling two open risks. Do it before the
first live test, then verify headlessly.

**Unblocks** — every modded menu preset currently fails with `GUI (E): Menu preset '<name>' not
found!`, so none of these can open: `TBD_UIShell`, `TBD_Spectator`, `TBD_UIBriefing`, `TBD_UILobby`
(+ admin). Non-script resources (`.conf`, `.layout`) are invisible to the engine until the rdb is
rewritten; `.c` files are directory-scanned and are NOT affected.

**Measured inventory (2026-07-25)** — `resourceDatabase.rdb` is 4,513 bytes and knows about
**4 of the 14** non-script resources in the addon. The 10 it cannot see:

| Missing from rdb | Gates |
|---|---|
| `Configs/System/chimeraMenus.conf` | all five menu presets |
| `UI/layouts/TBD_ScreenShell.layout`, `TBD_ListRow.layout` | every screen's widgets |
| `Configs/System/ActionContext/TBD_SpectatorContext.conf` | spectator input context |
| `Configs/System/Actions/TBD_Spec{Free,Next,Prev,Roster,View}.conf` | spectator keybinds |
| `Configs/System/Actions/TBD_AdminMenu.conf` | admin menu keybind |

It does contain `TBD_GameMode.et` and `TBD_PlayerController.et`, which is exactly why the game mode
wires up fine today while no screen can open. This is also why wave 5 deliberately avoided authoring
new `.et`/`.conf`/`.layout` files: the spectator streaming host is prefab-free
(`GetGame().SpawnEntity(typename, …)`) and markers ride the vanilla placed-marker system, so
**neither of those slices is gated on this pass**.

**Settles** — T-181.25: whether four separate `modded enum ChimeraMenuPreset` blocks produce
distinct values (unprovable from the compile lane), and whether three `modded class
SCR_PlayerController` blocks coexist at runtime.

**SECOND JOB IN THE SAME SITTING (T-181.40) — place a `RadioManagerEntity` in the world.**
Radio tuning turned out to be fully reachable from script; the blocker is a missing world asset.
The engine says so itself on every boot: `DEFAULT (W): World doesn't contain RadioManagerEntity to
support any BaseRadioComponent.` `worlds/TBD_Dev_POC.ent` is a 62-byte bare SubScene of vanilla
`Eden.ent` and places nothing. The whole tuning chain
(`SCR_GadgetManagerComponent.GetGadgetManager` → `GetGadgetsByType(EGadgetType.RADIO)` →
`SCR_RadioComponent.GetRadioComponent()` → `BaseRadioComponent.GetTransceiver(i)` →
`BaseTransceiver.SetFrequency(kHz)`) is `proto external` and compile-proved with a failing negative
control. So this is a world edit, not a code change — and once it lands, automatic tuning works with
**no code change**, because `TBD_RadioTuner` already reads the frequency back and only counts a net
as tuned when it observes it. Until then it honestly says *"Radio tuning is unavailable on this
world — dial these in by hand."*

**Green light:** the `Menu preset … not found!` lines disappear from
`<profile>/logs/logs_*/error.log`. That check is headless and takes ~20 s — no GUI needed to confirm.

**Then, and only then:** flip `m_bAutoDeploy` to `0` in `TBD_SpawnManager`. It is `1` today because
the LOBBY auto-deploy wave is currently the ONLY working way into the world; flipping it before the
picker can open would ship a mod nobody can deploy into.

## Landmines (measured — do not relearn)

- **`-config` and `-addons` are MUTUALLY EXCLUSIVE** on `ArmaReforgerServer` (measured 2026-07-25,
  engine 1.7.0.54). Passing both is a hard fatal: `DEFAULT (F): -config cannot be used together with
  addons!` → `ENGINE (E): Unable to initialize the game`. To boot a world with a LOCAL addon, pass
  `-addonsDir <dir>` **and** list the addon in the config's `game.mods[]` keyed by the **GUID from
  `addon.gproj`** (not a Workshop id): `"mods":[{"modId":"B2C3D4E5F6A78901","name":"TBD_Framework"}]`.
- **`-addons` + `-scenarioId` with no `-config` looks like it works and does not.** The engine prints
  `Game successfully created` and then idles — it never starts hosting, so the world never loads and
  no game-mode prefab is ever instantiated. This is the harness that previously "proved" the
  components could not be verified; it proves nothing either way. Absence of `[TBD]` output under it
  is not evidence.
- **A component listed on a prefab whose class fails to resolve is dropped SILENTLY** — the `.et`
  still lists it, every script still compiles clean, and the only symptom is a feature that never
  runs. The compile gate cannot see prefab wiring at all. `scripts/mod/world-boot.sh` (in
  `wave.sh gate`) boots the real scenario and catches this **two** ways, and you need both:
  - `WORLD (E): Unknown class '<Name>'` — the engine's own diagnostic. **This is the load-bearing
    one**: name-independent, needs no maintenance, and covers every prefab in the mod.
  - `TBD_FrameworkManager.PrintComponentRollCall()` → `[TBD] roll-call: …=ok|MISSING`, which
    catches a class that *exists* but is not on the prefab.
  **The roll-call ALONE is not sufficient and must not be trusted as if it were.** The wave-4
  verifier added `TBD_ThisComponentDoesNotExist` to `TBD_GameMode.et` and the gate PASSED with
  `roll-call clean` — it only checks the five names it was handed, and `SCR_EditableEntityComponent`
  is on the prefab and deliberately not among them. Both negative controls are now reproduced in
  the selftest.
- **`Print(someLocalVariable)` emits the DECLARATION, not the value** — the log reads
  `string line = '[TBD] roll-call: …'`, quotes included. Use `PrintFormat("%1", v)` when the value
  is what matters. This silently broke a selftest: its fixtures used an idealised shape that never
  occurs, so it passed without ever exercising the real one.
- **The engine emits BOTH `@"Scripts/Game/…"` and `@"scripts/Game/…"` in the same run** (differing
  case), and plenty of TBD-relevant errors carry neither a `[TBD]` tag nor a path at all — e.g.
  `Instance of class TBD_SpawnManager is null`, or a `Virtual Machine Exception` with no `(E)`
  marker. **Never classify error ownership by message text and let the remainder pass.** That rule
  let six genuine TBD failures through. `world-boot.sh` now triages **fail-closed**: TBD-owned (case
  -insensitive) fails, an explicit vanilla allowlist is noted, and anything unrecognised **fails**.
- **Booting the real Eden world emits VANILLA script errors the mod does not own** (measured:
  `'SCR_BaseResupplySupportStationComponent' needs a entity catalog manager!`). They are allowlisted
  by exact pattern with a reason — not by "it doesn't look like ours".
- `[RplProp(onRplName:)]` fires automatically **only on the proxy**; authority must invoke its
  own handler (CRF says so at `CRF_Gamemode.c:8`).
- **Corollary that bites: on a LISTEN HOST, authority IS the local player**, so an `onRplName`
  handler alone never drives that player's UI. Client-side UI must be driven from BOTH the
  replicated callback (proxy) and the authority-side setter, through one guarded helper. Wiring only
  the callback silently breaks the host — and a poll, however ugly, does not have this bug.
- The join hook is `OnPlayerAuditSuccess(int)`, **not** `OnPlayerConnected`
  (`CRF_Gamemode.c:411`). Disconnect takes 3 args (`:465`).
- `SetInitialMainEntity` possesses a body but is **not a spawn** — the client hangs on the
  loading screen. Use the vanilla POSSESS request.
- The dedicated server **exits 0 even when compilation fails**. Read the logs, never `$?`.
- `int x = ;` compiles clean — Enfusion is lenient. Undefined symbols are what actually error.
- Enforce `set`/`array.Remove` is **by index**.
- **`resourceDatabase.rdb` gates NON-SCRIPT resources.** New `.c` files are directory-scanned and
  compile without an rdb entry, but new `.conf`/`.layout` files stay INVISIBLE to the engine until
  Workbench rewrites the rdb. A modded menu preset therefore cannot resolve from a script-only slice.
- **A missing/stale rdb makes the engine skip script compilation for every LOOSE addon** — the Game
  module silently drops to vanilla-only and the compile gate used to still print "compiled clean".
  `compile.sh` now ratchets the loaded-file count and fails on a large drop. A canary addon does NOT
  work for this (it dies with the mod — measured).
- **Clients have NO mission document and no slot assignment.** `TBD_FrameworkManager.OnPostInit`
  returns early for `RplMode.Client` before `BeginLoad()`, and `m_mPlayerSlot` is a plain map, not
  an `RplProp`. Every client-side screen must be SERVER-FED over RPC — which is also what makes
  side-discipline enforceable at the wire instead of in a widget.
- **`Formula too complex`** on a long `+` chain (measured at 9 fields). Worse, the SECOND diagnostic
  on that line is a misleading `Incompatible parameter` that sends you hunting a type error that does
  not exist. Fix: append in steps.
- **`string.Split(sep, out, false)` KEEPS empty tokens — MEASURED, and this retires the entry that
  said it was unknowable.** Engine 1.7.0.54, live dedicated boot, 2026-07-25 (T-181.26): `"a\t\tb"`
  splits to **three** tokens. Negative control in the same run — the same code on `"a\tb"` reported
  two, so the verdict reads the real count and is not a constant. The old entry here was right that
  a *compile probe* cannot settle it and wrong to conclude the program could not know: the answer
  was one `SelfCheckWire()` and one boot away, and four shipped files hand-rolled splitters to avoid
  a question nobody had asked the engine. **"Unprovable from the compile lane" is not "unobservable"
  — if a runtime property is load-bearing, arm a self-check and boot.**
  Two things this does NOT license:
  - **Do not delete the hand-rolled splitters.** The measurement pins ONE build of one engine;
    `TBD_BriefingService.SplitLines`'s output is determined by its own code on every build. Nine
    lines is cheaper than a re-measurement per engine bump.
  - **Do not drop the wire sentinels.** Every wire format still marks or sentinels its fields
    (`TBD_AdminData.FIELD_MARK`, `TBD_BriefingService.FIELD_MARK`, `TBD_LobbyData.EMPTY`), because
    the correctness of a format should not rest on a fact that could change under it — and a
    `trim = true` caller reintroduces the hazard immediately.
  Consequence worth stating: the pre-T-181.26 briefing wire was **accidentally correct** on this
  build. Its empty-field defect was latent, not live.
- **Three delimited wire formats, three different answers — converging.** `TBD_AdminData.FIELD_MARK`
  (`<TAB>.<value>` on every field) is **bijective**: `Unmark(Field(x)) == x` for every `x`.
  `TBD_LobbyData.EMPTY = "~"` is a plausibility argument and is **lossy** where it is wrong — a field
  authored as literally `~` round-trips to the empty string. T-181.26 put `TBD_BriefingData` on the
  marker, so `TBD_LobbyData` is now the odd one out; converging it is a two-helper change.
  The marker must stay a **single ASCII byte**: `Unmark` is `Substring(1, len - 1)` and `Substring`
  is BYTE-indexed, so a prettier multi-byte marker would corrupt every accented field.
- **`world-boot.sh` does NOT exercise any client-fed screen, and reading it as coverage is a
  mistake that has already been briefed to a slice agent.** MEASURED 2026-07-25: a `--mission=` boot
  runs with **zero players**, so `TBD_BriefingService.BuildForPlayer` / `Serialise` / `Parse` never
  execute and `grep -i briefing` over the whole console log returns only `flow.briefingSeconds` and
  the JIP stage list. The same holds for the lobby, admin, marker and radio payloads. The gate proves
  the game mode wires up and the mission document parses — not that a screen's wire format works.
  What it CAN do is catch a self-check: a deliberately broken `Unmark` produced
  `SCRIPT (E): [TBD][Briefing] wire self-check FAIL …` and the fail-closed triage turned it into
  `WORLD BOOT: FAIL`. So a service that self-checks at boot IS gated; one that self-checks lazily on
  first use is not.
- **`→` is not in the proven glyph set.** `·`, `—`, `…` are rendered by shipped screens; `→` is used
  nowhere. Prefer `->` on anything load-bearing rather than risk a tofu box.
- **Duplicate `switch` case labels compile CLEAN** — so a switch cannot be used to prove enum
  values are distinct, and any probe built on one is worthless without a failing negative control.
- **`ScriptCallQueue.Remove` cancels by FUNCTION, not by arguments.** You cannot cancel one
  player's pending `CallLater` without cancelling everyone's. Any deferred callback carrying a raw
  `playerId` therefore survives that player's disconnect — and on a dedicated server the id is
  recycled. Measured worst case: `RedeployAfterDeath` would have DEPLOYED a fresh joiner into a dead
  player's slot. Stamp a connection epoch on every deferred per-player callback.
- **Enfusion DOES compile-check `CallLater` callback arity** (negative control: `Not enough
  parameters in callback 'Three'`). So re-threading a deferred callback's signature is genuinely
  compile-verified, not compile-silent — a rare piece of good news.
- **`SCR_BaseGameMode.OnPlayerDisconnected` DELETES the disconnecting player's controlled entity** —
  i.e. our materialized slot body. `SCR_ReconnectComponent` would reserve it instead, but the
  matching re-apply hangs off the join path TBD swallows, so the reservation is never honoured and
  expiry deletes the body up to 120 s later.
- **THERE IS NO TERNARY OPERATOR.** `cond ? a : b` fails with `Broken expression (missing ';'?)`
  pointing at the whole statement and never mentioning `?`.
- **A `class X : SomeClass {}` component descriptor may need a trailing `;`** — without it the NEXT
  class fails with a misleading `Syntax error` / `Unexpected scope`.
- **The cached vanilla source is a DIFFERENT BUILD from the retail runtime.** Measured:
  `SCR_AIGroup.GetCallsignSingleString()` is called in cached `SCR_GroupsManagerComponent.c:434` but
  does not exist at runtime. The cache is a strong hint, never proof — **probe anyway**.
- **`GetGame().SpawnEntity(typename, world, params)` spawns a scripted entity with NO prefab** —
  which is how the spectator camera avoids the rdb blocker entirely.
- **CORRECTION #2 to the probe advice: assign-to-expected-type is NOT a complete control either.**
  It discriminates for SCALARS (`string x = tsv.GetFrequency()` correctly errors) but **a class
  reference assigns to `int` silently** — `int wrong = world.GetRadioManager();` compiles clean.
  So for anything returning a CLASS, the only reliable control is an **undefined-symbol** probe:
  fabricate `Foo.BarThatDoesNotExist()` alongside the real call and require the fake to error.
  Two separate slices have now had to correct this program's probe guidance; treat every probe
  technique here as provisional until its control has failed in the same run.
- **On a TRANSPORT failure, `RestCallback.GetData()` returns YOUR OWN REQUEST BODY, not a response.**
  Measured (T-181.35): a POST to a dead port came back carrying the request payload verbatim. So any
  body-text matcher is reading its own request — and naively logging it prints the player's secret
  link code into `console.log` labelled as the website's answer. **Branch on
  `RestCallback.GetHttpCode()`**, which does return the real HTTP status at runtime (verified for
  200/409/404/401/400, and `HTTP_CODE_NULL` for transport failure), and drop the body on the
  no-status branch.
- **`pgrep` through the host bridge FALSE-NEGATIVES — and this has now bitten twice.** An
  `ArmaReforgerServer` orphan was reported gone by `pgrep -a` and was still running 4 h 18 m later;
  `ps -o pid,etime -p <pid>` saw it immediately both times. **Confirm a process is dead with `ps`
  against the specific PID.** The first occurrence is recorded below; the second was the command
  center trusting its own earlier "confirmed gone".
- **A modded menu preset needs a `MenuConfigs` entry in `addon.gproj` — the rdb was NEVER the
  blocker.** This program spent a long stretch believing `GUI (E): Menu preset '<name>' not found!`
  was a stale `resourceDatabase.rdb`, and gated five screens plus a whole slice behind an operator
  Workbench pass on that basis. The operator ran it, and the evidence killed the theory: the rdb
  rewrote (8,791 bytes), `chimeraMenus.conf` registered, its `.meta` GUID matched the reference —
  and all five presets still failed. The actual cause is that vanilla's `ArmaReforger.gproj` carries
  an explicit list, and ours had **empty** `GameProjectConfig` blocks, so the config was a
  registered resource that `MenuManager` never read:
  ```
  GameProjectConfig PC {
   MenuManagerSettings MenuManagerSettings {
    MenuConfigs {
     "{C747AFB6B750CE9A}Configs/System/chimeraMenus.conf"   <- vanilla's, MUST be repeated
     "{7BD1A70000000703}Configs/System/chimeraMenus.conf"   <- ours
    }
   }
  }
  ```
  Two things measured the hard way while fixing it:
  - `MenuManagerSettings` is a **direct child of `GameProjectConfig`**, sibling to `DefaultSettings`.
    Nesting it inside `DefaultSettings` gives `INIT (E): Unknown keyword/data 'MenuManagerSettings'`.
  - **A mod's `MenuConfigs` REPLACES vanilla's list, it does not extend it.** Listing only ours took
    the error count from 5 to **59** — every vanilla menu broke. That 59 was the useful signal: it
    proved the config was finally being read. Listing both took it to **0**.
  `.conf` and `.layout` files still need a `.meta` sidecar to register at all — 10 action `.conf`
  files under `Configs/System/Actions/` still lack one (`resource not registered: Setting null
  GUID`), which is the spectator/admin KEYBINDS, a separate problem from the screens.
- **An EMPTY `Slot` block collapses a widget to nothing.** With no `HorizontalAlign`, a child keeps
  its **desired** size — and a `FrameWidgetClass`'s desired size is **ZERO**. `TBD_ListRow.layout`
  had `Slot ButtonWidgetSlot { }` wrapping a Frame, so every list row rendered as a ~10px sliver,
  and with no `Wrap 0` the text wrapped one character per line: the unreadable vertical column from
  the first live load-in. `HorizontalAlign 3` = **Stretch**, confirmed from
  `vanilla_reference/Scripts/Core/generated/UI/LayoutHorizontalAlign.c`, not inferred.
  `scripts/mod/verify-ui-layouts.sh` now gates this class.
- **The engine binary's string table is NOT a usable oracle for layout keywords — do not repeat this
  mistake, the command center made it.** A brief told an agent that `Anchor`/`SizeX`/`SizeY` were
  absent from the binary and therefore invalid. With controls, `ImageWidgetClass`, `TextWidgetClass`,
  `HorizontalAlign`, `Padding` and `HeightOverride` **all miss too**, and all are unquestionably
  valid. A hit may be evidence; **a miss is evidence of nothing**. Worse, the first probe returned a
  silent all-miss because `strings` is absent in the container and stderr was suppressed — a
  "no results" that looked like a finding. `Anchor` IS honoured: `PrimaryAction` uses
  `Anchor 1 1 1 1` with `OffsetLeft -288` and renders bottom-right on the operator's screen.
  **Shipped `.layout` usage is the real oracle** (CRF has 89, vanilla's generated UI enums are
  authoritative for values).
- **`GetGame().GetWorkspace()` is NON-NULL on a HEADLESS DEDICATED SERVER** (engine 1.7.0.54,
  measured T-181.28). It is therefore **not** a dedicated-server test, and shipped code used it as
  one: `TBD_LobbyStage.Start()` gates on `if (!GetGame().GetWorkspace()) return;`, so on a headless
  boot with ZERO players it proceeds and tries to open the lobby, producing
  `[TBD][ui] preset 60 did not open` — the intermittent `world-boot` failure. The same false claim
  is documented on `TBD_FrameworkManager.NotifyLocalStageUI()`, harmless there only because the very
  next line checks the player controller. **The reliable test is a null LOCAL PLAYER CONTROLLER.**
- **`world-boot.sh`'s settle window makes late errors nondeterministic.** The default 4 s usually
  ends before the lobby watcher fires; `TBD_WORLDBOOT_SETTLE=12` makes the flake above reproduce
  3/3 on a clean baseline. When attributing an intermittent gate failure, raise the settle and
  re-run the BASELINE before blaming a change.
- **What `world-boot.sh --mission=` does and does not exercise — the line is MISSION-START vs
  PLAYER-TRIGGERED, not "no runtime at all".** Both halves were measured, and the command center got
  this wrong in both directions before it was pinned:
  - **DOES run, with zero clients connected:** loader, validator, zone registry, objective registry,
    radio plan, **and the entire slot lineup** — `TBD_SpawnManager` materialises every slot body at
    MISSION START, not on join. T-181.41 measured 18 bodies dressed and **18 worn-audits executed**
    on `bridgehead-at-levie`. So kit equipping and the nakedness audit are among the few things
    proven END-TO-END by this gate rather than compile-only — and a false ERROR there breaks the
    gate for every future slice.
  - **Does NOT run:** `BuildForPlayer`, `Serialise`, `Parse`, any RPC, any briefing/lobby/admin/
    marker payload, any screen, or any stage past LOBBY. T-181.26 measured this (`grep -i briefing`
    over a full `--mission` log returns only `flow.briefingSeconds`).
  **Corollary worth designing around:** a service that self-checks AT BOOT is gated by this harness;
  one that self-checks lazily on first player use is not.
- **`string.Split` KEEPS empty tokens** (measured on engine 1.7.0.54 — previously recorded here as
  an unprovable runtime unknown). The briefing wire was therefore *accidentally* correct on this
  build and its empty-field defect was **latent, not live**. Do not read this as "delimited wire
  formats are safe": it is one measured behaviour of one engine build, and the bijective
  `FIELD_MARK` convention (`TBD_AdminData`, now `TBD_BriefingData`) does not depend on it.
  `TBD_LobbyData` still uses a lossy `~` sentinel and is now the odd one out.
- **`set` and `out` are RESERVED in Enfusion.** `TBD_RadioSet set` → *"Variable name 'set' is
  already used as type name"*; `array<X> out` → *"Expected name, not a keyword 'out'"*.
- **`Rpc` takes at most 8 parameters** — nine gives *"Too many parameters for 'Rpc' method"*.
- **`array<bool>` is not a safe RPC parameter type.** It compiles, but `array<int>` / `array<string>`
  are the ONLY array element types appearing in replicated methods across both oracles (12 sites);
  `array<bool>` appears in neither. Use `array<int>` 0/1.
- **BI's own docs transpose `GetMinFrequency` / `GetMaxFrequency`** — each describes the other.
  Order them by value, not by name.
- **`string.Length()` counts BYTES and `Substring` is BYTE-INDEXED.** Measured: `"…".Length()` is
  3, `"·".Length()` is 2, and `"café latte".Substring(0,4)` returns a **broken UTF-8 sequence**.
  Any truncation of authored prose must back off to a space (0x20 cannot appear inside a multi-byte
  sequence) or it will emit invalid UTF-8. Known remaining exposure: `TBD_MarkerService.CapLabel`.
- **`string.Trim()` is NOT in the mutate-in-place family** — it returns a real string. The landmine
  holds for `Replace`/`ToUpper`/`ToLower` (all three measured returning an int count), but not for
  `Trim`. Do not generalise the family by guessing which members belong.
- **CORRECTION to this program's own probe advice: `int n = s.Foo()` proves NOTHING.** Enfusion
  coerces **string → int implicitly** (but not int → string), so an int-returning method AND a
  string-returning one both compile under `int n = …`. The discriminating test is the other
  direction — **`string x = s.Foo();`**, which FAILS with `Incompatible parameter` for the
  int-returning ones. Earlier guidance here recommended the useless direction; it has been used in
  real probes and would have "confirmed" either answer.
- **NEVER `git stash` inside a slice worktree.** It runs despite the git-lfs hook error and deletes
  all 983 `packages/map-assets/**` pointer files. Recoverable with a filter-neutralised
  `git checkout -- packages/map-assets`, but it looks like catastrophic data loss when it happens.
- **`\uXXXX` escapes are not supported** — Enfusion drops the backslash silently.
- **`RegisterScriptHandler` compile-checks NOTHING about the callback** — not the event name (a
  string), not the arity, not the parameter types. Measured: `void H(int)` and
  `void H(string,string,string)` BOTH bind clean to the same 3-arg event, and the function returns
  `void` so there is no registration status to test. This does NOT match `CallLater`, whose arity
  IS checked — the intuition does not carry over, which is what makes it dangerous. The only oracle
  is a live test; for safestart's projectile sink that means one shot during SAFE_START, read back
  via `#tbd safestart status`.
- **`SCR_CharacterDamageManagerComponent.IsDamageHandlingEnabled()` exists** and is a general-purpose
  state query, not a set-then-confirm idiom — corroborated by two shipped vanilla sites that read it
  without ever having set it (`SCR_DamageDisabledTooltipDetail.c:20`, `SCR_HealthTooltipDetail.c:21`).
- **`cargo xtask ci schema-validate` is a MAIN-TREE target and dies inside a slice worktree.** LFS content is
  not smudged there, so `packages/map-assets/**` is ~133-byte pointers and `schema height-labels`
  fails with `PNG decode: Invalid PNG signature` while passing on main. TWO agents burned effort on
  this. Inside a worktree run `cargo run -p xtask -- schema validate` instead. Do NOT "fix" it by
  symlinking the real assets in — tried and reverted, because git then reports all 983 tracked files
  under that path as DELETED and every worktree is permanently dirty.
- **`$profile:` resolves to `<-profile-arg>/profile/`, NOT `<-profile-arg>/`.** Seeding a mission or
  config one level up loads **nothing, silently** — the loader just reports the file missing. Cost
  the wave-5 verifier two dead boots before it noticed.
- **`Data/*.json` is rdb-gated exactly like `.conf`/`.layout`.** `$TBD_Framework:Data/registry.json`
  does **not** resolve for a loose addon, so every slot fails with `kit resolve failed` and the
  mission is rejected. The `$profile:TBD_Registry.json` fallback is not a convenience — it is the
  only working path until Workbench rewrites the rdb. `world-boot.sh --mission=` seeds it for this
  reason. Only `.c` files are directory-scanned and rdb-independent.
- **`pgrep` through `distrobox-host-exec` can return a FALSE NEGATIVE.** A `ArmaReforgerServer`
  orphan was live for 2 h 38 m while `pgrep -a ArmaReforgerServer` through the bridge reported
  nothing; `ps -o pid,pgid -p <pid>` saw it immediately. Confirm a process is really gone with `ps`
  against the specific PID, not a name search — and kill by the PGID `ps` reports, not the one you
  assumed (mine differed, so the first kill silently hit nothing).
- **`JsonLoadContext` ALLOCATES a nested `ref <class>` field even when the JSON key is ABSENT — so
  a null check is NOT a presence test.** Measured 2026-07-25 on a live boot against
  `golden-missions/bridgehead-at-levie.json`: zone `z4` authors a polygon and **no** circle, yet
  `shape.circle` came back non-null with `x=0 z=0 r=0`; zone `z5` authors no `rules` key at all, yet
  `rules` came back non-null full of sentinels. **`if (shape.circle)` is always true and tells you
  nothing.** The only reliable presence tests are a **scalar sentinel** (`circle.r > 0`) or a
  **container count** (`polygon.Count() > 0`).
  This is not theoretical — it had already produced three latent bugs in shipped code, all of the
  same shape, all fixed:
  - `GetSpawnZoneForFaction` would have placed a faction at the map corner `(0,0)` for a
    polygon-only spawn zone;
  - `TBD_MissionValidator`'s warning about exactly that case was **unreachable**, so nothing caught it;
  - `TBD_BriefingData` rendered a polygon boundary as the literal `"0, 0 · r0"` instead of `"area"`.
  Any new struct added to `TBD_MissionLoader.c` needs the same treatment. Related dead guards that
  can never fire for this reason (harmless today, filed under T-181.30): `if (!mission.meta)` in the
  validator and `if (!doc.winConditions)` in `TBD_BriefingData`.
  - **`ref array<>` is NOT over-allocated — only `ref <class>` is. MEASURED, and this corrects an
    earlier entry here that told agents not to bother checking.** T-181.13.1 instrumented a boot
    dumping `loadoutNull/gearNull/gearRefs/cargoNull/cargoCount` per slot and saw **both polarities
    in one run**: `loadout.cargo` came back non-null on exactly the slots authoring a `cargo` key
    and null on every slot that did not, while `ref <class>` fields were allocated regardless.
    **It matters, and this doc previously said it did not.** A non-null `ref array<>` is genuine
    PROOF the key was authored, which is the only way to tell an authored-but-empty block from an
    absent one — `gear` presence is unobservable (absent `gear` and `gear: {}` both parse to ten
    empty strings with no sentinel), but `cargo` presence is knowable. That is what finally made
    `CheckSlotLoadout`'s "neither gear nor cargo" branch reachable.
    The old reasoning was that every container null-test in the tree is paired with `IsEmpty()` or
    falls through to a `foreach`, so the two cases behave identically. That is true of the sites
    that existed then, and it is the wrong thing to conclude from: it says the distinction was
    unused, not that it was unavailable.
- **In `mission.schema.json`, `required` does NOT mean non-empty.** Most string fields declare no
  `minLength`, so a key can be present and blank and still validate. `$defs/marker` requires all
  four of `x/z/icon/label`, yet `golden-missions/empty-warning-fields.json` ships a committed
  marker with `icon: ""` **and** `label: ""`. The command center briefed a wave-5 agent that "all
  four required, so a marker that exists is complete" — that is true of PRESENCE and false of
  CONTENT, and the agent was right to correct it. Consequences: never use a required string as a
  delimiter-safe wire field, and never treat empty as impossible. (The four `minLength: 1` fields
  behind T-181.26/.31 are the exception, not the rule.)
- **`modded class SCR_PlayerController` — how many are safe is UNSETTLED, and the count keeps
  growing.** Two earlier entries here flatly contradicted each other ("only ONE is safe" vs "two
  compile fine"); both wave-5 agents caught it. The measured facts, and only these:
  - **N blocks COMPILE fine and methods declared in one are callable from the others.** Verified at
    N=2, 3, and now **5** — `TBD_MissionBrowser.c`, `TBD_BriefingController.c`,
    `TBD_LobbyController.c`, `TBD_SpectatorHost.c`, `TBD_MarkerController.c`,
    `TBD_RadioController.c`. Re-checked at N=6: still no duplicate method name, still exactly one
    vanilla override across the whole set, still every RPC handler name unique.
  - **Runtime coexistence has NEVER been observed.** No gate on the fast lane can see it:
    `world-boot.sh` boots with zero players, and every one of these blocks only does anything when
    a client is connected. "Compiles" is not "works".
  - The "only ONE is safe" claim appears to have been an inference, not a measurement, and is
    recorded as such rather than deleted — if it turns out to be true, the cost is high and this
    note is where to look.
  - **Mitigation in force:** each new block is written to minimise blast radius — the two added in
    wave 5 override **no** vanilla method and add **no** `modded enum ChimeraMenuPreset` entry, so
    they contribute nothing to the menu-preset collision that is the substance of T-181.25.
  - **This is the first thing T-181.25 (operator, dedicated server) must settle.**
- **`string.Replace()` / `ToUpper()` / `ToLower()` mutate in place and return a COUNT**, not the new
  string. `s = s.Replace(a,b)` fails to compile; `int n = s.ToLower();` is what actually typechecks.
- **The headless server validates menu presets**: `GUI (E): Menu preset '<name>' not found!` is a
  free, ~20 s, no-Workbench check that a modded preset is wired up.
- `wb_reload` never recompiles; Workbench must restart per compile (the fast lane avoids this).
