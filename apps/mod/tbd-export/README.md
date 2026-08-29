# TBD Framework

Greenfield Enfusion game mode for the TBD Reforger platform. **TBD-owned code only.**

Mod GUID: `B2C3D4E5F6A78901` · Vanilla dependency: `58D0FB3206B6F859`

---

## Coalition / CRF — do not use in Workbench

| Folder | Role | Open in Workbench? |
|---|---|---|
| **`tbd-framework/`** (this mod) | Production TBD framework | **Yes** |
| **`Tbd_framework/`** | CRF reference (read patterns in Cursor only) | **No** — 60+ Coalition workshop deps |

See `Tbd_framework/REFERENCE-ONLY.md` (gitignored reference copy — present only in local checkouts).

---

## Features (current)

- Backend config from `$profile:TBD_BackendConfig.json`
- Mission loader: REST `GET /api/v1/missions/{id}/compiled` (service-token / `X-Service-Token`; handler `get_compiled_mission` at `apps/website/api/src/app.rs` + `handlers/missions.rs`, T-092.2) → `$profile:missions/{id}.json` fallback on REST failure
- Registry alias resolution (`TBD_Registry.c`)
- **Per-slot spawn:** `TBD_SpawnManager` + modded `SCR_MenuSpawnLogic` from mission `slots[]` (schema 1.1) — **kit aliases** + round-robin/roster assign; **no in-game slot picker yet** (**T-068.13** production LOBBY UI, after **T-092.2**)
- **Player loadout on spawn:** **T-068.12** — per-slot compiled loadout → `EquipCloth`/`EquipWeapon` on **human player** (not test NPC)
- **Loadout equip test (T-068.5 / T-068.5.1):** `TBD_LoadoutEquipComponent` — `$profile:TBD_LoadoutTest.json`, **test NPC** @ 6400 only
- Roster loader (`TBD_RosterLoader.c`) — polls `GET /api/game/events/{id}/roster`
- Game stage enum + manager (`LOADING → … → DEBRIEF`)
- Radio bridge hook stubs (partner VOIP wires later)
- **`TBD_GameMode.et`** prefab — managers + `TBD_LoadoutEquipComponent` (dev loadout test)

---

## Dev scenario

| Resource | Path |
|---|---|
| Mission | `Missions/TBD_Dev_POC.conf` (`{69A85365FC09E2CA}`) |
| World | `worlds/TBD_Dev_POC.ent` — Eden subscene (`{853E92315D1D9EFE}worlds/Eden/Eden.ent`) |
| Layer | `worlds/TBD_Dev_POC_Layers/default.layer` — places `TBD_GameMode` at 6400,0,6400 |
| Game mode prefab | `Prefabs/Systems/TBD_GameMode.et` |

Golden mission `msn_8f3a2c` defines **18 slots** with exact spawn positions.

---

## Workbench setup

```bash
cargo xtask setup workbench
```

1. Locate `~/ArmaReforger-Base/data/ArmaReforger.gproj` as base game
2. **+ Add Project → Add Existing** → `tbd-framework/addon.gproj`
3. Open **TBD_Framework** in the launcher
4. Use **enfusion-mcp** before editing any `.c` file

**New script file:** Workbench builds its script-file list at project load — a freshly added `.c` stays "Unknown class" until **Workbench cold restart** (not just `wb_reload`). Kill Workbench + re-run `cargo xtask mod dev-bootstrap`.

**MCP verify spawn:**

```bash
cargo xtask mod spawn-verify
```

---

## Dedicated server (Linux)

```bash
cargo xtask setup server-profile     # default profile: apps/mod/.local-test-profile/
cargo xtask mod dev-server
```

Prereqs: Steam app **1890870** (Arma Reforger Server), website API on `:8080`.

Local unpublished mods use **`-server` + `-addons`**, not `-config` + `-addons`.

**Staging:** see [`docs/STAGING-SERVER.md`](../../../docs/mod/STAGING-SERVER.md) — `cargo xtask deploy staging`.

