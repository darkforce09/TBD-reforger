# FNF (Friday Night Fight) mission framework — v3.6.9

**Subject:** the *pre-revamp* era of the FNF Arma 3 PvP mission framework.
**Source:** `/run/media/system/Disk_2/tbd-framework-analysis/fnf/FNF-v3.6.9/` — git worktree pinned to tag
`v3.6.9`, commit `285e44412409dc4ec6a8b659e31b4875166bfdbd`, dated **2023-09-12**, subject `Version Bump`.
Read-only; nothing under that path was modified.

**Path convention.** Every citation below is relative to that worktree root. `FNF_MissionTemplate.VR/…`
is the mission template; `client_mod/…`, `server_mods/…`, `tools/…` are the mod/tooling trees.

**Scope note.** This document describes v3 on its own terms. It makes no claims about v4 and draws no
comparison to it.

---

## Source inventory

Files and directories actually read, and what each yielded. Three sub-agents contributed the rows
marked *(mods)*, *(modes)*, *(tooling)*; everything else was read directly.

| Path | What it gave |
|---|---|
| `README.md` | Feature list, mode taxonomy, links to external Rules and Mission-Making Guide docs |
| `LICENSE` | BSD 3-Clause, "Copyright (c) 2022, Friday Night Fight" |
| `.editorconfig`, `.gitattributes` | 2-space / CRLF / final-newline convention; `* text=auto` |
| `.github/ISSUE_TEMPLATE/{break_fix,feature_request,testing_required}.md` *(tooling)* | The whole of `.github` — three issue templates, **no workflows** |
| `FNF_MissionTemplate.VR/config.sqf` | **The** mission-maker config file — every knob, verbatim |
| `FNF_MissionTemplate.VR/configGuide.txt` | Reference list of modes, 63 uniform+gear pairs, MAT options, SHQ aux roles |
| `FNF_MissionTemplate.VR/description.ext`, `description_SA.ext` | Include chain; `author`/`onLoadName`/`onLoadMission` placeholders |
| `FNF_MissionTemplate.VR/version.txt` | `"3.6.9"` |
| `FNF_MissionTemplate.VR/mission.sqm` (617 KB, plain text) | 12 Eden layers, 267 playable units, 54 groups, marker/trigger/object names, 28 in-editor Comment objects, Eden custom attributes, `ScenarioData`, `Intel` |
| `FNF_MissionTemplate.VR/mission_SA_Modern.sqm`, `mission_SA_VN.sqm` | **Binarized** (`\0raP` header) — structure unreadable; only extractable strings reported |
| `FNF_MissionTemplate.VR/cba_settings.sqf` (22 KB) | ~520 forced CBA/ACE/TFAR/EMR/GRAD settings |
| `FNF_MissionTemplate.VR/description/configDefs.hpp` | Mode `#define`s, MAT macros, CSW macros |
| `FNF_MissionTemplate.VR/description/definitions.hpp`, `definitions_SA.hpp` | Respawn/lobby/`Header` settings, `onPauseScript[]` |
| `FNF_MissionTemplate.VR/description/cfgParams.hpp` | The 4 lobby params |
| `FNF_MissionTemplate.VR/description/cfgFunctions.hpp` | The entire function registry (≈180 functions in 14 namespaces) |
| `FNF_MissionTemplate.VR/description/cfgFNFORBAT.hpp` (974 lines) | `CfgFNFORBAT` tree — 3 sides × HQ + 4 squads + G1/G2 + H1/H2 |
| `FNF_MissionTemplate.VR/description/cfgFNFLoadouts.hpp` | Include manifest: 93 uniform sets, 63 gear sets |
| `FNF_MissionTemplate.VR/description/KITS/GEAR/common.hpp`, `examplebase.hpp`, `undef.hpp` | Loadout macro vocabulary, optics tiers, `common`/`optics` config classes |
| `FNF_MissionTemplate.VR/description/KITS/UNIFORMS/RHS_UNI_NATO_US_ARMY_2020.hpp` | Uniform-set shape (BASE + 25 role subclasses) |
| `FNF_MissionTemplate.VR/description/cfgEventHandlers.hpp`, `cfgFNFVehicleLoadouts.hpp` | CBA XEH wiring; 18 vehicle-loadout class hooks |
| `FNF_MissionTemplate.VR/description/cfgNotifications.hpp`, `cfgSounds.hpp` | 3 notifications, 1 sound |
| `FNF_MissionTemplate.VR/description/cfgVNArtillery.hpp` | Prairie Fire artillery availability config |
| `FNF_MissionTemplate.VR/description/sekrit.sqf` | **Non-maker-facing** timing constants + in-game info-panel text + credits |
| `FNF_MissionTemplate.VR/description/RulesAndPolicies.txt`, `credits.txt` | External-rules pointer; third-party component credits |
| `FNF_MissionTemplate.VR/description/changelog.txt` *(tooling)* | 272 lines, v3.1.0 → 3.6.8; churn analysis, breaking changes |
| `FNF_MissionTemplate.VR/description/vehicleLoadouts/*` | `fn_shouldModify`, `compileHtml.ps1`, `loadoutData.json` (48 KB), 18 per-vehicle fns |
| `FNF_MissionTemplate.VR/mode_config/*.sqf` (all 9) | The complete per-mode maker knob surface |
| `FNF_MissionTemplate.VR/modes/**` *(modes)* | Win conditions, editor-object literals, ATK/DEF vs NEUTRAL in code |
| `FNF_MissionTemplate.VR/server/init/*.sqf` (all 15) | Server boot, safe zones, safety, fortify, keying, auto-marking, uniforms broadcast |
| `FNF_MissionTemplate.VR/server/end/*.sqf` *(modes)* | Time limit, elimination, overtime, game end |
| `FNF_MissionTemplate.VR/server/damage/fn_customDamage.sqf` | Destroy-objective damage model |
| `FNF_MissionTemplate.VR/server/webhook/*` *(tooling)* | Discord round-start/round-end payloads |
| `FNF_MissionTemplate.VR/client/init/*.sqf` | Client boot order, role table, staggered load, teleport actions |
| `FNF_MissionTemplate.VR/client/loadout/**` | `fn_applyLoadout`, 17 `procedure/` fns, gear selector, MAT/SHQAUX tools |
| `FNF_MissionTemplate.VR/client/briefing/**` | Diary construction, ORBAT generation, group-ID/radio table, briefing tables, asset diary, recon |
| `FNF_MissionTemplate.VR/client/restrictions/*.sqf` (all 15) | Every restriction the framework imposes |
| `FNF_MissionTemplate.VR/client/misc/**` | Safe zones, fortify client, lobby text generator, objective preview, admin menu + 11 admin fns, staff contact |
| `FNF_MissionTemplate.VR/client/safety/fn_init.sqf` | Safe-start client behaviour |
| `FNF_MissionTemplate.VR/client/spectator/fn_init.sqf` | Spectator entry, objective icons |
| `FNF_MissionTemplate.VR/client/ui/**` | Mission Info panel tree, map polygon shading, notifications |
| `FNF_MissionTemplate.VR/missionSpecials/fn_config.sqf` | Ambient airdrop / artillery maker config |
| `client_mod/**` (16 addons) *(mods)* | What ships as a client mod; `fnf_eden` attribute definitions; balance overrides |
| `server_mods/{common_mod,early_mod,late_mod}/**` *(mods)* | Staff list, Pythia Python webhooks, per-server CBA settings split |
| `tools/**` *(tooling)* | Kit export/screenshot pipeline, PowerShell post-processing |

**Unreadable / binary, stated rather than inferred:** `mission_SA_Modern.sqm` and `mission_SA_VN.sqm`
are rapified (binarized) `.sqm`; `description/images/*.paa`, `description/sound/bomb_alarm.ogg`,
`client_mod/**/*.paa` (759 files) and `*.png` (18) are binary;
`server_mods/common_mod/python_files/__pycache__/config.cpython-310.pyc` is compiled CPython bytecode
whose *source is absent from the repo*. Nothing is claimed about the contents of any of these beyond
extracted plain strings, which are flagged as such.

---

## 1. Identity

- **Name / version.** "Friday Night Fight Mission Framework"; `FNF_MissionTemplate.VR/version.txt` contains
  `"3.6.9"`, loaded at runtime by `FNF_MissionTemplate.VR/description/sekrit.sqf:8-12` into
  `fnf_templateVersion` and shown in the in-game info panel.
  **Note:** `FNF_MissionTemplate.VR/description/changelog.txt:1` — the newest changelog entry is **3.6.8**;
  there is no 3.6.9 entry.
- **Date.** Tag commit dated 2023-09-12.
- **Licence.** BSD 3-Clause, `LICENSE:1-4`, "Copyright (c) 2022, Friday Night Fight".
- **Game.** Arma 3. Confirmed by `mission.sqm:1` `version=54;` and the Arma-3 `addons[]` block
  (`mission.sqm:29-49`).
- **Dependency surface** (read from forced settings and config references, not from a manifest — the repo
  contains no `mod.cpp`/`meta.cpp`):
  CBA (`cba_settings.sqf` throughout, `CBA_fnc_*` everywhere), ACE3 + ACEX
  (`cba_settings.sqf:2-307`, `CfgDebriefingSections >> acex_killTracker` in `description/definitions.hpp:31-36`),
  TFAR (`cba_settings.sqf:403-449`), RHS USF/AFRF/GREF/SAF (`mission.sqm:29-49`, all `KITS/GEAR/RHS_*`),
  3CB (`description/cfgFNFVehicleLoadouts.hpp:76-113` `UK3CB_*`), S.O.G. Prairie Fire
  (optional — `configGuide.txt:126` "Era: Vietnam (requires Prairie Fire)"),
  Enhanced Movement Rework (`cba_settings.sqf` `emr_main_*`), GRAD Trenches (`grad_trenches_functions_*`),
  DUI (`diwako_dui_enable_compass_dir`), **Pythia** (`server/init/fn_serverInit.sqf:323` `py3_fnc_callExtension`),
  **OCAP2** (`server/end/fn_gameEnd.sqf:52-54` `isClass (configFile >> "CfgPatches" >> "OCAP")`),
  and `CAU_DiscordEmbedBuilder` for the teamkill webhook (`server/init/fn_serverInit.sqf:186`).
  README badges point at Steam collections 1551644814 (required) and 1551648858 (optional clientside).
- **Repo layout** (top level): `FNF_MissionTemplate.VR/` (421 files), `client_mod/` (803 files, 16 addon
  folders), `server_mods/` (12 files, 3 mods), `tools/` (6 files), `README.md`, `LICENSE`,
  `.editorconfig`, `.gitattributes`, `.github/ISSUE_TEMPLATE/` (3 files).
- **What ships as mod vs mission** *(mods agent)*:
  - **Client mod** — 16 addon source folders under `client_mod/`. There is **no `$PBOPREFIX$`, no
    `mod.cpp`, no `meta.cpp`, and no pre-built `.pbo` anywhere in the repo**; each folder is packed with
    its folder name as prefix, confirmed by internal path references such as
    `'\fnf_logos\data\fnflogo.paa'` (`client_mod/fnf_logos/config.cpp:27`) and
    `"fnf_patches\staffpatch.paa"` (`client_mod/fnf_patches/cfgUnitInsignia.hpp:7`).
    Contents: `fnf_ammo`, `fnf_armor`, `fnf_backpacks`, `fnf_eden`, `fnf_frags`, `fnf_logos`,
    `fnf_magazines`, `fnf_media`, `fnf_patches`, `fnf_rpk`, `fnf_setup`, `fnf_smoke`, `fnf_vehicles`,
    `fnf_vn`, `fnf_weapons`, `fnf_ww2`. Only `fnf_setup` contains SQF; only `fnf_eden` contains editor
    logic; the rest are config-only rebalances plus three asset carriers.
  - **Server mods** — `server_mods/common_mod/` (staff list + Pythia Python webhook package),
    `server_mods/early_mod/` and `server_mods/late_mod/` (per-server forced CBA settings). Early/late is
    **not load order** — they are two separate FNF game sessions on two servers, evidenced by the two
    main-menu connect buttons `FnfServerEarly` / `FnfServerLate` at
    `client_mod/fnf_logos/config.cpp:82-114`.
  - **Mission** — `FNF_MissionTemplate.VR/`, shipped as a source folder that the maker copies. All game
    logic (modes, loadout application, briefing, zones, safety, admin tools) lives here, not in the mods.

---

## 2. Mission file layout

A mission *is* a copy of `FNF_MissionTemplate.VR/` renamed to `<MissionName>.<MapClass>`. Root files:

