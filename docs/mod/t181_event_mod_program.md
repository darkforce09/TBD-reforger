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
make mod-compile             # Enfusion compile gate — ~1.3s, native, no Workbench
make mod-compile-selftest    # prove the gate still catches a broken .c
make verify-capability       # fails if any CRF capability has no TBD verdict
make enf-index               # rebuild the CRF symbol index

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
| T-181.1 | `scripts/mod/compile.sh` + `make mod-compile` | 0 clean / 1 + `file:line` broken |
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
| `make verify-no-crf-leak` | no Arma-Public-License code leaked into prod |
| `make verify-capability` | no capability silently forgotten |
| `make verify-oracle` | no invented `file:line` in docs |

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

**Settles** — T-181.25: whether four separate `modded enum ChimeraMenuPreset` blocks produce
distinct values (unprovable from the compile lane), and whether three `modded class
SCR_PlayerController` blocks coexist at runtime.

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
- **`string.Split`'s empty-token behaviour is a RUNTIME property** — unprovable by compile probe, and
  absent from every oracle. Every wire format in the mod must therefore avoid ever emitting an empty
  field (use a sentinel), or be proven once on a live run.
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
    `TBD_LobbyController.c`, `TBD_SpectatorHost.c`, `TBD_MarkerController.c`.
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