### Profile layout

Enfusion `$profile:` = `<profileDir>/profile/`:

```
profile/
  TBD_BackendConfig.json    # copy from Data/backend.example.json
  TBD_Registry.json         # optional override
  TBD_LoadoutTest.json      # copy from web loadout-export.json (T-068.4 download) for loadout equip test
  missions/
    msn_8f3a2c.json         # cached after successful REST fetch
```

**Workbench `$profile:`** resolves under the Proton prefix, e.g.  
`…/compatdata/1874910/pfx/drive_c/users/steamuser/Documents/My Games/ArmaReforgerWorkbench/profile/`  
(paste exact path in verify — differs from dedicated-server `.local-test-profile/`).

Setup script writes these automatically; token from `GAME_SERVER_TOKEN` env or `apps/website/.env`.

### Expected log lines

Verified against a real boot (T-612, 2026-08-01). Everything after each tag/`key=` prefix is
expected to vary — pin the prefix, never the sentence (`scripts/mod/remote-log-grep.sh:34`):

```
[TBD][Mission] loaded id=msn_8f3a2c name='Bridgehead at Levie' slots=18 source=backend
[TBD] Registry loaded (21 aliases).
[TBD][Slots] Slot-1 blufor:Alpha:SL:0 (blufor:Alpha:SL:0) kit kit:rifleman_m16 at <…>   ← ×18 for msn_8f3a2c
[TBD][Loadout][Slot] slot=… primary equip OK {GUID}…Rifle_M16A2.et                      ← per authored gear item
[TBD][Loadout][Slot] slot=… loadout pass complete gear=…/… cargo=…/…                    ← per dressed slot
[TBD][Slots] materialized 18/18 bodies — … with a JSON loadout, … kit-only, 0 failed
[TBD][Slots] loadout settle complete — … application(s), 0 unplayable, …
[TBD][Stage] LOADING -> LOBBY
[TBD] Stage → LOBBY
NETWORK : Starting RPL server, listening on address 0.0.0.0:2001
[TBD] Roster loaded (… assignments).                                                    ← when a roster is configured
[TBD] SpawnManager: assigned slot blufor:Alpha:SL:0 to player 1 at (…)                  ← once a client joins
[TBD] SpawnManager: bound player 1 to slot blufor:Alpha:SL:0 body (kit …)
```

**Gone since June (T-612 — do not grep for these):** `[TBD] Mission loaded from backend:`,
`built slot spawn`, `spawn requested`, `[TBD][Loadout][Player]`. The only `Mission loaded`
still printed is the **failure** line `[TBD] Mission loaded but invalid — staying in LOADING.`
— a check satisfied by that string is passing on the error case.

**Important:** `[TBD][Loadout][TestNPC]` = the Phase 1 dev harness (`$profile:TBD_LoadoutTest.json`).
**`[TBD][Loadout][Slot]`** = the production slot-body path human players receive (**T-068.12**);
pick slot in LOBBY = **T-068.13**; production roster sync = **T-114**.

---

## Registry

Shipped at `Data/registry.json` (vanilla POC aliases).  
Spec: [`shared/tbd-schema/spikes/registry-poc-0.4.md`](../../../packages/tbd-schema/spikes/registry-poc-0.4.md) (historical spike).

Replace with TBD-Content export in Phase 1+.

---

## Scripts layout

```
Scripts/Game/TBD/
  Backend/     TBD_BackendConfig.c, TBD_MissionLoader.c
  Gamemode/    TBD_FrameworkManager.c, TBD_GameStage.c, TBD_SpawnManager.c,
               TBD_SCR_MenuSpawnLogic.c, TBD_RosterLoader.c, TBD_LoadoutEquipComponent.c
  Registry/    TBD_Registry.c, TBD_RegistryPocComponent.c (optional POC)
  Radio/       TBD_RadioBridgeStub.c
```