| File | Hand-authored? | Purpose |
|---|---|---|
| `config.sqf` | **Yes — the primary maker file** | 24 scenario knobs (§10) |
| `configGuide.txt` | No (reference) | Valid values for uniforms/gear/MAT/CSW |
| `description.ext` | Yes (3 lines) | `author`, `onLoadName`, `onLoadMission`, then 13 `#include`s |
| `description_SA.ext` | Alternative | Sustained-Assault variant of the above |
| `mission.sqm` | **Yes — in Eden** | 617 KB, plain text, `binarizationWanted=0` (`mission.sqm:27`) |
| `mission_SA_Modern.sqm` / `mission_SA_VN.sqm` | Alternative | **Binarized**; the maker renames one over `mission.sqm` for SA |
| `cba_settings.sqf` | No (do not touch) | ~520 forced settings |
| `version.txt` | No | `"3.6.9"` |
| `.editorconfig` | No | Editor hygiene |

`description.ext:5-17` is the whole include chain:

```
description\definitions.hpp        description\cfgParams.hpp
description\cfgEventHandlers.hpp   description\cfgNotifications.hpp
description\cfgSounds.hpp          description\cfgFunctions.hpp
description\cfgFNFORBAT.hpp        description\cfgFNFLoadouts.hpp
description\cfgVNArtillery.hpp     client\ui\defines.hpp
client\ui\RscTitlesDisplay.hpp     client\ui\PauseMenuDisplays.hpp
client\ui\InfoPanel.hpp
```

Subtrees: `client/` (11 dirs, ~120 SQF + 4 HPP), `server/` (5 dirs, 25 SQF), `modes/` (9 dirs, 16 SQF),
`mode_config/` (9 SQF), `missionSpecials/` (6 SQF), `description/` (config + `KITS/` with 93 UNIFORMS and
63 GEAR HPPs + `vehicleLoadouts/`).

**Nothing is generated at build time.** There is no build step, no packer, no codegen — every file in the
mission is either hand-written, hand-edited in Eden, or shipped as-is. The only "generated" artifacts in
the whole project are the kit preview `.paa` images, produced by an out-of-band screenshot pipeline (§11)
and committed into the *client mod*, not the mission.

**Mission `.pbo` vs server mod vs client mod:**
- Anything a **player** must have installed to see correct textures/UI/balance → `client_mod`.
- Anything **secret or host-specific** (staff SteamIDs, Discord webhook URLs, per-server ACE stamina
  values) → `server_mods`.
- Everything the **mission maker** authors → the mission folder.
  Note the coupling runs *backwards* too: `client_mod/fnf_setup/XEH_postInit.sqf:102,127` calls
  mission-defined functions (`fnf_ui_fnc_missionInfoPanel`, `fnf_admin_fnc_adminUI`) and
  `client_mod/fnf_vehicles/config.cpp:32` reads the mission variable `fnf_safetyEnabled` — the client mod
  is not self-contained.

---

## 3. Authoring workflow

*The critical section. Ordered, concrete, as the source dictates.*

### Step 0 — obtain the template
Copy `FNF_MissionTemplate.VR/` into the Arma 3 missions folder and rename to `<Name>.<Map>`. There is no
scaffolding tool, no generator, no CLI. The README states the whole contract in two lines
(`README.md:49-51`): *"All configuration changes should be made in config.sqf"* / *"Some options
available for use are present in configGuide.txt"*.

### Step 1 — choose Standard or Sustained Assault
`config.sqf:19-35` gives two literal recipes. **Standard (1-life FNF):**
```
1. rename "mission_normal.sqm" to "mission.sqm"
2. rename "description_normal.ext" to "description.ext"
3. below, set fnf_gameMode to any of the valid values from configGuide.txt
4. delete "description_SA.ext", "mission_SA_Modern.sqm", and "mission_SA_VN.sqm" (for filesize purposes)
```
**Those files do not exist at v3.6.9.** `changelog.txt:96` (v3.3.1) records *"Renamed normal
description.ext and mission.sqm files to create useable defaults."* — so in practice Standard = do
nothing, and steps 1–2 are stale instructions still shipping in the maker's primary config file.
Sustained Assault (`config.sqf:27-35`) **"requires advance permission from Missions Team Lead"**, renames
`mission_SA_VN.sqm` or `mission_SA_Modern.sqm` over `mission.sqm` and `description_SA.ext` over
`description.ext`, and sets `fnf_gameMode = sustainedAssault`.

### Step 2 — fill the three lines of `description.ext`
`description.ext:1-3` ships as `author = "YOUR_NAME"; onLoadName = "MISSION_NAME";
onLoadMission = "SHORT_DESCRIPTION";`.

### Step 3 — open `mission.sqm` in the Eden 3D editor
The template arrives pre-populated with **12 named layers** (`mission.sqm`, `dataType="Layer"` at lines
299, 414, 2540, 2682, 2737, 3726, 3780, 3941, 4111, 11530, 11579, 18965):

```
FNF Gamemode: Destroy        FNF Gamemode: ScavHunt
FNF Gamemode: Rush, Uplink, Connection
FNF Gamemode: CTF            FNF Gamemode: NSector
FNF Gamemode: ADSector       FNF Gamemode: Assassin
FNF System Objects   └─ ZoneBoundary
FNF Units: BLUFOR    FNF Units: INDFOR    FNF Units: OPFOR
```

The layer names are a **hard runtime contract**: `server/init/fn_setupGame.sqf:92-154` deletes every
object and marker in the layers belonging to modes you did not pick, e.g.
`_test = (getMissionLayerEntities "FNF Gamemode: Destroy");` (`:93`). This is why `config.sqf:43-44` says
*"DO NOT DELETE ANY OF THE OTHER TEMPLATE OBJECTIVE OBJECTS / they will be deleted automatically if not
in use"*.

The template also carries **28 Eden `Comment` objects** — in-editor documentation placed next to the thing
it describes. Examples, quoted from `mission.sqm`:
- `:405-406` "DESTROY … **Place the markers so they contain the position of the destroy objectives, but
  with some offset so they're not directly atop them. Tasks and briefing tables will use this offset
  position."
- `:4133` "BLUFOR, delete if not using"
- `:4171` (BLUFOR Briefing Table) "Place this in a flat area near the inner radius of the safe zone /
  Squad tables will be placed around it using this one as a reference for position"
- `:26278` "Delete this group if your mission doesn't need a MAT team."
- `:26300` "Delete these crewman and pilots if not needed"
- `:4093` (Custom Boundarys) — a 7-paragraph tutorial on the polygon-marker naming scheme, ending
  *"To activate a custom boundary simply delete the standard boundary/marker"*.

### Step 4 — place the per-side furniture
Per side, the maker moves (never renames) these exact objects, all present in the `FNF Units: <SIDE>`
layers:
- `west_safeZone_marker_1` — an **ELLIPSE area marker**, `type="mil_start"`, `colorName="ColorWEST"`,
  `a=100; b=100` (`mission.sqm:4120-4126`). Same for `east_`, `guer_`.
- `west_safeZone_flag_1` — a `FlagPole_F` (`mission.sqm:4153-4156`). This is the respawn/teleport anchor.
- `fnf_briefingTable_west` — a `Land_PortableDesk_01_black_F` (`mission.sqm:4213-4216`).
- Optional polygon safe zone: `fnf_custom_safeZone_west_1_marker_1..4` (shipped, 4 markers).

Warning path: if `<side>_safeZone_flag_1` is missing, `server/init/fn_safeZoneTeleportInit_STD.sqf:16-18`
emits, **during 3DEN preview only**, `"[FNF] (safeZoneTeleport) [PreviewOnly] %1_safeZone_flag_1 not
present. Safezone teleport disabled for %2"`.

### Step 5 — set the play area
Either move/resize the `zoneTrigger` `EmptyDetector` (`mission.sqm:3961-3968`), or delete it and lay out
`fnf_custom_zoneBoundary_1_marker_1..N` (`mission.sqm:3950-4088`, 12 shipped `mil_marker` icons).
`client/restrictions/fn_zoneBoundary.sqf:7` branches on `if (!isNil "zoneTrigger")`; the polygon branch
(`:36-80`) scans `allMapMarkers` for `^fnf_custom_zoneBoundary_1_marker_\d+$` and sorts numerically.

### Step 6 — trim the ORBAT
Delete whole sides ("BLUFOR, delete if not using"), the Echo MAT group, or the Golf/Hotel crew groups.
The framework tolerates missing groups: `client/briefing/fn_setGroupIDs.sqf:174-175` does
`_grp = missionNamespace getVariable [_identifier,grpNull]; if (!isNull _grp) then {…}`.

### Step 7 — place vehicles, compositions and props
Any object with a bounding sphere > 1.5 m that is not in a safe zone is **automatically drawn on the map**
as a black rectangle matching its footprint (`server/init/fn_markCustomObjs.sqf:34,49-75`). To opt an
object out, tick the Eden attribute **FNF Properties → "Exclude from Map Auto-Mark"**.

Three Eden attributes are added by `client_mod/fnf_eden/config.cpp` under a collapsed category
`fnf_properties` / `displayName = "FNF Properties"` *(mods agent)*:

| Eden class | `property` written into `.sqm` | Label | Shown on | Default |
|---|---|---|---|---|
| `fnf_autoMarkExclude` (`:24`) | `FNF_MarkingExclude` (`:26`) | Exclude from Map Auto-Mark | props only (`1 - objectControllable - objectVehicle`) | false |
| `fnf_clearInventory` (`:35`) | `FNF_InventoryAutoClear` (`:37`) | Clear Inventory | vehicles | true |
| `fnf_vehicleLoadouts_useDefault` (`:46`) | `FNF_vehicleLoadouts_useDefault` (`:48`) | Use Default Loadout | vehicles | true |

The class name and the `property` name **differ**; the `.sqm` stores `property=` while the runtime
`setVariable` uses the class name — verifiable side by side at `mission.sqm:451-452`
(`property="FNF_MarkingExclude"; expression="_this setVariable ['fnf_autoMarkExclude',_value];"`).
Consumers: `server/init/fn_markCustomObjs.sqf:14`, `server/init/fn_serverInit.sqf:141`,
`description/vehicleLoadouts/fn_shouldModify.sqf:3`.

Note that **all vehicle cargo is wiped by default** (`server/init/fn_serverInit.sqf:134-147`) unless
"Clear Inventory" is unticked, and 18 vehicle classes get a curated turret loadout
(`description/cfgFNFVehicleLoadouts.hpp`) unless "Use Default Loadout" is unticked.

### Step 8 — pick the game mode and configure it
`config.sqf:40` `fnf_gameMode = destroy;` — a **bare identifier**, resolved by
`description/configDefs.hpp:1-13`. Then edit the matching `mode_config/<mode>.sqf` and move that mode's
objects (§7). `config.sqf:46-47` sets `fnf_defendingSide` / `fnf_attackingSide`; both must be `sideEmpty`
for the three neutral modes.

### Step 9 — set uniforms, gear, MAT and the scenario knobs
From `configGuide.txt`, pick a `RHS_UNI_*`/`VN_UNI_*` and a matching `RHS_GEAR_*`/`VN_GEAR_*` per side
(`config.sqf:118-128`), a MAT launcher per side (`config.sqf:133-142`), then the remaining knobs (§10).

### Step 10 — write the briefing
Four HTML-capable strings at the very top of `config.sqf:9-15`: `fnf_briefingBackground`,
`fnf_briefingWorldInfo`, `fnf_briefingNotes`, `fnf_briefingRules`. `config.sqf:6` — *"0 or more of the
below can be included. At minimum, it's suggested to populate the 'briefingNotes' item."*

### Step 11 — preview in 3DEN and read the advisories
The framework's only validation is runtime and preview-gated:
- `server/init/fn_setupGame.sqf:12-31` — if the mode/side pairing is wrong, a red on-screen notification
  plus `BIS_fnc_error` plus `systemChat`: *"…isn't set, but this is an attack/defend gamemode…"* and
  *"The framework may not work properly!"*.
- `server/init/fn_safeZoneTeleportInit_STD.sqf:4-6` — *"During preview, you will see all safezone markers.
  On Dedicated, players will only see those matching their side."*
- `missionSpecials/fn_config.sqf:132-134` — lists any active ambient specials in preview.
- `description/cfgEventHandlers.hpp:36` — confirms air pylon loadouts were captured.

