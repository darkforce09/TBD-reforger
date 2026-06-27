# TBD Framework

Greenfield Enfusion game mode for the TBD Reforger platform. **TBD-owned code only.**

Mod GUID: `B2C3D4E5F6A78901` · Vanilla dependency: `58D0FB3206B6F859`

---

## Coalition / CRF — do not use in Workbench

| Folder | Role | Open in Workbench? |
|---|---|---|
| **`tbd-framework/`** (this mod) | Production TBD framework | **Yes** |
| **`Tbd_framework/`** | CRF reference (read patterns in Cursor only) | **No** — 60+ Coalition workshop deps |

See [`Tbd_framework/REFERENCE-ONLY.md`](../Tbd_framework/REFERENCE-ONLY.md).

---

## Features (current)

- Backend config from `$profile:TBD_BackendConfig.json`
- Mission loader: REST `GET /api/missions/{id}/compiled` → `$profile:missions/{id}.json` fallback
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
bash scripts/setup-workbench-linux.sh
```

1. Locate `~/ArmaReforger-Base/data/ArmaReforger.gproj` as base game
2. **+ Add Project → Add Existing** → `tbd-framework/addon.gproj`
3. Open **TBD_Framework** in the launcher
4. Use **enfusion-mcp** before editing any `.c` file

**New script file:** Workbench builds its script-file list at project load — a freshly added `.c` stays "Unknown class" until **Workbench cold restart** (not just `wb_reload`). Kill Workbench + re-run `tbd-dev-bootstrap.sh`.

**MCP verify spawn:**

```bash
bash scripts/tbd-spawn-verify.sh
```

---

## Dedicated server (Linux)

```bash
bash scripts/setup-server-profile.sh     # default profile: ../.local-test-profile/
bash scripts/run-dev-server.sh
```

Prereqs: Steam app **1890870** (Arma Reforger Server), website API on `:8080`.

Local unpublished mods use **`-server` + `-addons`**, not `-config` + `-addons`.

**Staging:** see [`docs/STAGING-SERVER.md`](../docs/STAGING-SERVER.md) — `bash scripts/deploy-staging.sh`.

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

Setup script writes these automatically; token from `GAME_SERVER_TOKEN` env or `website/.env`.

### Expected log lines

```
[TBD] Mission loaded from backend: Bridgehead at Levie
[TBD] Registry loaded
[TBD] SpawnManager: built slot spawn ... (×18 for msn_8f3a2c)
[TBD] Stage → LOBBY
[TBD] Roster loaded
[TBD] SpawnManager: assigned slot blufor:Alpha:SL:0
[TBD] SpawnManager: spawn requested
[TBD][Loadout] Loaded TBD_LoadoutTest.json (version 1, modpack …)
[TBD][Loadout] test spawn 0x… @ <6400, …, 6400>
[TBD][Loadout] primary equip OK {GUID}…MG_M60.et
[TBD][Loadout] uniform equip OK {GUID}…Jacket_US_BDU_rolledup.et
[TBD][Loadout] vest    equip OK {GUID}…Vest_PASGT.et
[TBD][Loadout] helmet  equip OK {GUID}…Helmet_PASGT_01_cover_w_goggles.et
[TBD][Loadout] equip pass complete
NETWORK : Starting RPL server, listening on address 0.0.0.0:2001
```

**Important:** `[TBD][Loadout]` on test NPC = Phase 1 dev harness. **Human player** loadout = **T-068.12**; pick slot in LOBBY = **T-068.13**; production roster sync = **T-114**.

---

## Registry

Shipped at `Data/registry.json` (vanilla POC aliases).  
Spec: [`shared/tbd-schema/spikes/registry-poc-0.4.md`](../../shared/tbd-schema/spikes/registry-poc-0.4.md) (historical spike).

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