### Step 12 — generate the lobby text
In-game, **Escape → "Generate Lobby Text"** (button registered via `description/definitions.hpp:19`
`onPauseScript[]` → `fnf_fnc_lobbyTextGenButton`; host-only per `fn_lobbyTextGenButton.sqf:8`).
`client/misc/fn_lobbyTextGenerator.sqf` walks every unlocked vehicle inside each side's safe zone, sorts
and counts them, appends transports and the MAT summary, and `copyToClipboard`s a one-liner (`:189-192`):

`(DEST) // ATK: BLUFOR x% adv - DEF: OPFOR // BLU: 2xM1151A1, 1xM1117 + 3 transports, MAT:JAVELIN[1] // OPF: …`

The `%` is left as a literal for the maker to fill — `fn_lobbyTextGenButton.sqf:24` hints *"Don't forget
to change the attacker's advantage."* The `Intel` block of the shipped `mission.sqm:258` says so too:
`overviewText="Use the lobby text generator to fill this out (Go in game, hit Escape, and click the
generator button on the left side. Make sure to change the attacking advantage percentage)"`.

### Step 13 — trim and submit
Delete the unused `.sqm`/`.ext` variants "for filesize purposes" (`config.sqf:24,34`), then submit to the
FNF Missions team. Vetting is a **human role**, not a pipeline — `changelog.txt:230` records
*"additional notifications for vetters if mission has a bad configuration"*, and
`.github/ISSUE_TEMPLATE/testing_required.md:6` carries labels `'dedi testing, local testing'`.

**What the maker never touches:** `cba_settings.sqf`, `description/sekrit.sqf` (round length, safe-start
length), `description/KITS/**`, `description/cfgFNFORBAT.hpp`, and everything under `client/`, `server/`,
`modes/`.

---

## 4. Slotting / ORBAT model

### Pre-baked, not generated
Every slot is an Eden unit that already exists in `mission.sqm`. The maker **deletes** what is not needed;
nothing is created. Counts read directly from `mission.sqm`:

- **267 `isPlayable=1` units** total.
- **6** of those are `ace_spectator_virtual` Logic entities in the `FNF System Objects` layer
  (`mission.sqm:3793-3859`), described `"Spectator"`.
- **261 combat slots = 3 sides × 87**, laid out identically per side in the
  `FNF Units: {BLUFOR,OPFOR,INDFOR}` layers — **18 groups each**.

### The per-side structure (BLUFOR shown; OPFOR/INDFOR are byte-identical apart from side)

| Group (Eden init var) | Slots | Roles, verbatim from `description=` |
|---|---|---|
| `Blue_CC` (Company HQ) | 4 | Company Commander, Executive Officer, Company Sergeant, Medic |
| `Blue_A` (Alpha PLT) | 4 | Platoon Leader, Platoon Sergeant, Marksman, Medic |
| `Blue_A1` | 7 | Squad Leader, Automatic Rifleman, Asst. Automatic Rifleman, Rifleman (LAT), Grenadier, Combat Engineer, Ammo Bearer |
| `Blue_A2` | 7 | same |
| `Blue_B` / `Blue_B1` / `Blue_B2` | 4 / 7 / 7 | same shape as Alpha |
| `Blue_C` / `Blue_C1` / `Blue_C2` | 4 / 7 / 7 | same shape as Alpha |
| `Blue_D` (Delta PLT) | 3 | Platoon Leader, Platoon Sergeant, Medic (**no Marksman**) |
| `Blue_D1` / `Blue_D2` | 5 / 5 | Squad Leader, Machine Gunner ×2, Asst. Machine Gunner ×2 |
| `Blue_E` (Echo) | 4 | Section Leader, Missile Specialist, Asst. Missile Specialist ×2 |
| `Blue_G1` / `Blue_G2` | 3 / 3 | Vehicle Commander, Vehicle Gunner, Vehicle Driver |
| `Blue_H1` / `Blue_H2` | 3 / 3 | Pilot, Co-Pilot/Gunner, Gunner |

So the shape is: **one Company HQ, four platoon-sized squads (Alpha/Bravo/Charlie assault-and-support,
Delta machine-gun), one detached AT section (Echo), two vehicle crews (Golf), two air crews (Hotel).**

### Naming and callsign conventions
1. **Slot label = `Role@Callsign`.** `mission.sqm` `description=` values are literally
   `"Squad Leader@Alpha 1"`, `"Asst. Missile Specialist@Echo"`. The `@` is parsed at runtime:
   `server/init/fn_serverInit.sqf:197-199` —
   `if ((roleDescription _killer) find '@' > -1) then { _killerOrgGroup = (roleDescription _killer) splitString '@' select 1; };`
   The right half becomes the org label in the Discord teamkill embed. `client/loadout/tools/fn_handleSHQAUX.sqf:31-34`
   also reads the callsign back out of `roleDescription` to pick the right config variable.
2. **Group identity is an init-field global**, not a group name: `init="Blue_A1 = this;"` on the group
   (all 54 group inits enumerated by grep: `Blue_*`, `Red_*`, `Green_*` × `CC,A,A1,A2,B,B1,B2,C,C1,C2,D,D1,D2,E,G1,G2,H1,H2`).
3. **Role is an init-field variable on the unit**: `init="this setVariable [""fnfLoadout"", ""SL""];"`.
   The 20 distinct values present in `mission.sqm`, with occurrence counts (3 sides × n):
   `SL`×27, `PI`×18, `LAT`×18, `GR`×18, `CE`×18, `ARA`×18, `AR`×18, `AB`×18, `SGT`×15, `MED`×15,
   `PL`×12, `MGA`×12, `MG`×12, `CR`×12, `DM`×9, `MATA1`×6, `CRL`×6, `MAT1`×3, `EO`×3, `CC`×3.
4. **The framework's role vocabulary is 29 entries**, `client/init/fn_init.sqf:1-31`, each
   `[key, [display name, rank]]` — e.g. `["CC",["Company Commander","CAPTAIN"]]`,
   `["MATA1",["Asst. AT Specialist","PRIVATE"]]`, `["SHQAUX",["Crew/Wpn Operator","PRIVATE"]]`.
   Nine of the 29 (`TL`, `GRIR`, `RI`, `RIS`, `SNP`, `MAT2`, `MATA2`, `SHQAUX`, `BASE`) are **not used by
   any shipped slot** — they exist for gear-set inheritance and for runtime substitution.

### The three parallel ORBAT declarations
The same tree is declared three times, in three formats, and they must be kept in sync by hand:

1. **`description/cfgFNFORBAT.hpp`** (974 lines) — a BIS `CfgORBAT`-shaped config
   (`class FNFBLUPLTHQ` with `id`, `idType`, `side`, `size`, `type`, `text`, `textShort`, `subordinates[]`,
   `assets[]`). Squads carry three fireteam children (`FNFBLUPLTA1` "Team 1 - Assault Team",
   `FNFBLUPLTA2` "Team 2 - Support Team", `FNFBLUPLTAC` "Team 3 - Vehicle Crew");
   `FNFBLUPLTG1/G2` are `type="Cavalry"`, `texture="b_mech_inf"`; `FNFBLUPLTH1/H2` are
   `type="AviationSupport"`. A commented-out `FNFBLUPLTX` "X-Ray Recon Team"
   (`cfgFNFORBAT.hpp:29-44`) survives from a removed element (`changelog.txt:85` "Removed XRAY element.").
2. **`client/briefing/fn_setGroupIDs.sqf:47-164`** — the runtime table, format
   `[group, groupID, fnf_LongName, unitSize, radioSettings]`, adapted from the **F3 framework**
   (`:19-21` "F3 Set Group IDs / Credits: Please see the F3 online manual (ferstaberinde.com/f3/en/)").
   `unitSize` is `Company = 3, Platoon = 2, Squad = 1, Fireteam = 0` (`:35`). Example row:
   `["Blue_A1","A1","Alpha One",0, [2, 1, [2, 2.1, 2.2, 2.3]] ]`.
   It declares **more groups than the template ships** — `Blue_A3`, `Blue_B3`, `Blue_C3`, `Blue_D3`,
   `Blue_G3`, `Blue_G4`, `Blue_H3` and a `Blue_Pilot` marked `//not used` (`:50-51`) — headroom for a
   maker who copies a group in Eden and names it `Blue_A3`.
3. **`mission.sqm`** itself — the physical slots.

### How a player picks a slot
Standard Arma multiplayer role-selection lobby. There is no custom slotting UI in v3.
`description/definitions.hpp:25-29` sets `class Header { gameType = Unknown; minPlayers = 20;
maxPlayers = 124; }` and `joinUnassigned = 1`. On slot-in, `client/init/fn_init.sqf:101-107` waits for
the staggered-load window then calls `[player getVariable "fnfLoadout"] call fnf_loadout_fnc_applyLoadout`.
If that fails, `client/loadout/fn_checkLoadout.sqf:26-31` **ends the mission for that client after 30
seconds** so they are returned to the slot screen rather than playing naked.

Late joiners are blocked: `client/init/fn_canPlay.sqf:11-13` returns false if
`didJIP && !fnf_safetyEnabled`, and `client/init/fn_init.sqf:272-275` then hides and kills the unit.

### Radio channels are derived from the ORBAT
The `radioSettings` array in `fn_setGroupIDs.sqf` is `[altChannel, mainChannel, [channelOffsets]]`
(`:36-43`). Base frequencies are randomised per side per round —
`server/init/fn_genRadioFreqs.sqf:4-11` picks `floor(random 40) + 30` MHz for each of BLUFOR/OPFOR/
INDFOR/civilian and broadcasts them. `client/radio/fn_setRadios.sqf` then programmes each player's TFAR
radio from their group's row. The resulting frequency table is printed into the ORBAT diary entry
(`client/briefing/fn_createOrbat.sqf:99-120`).

---

## 5. Loadouts / arsenal

### There is no arsenal. Loadouts are config, and they are per-role.
`ace_arsenal` is loaded (`cba_settings.sqf:32-38`) but only as an Eden attribute on ammo boxes; players
never open it. A player's kit is computed at spawn from **two config axes**:

```
CfgFNFLoadouts >> UNIFORMS >> <fnf_XUniform> >> <fnfLoadout role>   // clothing
CfgFNFLoadouts >> GEAR     >> <fnf_XGear>    >> <fnfLoadout role>   // weapons, ammo, items
```
(`client/loadout/fn_applyLoadout.sqf:109-110`, resolved from `playerSide` at `:76-82`.)

`description/cfgFNFLoadouts.hpp` is a pure include manifest: **93 uniform sets** (`:6-97`) and
**63 gear sets** (`:104-164`), plus `KITS/GEAR/common.hpp` (`:2`) which supplies shared macros and the
`common`/`optics` classes.

### Uniform set shape
`description/KITS/UNIFORMS/RHS_UNI_NATO_US_ARMY_2020.hpp` is representative. It opens with
`#include "..\undef.hpp"` (`:1`) — a 228-line `#undef` block that resets every macro so sets cannot leak
into each other — then defines ~15 macros (`UNIFORM`, `VEST`, `VEST_LEADER`, `VEST_AR`, `HELMET`,
`HELMET_CMDR`, `HELMET_RECON`, `HELMET_CREWMAN`, `HELMET_PILOT`, `BACKPACK`, `BACKPACK_RADIO`,
`BACKPACK_AR`, `BACKPACK_AT`, `BACKPACK_MEDIC`, …), then a class per role built by inheritance:

```cpp
class BASE { uniform[]={UNIFORM}; vest[]={VEST}; headgear[]={HELMET}; backpack[]={BACKPACK}; };
class RI : BASE {};
class TL : BASE { vest[]={VEST_LEADER}; backpack[]={BACKPACK_RADIO}; };
class SL : TL {};  class SGT : SL {};  class PL : SGT {};
```
(`:31-47`.) 25 role classes per uniform set, plus `author` and `description` strings (`:28-29`).

### Gear set shape
`description/KITS/GEAR/examplebase.hpp` is the documented template. Macros first (`RIFLE`, `RIFLE_MAG`,
`RIFLE_MAG_RI`, `SIDEARM`, `RIFLE_GL`, `CARBINE`, `SMG_RIFLE`, `SHOTGUN`, `AR_RIFLE`, `AT_LAUNCHER`,
`DM_RIFLE`, `MMG_RIFLE`, `SPOTTER_RIFLE`, `SNP_RIFLE` and their magazine partners), then:

```cpp
defaultMAT[] = { {CARLG,1}, {STINGER,1} };
class BASE {
  backpackItems[]={}; launchers[]={}; sidearms[]={{{SIDEARM},{SIDEARM_MAG}}};
  weaponChoices[]={ {{RIFLE},{RIFLE_MAG}}, {{CARBINE},{CARBINE_MAG}} };
  magazines[]={BASE_GRENADES}; items[]={TOOLS,GRUNT_MEDICAL}; linkedItems[]={LINKED};
  attachments[]={}; launcherAttachments[]={};
  explosiveChoices[]={}; grenadeChoices[]={};
  giveSideKey = 0;   // 0 none, 1 side key, 2 GLOBAL key
  giveSilencer = 0;
};
```
(`examplebase.hpp:64-88`.) Subclasses use `+=` to extend arrays, e.g. `class TL : BASE { magazines[] +=
{LEADER_SMOKES}; items[] += {LEADER_TOOLS}; linkedItems[] += {VECTOR}; giveSideKey = 1; };` (`:97-114`).

**Magazine counts use a `"class:count"` string convention**, e.g. `"vn_m16_20_mag:7"` (`:6`), parsed at
runtime by `client/loadout/fn_addGear.sqf`. `description/KITS/GEAR/common.hpp` holds the shared vocabulary:
`GRUNT_MEDICAL "ACE_fieldDressing:8","ACE_morphine:2"` (`:2`),
`MEDIC_MEDICAL` (42 dressings, 16 morphine, 8 epi, 4 tourniquets, 16 blood bags, 1 PAK) (`:3`),
`LINKED "ItemMap","ItemCompass","ItemWatch","ItemGPS","TFAR_microdagr"` (`:4`),
`FRAG_GRENADES "rhs_mag_m67:2"` (`:16`), `GR_GRENADECOUNT 9` (`:21`), `GRIR_GRENADECOUNT 3` (`:22`),
`UGL_SMOKECOUNT 6` (`:23`), `UGL_FLARECOUNT 3` (`:24`), and the CE explosive menu entries which carry a
**human label as the first array element** — `CE_MINEAP "2x AP mine, 4x flare mine",
"APERSTripMine_Wire_Mag:2","rhs_mine_sm320_red_mag:4"` (`:45`).

### Application pipeline
`client/loadout/fn_applyLoadout.sqf` strips the unit bare (`:36-50`), resolves the role
(with `CC`→`PL` and `EO`→`PL` collapsing at `:56-60`), then calls 17 `client/loadout/procedure/`
functions in order (registered in `description/cfgFunctions.hpp:157-177`): `addUniform`, `giveRadios`,
`giveGear`, `givePrimaryWeapon`, `prepWeaponsSelector`, `giveSidearmWeapon`, `giveSilencer`, `giveNVG`,
`giveAT`, `prepOpticsSelector`, `giveCECharges`, `giveCEGrenades`, `setAttributes`, `giveSideKey`,
`giveBinoculars`, `loadWeapons`, `setRank`, `setFace`. Every step has its own error notification
(`fn_applyLoadout.sqf:191-382`, 16 distinct messages), so a broken config tells the player which stage
failed.

### The in-game "Gear Selector" — the only player choice
`client/loadout/selector/fn_init.sqf` builds an **ACE self-interaction** menu (`:398`
`[(typeOf player), 1, ["ACE_SelfActions"], _mainAction]`) with children Weapon, Sidearm, Optic, Explosives,
Grenades, Crew-Served. Availability is gated by mode (`:32-49`): in Standard it is available **only while
safe start is running**; in Sustained Assault, whenever the player is inside their own safe zone.
Choices come from the arrays the loadout pipeline published (`fnf_selector_weapons`,
`fnf_selector_optics`, `fnf_selector_explosives`, `fnf_selector_grenades`), which are exactly the
`weaponChoices[]`, `explosiveChoices[]`, `grenadeChoices[]` of the player's role class plus the optic tier
their role earns. A CBA FlexiMenu alternative exists behind the client-mod setting
`fnf_pref_loadoutInterface` (`client_mod/fnf_setup/XEH_preInit.sqf:7-29`), reached with `Ctrl+Shift+N`
(`XEH_postInit.sqf:74`).

### Optic tiers — a hard balance rule expressed in config
`description/KITS/GEAR/common.hpp:106-120` defines three tiers (`STD_OPTICS` 9 red dots,
`MAG_OPTICS` 14 magnified, `SNP_OPTICS` 2 sniper scopes, plus VN equivalents) and exposes them as
`class optics { standard[]; magnified[]; sniper[]; sniperNVG[]; }`.
`client/loadout/procedure/fn_prepOpticsSelector.sqf:32-63` then applies `fnf_magnifiedOptics`:
`-1` = nobody gets an optic except DM (magnified) and SNP (sniper); `0` (default) = everyone gets
standard, DM gets magnified, SNP gets sniper; `1` = everyone gets magnified **except `MG`** —
`:46` `if !(_role in ["MG"]) then {…}` with the comment *"ensures MMGs never get magnified optics"*.

### MAT (medium anti-tank) — squad-level, config-selected
`description/configDefs.hpp:20-43` defines launcher macros as
`[classname, [magazines], [optics], reloadable, shortname]`, e.g.
`#define CARLG(_HEATCount,_HECount) ["rhs_weap_maaws",[…HEAT:_HEATCount, …HE:_HECount],["rhs_optic_maaws"],"RELOAD","CARLG"]`.
`configGuide.txt:146-185` lists the 15 valid launchers with recommended counts and explains
*"The 2 refers to how many HEAT rockets/missiles for each: gunner, assistant, assistant. a value of 2
would mean a total of 6 rockets/missiles in this squad"*.
`GEARDEFAULT` (`configDefs.hpp:43`) defers to the gear set's own `defaultMAT[]`, resolved by
`client/loadout/tools/fn_setMAT.sqf:61-71` with `selectRandom`.
A neat consequence: if the selected launcher is `DISPOSABLE`, the two assistant slots are silently
**demoted to Rifleman** — `fn_applyLoadout.sqf:92-98`.

### Vehicle loadouts
`description/cfgFNFVehicleLoadouts.hpp` hooks 18 vehicle base classes (`rhs_2b14_82mm_Base`,
`RHS_AH1Z_base`, `RHS_Ka52_base`, `rhs_tigr_base`, `RHS_M252_Base`, `RHS_MELB_AH6M`, `RHS_Mi24P_VVS_Base`,
`RHS_Mi24V_Base`, `rhsusf_mkvsoc`, `rhsusf_M1117_base`, `Boat_Armed_01_minigun_base_F`,
`O_Boat_Armed_01_hmg_F`, `UK3CB_AAV`, `UK3CB_BMP1Tank_Base`, `UK3CB_BMP2Tank_Base`, `UK3CB_LAV25`,
`UK3CB_LAV25_HQ`, `UK3CB_Warrior_Base`) to per-vehicle SQF that rewrites turret magazines, gated by the
Eden checkbox via `fn_shouldModify.sqf:3`. `loadoutData.json` (48 KB, 64 vehicles) is the exported record,
rendered to HTML by `compileHtml.ps1` for the community wiki.

### Kit preview images
`client/briefing/tools/fn_getUniformPics.sqf:14,20` builds
`format["%1--%2", uniformSet, gearSet]` then `format["fnf_media\images\kits\%1\%2.paa", folder, role]`,
guarded by `fileExists`. 29 folders × 25 role images ship in `client_mod/fnf_media/images/kits/`. They are
generated by the pipeline in §11.

---

## 6. Briefing / intel

### Author surface: four strings
`config.sqf:9-15` is the entire briefing-authoring API:
`fnf_briefingBackground` ("any lore you wish to explain"), `fnf_briefingWorldInfo` ("notable details about
the AO"), `fnf_briefingNotes` ("anything else"), `fnf_briefingRules` ("mission-specific rules").
`client/briefing/fn_createBrief.sqf:394-421` renders each into an HTML section headed BACKGROUND / AREA OF
OPERATIONS / NOTES / MISSION RULES, skipping empty ones, and sanitises the first three with
`CBA_fnc_sanitizeHTML` (`:397,404,411`) — `fnf_briefingRules` is **not** sanitised (`:418`), i.e. raw HTML
is permitted there only.

### Everything else in the briefing is generated
`client/briefing/fn_createBrief.sqf` (24 KB) builds the map diary at load, gated on
`getClientStateNumber >= 8 && fnf_groupIDset` (`client/briefing/fn_init.sqf:22-26`). Auto-generated
content includes:
- **Environment block** (`:384-391`): temperature at sea level via `ace_weather_fnc_calculateTemperatureAtHeight`,
  wind direction and strength, sunrise/sunset, moon fullness.
- **Game mode + overtime conditions** (`:425-434`), where the overtime string is authored *per mode* in
  `server/init/fn_setupGame.sqf:37-77`, e.g. *"The mission will go into overtime if there is only 1 alive
  objective remaining and attackers stay near the objective."*
- **ORBAT** (`client/briefing/fn_createOrbat.sqf`) — every group, its long name, its members, and its TFAR
  frequency, colour-coded, regenerated on every player connect/disconnect
  (`server/init/fn_serverInit.sqf:257-287`).
- **Asset diary** (`client/briefing/fn_assetDiary.sqf`, 23 KB) — walks every vehicle, enumerates each
  turret's weapons and magazine counts by seat category, and writes a per-side asset list. Vehicles locked
  at mission start are excluded (`changelog.txt:86`).
- **Loadout pages** — `client/briefing/tools/fn_parseGear.sqf` (14 KB), `fn_parseLoadout.sqf`,
  `fn_parseMAT.sqf`, `fn_parseCSW.sqf`, `fn_weaponDetails.sqf` render every side's kit, with the kit
  screenshots inlined two-per-row (`fn_getUniformPics.sqf:33`).
- **Reconnaissance photos** — `client/briefing/fn_objectiveRecon.sqf` creates a `"Recon"` diary subject
  (`:4`), one record per objective, each containing an `<executeClose expression='… fnf_fnc_objectivePreview'>`
  link. Clicking it flies a camera to the objective and renders a film-grain, sepia "aerial photo"
  (`client/misc/fn_objectivePreview.sqf:5-56`), oriented north, with enemy players and non-objective
  vehicles hidden. **The subject is deleted the moment safe start ends** (`fn_objectiveRecon.sqf:5`).

### Delivery surfaces
1. **Map diary** (standard Arma briefing).
2. **Mission Info panel** — a custom dialog (`client/ui/InfoPanel.hpp`, opened with `Ctrl+J` per
   `client_mod/fnf_setup/XEH_postInit.sqf:102`, or a button in the ACE spectator UI). Its tree
   (`client/ui/fn_missionInfoPanel.sqf:24-60`): Briefing, Gamemode, Mission Variables, My Starting
   Loadout, My Starting Radios, ORBAT, MAT Settings, then per side {Uniform, Loadout, CSW, Assets},
   Other Assets, Framework Info, Credits, Rules. Available to spectators too, via
   `fn_createBriefSpec.sqf`.
3. **Physical briefing tables.** `fnf_briefingTable_<side>` is a single reference desk the maker places
   (`mission.sqm:4213`); at runtime `client/briefing/table/fn_setupTables.sqf:24,41-96` **spawns seven
   more desks in a 50 m radius around it** — one per `["PLT","ALPHA","BRAVO","CHARLIE","DELTA","GOLF","HOTEL"]` —
   avoiding roads, other tables and vehicles (`:52-57`), each marked on the map with a `mil_box_noShadow`
   icon labelled with the squad name (`:84-88`). Standing at a table shows that squad's objective
   overview. All tables and markers are removed when safe start ends
   (`server/init/fn_safety.sqf:74`, `:89-93`). Credited to *"Seb's Briefing Table (Modified by IndigoFox)"*
   in `description/credits.txt`.
4. **Map markers.** Safe zones are drawn from the maker's area markers; irregular zones are triangulated
   and shaded client-side (`client/ui/map/fn_initPolygonShading.sqf`, `fn_triangulatePolygon.sqf`,
   `fn_invertPolygon.sqf` — 16 KB of polygon maths). Every large placed object is auto-marked (§3 step 7).
   `fnf_enemyStartVisible` (`config.sqf:60`) decides whether the enemy's start zone is drawn for you
   (`client/ui/map/fn_initUnregularZones.sqf:236-292`).
5. **Text-tile splash** on mission start — server name, briefing name, date/time, nearest named location,
   game mode (`client/init/fn_init.sqf:146-172`).

---

## 7. Objectives / game modes

Nine modes ship with `mode_config` files, plus `sustainedAssault` which has none. The split
(`README.md:66-80`, `configGuide.txt:6-17`) is **ATK/DEF** — `adSector`, `assassin`, `captureTheFlag`,
`destroy`, `rush`, `uplink` — versus **NEUTRAL** — `connection`, `neutralSector`, `scavHunt`.
Setting `fnf_gameMode = "";` disables the mode system entirely (`configGuide.txt:5`).

### What the split means in code *(modes agent)*
- **Init guard.** `server/init/fn_setupGame.sqf:35,40,45,50,55,60` — ATK/DEF modes
  `exitWith {call _fnc_warnForAD}` if `sideEmpty in [fnf_attackingSide, fnf_defendingSide]`;
  `:64,69,74` — NEUTRAL modes do the inverse (`if !(sideEmpty in …)`).
- **Interaction symmetry.** Same data terminal, two modes:
  uplink `modes/uplink/uplink_client.sqf:9` gates hacking on `playerSide == fnf_attackingSide` with a
  separate defender-only "Stop Hack"; connection `modes/connection/connection_client.sqf:12` gates on
  `!(playerSide == fnf_term1HackingSide) && !(playerSide == civilian)` — any side may take it.
- **Fortify is disabled in the two sector/terminal neutral modes** —
  `client/misc/fn_fortifyClient.sqf:19`.
- **Defenders have no positive win path in any ATK/DEF mode file** — they win only on the clock
  (`server/end/fn_overTimeEnd.sqf:8-15` always awards `fnf_defendingSide`) or by admin adjudication.

### Per-mode: maker knobs, required editor objects, win condition

| Mode | `mode_config/*.sqf` knobs (default) | Editor objects the maker must supply | Win |
|---|---|---|---|
| **destroy** | `_obj1` `[destroy_obj_1,"destroy_obj_1_mark",""]`, `_obj2`, `_obj3` (`objNull` = unused) | `destroy_obj_1..3` (any destructible object) + **AREA** markers `destroy_obj_N_mark` | attackers destroy **all** configured objectives (`destroy_server.sqf:188-194`) |
| **uplink** | `_numberOfTerminals` 2, `_terminalHackTime` 90 (scalar or per-terminal array) | `term1`,`term2`,`term3` — `Land_DataTerminal_01_F` | attackers hack all N, **any order** (`uplink_server.sqf:277-283`) |
| **rush** | `_numberOfTerminals` 3, `_terminalHackTime` 90 | same `termN` | attackers hack all N **in sequence** (`rush_server.sqf:218-244`) |
| **captureTheFlag** | `_flagCaptureTime` 600, `_flagMarkUpdateTime` 15, `_showCapZoneGlobal` false | `ctf_flagPole` (`FlagPole_F`) + `ctf_attackTrig` (Trigger) | flag sits dropped in the capture zone for 600 s (`ctf_server.sqf:160-170`) |
| **adSector** | `_numberOfSectors` 3, `_inOrder` false, `_captureTime` 60 | `fnf_sec1..3` — **Triggers** | attackers capture all sectors (`adSector.sqf:173`) |
| **assassin** | `_targets` (3 rows), `_requiredKills` 3 | `HVT_1..N` playable **units** + `fnf_assassin_boundaries_N` area markers | **no automatic win** — staff call `fnf_assassin_fnc_endGame` (`assassin_server.sqf:166-171`) |
| **neutralSector** | `_numberOfSectors` 3, `_pointAddTime` 11 (auto-scaled ×1.4/×1.7/×2) | `fnf_sector1..4` — **`ModuleSector_F` modules**, each synchronized to three side logics | first side to **100 points** (`neutralSector.sqf:82`) |
| **connection** | `_numberOfTerminals` 3, `_pointAddTime` 40 | `term1..3` | first side to **100 points** (`connection_server.sqf:150`) |
| **scavHunt** | `_numberOfObjectives` 10, `_numberOfTransportsPerSide` 3 | `scav_obj_1..N`, `scav_transport_1..M` (flat namespace, `M = perSide × sides`), `scavHuntCapWEST/EAST/GUER` area markers | **strict majority** of objects returned (`scavHunt_server.sqf:452-456`) |

Note the near-collision: **`fnf_sec`N are adSector triggers; `fnf_sector`N are neutralSector modules** —
different prefix, different Eden entity type.

### Scoring
Binary for all six ATK/DEF modes — no score variable is published. Points for all three neutral modes:
`connection` and `neutralSector` **reuse Arma's respawn-ticket counter as an abstract point store** —
`[side, 1] call BIS_fnc_respawnTickets` (`neutralSector.sqf:78`, `connection_server.sqf:146-148`), first
to 100 wins, displayed by `BIS_fnc_showMissionStatus`. `scavHunt` counts held objects in a hashmap
(`scavHunt_server.sqf:426-449`). `neutralSector.sqf:72-73` imposes a hard 5-minute grace period after the
first capture before any points accrue.

### Round clock and overtime
Round length is **not** a maker knob. `description/sekrit.sqf:2,4,6`:
`fnf_missionTimeLimit = 50;` (120 for SA) and `fnf_safeStartTime = 15;`.
`scavHunt_server.sqf:7-8` overrides it to 40.
`server/end/fn_checkTime.sqf:6` computes `(fnf_missionTimelimit * 60) + fnf_safetyEndTime`, warns at
T-15 min (`:17-22`), then sets `fnf_overTime` and spawns `fnf_server_fnc_overTimeEnd` (`:24-29`).
`fn_overTimeEnd.sqf` is a per-mode switch (destroy `:18-50`, uplink `:53-70`, rush `:73-99`,
adSector `:102-124`, ctf `:127-138`, neutralSector/connection `:141-206` with a 20-point lead
requirement, scavHunt `:209-227`); **assassin has no case**, so an assassin round that runs out of time
never ends itself. There is no sudden-death mechanism anywhere in the tree.

All wins funnel into `server/end/fn_gameEnd.sqf`, which shows a winner banner (`:31-39`), posts the round
result to Discord (`:41`), sets debriefing text (`:44-50`), calls `ocap_fnc_exportData` if OCAP2 is loaded
(`:52-54`), and ends with `"end1" call BIS_fnc_endMissionServer;` (`:56`).

### Elimination
`server/end/fn_checkAlive.sqf` runs a 10-second poll that announces `"<Side> eliminated!"` (`:56-61`),
suppressed mid-hack in uplink/rush (`:70-76`). **The block that would end the round on elimination is
commented out** (`:79-93`); the live path is the admin button
`client/misc/admin/fn_adminGameEnd.sqf:75` → `server/end/fn_endElimination.sqf`.

### Mission specials (optional set dressing)
`missionSpecials/fn_config.sqf` gives the maker two opt-in systems, both default `false`:
**ambient airdrop** (per-side marker-targeted cargo drops — C-130Js for BLUFOR, An-2s for OPFOR, `:56-60`)
and **ambient artillery** (a target position/object/marker, ammo class, radius, round count, delay range,
end condition, internal safe radius, spawn altitude, fall speed — `:30-43`).

---

## 8. Respawn / tickets / medical / revive

### Standard FNF is one life
`description/definitions.hpp:6-11`:
```
respawn = 3;  respawnDialog = 0;  respawnButton = 1;
respawndelay = 99999;  respawnOnStart = -1;  respawnTemplates[] = {};
```
`respawndelay = 99999` is the one-life rule. The respawn button is additionally disabled from the pause
menu whenever the player is dead, safe start is running, or they are spectating —
`client/restrictions/fn_removeRespawnButton.sqf:6-8`, wired via `onPauseScript[]`
(`description/definitions.hpp:14-20`). On death the player becomes an ACE spectator after 3 s
(`client/init/fn_init.sqf:193-197`).

**Sustained Assault is not.** `description/definitions_SA.ext`… i.e. `description/definitions_SA.hpp:5-9`
sets `respawnDialog = 1; respawndelay = 25;` — a 25-second respawn — and `disableChannels[] = {2}`.

### Tickets
There is **no ticket-based attrition system in Standard mode**. `BIS_fnc_respawnTickets` appears only as
the point store for `connection` and `neutralSector` (§7), where the count only ever *increases* toward
100. The "each side starts with a set number of tickets / decrease by 1 when a player dies / when one side
reaches 0 the opposing side wins" text in `description/sekrit.sqf:96-99` and `:142-145` is a **Sustained
Assault** description — `client/ui/fn_missionInfoPanel.sqf:55-57` only adds the "Game Mechanics" node
`if (fnf_gameMode == "sustainedAssault")`. The SA `.sqm` files are binarized, so the SA ticket
implementation was not read.

### Respawn handling
`client/misc/fn_handleRespawn.sqf` re-applies the loadout (`:9`), re-applies the admin patch (`:10`),
clears spectator state (`:13-15`), teleports the player to `fnf_startGoodPos` (`:19-24`), re-registers
them in `fnf_playersInMission` (`:27-31`), then re-runs restrictions and safety (`:44-45`).
`changelog.txt:19` records post-safe-start respawn being added for late joiners.

### Medical — ACE, tuned hard toward lethality with long unconsciousness
From `cba_settings.sqf` (all `force force`, i.e. locked):
`ace_medical_deathChance = 1` (`:212`), `ace_medical_fatalDamageSource = 2` (`:214`),
`ace_medical_playerDamageThreshold = 1.5` (`:222`), `ace_medical_bleedingCoefficient = 0.3` (`:208`),
`ace_medical_painCoefficient = 0.6` (`:220`), `ace_medical_painUnconsciousChance = 1` (`:221`),
`ace_medical_spontaneousWakeUpChance = 0.75` (`:223`),
`ace_medical_statemachine_cardiacArrestTime = 420` (`:227`) — seven minutes before bleed-out,
`ace_medical_statemachine_cardiacArrestBleedoutEnabled = true` (`:226`),
`ace_medical_statemachine_fatalInjuriesPlayer = 0` (`:229`),
`ace_medical_fractureChance = 0.8` (`:215`) with `ace_medical_fractures = 0` (`:216`),
`ace_medical_limping = 1` (`:219`),
`ace_medical_treatment_advancedBandages = 0` (`:230`), `advancedDiagnose = 1` (`:231`),
`advancedMedication = false` (`:232`), `woundReopenChance = 0` (`:264`),
`ace_medical_gui_maxDistance = 3` (`:217`), `ace_medical_ivFlowRate = 3` (`:218`),
`ace_medical_treatment_clearTrauma = 2` (`:239`).
**Medic-gated items:** PAK and surgical kit require a medic (`medicPAK = 1`, `medicSurgicalKit = 1`,
`:255-256`) but epinephrine and IV do not (`medicEpinephrine = 0`, `medicIV = 0`, `:253-254`);
self-PAK is banned (`allowSelfPAK = 0`, `:236`) while self-IV and self-stitch are allowed (`:235,237`).
Treatment times: CPR 10 s, IV 6 s, splint 7 s, tourniquet 5 s, body bag 15 s, autoinjector 4 s
(`:258-263`). CPR succeeds 40–80 % of the time (`:243-244`).

There is **no revive template** — `respawnTemplates[] = {}`. Recovery is entirely ACE medical: stabilise,
epinephrine, blood, PAK. The Medic role carries 42 field dressings, 16 morphine, 8 epinephrine,
4 tourniquets, 16 blood IVs and one PAK (`description/KITS/GEAR/common.hpp:3`).

### Body/disconnect handling
`ace_respawn_removeDeadBodiesDisconnected = false` (`cba_settings.sqf:417`) and
`ace_respawn_savePreDeathGear = false` (`:418`). During safe start, a disconnecting player's unit is
deleted outright (`server/init/fn_serverInit.sqf:290-301`); after safe start the body is kept.

---

## 9. Zones / areas / triggers / play area

Five distinct zone systems, all keyed on **naming conventions the maker must obey exactly**.

### 1. Safe zones (per side)
Two forms, both discovered by regex over `allMapMarkers` at runtime
(`client/misc/fn_inSafeZone.sqf:19-207`):
- **Simple:** `^(west|east|guer)_safeZone_marker_\d+$` — any Arma area marker (ellipse or rectangle);
  the test is `vehicle _unit inArea _x` (`:241`).
- **Polygon:** `^fnf_custom_safeZone_(west|east|guer)_\d+_marker_\d+$` — the first `\d+` groups markers
  into one zone, the second orders the vertices; the test is `inPolygon` (`:239`).
  `mission.sqm` ships four such markers per side.

Both are enumerated in a single pass and sorted (`:210-225`). The function doubles as a *marker lister*
via its `_justReturnMarkers` parameter (`:227`), which is how safe-zone markers get deleted at safe-start
end (`server/init/fn_safety.sqf:81-88`) and how `fn_markCustomObjs.sqf:3-8` builds its exclusion list.

Effects of being inside your own safe zone: invincibility during safe start
(`client/safety/fn_init.sqf:7`, and in SA a continuous check at `:80-91`), access to the Gear Selector
(`client/loadout/selector/fn_init.sqf:37-47`), and vehicle keying —
`server/init/fn_keyVehicles.sqf:9-28` assigns `ace_vehiclelock_lockSide` to whichever side's safe zone a
vehicle spawned in, and anything outside all safe zones becomes `sideUnknown` (locked to everyone).
Vehicles left unoccupied are locked 5 minutes after safe start ends
(`server/init/fn_safety.sqf:77-79` → `fn_lockVehicles.sqf`).

### 2. Play-area boundary
`zoneTrigger` (an `EmptyDetector`) **or** the `fnf_custom_zoneBoundary_1_marker_N` polygon.
`client/restrictions/fn_zoneBoundary.sqf:5,15-25` — outside the area and not in/under an aircraft, the
player gets a 20-second countdown (`titleText "You have %1 seconds to get back into the mission zone."`),
then their vehicle is neutralised and `player setDamage 1` 5 s later (`:18-21`). The polygon branch
requires **at least 3 markers** (`:61`). Server-side, `server/init/fn_serverInit.sqf:107-112` also feeds
`zoneTrigger` to `BIS_fnc_moduleCoverMap` so the map is greyed outside the play area.

### 3. Safe-start boundary
`client/restrictions/fn_startBoundary.sqf` confines players to their base/forward zone during safe start
(described in `description/sekrit.sqf:76` — *"during safe start, players may teleport to their forward
zones, but must stay in main base or forward zone"*).

### 4. Restricted zones (enemy entry denial)
`client/restrictions/fn_restrictedZones.sqf` — **Sustained Assault only** (`:1`
`if !(fnf_gameMode == "sustainedAssault") exitWith {};`). Every 2 s it records the last legal position and
snaps the player back if they entered an enemy zone, with the notification *"You're too close to the
enemy's entry point!!"* (`:21`).

### 5. Mode-specific trigger/marker zones
Per §7 — `fnf_sec1..3` (adSector triggers), `fnf_sector1..4` (neutralSector modules),
`ctf_attackTrig`, `scavHuntCapWEST/EAST/GUER`, `fnf_assassin_boundaries_N` (jammer zones,
discovered by prefix scan so the count is unbounded), and the `destroy_obj_N_mark` area markers.

### Irregular-zone rendering
`client/ui/map/` contains a small computational-geometry library used to shade non-rectangular zones on
the 2D map: `fn_initUnregularZones.sqf` (11 KB), `fn_invertPolygon.sqf` (16 KB),
`fn_triangulatePolygon.sqf`, `fn_triangulateAndShadePolygon.sqf`, `fn_genPolylineFromMarkers.sqf`,
`fn_initPolygonShading.sqf`, `fn_removeShadedPolygon.sqf`. Ear-clipping triangulation plus polygon
inversion, in SQF, so that a hand-placed marker ring renders as a filled hatched region.

### Fortification zones
`ace_fortify` is registered **for the defending side only** —
`server/init/fn_fortifyServer.sqf:79-88` `[fnf_defendingSide, fnf_fortifyPoints, _set] call
ace_fortify_fnc_registerObjects`. Four object sets are defined in the same file: `ModernGreen` (`:8-24`),
`ModernTan` (`:25-41`), `NVA` (`:42-55`), `MACV` (`:56-71`), each `[classname, cost]`, and `"Modern"`
auto-picks tan on 13 named desert maps (`:4-5`). Trench digging is restricted near roads and objectives:
`client/restrictions/fn_restrictETool.sqf:5,16-18` — *"Cannot place trench within 30m of an objective or
within 12m of a road"*. Fortify is force-disabled the moment safe start ends
(`server/init/fn_safety.sqf:68` `["off"] call acex_fortify_fnc_handleChatCommand`).

---

## 10. Configuration surface

Exhaustive. Every knob a mission maker can set, grouped by file.

### A. `FNF_MissionTemplate.VR/config.sqf` — 24 variables

| # | Variable | Line | Default | Values / effect |
|---|---|---|---|---|
| 1 | `fnf_briefingBackground` | 9 | `""` | HTML lore block |
| 2 | `fnf_briefingWorldInfo` | 11 | `""` | HTML AO description |
| 3 | `fnf_briefingNotes` | 13 | `""` | HTML free-form notes |
| 4 | `fnf_briefingRules` | 15 | `""` | HTML mission-specific rules (not sanitised) |
| 5 | `fnf_gameMode` | 40 | `destroy` | one of the `configDefs.hpp` identifiers, or `""` for none |
| 6 | `fnf_defendingSide` | 46 | `west` | `west`/`east`/`independent`; `sideEmpty` for neutral modes |
| 7 | `fnf_attackingSide` | 47 | `east` | as above |
| 8 | `fnf_vnArtillerySide` | 50 | `sideEmpty` | which side's PLT SGT gets the Prairie Fire artillery interface (`client/loadout/procedure/fn_setAttributes.sqf:41`) |
| 9 | `fnf_SWRadioForAll` | 54 | `-1` | `-1` use loadout setting, `0` nobody gets a SW radio, `1` everybody does (`client/loadout/procedure/fn_giveRadios.sqf:28-40`) |
| 10 | `fnf_enemyStartVisible` | 60 | `true` | draw enemy start-zone markers (`client/ui/map/fn_initUnregularZones.sqf:236-292`) |
| 11 | `fnf_maxViewDistance` | 61 | `1500` | metres; LOW 500 / MEDIUM 1500 / HIGH 2000. Enforced every 0.5 s (`client/restrictions/fn_viewDistance.sqf:9-12`) |
| 12 | `fnf_fortifyPoints` | 69 | `125` | per-Combat-Engineer fortify currency; LOW 60 / MEDIUM 125 / HIGH 250; `0` disables |
| 13 | `fnf_fortifyStyle` | 77 | `"Modern"` | `"Modern"` (auto tan/green) / `"ModernGreen"` / `"ModernTan"` / `"NVA"` / `"MACV"` |
| 14 | `fnf_magnifiedOptics` | 87 | `0` | `-1` ironsights only, `0` 4×+ restricted to DM/SNP, `1` magnified for all except MGs |
| 15 | `fnf_isNightMission` | 95 | `-1` | `-1` auto-detect from sunrise/sunset, `0` force day, `1` force night (`server/init/fn_serverInit.sqf:40-46`) |
| 16 | `fnf_addNVG` | 104 | `0` | `0`/`[]` none, `1`/`[east,west,independent]` all, or a side array |
| 17 | `fnf_bluforUniform` | 119 | `"RHS_UNI_NATO_US_ARMY_2020"` | any of 93 sets |
| 18 | `fnf_bluforGear` | 120 | `"RHS_GEAR_US_ARMY_2010_M16A4"` | any of 63 sets |
| 19 | `fnf_opforUniform` | 123 | `"RHS_UNI_RU_COSSACKS_2010"` | |
| 20 | `fnf_opforGear` | 124 | `"RHS_GEAR_RU_ARMY_2010_AK74M"` | |
| 21 | `fnf_indforUniform` | 127 | `"RHS_UNI_ID_IRAQI_ARMY_2000"` | |
| 22 | `fnf_indforGear` | 128 | `"RHS_GEAR_ID_IRAQI_ARMY_2000_AKMN"` | |
| 23 | `fnf_bluAT` / `fnf_redAT` / `fnf_grnAT` | 136 / 139 / 142 | `GEARDEFAULT` | MAT macro per side, or `NOMAT()` |
| 24 | `fnf_showAlliedFactions` | 173 | `true` | show allied factions on the map (`client/icons/QS_icons.sqf:154`) |

Plus **12 SHQ auxiliary-role variables** (`config.sqf:151-166`), marked `//*NOT USED*` in the shipped file
but still read at runtime by `client/loadout/tools/fn_handleSHQAUX.sqf:27-36`:
`fnf_{west,east,guer}{Alpha,Bravo,Charlie,Delta}AuxRole`, each `0` (plain crewman) or one of the CSW
macros. And **`fnf_debug`** (`config.sqf:177`, default `false`) which unhides airdrop reference markers
(`missionSpecials/fn_ambientAirdrop.sqf:41`) and enables per-vehicle loadout hints
(`description/vehicleLoadouts/functions/fn_*.sqf:5`).

**MAT values** (`configGuide.txt:146-176`, macros at `description/configDefs.hpp:22-43`):
reloadable-with-HE `CARLG(_HEAT,_HE)`, `SMAW`, `RPG32`, `RPG7`; reloadable HEAT-only `TITAN(_n)`,
`JAVELIN`, `METIS`, `STINGER`, `IGLA`; disposables `M72LAW(1)`, `M80(1)`, `RPG26(1)`, `NLAW(1)`;
SOG-only `VN_LAW(1)`, `VN_RPG7(2)`, `VN_STRELA(1)`; plus `GEARDEFAULT` and `NOMAT()`.
(`AT4(_count)` exists in `KITS/GEAR/common.hpp:70` but is **not** in `configDefs.hpp` and is undocumented
in `configGuide.txt`.)

**SHQ aux / CSW values** (`configGuide.txt:188-207`, macros at `configDefs.hpp:47-65`):
`HMG_M2(_boxes)`, `HMG_M2_LO(_boxes)`, `MORTAR_2B14(_he,_smk,_illum)`, `MORTAR_M252(…)`,
`AT_SPG9(_he,_heat)`, `AT_METIS(_he,_heat)`, `AT_TOW(_tow)`; Vietnam-only
`VN_MORTAR_TYPE53(…)`, `VN_MORTAR_M2_60mm(…)`, `VN_MORTAR_M29_81mm(…)`;
and four listed under a literal `BROKEN:` heading (`configGuide.txt:203-207`) —
`HMG_KORD`, `HMG_KORD_LO`, `HMG_DSHKM`, `HMG_DSHKM_LO`.

### B. `mode_config/*.sqf` — 19 variables across 9 files
Enumerated in §7's table. In full: `_obj1/_obj2/_obj3` (destroy); `_numberOfTerminals`,
`_terminalHackTime` (uplink, rush); `_numberOfTerminals`, `_pointAddTime` (connection);
`_flagCaptureTime`, `_flagMarkUpdateTime`, `_showCapZoneGlobal` (ctf); `_numberOfSectors`, `_inOrder`,
`_captureTime` (adSector); `_numberOfSectors`, `_pointAddTime` (neutralSector); `_targets`,
`_requiredKills` (assassin); `_numberOfObjectives`, `_numberOfTransportsPerSide` (scavHunt).

### C. `description.ext` — 3 maker-set entries
`author`, `onLoadName`, `onLoadMission` (`description.ext:1-3`). Everything else in the include chain is
framework-owned. Framework values a maker could in principle change but is not told to:
`description/definitions.hpp` `Saving = 0`, `disabledAI = 1`, `enableDebugConsole = 1`, the 6 respawn
entries, `joinUnassigned = 1`, `onPauseScript[]` (5 entries), `class Header {gameType; minPlayers = 20;
maxPlayers = 124;}` (with a comment at `:22-24` suggesting `CTF` or `SC` as alternative `gameType`), and
`CfgDebriefingSections >> acex_killTracker`.

### D. `mission.sqm` Eden attributes
- **Scenario `Intel`** (`mission.sqm:256-274`): `overviewText`, `timeOfChanges=28800`, `startWeather=0`,
  `startWind=0.1`, `startWaves=0.1`, `forecastWeather=0`, `forecastWind=0.1`, `forecastWaves=0.1`,
  `forecastLightnings=0.1`, `year=2022 month=9 day=20 hour=12 minute=0`, `startFogDecay=0.014`,
  `forecastFogDecay=0.014`, plus the 3den-Enhanced attribute `ENH_timeMultiplier` (`:280`).
- **`ScenarioData`** (`:193-201`): `disabledAI=1`, `wreckRemovalMaxTime=3600`, `corpseManagerMode=1`,
  `corpseLimit=45`, `corpseRemovalMinTime=5`, `minPlayerDistance=15`.
- **Per-object FNF attributes**: the three checkboxes in §3 step 7.
- **`cba_settings_hasSettingsFile=1`** (`mission.sqm`, `class CustomAttributes`) — tells CBA to read the
  mission's `cba_settings.sqf`.

### E. Lobby parameters — `description/cfgParams.hpp`, 4 entries
`fnf_mapUnitIcons` (Show/Don't show unit icons on map, default 1), `fnf_gpsUnitIcons` (on GPS, default 1),
`fnf_gpsParam` ("All units get GPS" / "Only leadership roles get GPS", default 1),
`fnf_gps_map_master` ("Default" / "All off", default 1). These are chosen by the **admin at mission
start**, not by the maker.

### F. `missionSpecials/fn_config.sqf`
`ambientAirdrop`: `[enabled, requireIntact, hideMarkers, delaySeconds, [[side, [[planeCount, radiusM,
markerOrPos, headingDeg, [[cargoClass, count], …]], …]], …]]` (`:8-27`).
`ambientArtillery`: `[enabled, [[["_startDelay",10], ["_target",[500,500,0]], ["_mag","Sh_82mm_AMOS"],
["_radius",200], ["_rounds",15], ["_delay",[5,10]], ["_conditionEnd",{false}], ["_safeZone",0],
["_spawnAlt",250], ["_speed",150]], …]]` (`:28-44`).

### G. `cba_settings.sqf` — ~520 forced settings, **not** a maker surface
Namespaces present: `ace_advanced_ballistics`, `ace_advanced_fatigue`, `ace_advanced_throwing`,
`ace_vehicle_damage`, `ace_arsenal`, `ace_artillerytables`, `ace_mk6mortar`, `ace_captives`,
`ace_common`, `ace_noradio`, `ace_parachute`, `ace_cookoff`, `ace_csw`, `ace_explosives`, `ace_fire`,
`ace_fortify`, `acex_fortify`, `ace_frag`, `ace_goggles`, `ace_hearing`, `ace_interaction`,
`ace_gestures`, `ace_interact_menu`, `ace_cargo`, `ace_rearm`, `ace_refuel`, `ace_repair`,
`ace_magazinerepack`, `ace_map`, `ace_markers`, `ace_map_gestures`, `ace_maptools`, `ace_medical*`,
`ace_nametags`, `ace_nightvision`, `ace_overheating`, `ace_finger`, `ace_pylons`, `ace_quickmount`,
`ace_respawn`, `ace_scopes`, `ace_spectator`, `ace_switchunits`, `ace_fastroping`, `ace_gforces`,
`ace_hitreactions`, `ace_inventory`, `ace_laser`, `ace_laserpointer`, `ace_microdagr`,
`ace_optionsmenu`, `ace_overpressure`, `ace_tagging`, `ace_vehiclelock`, `ace_vehicles`,
`ace_viewdistance`, `ace_reload`, `ace_weaponselect`, `ace_weather`, `ace_winddeflection`,
`TFAR_*` (47 settings), `diwako_dui_enable_compass_dir`, `emr_main_*` (24), `grad_trenches_functions_*` (35).
Three divergent copies of this file exist *(mods agent)*: the mission copy, `server_mods/early_mod/…`,
and `server_mods/late_mod/…`. Early vs late differ in exactly five values —
`ace_advanced_fatigue_loadFactor` 0.5/0.7, `performanceFactor` 0.5/1.1, `recoveryFactor` 5/2.5,
`swayFactor` 1.1/0.9, `ace_markers_moveRestriction` 2/0. The mission copy additionally diverges on
`ace_medical_bleedingCoefficient` (0.6 → 0.3) and `ace_medical_painCoefficient` (1 → 0.6).

### H. Client-mod CBA settings (player preference, not mission config)
`fnf_pref_loadoutInterface` (`ACE` / `CBA`) and `fnf_pref_spectatorInterface` (single-option list, never
read) — `client_mod/fnf_setup/XEH_preInit.sqf:7-53`. Keybinds: `Ctrl+F12` hide UI, `Ctrl+Shift+N` loadout
fleximenu, `Ctrl+J` Mission Info panel, `Shift+J` admin panel, `]` spectator mute (dead — handler
commented out) (`XEH_postInit.sqf:12-146`).

---

## 11. Tooling

*(largely from the tooling agent; all in-game or desktop, none in CI)*

| Tool | Runtime | Input → output |
|---|---|---|
| `tools/export_kits.sqf` | SQF pasted into the in-game debug console | 30 hardcoded uniform+gear pairs → a serialized CBA hash, pasted by hand into `tools/portableScreenshotsSystem/kitHash.txt`. `:5` warns *"The game might freeze up for a bit while processing."* |
| `tools/screenshot_kits.sqf` | SQF, in-game single player | Adapted from `BIS_fnc_exportEditorPreviews` (`:5`). Spawns `B_Soldier_F` (`:148`), applies each role via `fnf_loadout_fnc_screenshotLoadout` (`:181`), frames the torso, and takes `<ROLE>_F.png` / `<ROLE>_R.png` (`:284,287-288`) into `[profile]\Screenshots\LoadoutPreviews\<UNI>--<GEAR>\` |
| `tools/postProcessingForScreenshotKits/enhance-images.ps1` | PowerShell + ImageMagick | `-brightness-contrast -10x20` (`:31`), `-crop 980x1338+804+102` (`:32`), `montage … -tile 2x` front+rear (`:46`), deletes the `_F`/`_R` originals (`:50-52`) |
| `tools/postProcessingForScreenshotKits/square-and-label-for-addon.ps1` | PowerShell + ImageMagick + Arma 3 Tools | Pads to 1560×1560 on `#323837`, burns the role name in Tahoma 120 pt (`:8`), then `Pal2PacE.exe` → `.paa` (`:12`), committed to `client_mod/fnf_media/images/kits/` |
| `tools/portableScreenshotsSystem/init.sqf` | SQF `init.sqf` of a throwaway mission | Loads `kitHash.txt` (484 KB) and `setUnitLoadout`s every `CAManBase` (`:56-63`) — lets media people dress units with **no framework loaded**. `:12` notes the hash is *"from v3.2.0 of primary framework"* |
| `client/misc/fn_lobbyTextGenerator.sqf` | SQF, pause-menu button, host only | Live mission state → clipboard blurb (§3 step 12) |
| `description/vehicleLoadouts/compileHtml.ps1` | PowerShell | `loadoutData.json` (64 vehicles) → `FNF_VehicleLoadouts.html` (not committed) |
| `server/debug/fn_config.sqf` + `fn_init.sqf` | SQF, gated on `fnf_debug` | Regenerates `loadoutData.json` by dumping every spawned vehicle |
| `client/misc/admin/menu/fn_adminUI.sqf` (17 KB) + 11 functions | In-game admin panel, `Shift+J` | Kick, ban, set Indfor allegiance, set loadout, reset anim state, respawn player, kill player, message player, copy player UIDs, adjust game clock, player-number tracker, end by elimination, Zeus/ACE options |
| `client/misc/contactStaff/fn_contactStaff.sqf` | In-game, pause menu | Player report → server → Discord @-mention of the logged-in admin |

### Discord webhooks *(tooling agent)*
Transport is SQF → **Pythia** → Python `requests` → Discord.
`server_mods/common_mod/python_files/__init__.py` defines three embed builders:
`roundStart` (green, per-side player counts), `roundEnd` (red, with hardcoded links to the OCAP2 AAR site
`http://aar.fridaynightfight.org/` and a Google Forms feedback link, `__init__.py:67`), and
`adminAction` (yellow, `"content": atID` so the on-duty staff member is pinged, `:162`).
Both round hooks are guarded by `if !(isDedicated) exitWith` and a **14-player floor**
(`server/webhook/fn_webhook_roundStart.sqf:1-2`). A fourth, separate path handles teamkills through the
`CAU_DiscordEmbedBuilder` mod (`server/init/fn_serverInit.sqf:183-246`), posting killer, victim, both
org-groups parsed from the `@` in `roleDescription`, vehicle, weapon, range in metres, elapsed time, UTC
and mission name.
Webhook URLs are **not in the repo** — `grabURL.py:2,5` returns the literal placeholder
`"ON SERVER THIS IS FILLED OUT WITH WEBHOOK URL"`.

### CI / validation / linting — **there is none**
`.github/` contains three issue templates and nothing else. A repo-wide search finds no
`.github/workflows`, no `Makefile`, no `*.yml`/`*.yaml`, no `*.sh`/`*.bat`, no HEMTT config, no SQF
linter config, no test harness, and no `.pbo`. Releases are cut by hand from tags.
The only correctness checking is **runtime and advisory** — the 3DEN-preview advisories and the
attack/defend-side guard listed in §3 step 11 — and **human vetting**, which the
`testing_required.md` issue template with labels `'dedi testing, local testing'` exists to track.

---

## 12. Conventions and house rules encoded in the framework

**Balance rules baked into the client mod** *(mods agent)*, therefore un-overridable by a maker:
- **Body armour is cosmetic.** All 94 patched vests get a byte-identical protection profile —
  `HitChest`/`HitDiaphragm`/`HitAbdomen` `armor = 15, passThrough = 0.1`; `HitNeck` `armor = 0,
  passThrough = 0.5`; `HitPelvis` `armor = 0, passThrough = 0.1`
  (`client_mod/fnf_armor/config.cpp:23-55`, verified by value counts: `armor=15` ×282 = 94×3, no
  outliers). A plate carrier and a chest rig stop the same round. Helmets are untouched.
  34 S.O.G. vests get the same chest/diaphragm/abdomen treatment (`client_mod/fnf_vn/CfgWeapons.hpp:6-31`).
- **Backpack capacity is flat** — `maximumLoad = 1000` for ~90 modern and ~75 WW2 packs
  (`fnf_backpacks/config.cpp:11-333`, `fnf_ww2/CfgVehicles.hpp:7-245`); **850** for the ~55 Vietnam packs
  (`fnf_vn/CfgVehicles.hpp:8-156`).
- **Hand grenades bypass ACE fragmentation.** `GrenadeHand` and `rhs_ammo_m67` get
  `ace_frag_enabled = 0; ace_frag_skip = 1; ace_frag_force = 0; hit = 13; indirectHit = 13;
  indirectHitRange = 7.5;` (`fnf_frags/config.cpp:16-33`) — a deliberate override of the server-wide
  `force force ace_frag_enabled = true`.
- **Coloured smoke is a signal, not concealment.** All six colours are overridden to a small grey puff
  (`size[] = {0.1,1,5}`) while white smoke is enlarged to `{0.2,3,9}`
  (`fnf_smoke/config.cpp:51-108`). Lifetimes: hand smoke 75 s, 40 mm smoke 45 s (`:12-13`).
- **The RPK cannot share AK magazines** — its `magazines[]` is replaced with a single dedicated,
  `scope = 1` 45-round mag (`fnf_rpk/CfgMagazines.hpp:3-11`, `CfgWeapons.hpp:3-8`).
- **WW2 iron-sight zoom is capped** at `opticsZoomMax = 1.25` (`fnf_ww2/CfgWeapons.hpp:3-8`).
- **The SCUD control panel is locked during safe start** — a mod config reading a mission variable
  (`fnf_vehicles/config.cpp:32`).

**What the framework prevents the player doing** (`client/restrictions/`, 15 scripts):
- **Global and side chat typing is off** unless you are staff (`fn_disableTyping.sqf:5-14`); channels 0,
  1 and 6 are disabled and re-enabled only for `fnf_staffInfo` UIDs.
- **Gamma is capped at 1.3 on night missions** by polling the video-settings dialog and blacking out the
  screen if exceeded (`fn_restrictGamma.sqf:7-29`) — the comment admits *"Not a perfect solution, but best
  that can be done via scripting"*.
- **View distance is force-clamped** to `fnf_maxViewDistance`, terrain grid to 25, and the commanding
  menu suppressed, every 0.5 s (`fn_viewDistance.sqf:6-19`).
- **Thermal imaging is stripped from every vehicle** — `disableTIEquipment true` on `Car`, `Tank`,
  `StaticWeapon`, `Ship_F`, `Air` (`description/cfgEventHandlers.hpp:12-30`) — and the NVG key is
  swallowed in the Vorona gunner seat (`fn_restrictWeaponThermal.sqf:5-10`).
- **Vehicle sensors are disabled** (`server/init/fn_serverInit.sqf:150-157`).
- **Shouting is leader-only during safe start** — everyone starts on whisper; if you are not in
  `["CC","EO","CSGT","PL","SGT","SL","TL","CRL"]` and not a group leader, TFAR is forced back down
  (`fn_restrictVoiceVolume.sqf:7-21`).
- **Team management is locked** until someone on your side is dead, and leaving your group is removed
  outright (`fn_restrictTeamManagement.sqf:8-23`).
- **AI radio chatter, subtitles and conversation are off** (`fn_disableMisc.sqf`).
- **Uniform swapping is restricted** after safe start (`fn_restrictUniform.sqf`).
- **Trenches cannot be dug within 30 m of an objective or 12 m of a road**
  (`fn_restrictETool.sqf:5,16-18`).
- **You cannot shoot or throw during safe start** — the weapon-reload time is set to 1 s continuously via
  a dummy action condition, with careful carve-outs so ACE placing, dragging, carrying, trench-digging and
  tripod-adjusting still work (`client/safety/fn_init.sqf:20-52`).
- **Late joiners after safe start cannot play** (`client/init/fn_canPlay.sqf`).
- **You cannot leave the play area** for more than 20 s (§9).
- **You cannot enter an enemy safe zone**, and your own gives invincibility during safe start (§9).
- **Zeus pinging is disabled** (`client/restrictions/fn_init.sqf:17`).
- **Friendly-fire rating penalties are neutralised** (`+100000` rating on every man,
  `description/cfgEventHandlers.hpp:10`; `HandleRating` returns 0, `client/init/fn_init.sqf:187`) — but
  every teamkill is reported to Discord instead (§11).

**What the framework prevents the *maker* doing:**
- **Round length and safe-start length are not maker-settable.** They live in
  `description/sekrit.sqf:2,6` — the filename is the statement. 50 minutes + 15 minutes safe start.
- **Sustained Assault requires prior authorisation** — `config.sqf:28` *"requires advance permission from
  Missions Team Lead"*.
- **You cannot delete the other modes' objective objects** — `config.sqf:43-44` *"DO NOT DELETE ANY OF THE
  OTHER TEMPLATE OBJECTIVE OBJECTS / they will be deleted automatically if not in use"*.
- **Objective counts are hard-capped at 3** (1–4 for neutralSector).
- **Object and marker names are fixed literals**, not maker-chosen (§7).
- **Fortification is defenders-only** (`server/init/fn_fortifyServer.sqf:79-88`).
- **The ORBAT is fixed** — you may delete groups but the shape, callsigns and radio plan are framework
  property.
- **Kit composition is fixed** — you pick a uniform set and a gear set; you cannot author a loadout in the
  mission.
- **Four CSW options are documented as broken and therefore forbidden**
  (`configGuide.txt:203-207`), and eight Vietnam uniform/gear entries are flagged `(DONT USE)`
  (`configGuide.txt:128-140`).

**Player-experience conventions:** a mandatory staggered load (randomised 10–25 s black screen so 60+
clients do not apply loadouts simultaneously, `client/init/fn_staggeredLoad.sqf:7-25`); a "new player"
welcome that names anyone with fewer than four recorded FNF sessions to their squad
(`client/init/fn_init.sqf:110-116`, `server/init/fn_newPlayers.sqf:17-34`); staff-only radio channel
created at boot (`server/init/fn_serverInit.sqf:249-255`); and an in-mission "Contact Connected Staff"
report path.

---

## 13. What this framework does better than anyone else

1. **The Eden layer is a first-class authoring primitive, and the framework programs against it.**
   Layer names like `"FNF Gamemode: Destroy"` are read at runtime with `getMissionLayerEntities`
   (`server/init/fn_setupGame.sqf:93`) and everything in an unselected mode's layer is deleted. This is
   how a *single* mission file can contain nine mutually exclusive game modes with all their objects
   pre-placed, and the maker's job reduces to "move the ones you're using". A web editor should copy the
   idea wholesale: layers that carry semantics, not just visibility.

2. **In-editor documentation placed next to the thing it documents.** 28 Eden `Comment` objects sit
   physically beside the objects they explain — "BLUFOR, delete if not using", "Place this in a flat area
   near the inner radius of the safe zone", and a seven-paragraph tutorial on polygon boundary markers
   (`mission.sqm:4093`). No other framework in this study puts its manual *inside the map*.

3. **The briefing is almost entirely derived, not written.** The maker writes four prose strings; the
   framework generates the ORBAT with live radio frequencies, a full per-side asset inventory down to
   individual turret magazine counts (`client/briefing/fn_assetDiary.sqf`, 23 KB), the complete kit
   breakdown with photographs, weather and astronomy, the mode rules, and the overtime conditions. Then it
   *spawns seven physical briefing tables per side* around the one desk the maker placed
   (`client/briefing/table/fn_setupTables.sqf:24,41-96`).

4. **Automatic map marking of placed geometry.** Any object over 1.5 m outside a safe zone is drawn on
   the 2D map as a correctly-oriented rectangle matching its real bounding box
   (`server/init/fn_markCustomObjs.sqf:59-74`), with a one-checkbox opt-out. Makers get accurate map
   representation of custom compositions for free.

5. **Reconnaissance photographs as a briefing artifact.** `fn_objectiveRecon.sqf` +
   `fn_objectivePreview.sqf` render a filtered, north-oriented, film-grained aerial photo of each
   objective, available only during safe start. This is intel design, not a debug camera.

6. **Two-axis loadout composition scales combinatorially.** 93 uniform sets × 63 gear sets, each with 25
   role subclasses built by inheritance and reset between files by a 228-line `#undef` header
   (`description/KITS/undef.hpp`). Changing a mission's entire visual and armament identity is two string
   edits in `config.sqf`.

7. **The lobby-text generator closes the authoring loop.** It reads the *actual placed vehicles* inside
   each safe zone, counts them, adds transports and the MAT choice, and copies a publishable one-liner to
   the clipboard (`client/misc/fn_lobbyTextGenerator.sqf:189-192`). The mission advertises itself from
   ground truth.

8. **Radio plan is derived from the ORBAT, and randomised per round.**
   `client/briefing/fn_setGroupIDs.sqf` carries a `[altChannel, mainChannel, [offsets]]` per group; base
   frequencies are re-rolled every round per side (`server/init/fn_genRadioFreqs.sqf:4-11`). Nobody hand-
   writes a signals annex.

9. **The framework tells vetters what is wrong, in preview, in plain English.** `is3DENPreview`-gated
   advisories for missing safe-zone flags, active mission specials, air pylon capture, and a loud
   attack/defend-side mismatch error.

10. **Failure is loud and safe.** If a loadout cannot be applied, the client is *ended out of the mission*
    after 30 seconds (`client/loadout/fn_checkLoadout.sqf:26-31`) rather than letting someone play naked;
    every one of the 17 loadout stages has its own named error toast.

---

## 14. Friction and known complaints

**Stale instructions in the maker's primary file.** `config.sqf:19-35` tells the maker to rename
`mission_normal.sqm` and `description_normal.ext`. Neither file exists — `changelog.txt:96` records them
being renamed to become the defaults in v3.3.1. The first thing a new mission maker reads is wrong.

**A live TODO admitting a shipped feature is broken and will not be fixed.**
`server/init/fn_airdropAssets.sqf:28`:
`//TODO disable this function (its bugged anyway and i dont want to fix it because me me no understand)`
— the function is still wired up and still runs.

**Documented-as-broken options.** `configGuide.txt:203-207` has a literal `BROKEN:` heading listing four
crew-served weapons in the maker-facing reference; `configGuide.txt:128-140` marks eight Vietnam
uniform/gear entries `(DONT USE)`.

**Config documentation that contradicts the code.**
- `mode_config/assassin.sqf:5` documents `_targets` as a flat 3-element array; the shipped data and
  `assassin_server.sqf:31,56-57` use a nested 2+2 shape.
- `mode_config/adSector.sqf:8` says a sector is captured with *"at least one attacker … and no conscious
  defenders"*; `adSector.sqf:42` implements a **strict majority**. The function that would implement the
  documented rule, `fnf_sector_fnc_sidePresent` (`:18-29`), is defined and never called.
- `mode_config/rush.sqf:7` shows a 2-element array example for `_terminalHackTime`, but the 3-terminal
  code path does `select 2` (`rush_server.sqf:39`) and will throw.

**Silent naming traps.**
- `assassin_server.sqf:97` looks up HVTs with `str _object`, which returns the Eden **Variable Name**
  field. A maker who instead writes `init="HVT_1 = this;"` gets `objNull`, `!alive objNull` is true, and
  every HVT is auto-killed the moment safe start ends.
- `destroy_server.sqf:19,27-28` rebuilds the marker name as `(str _x) + "_mark"`, ignoring the marker name
  the maker configured; `client/misc/fn_objectivePreview.sqf:398-399` hardcodes it harder.
- `fn_getUniformPics.sqf:21` swallows every missing kit image with `fileExists` — the briefing simply
  shows nothing, with no warning.

**Stale generated assets, quantified** *(tooling agent)*. `configGuide.txt` documents **63** uniform+gear
pairs; only **24** have kit preview images. **39 documented pairs have no previews at all** (all 1980s
Polish/Romanian/Czech, all Vietnam, all Grozovian, several RU and NATO sets), and **5 image folders are
orphaned** because the pairing they encode is no longer offered. `tools/export_kits.sqf:18` and
`screenshot_kits.sqf:41` still reference `RHS_UNI_NATO_NL_DUTCH_ARMY_2010`, which no longer exists.
Four of the 29 roles (`CC`, `EO`, `AB`, `SHQAUX`) have never been screenshotted at all.

**The tooling only runs on one person's machine.**
`tools/postProcessingForScreenshotKits/square-and-label-for-addon.ps1:12` hardcodes
`K:\SteamLibrary\steamapps\common\Arma 3 Tools\TexView2\Pal2PacE.exe`. Both PowerShell scripts assume
ImageMagick on `PATH` and contain `Start-Sleep` calls that look like race workarounds
(`enhance-images.ps1:35,49`). `tools/screenshot_kits.sqf:11` says of itself: *"This code is currently WIP
to achieve parity with the latest framework code."* `tools/portableScreenshotsSystem/init.sqf:12` notes
its data is *"from v3.2.0 of primary framework"* — four minor versions behind the shipped tag.

**No automation of any kind.** No CI, no linter, no schema validation, no packaging script, no tests.
Correctness rests on human vetting, which is itself formalised only as an issue template
(`testing_required.md`, labels `'dedi testing, local testing'`). The recurring failure mode that template
exists to catch — *works in editor preview, breaks on the dedicated server* — is visible throughout the
changelog: JIP fixes (`:41,130,133`), desync at safety end (`:110`), locality (`:184`).

**Three hand-maintained copies of a 516-line settings file** with no shared include, already divergent in
seven values *(mods agent)*.

**Configuration silently not applying** is a recurring changelog theme: `:40` *"Fixed sw radio config
override not working"*, `:48` *"Fixed an issue that overwrote performance settings unintentionally"*,
`:244-245` on the optics selector failing to initialise when the assigned primary supported no available
optic.

**Breaking changes for makers, never labelled as such.** `changelog.txt:96` (entry points renamed),
`:232-234` *"Updated Mission SQM — migrated everything into layers"* (requires re-basing custom SQMs),
`:86` (locked vehicles dropped from asset lists), and `:54` — an entire ORBAT rework documented **only as
an imgur link**, with no in-repo diagram.

**Changelog discipline lapsed.** `version.txt` says 3.6.9 but the changelog's newest entry is 3.6.8
(`changelog.txt:1`); the last three entries are variations on *"1. Misc bug fixes"*.

**Dead and non-functional code left in place.** `fn_checkAlive.sqf:79-93` — the automatic elimination win
is commented out. `client_mod/fnf_setup/XEH_postInit.sqf:142-146` — a bound key with an entirely
commented-out handler. `ctf_server.sqf:9` publicVariables `fnf_ctf_allowAirVehicleCarry`, which is never
assigned anywhere. `assassin_server.sqf:73-76` references an unbound `_name`, so the "custom dogtag name"
feature does not work. `server/webhook/fn_webhook_roundStart.sqf:4-117` computes a full per-side asset
inventory that is then never sent. `cfgFNFORBAT.hpp:29-44` still carries the removed X-Ray recon team,
commented out.

**Two `CfgPatches` name collisions** *(mods agent)*: `client_mod/fnf_logos/config.cpp:3` declares
`class fnf_ammo` — the same addon name as `client_mod/fnf_ammo/config.cpp:3`; and `client_mod/fnf_frags/`
packs an addon named `fnf_grenades`.

**No issue tracker evidence in-repo beyond templates** — the three templates are the only process
artifact. Player-facing rules live entirely outside the repo, in Google Docs
(`description/RulesAndPolicies.txt` is four lines pointing at one, plus README badges for a
"FNF Mission Making Guide" doc). The most authoritative authoring documentation for this framework was
**not in the source at all**.


