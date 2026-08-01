# FNF (Friday Night Fights) Mission Framework — v4.7.0 ("revamped" era)

Forensic read of `/run/media/system/Disk_2/tbd-framework-analysis/fnf/FNF-v4.7.0/` (git worktree pinned to tag `v4.7.0`, mission-template version stamp `4.7.0` per `FNF_Mission_Template.VR/Version.txt:1`), plus an explicit diff against `v3.6.9`.

All paths below are relative to the v4.7.0 worktree root unless prefixed otherwise. Nothing on Disk_2 was modified; all git commands were read-only queries against `FNF-full/`.

---

## Source inventory

| Path (relative to `FNF-v4.7.0/`) | Read | What it gave me |
|---|---|---|
| `README.md` | full (71 lines) | Feature list, external doc links, "module based mission systems" framing |
| `FNF_Mission_Template.VR/Version.txt` | full | `4.7.0`; loaded at runtime by the new-player system |
| `FNF_Mission_Template.VR/mission.sqm` | **binarized (`\0raP`)** — read as extracted strings only | Starter-template contents: 6× `ace_spectator_virtual`, 1× `fnf_module_init` with 9 saved attributes, 1× `ModuleCurator_F`, 2 reminder Comments, layer "Standard Objects" |
| `FNF_Mission_Template.VR/Description.ext` | full (18 lines) | Respawn constants, include list, `onPauseScript[]` |
| `FNF_Mission_Template.VR/Description/Endings.hpp` | full (59) | 8 `CfgDebriefing` endings |
| `FNF_Mission_Template.VR/Description/FortifyCategories.hpp` | full (24) | 6 ACEX fortify preset categories |
| `FNF_Mission_Template.VR/Description/Params.hpp` | full (18) | The single lobby param (`PerformanceTweaks`) |
| `FNF_Mission_Template.VR/Description/UiDescriptions.hpp` | full (58) | HUD timer RscTitle, `RscWebBrowser` |
| `FNF_Mission_Template.VR/Description/Functions.hpp` | structural grep (351) | 143 registered functions, category names |
| `FNF_Mission_Template.VR/init3DEN.sqf` | full (319) | The export pipeline: JIP sync-transfer, group-ID generation from role descriptions, marker alpha toggling, vehicle-inventory clearing, paste-increment, empty-layer GC |
| `FNF_Mission_Template.VR/Client/fn_init.sqf` | full (189) | Client boot order; which modules are mandatory vs optional |
| `FNF_Mission_Template.VR/Server/fn_init.sqf` | full (50) | Server boot order |
| `Client/Modules/fn_findFNFModules.sqf`, `fn_findSpecificModules.sqf` | full | Module discovery by `typeOf` substring / regex — no registry |
| `Client/Breifing/fn_initBreifing.sqf` | full (673) | Whole briefing/diary construction; all hardcoded rules text; kit + asset tabs |
| `Client/Breifing/fn_initOrbat.sqf`, `fn_updateOrbat.sqf` | full | Live ORBAT tab built from `roleDescription` split on `@` + TFAR freqs |
| `Client/Radio/fn_initRadios.sqf` | full (128) | Per-side TFAR encryption codes, ear assignment, freq publishing |
| `Client/Zones/fn_addZone.sqf`, `fn_verifyZone.sqf`, `fn_initZones.sqf` | full | Polygon zones from numbered markers; restriction-group evaluation loop |
| `Client/Objectives/fn_initObjs.sqf` | full (173) | Objective dispatch, side resolution from synced Side logics, allied-task logic |
| `Client/Objectives/Destroy/fn_initDestroy.sqf` | full (331) | Canonical objective shape: state machine, task text generation, hidden attributes |
| `Client/Objectives/CaptureSector/fn_initCaptureSector.sqf` | grep | Marker-prefix → zone reuse |
| `Client/Objectives/Steal/fn_initSteal.sqf` | partial | `...Target` custom-text variants |
| `Client/SequentialHandeler/fn_initSequentialHandler.sqf` | full (127) | Prerequisite gating loop |
| `Client/CodeUtilities/fn_sortByLocation.sqf` | full | Deterministic position-based ordering |
| `Client/CodeUtilities/fn_getDisplayObjNumber.sqf` | full | Objective numbering by shared target/prefix — proves the paired-module convention |
| `Client/Safezones/fn_initSafeZones.sqf` | full (194) | Safe-start model, restriction groups, 5-min warning |
| `Client/Death/fn_initDeath.sqf` | full (306) | All three death modes, reinsert flare mechanic |
| `Client/Selectors/fn_initSelectors.sqf` | full (98) | ACE self-interaction selector tree |
| `Client/Restrictions/fn_initPlayZones.sqf` | full (127) | Play-zone inversion + shading limits |
| `Client/Restrictions/fn_restrictPlayer.sqf` | full | Forced channel/view-distance/terrain-grid restrictions |
| `Client/Restrictions/fn_initAssetRestrictions.sqf` | partial (60) | 3 restriction levels, sync payload parsing |
| `Client/HidingZones/fn_initHidingZones.sqf` | full | Task-destination masking |
| `Client/TeleportPoles/fn_initTeleportPoles.sqf` | partial (60) | Sync payload shape |
| `Client/UI/fn_markEditorPlacedObjects.sqf` | full (65) | Auto map footprints from bounding boxes |
| `Client/UI/Spectator/fn_missionReviewScreen.sqf` | partial (115) | In-game review form: 2 sliders + free text |
| `Client/NewPlayerExperience/fn_initNewPlayerExperience.sqf` | full | Version-stamp onboarding gate |
| `Client/Fortify/fn_initFortify.sqf` | partial (60) | Fortify gating on ACE_Fortify item |
| `Client/Vehicles/fn_initVicRearmReplacement.sqf` | partial (50) | ACE rearm override |
| `Server/fn_initFortify.sqf` | full (71) | The two fortify object tables with point costs |
| `Server/fn_endGame.sqf` | partial + grep (128) | Ending selection, Pythia review upload |
| `Server/Handles/fn_handleVicInvincibility.sqf` | grep | `fnf_timeToBeVincible`, custom vehicle loadout capture |
| `client_mod/fnf_eden/config.cpp` | full (26) | CfgPatches `units[]`, category registration, include order |
| `client_mod/fnf_eden/modules.hpp` | full (879) | **Every module attribute** — the primary config surface |
| `client_mod/fnf_eden/systems.hpp` | full (437) | Editor categories/subcategories + all system & objective composition presets |
| `client_mod/fnf_eden/loadouts.hpp` | head/tail + greps (~775) | 85 kit composition registrations; commented-out VN/WW2 blocks |
| `client_mod/fnf_eden/attributesAndTools.hpp` | full (89) | Per-object "FNF Properties" attributes + the two Eden menu tools |
| `client_mod/fnf_eden/functions/fn_generateLobbyDescription.sqf` | full (245) | Lobby-string generator; proves `/2` objective pairing |
| `client_mod/fnf_eden/functions/fn_spawnCustomSidedKit.sqf` | full (334) | Kit re-siding transform keyed on `Squad_index_Role` |
| `client_mod/fnf_eden/kits/USArmy[2020]/header.sqe` | full | Composition metadata + 54 `requiredAddons[]` |
| `client_mod/fnf_eden/kits/USArmy[2020]/composition.sqe` | targeted (14 753 lines) | 3-layer kit anatomy, 77 playable slots, `Role@Group` descriptions, 17 sync links |
| `client_mod/fnf_mapColors/config.cpp` | full | 15 added `CfgMarkerColors` |
| `client_mod/fnf_ace/config.cpp` | full (23) | Disables ACE "Leave Group" |
| `client_mod/` (all 17 addon dirs) | file census | Which addons exist and their size |
| `server_mod/cba_settings_userconfig/cba_settings.sqf` | structural + greps (643) | 518 `force force` settings across 30 mod sections |
| `server_mod/fnf_server_vars/*` | full | Staff UID→(name, Discord ID) table, preInit hook |
| `server_mod/python_files/__init__.py`, `requirements.txt` | partial | Pythia → Google Sheets review/timing upload |
| `External Scripts/ORBATChange.sqf` | full (~200) | The canonical ORBAT shape + TFAR frequency plan, as executable code |
| `External Scripts/DoFunctionsToLoadouts.sqf` | full (~95) | Bulk composition re-export loop |
| `External Scripts/ExportLoadoutFromOldFramework.sqf` | full (~35) | **v3→v4 loadout migration** |
| `External Scripts/fnf_transform.py` | structural (1 604 lines, 58 KB) | ORBAT schema migration across all kit compositions |
| `External Scripts/Generate{Backpack,Hat,Vest}Mod.sqf` | head (20–30 lines each) | Armour-normalisation codegen: enumerate `CfgWeapons` by `_generalMacro`, walk `HitpointsProtectionInfo` hierarchy |
| `Kit Mission Files/*.VR/mission.sqm` (×3) | **binarized** — strings only | Kit-authoring workspaces; 3 / 13 / 13 kits respectively |
| `.github/ISSUE_TEMPLATE/*.md` | full | Process conventions (break-fix / feature / testing-required) |
| `.editorconfig`, `.gitattributes` | full | Tabs, CRLF, LF normalisation |
| `FNF-full/` git history | `log`, `diff --stat`, `show`, `ls-tree` | The v3→v4 delta evidence |
| `FNF-v3.6.9/` | via `git ls-tree`/`git show` on `FNF-full` + sibling agent report | v3 structure for the delta section |

**Explicitly unreadable:** `FNF_Mission_Template.VR/mission.sqm` and all three `Kit Mission Files/*/mission.sqm` are binarized Arma `raP` config blobs. I extracted printable strings from them and say so at every point of use; I did **not** infer their non-string structure. `server_mod/python_files/__pycache__/config.cpython-310.pyc` and `$PYTHIA$` are compiled/marker files and were not decompiled.

---

## 1. Identity

**FNF (Friday Night Fights)** is a long-running Arma 3 public-PvP milsim community. The repo is the community's *whole platform*, not just a mission template: it ships (a) a required client mod pack, (b) a server mod, and (c) a mission template that mission makers copy.

- Upstream: `https://github.com/FridayNightFight/FNF` (`README.md:4`).
- Licence file present at `LICENSE`.
- Primary author of nearly every mission-template function header is `Mallen` (e.g. `FNF_Mission_Template.VR/Client/fn_init.sqf` and 100+ siblings carry `Author: Mallen`); `Client/Death/fn_initDeath.sqf:2` credits `OrthyOliver`.
- The authoritative *how-to* is **not in the repo** — `README.md:49` points mission makers at an external Google Doc, "FNF Mission Making Guide". This matters: the repo has no in-tree tutorial, no `configGuide.txt` equivalent (v3 had one), and no per-module docs beyond Eden tooltips.
- Hard dependencies (`client_mod/fnf_eden/config.cpp:8`): `A3_Modules_F`, `cba_main`, `ace_arsenal`. In practice missions also require TFAR (`Client/Radio/fn_initRadios.sqf` calls `TFAR_fnc_*` unguarded), ACE, ACEX Fortify, GRAD Trenches, DUI, EMR, OCAP — all forced in `server_mod/cba_settings_userconfig/cba_settings.sqf`.
- Genre: **large-scale, one-shot, attacker/defender PvP** — 60–90 player, single-life-ish, 65-minute rounds with a 15-minute safe start.

The v4 identity in one line: *a module-and-sync mission compiler layered on Eden, with a 85-kit pre-built loadout library and an opinionated house ruleset baked into the briefing.*

---

## 2. Mission file layout

A v4 mission is an Arma **mission folder** (`.VR` in the template; renamed to `.<worldname>` in practice). The template is `FNF_Mission_Template.VR/`, 155 files:

```
FNF_Mission_Template.VR/
├── mission.sqm            # binarized; the ONLY file a mission maker edits (via Eden)
├── Description.ext        # 18 lines; includes 5 hpp files
├── Version.txt            # "4.7.0" — framework version stamp, read at runtime
├── init3DEN.sqf           # 319 lines; the export/save pipeline (see §3)
├── Description/
│   ├── Functions.hpp      # 143 function registrations
│   ├── Endings.hpp        # 8 CfgDebriefing endings
│   ├── UiDescriptions.hpp # HUD timer + RscWebBrowser
│   ├── FortifyCategories.hpp
│   └── Params.hpp         # 1 lobby parameter
├── Client/                # 116 files, ~20 subsystems
└── Server/                # 30 files
```

Key structural facts:

- **`Description.ext` is 18 lines and contains no mission-specific content.** It sets `respawn = 3; respawnDialog = 0; respawnButton = 0; respawndelay = 99999; respawnOnStart = -1; respawnTemplates[] = {};` (`Description.ext:1-6`), includes the five `Description/*.hpp` files, and registers two pause-menu scripts. A mission maker never edits it.
- **`Params.hpp` exposes exactly one lobby parameter**: `PerformanceTweaks` with values `{0,1,2,3}` / texts `{None, Low, Medium, High}`, default `0` (`Description/Params.hpp:3-17`).
- The 155 template files are **framework code the mission maker copies verbatim and does not touch.** The authored content lives entirely in `mission.sqm`.
- Two of the three "interesting" v4-only top-level directories are *not part of a mission at all*:
  - `Kit Mission Files/` — three standalone Eden missions used as **kit-authoring workspaces** (`FNF_Kits_Base.VR` 357 KB, `FNF_Kits_VN_Set_1.VR` 3.76 MB, `FNF_Kits_WW2_Set_1.VR` 3.67 MB). All three `mission.sqm` are binarized; extracted strings show `FNF_Kits_Base.VR` holds 3 `fnf_module_kitInformation` / 14 `fnf_module_selectorHost` / 77 `fnf_module_selectorOption` / 21 layers, and the VN and WW2 files hold 13 kits each (13 `fnf_kitName` occurrences, with names such as `ARVN [1970]`, `US SOG [1970]`, `German Fallschimjager [1945]`).
  - `External Scripts/` — 7 files run from the **3DEN debug console**, not from a mission (see §11).
- The mission **template's own `mission.sqm` is nearly empty by design**. Extracted strings show only: `sourceName FNF_Mission_Template`, addons `ace_spectator / fnf_eden / A3_Modules_F_Curator / Desert`, six `ace_spectator_virtual` logic entities, one `fnf_module_init` carrying 9 persisted attributes, one `ModuleCurator_F` with `Owner = "#adminLogged"`, a layer named `Standard Objects`, and two `Comment` entities whose text is:
  - `REMINDER! / Make sure to fill out the required fields in the options: Attributes > Multiplayer / Attributes > General`
  - `REMINDER! / Dont forget to disable Debug in the FNF Init module before exporting`

  Those two comments are, effectively, the framework's entire in-repo onboarding.

---

## 3. Authoring workflow

This is the ordered, concrete path from blank page to playable mission. Everything below is evidenced from code; the framework's own narrative documentation is off-repo (`README.md:49`).

### Step 0 — Prerequisites
Subscribe to the required-mods Steam collection (`README.md:17-19`), install `client_mod/` (which includes `fnf_eden`, the mod that supplies every module, kit and preset). Without `@fnf_eden` loaded, none of the authoring surface exists.

### Step 1 — Copy the template
Copy `FNF_Mission_Template.VR` into `…/Arma 3/mpmissions/` (or the Eden user folder) and rename the `.VR` suffix to the target world. You inherit `init3DEN.sqf`, `Description.ext`, `Description/`, `Client/`, `Server/` and `Version.txt` unchanged. `Version.txt` is what pins the mission to a framework release — `Client/NewPlayerExperience/fn_initNewPlayerExperience.sqf:14` does `loadFile "Version.txt"` at runtime.

### Step 2 — Open in Eden; the template pre-seeds four things
On open you get: the `FNF Init` module, a Zeus curator module bound to `#adminLogged`, six ACE spectator slots, and the two reminder comments (all from `mission.sqm` strings, §2).

### Step 3 — Fill the two vanilla attribute panels the comment tells you to
`Attributes > Multiplayer` (respawn templates, lobby overview) and `Attributes > General` (author, `onLoadMission`, `briefingName`, `overviewText`). The shipped placeholders are literally `author = "Your Name"`, `onLoadMission = "Text to show in the loading screen"`, `briefingName = "Mission Title"`, `overviewText = "Fill this out please"` (extracted `mission.sqm` strings).

### Step 4 — Configure the `FNF Init` module
13 attributes (full enumeration in §10.1). At minimum: `Time Limit` (default 65 min), `View Distance` (1000 m), `Fortify Points` (100), the four briefing free-text fields, and `Debug` (which the reminder comment tells you to turn **off** before export).

### Step 5 — Drop kits (this is the ORBAT step)
From the Eden asset browser, category **"FNF - Kits"**, subcategory **Blufor / Opfor / Independant** (`client_mod/fnf_eden/systems.hpp:3-38`), place one composition per side. 85 kit compositions are registered (`loadouts.hpp`, 85 × `editorCategory = "fnf_Kits"`; 32 Blufor, 18 Opfor, 15 Indfor active, plus 10 Vietnam and 10 WW2 blocks that are **commented out** at `loadouts.hpp:577-585`, `591-680`, `686-775`).

Each kit composition drops a complete, three-layer bundle (`client_mod/fnf_eden/kits/USArmy[2020]/composition.sqe`):
| Layer | Contents |
|---|---|
| `Selectors` (9 items) | 4 × `Land_WoodenBox_F` carrying `ammoBox` inventory strings, 1 × `fnf_module_selectorHost`, 4 × `fnf_module_selectorOption` |
| `Units` (16 groups) | 77 playable units with full `class Inventory` blocks and `description="Role@Group"` |
| `Info` (3 items) | 1 × `fnf_module_kitInformation` (name, author, bare-bones loadout), 1 × `SideBLUFOR_F` logic, 1 × `Comment` titled with the kit name |

and 17 pre-built `Sync` links (`composition.sqe:14570-14753`) wiring the selector boxes → selector options → selector host, and the kit-info module → the side logic.

If you need a faction that only exists on another side, use **Tools ▸ FNF Mission Maker Tools ▸ Spawn Custom Sided Kit** (`attributesAndTools.hpp:78-84`). It presents a searchable, side-grouped list of all `fnf_Kits` compositions, then re-sides the chosen kit: it harvests every unit loadout keyed by `"<Squad>_<indexInGroup>_<RoleDescription>"` (`fn_spawnCustomSidedKit.sqf:219-231`), deletes the source, spawns the reference kit for the target side (`fnf_USMarinesWoodland2020` / `fnf_RussianRatnikWoodland2020` / `fnf_AAF2020`, lines 242-255), re-applies loadouts by that key, patches the UAV terminal/backpack classnames per side (lines 260-273), and copies the kit-info attributes across.

### Step 6 — Trim the ORBAT to the mission's size
Kits arrive at full strength (77 playable slots). You delete the groups you don't want. The **role-description string is the ORBAT** — see §4. `External Scripts/ORBATChange.sqf` is the debug-console script that rebuilds a canonical ORBAT from selected groups if you want to reshape rather than trim.

### Step 7 — Place the terrain content and mark it
Place buildings/props normally. `Client/UI/fn_markEditorPlacedObjects.sqf` will automatically draw a black rectangle marker on the map for every `Static` or `Cargo_base_F` descendant with bounding-sphere > 1.5 m, sized and rotated to the object's real bounding box (lines 14, 25, 42-55). To suppress one object, tick **FNF Properties ▸ Exclude from Map Auto-Mark** (`attributesAndTools.hpp:12-22`).

### Step 8 — Draw zones as numbered marker polygons
Every zone in FNF is a **polygon defined by ordered, numbered markers sharing a prefix**. `Client/Zones/fn_addZone.sqf:23-33` walks `<prefix>1`, `<prefix>2`, … until a marker name is free, collects positions, and builds a `POLYLINE` marker plus a shaded triangulation. Minimum 3 markers (`fn_addZone.sqf:39-46`, otherwise `WARNING: Zone with prefix '…' has less than 3 markers, zone has not generated`). The **marker's colour becomes the zone colour** (`fn_addZone.sqf:21, 71`) — hence `client_mod/fnf_mapColors/config.cpp`, which adds 15 extra `CfgMarkerColors` (Neon Cyan, Burnt Orange, Forest Green, Deep Violet, Teal, Tan/Sand, Infrared Red, Gold, Crimson, Turquoise, Magenta, Olive Drab, Lavender, Grey, Brown, Orange, Khaki, Pink).

Default prefixes: play zone `fnf_marker_playzone_`, safe zone `fnf_marker_safezone_`, hiding zone `fnf_marker_hidingzone_`, sector `fnf_marker_sector_`, steal/escort drop-off `fnf_marker_steal_` (`modules.hpp:176, 196, 281, 641, 712, 823, 865`).

Two authoring affordances make this bearable:
- **Paste auto-increment**: `init3DEN.sqf:278-288` hooks `OnPaste` and rewrites the trailing `_N` of every pasted marker's text.
- **Invisible-in-game markers**: you author markers at `alpha = 0.99` so you can see them in Eden; `init3DEN.sqf:226-235` flips them to `alpha = 0` during export and back to `0.99` afterwards.

You can also drop pre-built marker rings from **"FNF - Systems"** → `Play Zone Preset [1..3]`, `Hiding Zone Preset [1..3]`, `(Blufor|Opfor|Indfor) Safe Zone Preset [1..3]` (`systems.hpp:95-237`).

### Step 9 — Place system modules and **synchronise** them
This is the heart of v4. **A module is configured by its Eden attributes *plus* what you sync to it.** The sync payload is polymorphic and every consumer parses it the same way:

| Synced entity | Meaning |
|---|---|
| `SideBLUFOR_F` / `SideOPFOR_F` / `SideResistance_F` logic | assigns the module to a side |
| a playable unit | assigns the module to that specific slot |
| a plain object / vehicle | the module's payload (objective target, briefing asset, teleport pole, restricted vehicle) |
| another FNF module | composition (hiding zone → objective, selector option → selector host, sequential planner → objective) |

Evidence: `Client/Objectives/fn_initObjs.sqf:47-83` (side resolution with `DANGER: Objective has no valid side synced to it` / `…more than one side…` guards), `Client/Safezones/fn_initSafeZones.sqf:44-94`, `Client/Restrictions/fn_initAssetRestrictions.sqf:~20-60`, `Client/Breifing/fn_initBreifing.sqf:26-57`, `Client/Selectors/fn_initSelectors.sqf:26-48`, `Client/Objectives/Destroy/fn_initDestroy.sqf:148-176`.

Typical placements: `FNF Play Zone`, `FNF Safe Zone` (one per side), `FNF Teleport Poles`, `FNF Breifing Assets` (one per side, sync the side logic + every vehicle you want documented), `FNF Misc options`, and optionally `FNF Asset Restrictions`, `FNF Mobile Spawn Point Handeler`, `FNF Personal Rearm`, `FNF Respawn Position`.

### Step 10 — Place objectives **in pairs**
Objective compositions live under **"FNF - Objectives"** (`systems.hpp:301-436`): `Destroy Objective Preset`, `Terminal Objective Preset`, `Assassin Objective Preset`, `Capture Sector Objective Preset [1..3]`, `Hold Sector Objective Preset [1..3]`, `Steal Objective Preset [1..3]`, `Escort Objective Preset [1..3]`.

The house convention is **two modules per logical objective** — an attacker-flavoured one and a defender-flavoured one, both synced to the *same* target object or the *same* marker prefix:
- Each objective type's `Objective Type` combo has an attack value and a defend value: Destroy `des`/`pro`, Capture Sector `cap`/`def`, Terminal `hck`/`def`, Assassin `elm`/`pro`, Steal `stl`/`kep`, Escort `esc`/`des` (`modules.hpp:597-598, 630-631, 672-673, 770-771, 812-813, 854-855`).
- `client_mod/fnf_eden/functions/fn_generateLobbyDescription.sqf:80-101` divides every objective count by 2 when producing the lobby string — e.g. `"Destroy(" + str(_destroyCount / 2) + "), "`.
- `Client/CodeUtilities/fn_getDisplayObjNumber.sqf:44-88` assigns the *same* display number to all modules that share a target object (or share a marker prefix), after sorting by location — so attacker and defender see "Objective 1" for the same thing.

Objective ordering is deterministic and **positional**: `Client/Objectives/fn_initObjs.sqf:18` sorts modules through `fn_sortByLocation.sqf`, which zero-pads X/Y/Z to 6 integer digits and concatenates them into a sortable string. *Move an objective module on the map and its number changes.*

### Step 11 — Sequence objectives (optional)
Place `FNF Sequential Objective Planner`, sync it to the prerequisite objective(s) and to the objective(s) it unlocks. Its one attribute, `Is next Objective shown` (default `true`, `modules.hpp:567-575`), decides whether locked objectives are visible in advance. `Client/SequentialHandeler/fn_initSequentialHandler.sqf:19-36` polls every second and initialises the downstream objectives once all prerequisites reach state ≥ 4 (completed), then fires a "New Objective(s) Available" notification.

### Step 12 — Add loadout selectors (optional)
Place `FNF Selector` (host), set `Selector Name` and `Selector Type` (`itm`/`opt`/`pri`/`sec`/`hnd`), sync it to the units it applies to and to N `FNF Selector Option` modules; sync each option to the physical container object holding that option's gear. One option can be flagged `Default?`. Non-chosen containers are `hideObject`-ed at runtime (`Client/Selectors/fn_initSelectors.sqf:94-97`).

### Step 13 — Generate the lobby description
**Tools ▸ FNF Mission Maker Tools ▸ Generate Lobby Description** (`attributesAndTools.hpp:71-77`). It scans placed objective modules and briefing-asset modules and writes a standardised string into the `IntelOverviewText` mission attribute *and* the clipboard (`fn_generateLobbyDescription.sqf:243-244`). Output shape:

`Destroy(1), Sector(2) // ATK: BLU X% adv - DEF: OPF // BLU: 2xM1151, 1xUH-60M - MAT: FGM-148 // OPF: No Vics - MAT: 9K115-2`

Attacker/defender are inferred by majority vote of attack-flavoured vs defend-flavoured objectives per side (lines 109-121); the `X%` balance figure is a **literal placeholder the mission maker must fill in by hand**.

### Step 14 — Export
Save, then **Export to Multiplayer**. `init3DEN.sqf` intercepts message id 6 and, inside a single `collect3DENHistory` undo block (lines 129-274):
1. Derives group IDs from leader role descriptions — splits on `@`, sets `groupID` to the part after `@`, and appends a side-disambiguating suffix of `""` / `" "` / `"  "` for west/east/independent (lines 153-178). It emits `WARNING: Group <id> has a duplicate group name` and `WARNING: Group <id> leader does not have its role description set properly` — **the framework's only structural validation**.
2. For every MP-controllable unit, creates a dummy `Logic` named `fnf_handleJIPLogic_<n>`, moves the unit's `Sync` connections onto it, and writes the unit's init field to `[fnf_handleJIPLogic_<n>, this] call FNF_ClientSide_fnc_requestJIPObjects;` (lines 184-224) — this exists because Eden sync links are **not replicated to JIP players**.
3. Sets markers at alpha 0.99 to alpha 0.
4. Clears vehicle inventories where `FNF_InventoryAutoClear` is set (lines 239-250).
5. Runs `MissionExportMP`, then **reverses all of the above** and re-saves so the editor state is untouched (lines 252-271).

On next open, `init3DEN.sqf:36-114` detects a previously-exported mission (more Logics than units) and unwinds any leftover JIP scaffolding automatically.

### Step 15 — Playtest and iterate
`FNF Init ▸ Debug` gates every `systemChat` warning (`Client/fn_init.sqf:15`, then ~30 guarded sites). Turn it on to see `DANGER:`/`WARNING:` diagnostics; the reminder comment says turn it off before shipping.

### Step 16 — Post-mission feedback
After the round, players open the pause menu ▸ **Mission Review** and submit two 0-10 sliders ("What would you rate this mission out of 10?", "What would you rate the commanding out of 10?") plus free-text notes (`Client/UI/Spectator/fn_missionReviewScreen.sqf:41, 53, 65`). Reviews are stored server-side in `fnf_missionsReviews` and pushed to a Google Sheet named `Mission Review Submissions` at mission end via Pythia (`Server/fn_endGame.sqf:123` → `server_mod/python_files/__init__.py`). **The mission maker gets a scored, written review of every mission they ship.**

---

## 4. Slotting / ORBAT model

There is **no ORBAT data structure**. The ORBAT is encoded in a single vanilla Arma field per unit — `description` (the Eden "Role Description") — using an `@` separator:

```
description="Platoon Command@Command HQ"     ← group leader: "<Role>@<GroupName>"
description="Executive Officer"              ← non-leader: "<Role>" only
```
(`client_mod/fnf_eden/kits/USArmy[2020]/composition.sqe:369` and the 76 units following it.)

Three consumers parse it:
1. **Export** — `init3DEN.sqf:159-177` splits the *leader's* description on `@` and writes the second half to the group's `groupID`, appending `""`/`" "`/`"  "` by side. This is what makes the slot list show `Alpha`, `Bravo`, … rather than `Alpha 1-1`.
2. **Runtime ORBAT tab** — `Client/Breifing/fn_updateOrbat.sqf:77, 91` takes `(roleDescription _unit splitString "@") select 0` as the displayed role.
3. **Kit re-siding** — `fn_spawnCustomSidedKit.sqf:219-226` composes the stable unit key `"<squad>_<indexInGroup>_<roleDescription>"`.

### The canonical v4.7.0 ORBAT
Derived from `USArmy[2020]/composition.sqe` (77 playable units, 16 groups, 15 named elements):

| Element | Composition |
|---|---|
| `Command HQ` | Platoon Command (LIEUTENANT), Executive Officer, Medic |
| `Alpha`, `Bravo`, `Charlie`, `Delta`, `Echo` | Squad Leader, Breacher **or** Scout, Rifleman (LAT) **or** Machine Gunner, Combat Engineer, Team Leader (GL), Automatic Rifleman, Marksman, Rifleman (LAT), Medic — 9 each |
| `Mike` | Squad Leader, Medium Anti-Tank, 2× Assistant Medium Anti-Tank |
| `Golf 1`, `Golf 2` | Vehicle Crew Lead + 2× Vehicle Crew |
| `Hotel 1`, `Hotel 2` | Pilot ×3 |
| `Xray` | Squad Leader, Sniper, Spotter |
| `Lima` | Squad Leader + 3× Sapper |
| `Sierra` | Squad Leader + 2× Systems Specialist (UAV) |
| `India` | Squad leader, Mortar Gunner, Assistant Gunner |

Note the typo `"Squad leader@India"` (lower-case *l*) in the shipped kit — role descriptions are free text with no validation.

### Radio plan is derived from the ORBAT
`External Scripts/ORBATChange.sqf` writes TFAR frequencies as it builds the ORBAT, which documents the convention as code:
- Infantry squads: SR starts at **40** and increments by **10** per squad; squad members get `"<sr>"`, squad leaders get `"<sr>,30"` (dual-channel), and `TFAR_freq_lr = "30"`. The 4th member of each squad also gets the `,30` dual channel.
- Vehicle crews (`Golf 1/2`): SR **32**, then 33.
- Pilots (`Hotel 1/2`): SR continues 34, 35; pilots also get `TFAR_freq_lr = "30"`.
- **30 is the company/platoon net.**

At runtime `Client/Radio/fn_initRadios.sqf` assigns a per-side encryption code (`fnf_blufor_code` / `fnf_opfor_code` / `fnf_indfor_code`, with friendly-side collapsing, lines 18-46), puts SR in the left ear and LR in the right (`setSwStereo 1` / `setLrStereo 2`, lines 63, 75), and if a player has only an SR it configures the alternate net into the right ear instead (lines 96-102). It then publishes live SR/LR frequencies to `fnf_freq_sr` / `fnf_freq_lr` once per second (lines 108-128) so the ORBAT tab can display them (`fn_updateOrbat.sqf:55-67`).

### The ORBAT tab
`Client/Breifing/fn_initOrbat.sqf` creates a diary subject `ORBAT` and re-renders it every 2 seconds while the map is open. `fn_updateOrbat.sqf` shows only groups **on the player's side that contain at least one human** (lines 25-28, 40-52), rendering `<GroupName> (<count>) - SR:<freq> - LR:<freq>` followed by `Role: PlayerName` lines, leader first. AI-only groups are skipped; AI members render as role `"AI"`.

### Spectator slots
Six `ace_spectator_virtual` entities ship in the template `mission.sqm`. `Client/fn_init.sqf:36-40` short-circuits the entire client boot for them and routes to `fn_initSpectatorSlot`; spectators see *all* objectives (`fn_initObjs.sqf:85-89`) and all safe zones (`fn_initSafeZones.sqf:96-99`).

**Not found / not applicable:** there is no slot-locking, no rank/whitelist gating, no per-slot description field beyond `description`, no "slot count" or reserved-slot concept, and no ORBAT tree data (v3's `CfgFNFORBAT` nesting is gone — see §15).

---

## 5. Loadouts / arsenal

### Loadouts are baked Eden inventories, not scripts
Every playable unit in a kit composition carries a literal `class Inventory` block: `primaryWeapon` (with `optics`, `muzzle`, `firemode`, `primaryMuzzleMag` + `ammoLeft`), `handgun`, `binocular`, `uniform`/`vest`/`backpack` with `ItemCargo`/`MagazineCargo` item-and-count lists (`USArmy[2020]/composition.sqe:370-470` and onward). There is **no runtime loadout application step** in v4 — the unit spawns wearing what Eden recorded.

`Client/fn_init.sqf:6` snapshots `fnf_playerLoadout = getUnitLoadout player` at boot, which is what the personal-rearm and selector systems restore against.

### The kit library
85 registered kit compositions (`loadouts.hpp`), 32 MB on disk (`client_mod/fnf_eden/kits/`), each `composition.sqe` ~12 000-15 000 lines of plain-text Eden config. Naming convention is `Faction[Decade]`, e.g. `USArmy[2020]`, `RussianVDV[1980]`, `GermanFallschimjager[1945]`, `PathetLao[1960]`. Each has a `header.sqe` declaring `version`, `name`, `author="FNF"`, and a `requiredAddons[]` array — 54 entries for `USArmy[2020]`, spanning RHS, UK3CB, KAR, TOTT optics, ACE submodules and `fnf_weapons`.

Kits are grouped in Eden by subcategory: 32 Blufor, 18 Opfor, 15 Indfor. The 10 Vietnam and 10 WW2 kits exist on disk but their registration blocks are **commented out** (`loadouts.hpp:577-585, 591-680, 686-775`), as are their subcategory declarations (`systems.hpp:23-30`) — so 20 of the 85 registered kits are dark in v4.7.0, leaving 65 selectable.

### `fnf_module_kitInformation`
Three attributes (`modules.hpp:377-403`): `Name` (default `"Unknown Kit"`), `Author` (`"Unknown Author"`), and **`Bare Bones Kit`** — a serialised `getUnitLoadout` array string used for reinserted players. The shipped US Army 2020 value is a full nested loadout string (`composition.sqe:14508`). The module must be synced to exactly one Side logic; `Client/Breifing/fn_initBreifing.sqf:128-143` warns `Kit information has no valid side synced to it` / `…more than one side…`.

### The briefing kit tab is *sampled*, not declared
`fn_initBreifing.sqf:149-162` picks **up to three random live players** on the side and renders *their* helmet/vest/uniform pictures as "the kit". Weapons are the union of every player's primary and secondary on that side (lines 193-226). So the kit tab reflects what people actually slotted into, not a canonical list — and it waits until >50% of the side's slots are filled before rendering (lines 483-490).

### Selectors — the in-mission "arsenal"
There is **no ACE Arsenal box** in the FNF flow (`ace_arsenal` is only a `requiredAddons[]` entry for ordering, `config.cpp:8`). Instead:

- `fnf_module_selectorHost`: `Selector Name` (free text) + `Selector Type` combo — `Item` (`itm`, default), `Optic` (`opt`), `Primary Weapon` (`pri`), `Launcher` (`sec`), `Handgun` (`hnd`) (`modules.hpp:293-317`).
- `fnf_module_selectorOption`: `Option Name` + `Default?` checkbox (`modules.hpp:328-345`).
- At runtime the host becomes an ACE self-interaction submenu under a single root action `"FNF Selectors"`; each option is a child action (`fn_initSelectors.sqf:60-81`). Selections are stored on the host module as `fnf_selection_<playerUID>` and published globally (line 86), so they survive disconnect/reconnect.
- Selection is only possible **during safe start**: `Client/Zones/fn_initZones.sqf:49-55, 82-88` sets `fnf_showSelectors` false when the player leaves a selector-enabled restriction group, and the briefing text confirms *"once safe start has ended you will no longer be able to change your selection"* (`fn_initBreifing.sqf:611`). You must still be carrying the previously-selected item to switch (line 612).
- Admins can force-toggle selectors for chosen players via a Zeus action, `"Switch selectors for selected players"` (`Client/Admin/fn_zuesAceOptions.sqf:110`).

### Personal rearm
`fnf_module_personalRearm` has one attribute, `Time between Rearms` (`fnf_timeBetweenRearms`, default **3600** seconds, `modules.hpp:547-555`). Combined with selectors it lets a player draw a replacement of their selected item once per cooldown.

### Vehicle loadouts
- **FNF Properties ▸ Clear Inventory** (`FNF_InventoryAutoClear`, default `true`, `attributesAndTools.hpp:23-33`) — applied at *export* time by `init3DEN.sqf:239-250`, which blanks the vehicle's `ammoBox` attribute, exports, then restores it. Also honoured in the briefing asset renderer (`fn_initBreifing.sqf:358-361`).
- **FNF Properties ▸ Use Default Loadout** (`FNF_vehicleLoadouts_useDefault`, default `true`, `attributesAndTools.hpp:34-44`) — tooltip promises *"the vehicle will be given a standardized FNF weapon set (if one is defined)"*. **Nothing in the v4.7.0 tree reads this variable.** A case-insensitive grep for `vehicleLoadouts_useDefault` across the whole worktree returns only the two declaration lines. It is a dead knob.
- Runtime custom vehicle loadouts/pylons are captured server-side into `fnf_vehicleCustomLoadout` / `fnf_vehicleCustomPylons` when invincibility lifts (`Server/Handles/fn_handleVicInvincibility.sqf:91, 104`) and used by the FNF rearm replacement (`Server/fn_rearmVic.sqf:17-19`) — this is a runtime snapshot, not an authoring surface.

### Backpack locking
Backpacks are locked on spawn and unlock automatically when safe start ends; players can toggle via ACE self-interact in the meantime (`Client/BackpackLocking/fn_initBackpackLocking.sqf`, described verbatim in `fn_initBreifing.sqf:623-628`).

---

## 6. Briefing / intel

The briefing is a **generated diary**, built entirely at runtime by `Client/Breifing/fn_initBreifing.sqf` (673 lines). The mission maker authors four free-text fields and syncs some vehicles; everything else is derived.

### Authored inputs (4 fields on `FNF Init`)
All four are `EditMulti5` multi-line controls (`modules.hpp:100-139`), each defaulting to `""`:

| Attribute | Display name | Tooltip |
|---|---|---|
| `fnf_breifingNotes` | Notes | "General notes about the mission" |
| `fnf_breifingAO` | Area of Operations | "A description of terrain or specific features of the terrain to look out for" |
| `fnf_breifingBackground` | Background | "Lore or the background of the battle and why its happening" |
| `fnf_breifingRules` | Mission Rules | "Any custom rules that must be followed in the mission" |

Each is emitted as a `Diary` record **only if non-empty** (`fn_initBreifing.sqf:554-569`), in the order Mission Rules → Area of Operations → Background → Notes.

### Derived diary content
| Diary subject | Source | Content |
|---|---|---|
| `Diary` → "Mission Details" | always | View distance, fortify points, fortify colour, time limit (`fn_initBreifing.sqf:570`) |
| `blufor` / `opfor` / `indfor` | `fnf_module_breifingAssets` sync payload | One record per distinct vehicle type: display name, `editorPreview` image, seat/crew counts, `canFloat`, per-turret weapon + magazine breakdown, and an icon grid of cargo inventory 5-per-row (`fn_initBreifing.sqf:266-419`) |
| same, "Loadout" record | live players | Helmet/vest/uniform pictures sampled from up to 3 random players + every weapon in use on the side (`fn_initBreifing.sqf:82-263`) |
| `orbat` → "My Orbat" | live groups | See §4; refreshed every 2 s while the map is open |
| `rules` ("FNF Info") | hardcoded | 8 records, ~90 lines of prose |

The `rules` subject is **hardcoded community documentation shipped in every mission**: *Contacting Staff*, *Reinsert*, *Selectors*, *Teleporters*, *Backpack Locking*, *Play Zone*, *Safe Start*, *In-Game Rules* (13 lettered rules A–M), *General Rules* (7 lettered rules A–G) (`fn_initBreifing.sqf:586-672`). One value is computed rather than static: the Reinsert article inserts the *actual* reinsert window, `fnf_timeToDisableReinsertsAfterSafeStart` minus the longest safe-zone lifetime (`fn_initBreifing.sqf:577-584, 602`).

### Lobby / overview intel
- `briefingName`, `overviewText`, `author`, `onLoadMission` are stock Eden mission attributes, shipped with placeholder text.
- `IntelOverviewText` is written by the **Generate Lobby Description** tool (§3 step 13).

### Other intel surfaces
- `Client/UI/Spectator/fn_missionDetailsScreen.sqf` (374 lines) and `fn_missionDetailsButton.sqf` (369) render an in-game mission-details panel.
- `Client/UI/Base64Image/` (3 functions) lets an image be transmitted and displayed to clients — `INFERRED:` intended for briefing imagery/maps pushed by staff; no authoring attribute references it.
- `Description/UiDescriptions.hpp:47-58` declares an unused-by-default `RscWebBrowser` control (`allowExternalURL = 0`).

**Not found:** there is no per-side briefing text, no per-group orders field, no map-marker-based intel authoring, no image attachment attribute, and no rich-text editor — the four `EditMulti5` boxes are plain multi-line strings that end up inside `createDiaryRecord`, so structured-text markup must be hand-typed.

---

## 7. Objectives / game modes

**There is no "game mode".** v4 replaced mode selection with **composable objective modules**. A mission is whatever set of objective modules you place.

### The 7 objective types
All inherit from an abstract `fnf_module_objective` (`modules.hpp:578`, no attributes, no `scope`, so not placeable).

| Module | Display name | Attack / defend values | Extra attributes |
|---|---|---|---|
| `fnf_module_destroyObj` | FNF Destroy Objective | `des` Destroy Object (default) / `pro` Protect Object | `fnf_zoneKnown` (bool, `true`) |
| `fnf_module_sectorCaptureObj` | FNF Capture Sector Objective | `cap` Capture Sector (default) / `def` Defend Sector | `fnf_prefix` (`fnf_marker_sector_`), `fnf_TimeToCapture` (60 s) |
| `fnf_module_terminalObj` | FNF Terminal Objective | `hck` Hack Terminal (default) / `def` Defend Terminal | `fnf_hackingTime` (90 s), `fnf_zoneKnown` |
| `fnf_module_sectorHoldObj` | FNF Hold Sector Objective | *(none — hold only)* | `fnf_prefix` (`fnf_marker_sector_`), `fnf_TimeToCapture` (60 s), `fnf_PointsPerSecond` (1), `fnf_PointsForCompletion` (1000) |
| `fnf_module_assassinObj` | FNF Assassin Objective | `elm` Eliminate Target (default) / `pro` Protect Target | `fnf_targetName` (`"the VIP"`), `fnf_zoneKnown` |
| `fnf_module_stealObj` | FNF Steal Objective | `stl` Steal Object (default) / `kep` Keep Object | `fnf_prefix` (`fnf_marker_steal_`), `fnf_zoneKnown` |
| `fnf_module_escortObj` | FNF Escort Objective | `esc` Escort Object (default) / `des` Destroy Object | `fnf_prefix` (`fnf_marker_steal_`), `fnf_zoneKnown` |

`fnf_module_sectorHoldObj` also has a **commented-out** `Global Sector Points` checkbox (`modules.hpp:732-740`) that would have pooled points across all hold sectors for a side.

### Objective lifecycle
Objectives are a 6-state machine, documented inline at `Client/Objectives/fn_initObjs.sqf:20-28`:
```
0 - Not Created        3 - Active
1 - Not Tracking, Not Known   4 - Completed
2 - Not Tracking, Known       5 - Failed
```
Each objective entry is `[ObjState, ObjModule, Task, AlliedTask, CodeOnCompletion, params]` (`fn_initObjs.sqf:20`). Both client and server run a 1 Hz watcher (`fn_init.sqf:162`, `Server/fn_init.sqf:40`).

### Tasks
Two parent tasks are created lazily: **"My Tasks"** and **"Ally Tasks"** (`fn_initDestroy.sqf:29-40`). Whether an objective lands under "Ally Tasks" is decided by `fn_initObjs.sqf:73-82` — same side ⇒ mine; friendly-but-different side ⇒ ally's. Task titles are auto-generated as `"<N>: Destroy the <DisplayName>"` / `"<N>: Defend the <DisplayName>"` (`fn_initDestroy.sqf:66-70`), with an auto-composed description including the object's `editorPreview` image, a "how to find it" sentence that varies by whether hiding zones are attached and whether `fnf_zoneKnown` is set (lines 82-104), and a prose sentence listing prerequisites (lines 106-125).

### The hidden objective attributes
Every objective reads these via `getVariable`, but **none is declared in `modules.hpp`** (verified: `grep -c 'customObjective\|codeOnCompletion' modules.hpp` → 0):

`fnf_customObjectiveTitle`, `fnf_customObjectiveDescription`, `fnf_customObjectiveAlliedTitle`, `fnf_customObjectiveAlliedDescription`, `fnf_codeOnCompletion`, plus the Steal/Escort-only `fnf_customObjectiveTitleTarget`, `fnf_customObjectiveDescriptionTarget`, `fnf_customObjectiveAlliedTitleTarget`, `fnf_customObjectiveAlliedDescriptionTarget`.

Sites: `fn_initDestroy.sqf:49-56, 271`; `fn_initSteal.sqf:50, 148, 239`; `fn_initEscort.sqf:49, 55-56, 147, 152-153, 238, 243-244`; `fn_initAssassin.sqf:44-45, 50-51, 294`; `fn_initTerminal.sqf:48-49, 54, 389`; `fn_initCaptureSector.sqf:44, 262`; `fn_initHoldSector.sqf:45, 50, 342`. `INFERRED:` a mission maker can only set these through the module's Eden **Init** field (`this setVariable [...]`) — there is no UI for custom task text or on-completion scripting.

### Hiding zones
Sync one or more `FNF Hiding Zone` modules to an objective and the target can be concealed. `Client/HidingZones/fn_initHidingZones.sqf` runs a 1 Hz loop: if the target is inside one of its assigned zones and `fnf_zoneKnown` is `true`, the task destination snaps to that zone's *visual centre*; if `zoneKnown` is `false`, the destination is cancelled entirely and players must search every hiding zone. If the target is outside all zones, the task points at its exact position.

### Win conditions and endings
`Server/fn_endGame.sqf` selects one of 8 `CfgDebriefing` endings (`Description/Endings.hpp`) from the winning-side array: `draw`, `bluforWin`, `opforWin`, `independentWin`, `bluforAndIndependentWin`, `opforAndIndependentWin`, `bluforAndOpforWin`, `allWin`. It broadcasts a result notification that also nags for a mission review, waits 30 seconds for reviews, then uploads them (`fn_endGame.sqf:110-123`).

Capture/hold sector objectives additionally enable Arma's stock scoreboard: `addMissionEventHandler ["Map", {call BIS_fnc_showMissionStatus}]` (`fn_initObjs.sqf:126-127, 136-137`).

**Not found:** no ticket/score-based victory, no time-based partial scoring, no per-objective weighting, no draw-condition authoring. Mission length is a flat `fnf_gameTime` timer.

---

## 8. Respawn / tickets / medical / revive

### Mission-level respawn is disabled
`Description.ext:1-6`:
```
respawn = 3; respawnDialog = 0; respawnButton = 0;
respawndelay = 99999; respawnOnStart = -1; respawnTemplates[] = {};
```
i.e. respawn type BASE, no dialog, no button, effectively-infinite delay, no templates, no respawn on start. All death handling is bespoke.

### Three death modes (`fnf_module_miscOptions ▸ Death Mode`)
`modules.hpp:423-436`, default **`reinsert`**:

**1. `reinsert` (default).** On death the player is added to their *group's* death queue (`fnf_deathQueue`, a list of player UIDs) and dropped into **limited spectator** (`Client/Death/fn_initDeath.sqf:105-159`). Any squad member holding the `fnf_weap_reinsert_flare` can fire it; 2 seconds after firing, the projectile's position is sampled and becomes the insertion point (`fn_initDeath.sqf:226-245`). A helicopter flies in and fast-ropes up to **4** dead squadmates in death order, then leaves (`Server/Reinsert/fn_startReInsert.sqf`; behaviour described verbatim at `fn_initBreifing.sqf:594-606`). Rules encoded in code: **one reinsert per squad, ever** (`fnf_reinsertRequested`, `fn_initDeath.sqf:196-197, 223`); calling with nobody dead **burns it** (lines 203-206); the window closes `fnf_timeToDisableReinsertsAfterSafeStart` minutes after mission start (default 20, `modules.hpp:149-157`), with 5-minute and 1-minute warnings (lines 25-64). Reinserted players receive the kit's **`Bare Bones Kit`** loadout, not their original one (commit `e7b0c784` "Changed: Reinsert now uses kit based bare bones loadout"). Reinsert has a 30-second delay (commit `a7746f5a`; `fn_initDeath.sqf:244` uses a 28 s wait after the 2 s sample).

**2. `onelife`.** `player addEventHandler ["Killed", …]` → full spectator after 3 seconds (`fn_initDeath.sqf:94-99`). No second chance.

**3. `respawn`.** A **per-player life counter** stored in `missionNamespace` as `fnf_livesLeft_<playerUID>`, seeded from `Default Respawns` (`fnf_defaultLives`, Eden default **2**, code fallback **3** at `fn_initDeath.sqf:253`). On death, lives decrement; if ≥ 0 the player goes to limited spectator with a countdown of `Respawn Time` seconds (`fnf_respawnTime`, default 300) and is teleported back in at a `FNF Respawn Position` module's location (`Client/Spectator/fn_startLimitedSpectator.sqf:146-190, 237`); if lives run out they go to full spectator (`fn_initDeath.sqf:290-303`). Remaining respawns are shown in the pause menu (`Client/UI/PauseMenu/fn_generalPlayerButtons.sqf:57`), and admins can set lives directly (commit `db4dd0a8`).

`fnf_module_respawnPosition` has **no attributes at all** (`modules.hpp:489-495`) — it is a pure position marker, assigned by syncing (commit `fba3fa53` "Changed: Respawn Position modules now get assigned via syncing system").

### Spectator
Two tiers. **Limited spectator** (`fn_startLimitedSpectator.sqf`) is governed by two misc-options attributes: `Limited Spectator Unit Visability` — `full` Everyone / `side` Their Side (**default**) / `squad` Their Squad (`modules.hpp:455-468`) — and `Mute TFAR for players in limited spectator?` (default `false`, lines 469-477). **Full spectator** (`fn_startSpectator.sqf`) uses ACE Spectator with `TFAR_spectatorCanHearEnemyUnits = true` and `TFAR_spectatorCanHearFriendlies = true` (`cba_settings.sqf:637-638`). `fn_upgradeSpectator.sqf` promotes limited → full.

### Medical
Entirely **ACE, forced from the server mod** — 31 `ace_medical_*` + 8 `ace_medical_gui_*`/`feedback` + ~50 `ace_medical_treatment_*` settings in `server_mod/cba_settings_userconfig/cba_settings.sqf:203-295`. Notable house values: `ace_medical_fatalDamageSource = 2`, `ace_medical_statemachine_fatalInjuriesPlayer = 0`, `ace_medical_statemachine_cardiacArrestTime = 420`, `ace_medical_playerDamageThreshold = 1.5`, `ace_medical_bleedingCoefficient = 0.6`, `ace_medical_fractureChance = 0.8`, `ace_medical_treatment_allowSelfPAK = 0`, `ace_medical_treatment_advancedBandages = 0`, `ace_medical_spontaneousWakeUpChance = 0.75`. `client_mod/fnf_medicalChanges/config.cpp` supplies FNF's own medical tweaks.

**A mission maker cannot change any medical setting** — these are server-forced (`force force`) and live outside the mission.

### Tickets
**Not applicable.** There is no ticket system. The closest analogue is `fnf_defaultLives` in `respawn` mode and `fnf_PointsForCompletion` on hold sectors.

---

## 9. Zones / areas / triggers / play area

### One zone primitive: the marker polygon
Every zone type is the same object — an ordered set of numbered markers sharing a prefix, turned into a polygon. `Client/Zones/fn_addZone.sqf` signature is `[prefix, displayName, shaded, inverted]`.

Mechanics:
- Vertices are discovered by *attempting to create* `<prefix>1`, `<prefix>2`, … locally; a name that fails to create already exists, so it's a real vertex (`fn_addZone.sqf:25-33`). Numbering must therefore be contiguous from 1.
- < 3 markers ⇒ zone silently does not generate (with a debug warning), and the caller usually degrades to "system will NOT function" (`fn_addZone.sqf:39-46`).
- A `POLYLINE` marker `<prefix>polyline` is drawn in the colour of `<prefix>1` (lines 21, 68-71).
- An optional `<prefix>displayName` ICON marker (`mil_dot`, black) is placed at the vertex with the largest X (lines 48, 61-64, 74-81).
- Shading uses ear-clipping triangulation: `fn_triangulatePolygon.sqf`, `fn_invertPolygon.sqf`, `fn_combineOffsetPoints.sqf`, `fn_shadeZone.sqf` / `fn_unShadeZone.sqf`. `inverted = true` shades *outside* the polygon — that is how the play zone renders.
- A **visual centre** (not centroid) is computed by `fn_calculateVisualCenter.sqf` and cached, along with nearest/furthest vertex distances (`fn_addZone.sqf:90-108`).
- Zones are registered in a global hashmap `fnf_zoneList` keyed by prefix (`fn_initZones.sqf:17`, `fn_addZone.sqf:108`).

### Restriction groups — the enforcement primitive
`fn_addRestrictionGroup.sqf` takes `[name, teleport, weaponDisable, shownSelectors, …]`-style flags. `Client/Zones/fn_initZones.sqf:27-103` runs a 1 Hz loop over `fnf_zoneRestrictionGroupsList`; a group is *satisfied* if the player (or their vehicle) is inside **any** of its zones (`inPolygon`, lines 38-46). On failure it either teleports the player back to `fnf_zoneRestrictionsLastKnownPosition` (on foot: `setPosASL`; in a ground vehicle: `findEmptyPosition` + `setVehiclePosition`; in an aircraft: `setVehiclePosition … "FLY"` only if the group's air flag is set) or lifts a weapon-disable request. Concretely:

| Group | Created by | Flags |
|---|---|---|
| `safeZoneGroup` | `fn_initSafeZones.sqf:112` | `[true, true, true, true]` — teleport, weapon-disable, air, selectors |
| `safeZoneSwitchingGroup` | `fn_initSafeZones.sqf:113` | `[true, false, false, true]` |
| `playZoneGroup` | `fn_initPlayZones.sqf:101` | `[false, true, false, false]` |

The play-zone group's air flag is `false` — which is exactly why aircraft may leave the play area, and why a player exiting an aircraft outside it is teleported back to the last in-zone position, possibly mid-air (behaviour documented to players at `fn_initBreifing.sqf:630-635`).

### The five zone flavours

**Play Zone** (`fnf_module_playZone`). One attribute, `Marker Prefix` (default `fnf_marker_playzone_`). Synced to a Side logic or a player. Rendered **inverted** (`fn_initPlayZones.sqf:109`). Placing more than one triggers `WARNING: Multiple play zones does not support complex shading, no shading has been applied` and shading is dropped entirely (lines 104-108, 123-126).

**Safe Zone** (`fnf_module_safeZone`). Five attributes: `Marker Prefix` (`fnf_marker_safezone_`), `Time until Zone is Deleted` (15 min), `Visible to Allies` (`true`), `Visible to Enemies` (`true`), `Switch to Non-Restrictive` (`false`). The longest `fnf_timeZoneIsDeleted` across all safe zones **defines safe start** — it drives the HUD timer (`fnf_timerMessage = "Safe Start Remaining: %1"`) and the 5-minute warning (`fn_initSafeZones.sqf:158-192`). Safe zones that belong to nobody the player can see are quietly stripped of their markers (lines 141-155).

**Hiding Zone** (`fnf_module_hidingZone`). One attribute, `Marker Prefix` (`fnf_marker_hidingzone_`). Never enforces anything; it exists only to mask objective locations (§7).

**Sector** (on capture/hold objectives). `fnf_marker_sector_` prefix; the same polygon machinery builds the capture area (`fn_initCaptureSector.sqf:144-182`).

**Steal / Escort drop-off**. `fnf_marker_steal_` prefix on both `fnf_module_stealObj` and `fnf_module_escortObj` — note **both default to the same prefix**, so a mission with one of each must be renamed by hand or they collide.

### Teleport poles
`fnf_module_teleportPoles`: `Time until Poles is Deleted` (15 min) and `Visible To Others` (`false`). Sync the module to a side/player plus the pole objects; every pole in one module becomes mutually reachable by ACE interaction. `fnf_customPoleName` is read at runtime but is **not** a declared attribute (commit `09cc2122` "Added: Custom names for teleport poles" — set via the module's init field).

### Mobile spawn points
`fnf_module_mobileSpawnPointHandeler`: one attribute, `Distance To Disable`, default 250, "How close do enemy players need to get before the mobile spawn points are disabled?"

**⚠ This attribute is inert.** The Eden expression writes `fnf_distanceToDisable` (`modules.hpp:506-507`) but the runtime reads `fnf_distanceForDisable` (`Client/MobileSpawnPoints/fn_initMobileSpawnPoints.sqf:67`). Those are different names, so the value always falls back to the hardcoded `250`. This is a genuine bug, not a case-sensitivity artefact — unlike the `attributesAndTools.hpp` attributes, `modules.hpp` expressions hardcode the variable name rather than using `%s`.

### Triggers
**Not applicable in the FNF model.** No FNF system reads Arma triggers; all area logic is `inPolygon` against marker-derived polygons evaluated client-side at 1 Hz. (v3 by contrast used a named `zoneTrigger` entity — see §15.)

---

## 10. Configuration surface

This section enumerates **every** knob. Nothing is elided.

### 10.1 `fnf_module_init` — "FNF Init" (13 attributes)
`client_mod/fnf_eden/modules.hpp:23-159`. Exactly one per mission; the client aborts with `DANGER: No FNF Init found, exiting mission prep` if absent and `DANGER: Multiple FNF Init found` if duplicated (`Client/fn_init.sqf:12-13`).

| Property | Control | Display name | Type | Default |
|---|---|---|---|---|
| `fnf_gameTime` | Edit | Time Limit | NUMBER | `65` (minutes) |
| `fnf_viewDistance` | Edit | View Distance | NUMBER | `1000` (m) |
| `fnf_fortifyPoints` | Edit | Fortify Points | NUMBER | `100` |
| `fnf_fortifyColour` | Combo | Fortify Colour | STRING | `Green` (also `Tan`) |
| `fnf_disableFortifyBlufor` | Checkbox | Disable Fortify Blufor | BOOL | `false` |
| `fnf_disableFortifyOpfor` | Checkbox | Disable Fortify Opfor | BOOL | `false` |
| `fnf_disableFortifyIndfor` | Checkbox | Disable Fortify Independent | BOOL | `false` |
| `fnf_breifingNotes` | EditMulti5 | Notes | STRING | `""` |
| `fnf_breifingAO` | EditMulti5 | Area of Operations | STRING | `""` |
| `fnf_breifingBackground` | EditMulti5 | Background | STRING | `""` |
| `fnf_breifingRules` | EditMulti5 | Mission Rules | STRING | `""` |
| `fnf_debug` | Checkbox | Debug | BOOL | `false` |
| `fnf_timeToDisableReinsertsAfterSafeStart` | Edit | Time to Disable Resinserts (minutes) | NUMBER | `20` |

(13 rows = the full attribute list; the `Attributes` class also inherits `AttributesBase`, which contributes vanilla module fields.)

### 10.2 `fnf_module_miscOptions` — "FNF Misc options" (7 attributes)
`modules.hpp:406-488`. Optional — if absent the client fabricates one locally with defaults (`Client/fn_init.sqf:26-32`).

| Property | Control | Display name | Default |
|---|---|---|---|
| `fnf_fortifyAfterSafeStart` | Checkbox | Fortify available after safe start? | `false` |
| `fnf_deathMode` | Combo | Death Mode | `reinsert` (also `onelife`, `respawn`) |
| `fnf_defaultLives` | Edit | Default Respawns | `2` |
| `fnf_respawnTime` | Edit | Respawn Time | `300` |
| `fnf_limitedSpectatorUnits` | Combo | Limited Spectator Unit Visability | `side` (also `full`, `squad`) |
| `fnf_limitedSpectatorMuteTFAR` | Checkbox | Mute TFAR for players in limited spectator? | `false` |
| `fnf_simulationControl` | Checkbox | Allow players to enable sim on vehicles? | `false` |

### 10.3 Zone / system modules

| Module | Property | Control | Display name | Default |
|---|---|---|---|---|
| `fnf_module_playZone` | `fnf_prefix` | Edit | Marker Prefix | `fnf_marker_playzone_` |
| `fnf_module_safeZone` | `fnf_prefix` | Edit | Marker Prefix | `fnf_marker_safezone_` |
| | `fnf_timeZoneIsDeleted` | Edit | Time until Zone is Deleted | `15` |
| | `fnf_visibleToAllies` | Checkbox | Visible to Allies | `true` |
| | `fnf_visibleToEnemies` | Checkbox | Visible to Enemies | `true` |
| | `fnf_switchToNonRestrictive` | Checkbox | Switch to Non-Restrictive | `false` |
| `fnf_module_hidingZone` | `fnf_prefix` | Edit | Marker Prefix | `fnf_marker_hidingzone_` |
| `fnf_module_teleportPoles` | `fnf_timePolesAreDeleted` | Edit | Time until Poles is Deleted | `15` |
| | `fnf_visibleToOthers` | Checkbox | Visible To Others | `false` |
| `fnf_module_breifingAssets` | `fnf_timeToBeVincible` | Edit | Time until connected vics are vincible | `15` |
| `fnf_module_mobileSpawnPointHandeler` | `fnf_distanceToDisable` | Edit | Distance To Disable | `250` **(inert — see §9)** |
| `fnf_module_assetRestriction` | `fnf_restrictionLevel` | Combo | Restriction Level | `1` Crew (also `0` All Slots, `2` Driving) |
| `fnf_module_personalRearm` | `fnf_timeBetweenRearms` | Edit | Time between Rearms | `3600` |
| `fnf_module_respawnPosition` | *(none)* | — | — | — |
| `fnf_module_sequentialObjectivePlanner` | `fnf_nextObjectiveKnown` | Checkbox | Is next Objective shown | `true` |

### 10.4 Kit / selector modules

| Module | Property | Control | Display name | Default |
|---|---|---|---|---|
| `fnf_module_kitInformation` | `fnf_kitName` | Edit | Name | `Unknown Kit` |
| | `fnf_kitAuthor` | Edit | Author | `Unknown Author` |
| | `fnf_bareBonesLoadout` | Edit | Bare Bones Kit | `[[[],[],[],[],[],[],"","",[],["","","","","",""]],[]]` |
| `fnf_module_selectorHost` | `fnf_selectorName` | Edit | Selector Name | `""` |
| | `fnf_selectorType` | Combo | Selector Type | `itm` Item (also `opt`, `pri`, `sec`, `hnd`) |
| `fnf_module_selectorOption` | `fnf_optionName` | Edit | Option Name | `""` |
| | `fnf_defaultSelection` | Checkbox | Default? | `false` |

### 10.5 Objective modules
See the table in §7 for all 7 types. In full-property form the objective knobs are: `fnf_objectiveType` (6 of 7 types), `fnf_zoneKnown` (destroy, terminal, assassin, steal, escort), `fnf_prefix` (sectorCapture, sectorHold, steal, escort), `fnf_TimeToCapture` (sectorCapture, sectorHold), `fnf_hackingTime` (terminal), `fnf_targetName` (assassin), `fnf_PointsPerSecond` + `fnf_PointsForCompletion` (sectorHold). Plus one commented-out knob, `fnf_GlobalPoints` (`modules.hpp:732-740`).

**Totals** (`grep -c 'property = "' modules.hpp` → **63** attribute instances; `grep -o 'property = "[^"]*"' modules.hpp | sort -u | wc -l` → **47** distinct names). One instance/name — `fnf_GlobalPoints` — is inside a comment block, so the live surface is **62 attribute instances across 22 placeable modules, using 46 distinct property names**. `fnf_prefix`, `fnf_objectiveType` and `fnf_zoneKnown` are each reused by several modules, which is why instances outnumber names.

### 10.6 Per-object "FNF Properties" (3 attributes)
`client_mod/fnf_eden/attributesAndTools.hpp:6-46` — a collapsed attribute category on *every* object.

| Class / property | Control | Display name | Condition | Default |
|---|---|---|---|---|
| `fnf_autoMarkExclude` / `FNF_MarkingExclude` | Checkbox | Exclude from Map Auto-Mark | `1 - objectControllable - objectVehicle` | `false` |
| `fnf_clearInventory` / `FNF_InventoryAutoClear` | Checkbox | Clear Inventory | `objectVehicle` | `true` |
| `fnf_vehicleLoadouts_useDefault` / `FNF_vehicleLoadouts_useDefault` | Checkbox | Use Default Loadout | `objectVehicle` | `true` **(unread — see §5)** |

### 10.7 Description.ext surface (7 knobs + 4 include files)
`respawn = 3`, `respawnDialog = 0`, `respawnButton = 0`, `respawndelay = 99999`, `respawnOnStart = -1`, `respawnTemplates[] = {}`, `onPauseScript[] = {FNF_ClientSide_fnc_generalPlayerButtons, FNF_ClientSide_fnc_adminButtons}`.

### 10.8 Lobby parameters (1)
`PerformanceTweaks` — values `{0,1,2,3}`, texts `{None, Low, Medium, High}`, default `0` (`Description/Params.hpp`).

### 10.9 Fortify object tables (server-side, code-only)
`Server/fn_initFortify.sqf:22-59` defines two hardcoded arrays of `[classname, pointCost, category]`, selected by `fnf_fortifyColour`, and registered identically for all three sides:

`_ModernGreen` / `_ModernTan` — 17 entries each: sandbag short 3 / long 4 / round 4; plank 4 m 5, 8 m 5; razorwire 10; hedgehog 5; H-barrier wall-4 12, corner 12, wall-6 18; bag bunker small 22, tower 35, large 50; `Land_Bunker_01_small_F` 75, `_big_F` 100, `_HQ_F` 100, `_tall_F` 130.

Categories are declared in `Description/FortifyCategories.hpp`: `fnf_fortify_sandbags` "Sandbags", `fnf_fortify_pOW` "Planks of Wood", `fnf_fortify_hBarriers` "H Barriers", `fnf_fortify_bunkersHM` "Bunkers (Handmade)", `fnf_fortify_bunkersCNSRT` "Bunkers (Constructed)", `fnf_fortify_miscellaneous` "Miscellaneous Items". **Editing the fortify catalogue means editing framework SQF, not a mission attribute.**

### 10.10 Server-forced CBA settings (518)
`server_mod/cba_settings_userconfig/cba_settings.sqf` — **518** `force force` lines across 30 commented sections: ACE Advanced Ballistics, Advanced Fatigue, Advanced Throwing, Advanced Vehicle Damage, AI, Arsenal, Artillery, Captives, Common, Cook-off, Crew Served Weapons, Dragging, Explosives, Field Rations, Fire, Fortify, Fragmentation Simulation, G-Forces, Goggles, Grenades, Headless, Hearing, Interaction, Logistics, Magazine Repack, Map, Map Gestures, Medical (+Interface, +Treatment), Name Tags, Nightvision, Overheating, Pointing, Pylons, Quick Mount, Repair, Respawn, Scopes, Sitting, Spectator, Switch Units, Trenches, Uncategorized, User Interface, Vehicle Lock, Vehicles, View Distance Limiter, View Restriction, Weather, Wind Deflection, Zeus; plus CBA, DUI Squad Radar (Indicators + Radar), Enhanced Movement Rework (~35 settings), GRAD Trenches (~40 settings), IFX ACE3 Window Break, OCAP Main + Recorder (~20 settings), and TFAR Global (~50 settings).

**None of this is mission-authorable.** It is the community's ruleset, applied server-side to every mission.

### 10.11 Staff list
`server_mod/fnf_server_vars/staffList.sqf` — 5 hardcoded `[steamUID, [name, discordMention]]` pairs published to `fnf_staffInfo`, loaded via a `serverInit` preInit event handler.

### 10.12 Marker colours
`client_mod/fnf_mapColors/config.cpp` adds/overrides 17 `CfgMarkerColors` entries (Grey, Brown, Orange, Khaki, Pink, Neon Cyan, Burnt Orange, Forest Green, Deep Violet, Teal, Tan/Sand, Infrared Red, Gold, Crimson, Turquoise, Magenta, Olive Drab, Lavender), all `scope = 2`. These exist because zone colour = marker colour.

### 10.13 Undeclared runtime variables (hidden config)
Read by code, never declared as an Eden attribute — settable only via a module's Init field: `fnf_customObjectiveTitle`, `fnf_customObjectiveDescription`, `fnf_customObjectiveAlliedTitle`, `fnf_customObjectiveAlliedDescription`, `fnf_customObjectiveTitleTarget`, `fnf_customObjectiveDescriptionTarget`, `fnf_customObjectiveAlliedTitleTarget`, `fnf_customObjectiveAlliedDescriptionTarget`, `fnf_codeOnCompletion`, `fnf_customPoleName`, `fnf_selectorIcon`.

### 10.14 Registration gap
`fnf_eden/config.cpp:5` lists **17** classes in `CfgPatches >> units[]`, but `modules.hpp` defines **23** classes (22 placeable + 1 abstract base). The five placeable modules missing from `units[]` are: `fnf_module_miscOptions`, `fnf_module_respawnPosition`, `fnf_module_mobileSpawnPointHandeler`, `fnf_module_assetRestriction`, `fnf_module_personalRearm`. `INFERRED:` this affects addon-dependency recording in `mission.sqm` rather than editor visibility (which is governed by `scope = 2`), but it is an inconsistency.

---

## 11. Tooling

### In-editor tools (Eden menu bar)
`client_mod/fnf_eden/attributesAndTools.hpp:51-88` injects a **"FNF Mission Maker Tools..."** submenu into Eden's `Tools` menu with two entries:

1. **Generate Lobby Description** → `FNF_ModFunctions_fnc_generateLobbyDescription` (§3 step 13). Writes `IntelOverviewText` + clipboard.
2. **Spawn Custom Sided Kit** → `FNF_ModFunctions_fnc_spawnCustomSidedKit` (`opensNewWindow = 1`). Searchable kit picker + side re-targeting (§3 step 5).

(A "Staff Tools folder" existed earlier and was deleted — commit `f871ed5c` "Removed: Staff Tools folder"; `48bfd586` moved the custom-sided-kit tool into the mission-maker folder.)

### The `init3DEN.sqf` pipeline
Not a "tool" a mission maker invokes — it is always-on automation attached to three Eden event handlers (`init3DEN.sqf:317-319`):
- `OnMessage` → the export pipeline (JIP scaffolding, group IDs, marker alpha, vehicle inventory).
- `OnPaste` → marker number auto-increment.
- `OnDeleteUnits` → empty-layer garbage collection, wrapped in `collect3DENHistory` so it is undoable.

### `External Scripts/` — 7 debug-console scripts (no v3 equivalent)

| Script | Lines | What it does |
|---|---|---|
| `ORBATChange.sqf` | ~200 | Rebuilds a canonical FNF ORBAT from the currently 3DEN-selected groups. Harvests one loadout per role shorthand (`SL`, `AR`, `AT`, `TL`, `CE`, `MED`, `MRK`, `MG`, `PL`, `CRL`, `CR`, `MAT`, `AMAT`), deletes non-leader units, re-types the leader, spawns replacement units at a computed grid, sets `description`, `TFAR_freq_sr`/`_lr`, `ControlMP`/`ControlSP`, and finally `save3DENInventory`. Encodes the squad-name list `[Alpha, Bravo, Charlie, Delta, Echo, Foxtrot]`, the `Mike` AT squad, `Golf 1/2` crews and `Hotel 1/2` pilots, plus the whole radio plan. |
| `DoFunctionsToLoadouts.sqf` | ~95 | Bulk re-export loop. Enumerates every `Cfg3DEN >> Compositions` class with `editorCategory == 'fnf_Kits'`, spawns them one at a time, and on each save turns every non-blacklisted layer (blacklist: `Info`, `Selectors`, `Units`) into a custom composition via `do3DENAction "CreateCustomComposition"`, driving the dialog controls directly (`findDisplay 317 displayCtrl 95/96`, author hardcoded `"FNF"`). This is how the 85 kit compositions are regenerated en masse. |
| `ExportLoadoutFromOldFramework.sqf` | ~35 | **v3→v4 migration.** Iterates `fnf_loadout_roles` (a *v3* global), calls the *v3* `fnf_loadout_fnc_applyLoadout` for each role, reads back `getUnitLoadout player`, patches in the side-appropriate TFAR radio (`TFAR_anprc152` / `TFAR_fadak` / `TFAR_anprc148jem`), adds `ACE_M14`+`ACE_wirecutter` for `CE`, `ToolKit` for `CRL`/`CR`/`PI`, downgrades the `DM` SVD variant, and `copyToClipboard`s the whole array. |
| `fnf_transform.py` | 1 604 (58 KB) | Python **schema migration for `composition.sqe` files**. Text-level surgery on the Eden config: `remove_foxtrot`, `remove_platoon_sergeant`, `fix_command_hq_after_ps_removal`, `move_mike_squad`, `restructure_infantry_squad`, `add_xray_squad`, `add_lima_squad`, `add_sierra_squad`, `add_india_squad`, `inject_new_squads`, `renumber_downstream_items`, `remap_ids`, `find_playable_units`, `find_selector_ids`, `parse_existing_connections`, `build_connections_block`, then `transform` → `verify` → `compare_with_reference`. CLI: `python fnf_transform.py <input.sqe> [output.sqe]` or `--all <kits_dir>`. |
| `GenerateHatMod.sqf` | ~90 | Scans `CfgWeapons` for `_generalMacro == 'HeadgearItem'`, walks `configHierarchy` on `ItemInfo >> HitpointsProtectionInfo`, and emits config for `client_mod/fnf_hats` — i.e. **codegen for the armour-normalisation mod**. |
| `GenerateVestMod.sqf` | ~120 | Same shape for `VestItem` → `fnf_armor`. |
| `GenerateBackpackMod.sqf` | ~70 | Same shape for backpacks → `fnf_backpacks`. |

The three `Generate*Mod.sqf` scripts are how FNF keeps a 100+-mod arsenal *balanced*: rather than trusting each source mod's armour values, they enumerate every hat/vest/backpack in the loaded config tree and regenerate an override mod.

### Kit-authoring workspaces
`Kit Mission Files/` (3 Eden missions) is where kits are built before being exported to compositions by `DoFunctionsToLoadouts.sqf`. All three `mission.sqm` are binarized. Commit `578ba60e` "Removed: Most kit mission files" deleted four earlier sets (`FNF_Kits_Set_1..4.VR`), so the shipped three are a *subset* of the real authoring corpus.

### Debug / diagnostics
- `FNF Init ▸ Debug` gates ~30 `systemChat` messages, tiered `DANGER:` (system will not function) vs `WARNING:` (degraded).
- `mission.sqm` sets the scenario attributes `EnableTargetDebug = 1` and `EnableDebugConsole = 1` in the shipped template (extracted strings).
- Server FPS is tracked at runtime (`fnf_serverFPS`, `fnf_serverFPSThreshold`).
- OCAP (after-action replay) is configured with ~20 forced settings (`cba_settings.sqf:572-592`) including `OCAP_settings_autoStart`, `OCAP_settings_minPlayerCount`, `OCAP_settings_trackTickets`.

### Admin / staff tooling (in-mission)
- Pause menu: **Contact Staff** and **Mission Review** buttons for everyone (`Client/UI/PauseMenu/fn_generalPlayerButtons.sqf:25, 40`); admins additionally get side-win buttons (`Blufor` / `Opfor` / `Independent`), **End Game**, and **Admin Menu** (`fn_adminButtons.sqf:39, 58, 77, 91, 137`).
- Zeus ACE actions: `Zoom to Last Admin Report`, `Kick players from vehicle`, `Switch selectors for selected players`, `Toggle Sim` (`Client/Admin/fn_zuesAceOptions.sqf:18, 72, 110, 221`).
- The urgent-help hotkey is bound to `Y` by default (`fn_initBreifing.sqf:591`).

### CI / repo hygiene
No build pipeline in-tree. `.github/` contains only three issue templates — `break_fix.md` (Problem Statement / To Reproduce / Expected / Screenshots / Additional context), `feature_request.md`, and **`testing_required.md`** (labels `dedi testing, local testing`; fields "What needs to be tested" / "Expected behavior of the test"). That last one is a real signal: **the framework has a formal "needs dedicated-server testing" workflow state.** `.editorconfig` pins tabs (width 2), CRLF, UTF-8, trim trailing whitespace, final newline.

---

## 12. Conventions and house rules encoded in the framework

These are *not* documentation — they are enforced or asserted by code.

**Naming and structure**
1. `Role@Group` in a leader's role description **is** the group name. No `@` ⇒ export warning (`init3DEN.sqf:176`).
2. Group names are made unique across sides by trailing-space padding: west `""`, east `" "`, independent `"  "` (`init3DEN.sqf:154`). Duplicates within a side ⇒ warning (line 168).
3. Zone markers must be `<prefix><1..N>`, contiguous, ≥ 3 (`fn_addZone.sqf:25-46`).
4. Marker alpha `0.99` means "visible to the author, invisible in play" (`init3DEN.sqf:104-108, 226-235`).
5. Kits are named `Faction[Decade]`; kit compositions have exactly three layers named `Info`, `Selectors`, `Units` (blacklisted from re-export, `DoFunctionsToLoadouts.sqf:14-18`).
6. Every FNF module classname starts with `fnf_module_` — that string *is* the discovery mechanism (`fn_findFNFModules.sqf:20`).

**Mission composition**
7. **Exactly one `FNF Init`.** Zero or two aborts the entire client mission prep (`Client/fn_init.sqf:12-13`).
8. **Every objective, safe zone, play zone, teleport pole, briefing-asset and kit-info module must have exactly one Side logic synced.** Zero or two ⇒ the system is disabled with a `DANGER:` message.
9. **Objectives come in pairs** — attacker module + defender module on the same target (§3 step 10).
10. **Objective order is spatial.** Numbering derives from sorted world position, not placement order (`fn_sortByLocation.sqf`, `fn_getDisplayObjNumber.sqf`).
11. A mission with no objectives, no play zone or no safe zones still runs — it just warns (`Client/fn_init.sqf:67-103`).

**Play conventions baked into runtime**
12. **Safe start is mandatory in practice.** If there are no safe-zone modules, fortify is disabled outright unless `Fortify available after safe start?` is ticked (`Client/fn_init.sqf:89-100`).
13. **Fortify requires the ACE Fortify item in the loadout** — no item, no fortify (`Client/Fortify/fn_initFortify.sqf:44-47`).
14. **One reinsert per squad, four players max, in death order** (`Client/Death/fn_initDeath.sqf:196-218`).
15. **Aircraft are exempt from the play zone; everything else is teleported back** (`fn_initPlayZones.sqf:101`, `fn_initZones.sqf:59-74`).
16. **Global and side text channels are disabled; radio is enabled** (`fn_restrictPlayer.sqf:17-19`). Rule L is "Use of in-game text chat is prohibited."
17. **View distance and terrain grid are clamped every 0.5 s**, not set once — players cannot cheat them upward (`fn_restrictPlayer.sqf:43-62`). `disableRemoteSensors true` on top.
18. **Thermals are stripped from any vehicle or UAV a player takes control of** (`Client/fn_init.sqf:166-182`).
19. **Backpacks lock on spawn, unlock when safe start ends** (`fn_initBreifing.sqf:623-628`).
20. **AI voice/subtitles/conversation are disabled globally** (`fn_restrictPlayer.sqf:22-26`).
21. **All dead bodies are deleted 0.1 s into the mission** (`Server/fn_init.sqf:43-49`).
22. **ACE "Leave Group" is removed** — you cannot leave your squad (`client_mod/fnf_ace/config.cpp:14-18`).
23. **TFAR radio codes are per-side and enforced**; friendly sides share a code (`fn_initRadios.sqf:18-46`).
24. **SR left ear, LR right ear**, always (`fn_initRadios.sqf:63, 75`).
25. **Company net is frequency 30; squad nets start at 40 and step by 10** (`ORBATChange.sqf`).

**Written rules shipped in every mission** (`fn_initBreifing.sqf:646-672`) — 13 in-game rules A–M and 7 general rules A–G, including "Follow the chain of command — lone-wolfing is prohibited", "Do not pick up enemy Helmets, Vests, Backpacks, or Uniforms", "Using fortify/entrenchment to block entrances or paths is prohibited", "Using enhanced movement to climb on plant life taller than a person is prohibited", "Using any out-of-game platform to send/receive game-related information is prohibited".

**Process conventions**
26. Commit-message prefixes are a strict taxonomy: across `v4.0.0..v4.7.0` (351 commits) — 157 `Fixed:`, 78 `Added:`, 39 `Changed:`, 26 `Version Bump`, 15 `Updated:`, 11 `Removed:`, 7 `Multiple Changes`.
27. `Version.txt` is bumped as its own commit and is read at runtime to drive the new-player experience (`fn_initNewPlayerExperience.sqf:14-49`, semver-compared component by component).
28. A dedicated GitHub issue type exists for "this needs dedicated-server testing".

---

## 13. What this framework does better than anyone else

**1. The export hook that makes Eden a compiler.**
`init3DEN.sqf` is the standout artefact. It turns Eden's "Export to Multiplayer" into a build step that (a) derives slot-list group names from authored role text, (b) works around Arma's JIP sync-replication hole by mechanically rewriting sync links into init-field calls and then undoing them, (c) toggles author-visible markers to invisible, (d) strips vehicle inventories, and (e) restores every one of those mutations and re-saves so the author's file is untouched. It is wrapped in `collect3DENHistory` so it is a single undo step, and it self-heals on next open if a previous export left scaffolding behind (`init3DEN.sqf:36-114`). No other Arma framework I've seen treats the editor as a compilation front-end this deliberately.

**2. Sync-as-typed-edges: one uniform composition mechanism.**
Every module is `(attributes) + (synced entities)`, and the synced entities are *discriminated by type at runtime*: a Side logic assigns ownership, a player assigns scope, an object is the payload, another module is composition. The same five-case `switch` appears in `fn_initObjs`, `fn_initSafeZones`, `fn_initPlayZones`, `fn_initTeleportPoles`, `fn_initAssetRestrictions`, `fn_initBreifing` and `fn_initSelectors`. A mission maker learns *one* interaction — "drag a link from A to B" — and it composes objectives with hiding zones, objectives with sequencing, selectors with options, zones with sides, and assets with briefings. That is a genuinely better authoring primitive than per-system bespoke config.

**3. The kit library as a first-class, migratable asset.**
85 faction kits, each a 12–15k-line Eden composition carrying a full 77-slot ORBAT with per-slot inventories, selector boxes, and a kit-info module — draggable into any mission in one action. Around it sits real tooling: a **kit re-siding transform** keyed on `Squad_index_Role` (`fn_spawnCustomSidedKit.sqf`), a **bulk re-export driver** (`DoFunctionsToLoadouts.sqf`), and a **1 604-line Python schema migrator** (`fnf_transform.py`) that restructured the ORBAT across 63 kit files in a single commit (`778f2333`, +322 176 / −191 893 lines). They treat their content library like a database with migrations. Nobody else in this space does that.

**4. A closed feedback loop from players back to the mission maker.**
Rate-the-mission and rate-the-commanding sliders plus free text, editable until the round ends, uploaded to a Google Sheet keyed by mission name and author (`fn_missionReviewScreen.sqf`, `Server/fn_endGame.sqf:123`). The end-of-round notification explicitly nags for it. Mission makers get scored, written feedback on every mission they ship.

**5. Derived documentation instead of authored documentation.**
The briefing is generated: vehicle stat cards with turret-by-turret weapon and magazine breakdowns and an icon grid of cargo, kit imagery sampled from live players, a live ORBAT with real TFAR frequencies, and a per-mission-accurate explanation of the reinsert window. The mission maker writes four paragraphs; the framework produces the rest. The **Generate Lobby Description** tool does the same for the server browser.

**6. Automatic map cartography.**
`fn_markEditorPlacedObjects.sqf` draws a correctly-sized, correctly-rotated rectangle for every placed structure above a size threshold, with a per-object opt-out checkbox. Authors get a readable tactical map of custom-built positions for free.

---

## 14. Friction and known complaints

**1. The real documentation is off-repo.**
`README.md:49` points at a Google Doc. In-repo, a new mission maker gets two `Comment` entities in `mission.sqm` and whatever Eden tooltips exist. v3 shipped a `configGuide.txt`; v4 ships nothing equivalent. Every convention in §12 is discoverable only by reading SQF or being told.

**2. Undeclared configuration.**
Eleven runtime variables are read but have no Eden attribute (§10.13) — including *all* custom objective titles/descriptions and `fnf_codeOnCompletion`, the only scripting hook in the entire objective system. Setting them requires typing `this setVariable [...]` into a module's Init field, with no discoverability and no validation.

**3. Confirmed dead and broken knobs.**
- `fnf_module_mobileSpawnPointHandeler ▸ Distance To Disable` is inert: writes `fnf_distanceToDisable`, runtime reads `fnf_distanceForDisable` (§9).
- `FNF Properties ▸ Use Default Loadout` is read by nothing in the entire worktree (§5).
- `fnf_visibleToAllies` is read for objectives (`fn_initObjs.sqf:40`) into a variable never used again, and is only *declarable* on safe zones (`modules.hpp:209`).
- `fnf_module_sectorHoldObj ▸ Global Sector Points` is commented out (`modules.hpp:732-740`).
- 5 placeable modules are missing from `CfgPatches >> units[]` (§10.14).

**4. Objective identity is positional and implicit.**
Objective numbers come from sorting module positions, and two modules are "the same objective" only because they happen to sync to the same object or share a marker-prefix string. Move a module and the numbering silently changes. Forget the second module and the lobby description's `/2` arithmetic produces a fractional count.

**5. Marker-polygon zones are laborious and fragile.**
Every zone is N hand-placed markers that must be numbered contiguously from 1, ordered correctly (they become polygon vertices in placement order), and coloured to set the zone colour. There is no in-editor preview of the resulting polygon, no vertex reordering, and no error unless you fall below three. `fnf_module_stealObj` and `fnf_module_escortObj` **share the default prefix `fnf_marker_steal_`** (`modules.hpp:823, 865`) so a mission with both silently collides unless renamed.

**6. Multiple play zones are second-class.**
Placing more than one drops shading entirely with a warning: `WARNING: Multiple play zones does not support complex shading, no shading has been applied` (`fn_initPlayZones.sqf:123-126`).

**7. Almost nothing is mission-authorable at the ruleset level.**
518 forced CBA settings, all medical, all movement, all trench rules, the fortify catalogue and point costs (hardcoded SQF in `Server/fn_initFortify.sqf`), and ~90 lines of house rules in the briefing are outside the mission. A mission maker who wants a different fortify item or a different medical profile must change the framework or the server mod.

**8. The kit tab is non-deterministic.**
It samples up to three random live players (`fn_initBreifing.sqf:151-162`) and waits for >50% of a side's slots to fill before rendering. Authors cannot preview it and it differs run to run.

**9. Debug is off by default and gates all diagnostics.**
Every `DANGER:`/`WARNING:` is behind `fnf_debug` (default `false`, `modules.hpp:147`), so the default authoring experience is silent failure. The template's own reminder comment tells you to turn it *off* before exporting — meaning the shipped state is the un-diagnosable one.

**10. Only 3 of 15 `Kit Mission Files` remain.**
Commit `578ba60e` "Removed: Most kit mission files" deleted `FNF_Kits_Set_1..4.VR`. The kit source-of-truth for most of the 85 kits is not in the repo — only the derived compositions are. Combined with binarized `mission.sqm`, kit editing for those factions is effectively closed.

**11. 20 of 85 registered kits are dark.**
All Vietnam and WW2 kit registrations are commented out (`loadouts.hpp:577-775`), as are their editor subcategories (`systems.hpp:23-30`), while the 32 MB of composition data still ships.

**12. Typos are load-bearing and pervasive.**
`Breifing` (as a directory name, module classname `fnf_module_breifingAssets`, and ~12 variable names), `Handeler`/`Handeling`, `Zues`, `SequentialHandeler`, `Resinserts`, `Vincible`, `slector`, `chaqnge`, `dissappear`, `unloacked`, `Squad leader@India`. These are baked into config property names and therefore into every saved `mission.sqm` — unfixable without a migration.

**13. Client-authoritative zone enforcement.**
`fn_initZones.sqf` runs the containment check locally at 1 Hz and teleports the *local* player. It is a fairness mechanism, not an anti-cheat one, and it costs a per-frame-handler polygon test per restriction group on every client.

**14. Sync links do not survive JIP — and the workaround is invasive.**
The entire `init3DEN.sqf` export dance exists because Arma does not replicate Eden sync connections to joining players. The fix rewrites every playable unit's init field and creates one throwaway Logic per slot. It works, and it self-heals, but it is a large amount of machinery that a mission maker must never accidentally break — and it means the exported `mission.sqm` differs structurally from the saved one.

---

## 15. v3.6.9 → v4.7.0 delta

### 15.0 Shape of the change: v4 is a rewrite, not an evolution

`v3.6.9` = commit `285e4441`, dated **2023-09-12**. `v4.0.0` = commit `68e6d38d`, dated **2023-12-16**, subject **"Manual Merge of 4.0.0"** — and its `%P` parent list is exactly `285e44412409dc4ec6a8b659e31b4875166bfdbd`. There is **one commit** between the two tags. Its own diffstat:

```
$ git show --stat v4.0.0
1504 files changed, 1045833 insertions(+), 85891 deletions(-)
```

and the full span:

```
$ git diff --stat v3.6.9 v4.7.0
1672 files changed, 1438844 insertions(+), 91349 deletions(-)
```

with **351 commits** from `v3.6.9` to `v4.7.0` (2026-06-09) — 350 of which are post-4.0.0 iteration. v4 was developed off-repo and landed as a single squashed drop. Nothing in v3's mission template survived: `git diff --name-status v3.6.9 v4.7.0 --diff-filter=D` deletes the entire `FNF_MissionTemplate.VR/` tree (421 files) and `git diff --name-status --diff-filter=A` creates `FNF_Mission_Template.VR/` (155 files, note the added underscores) plus `client_mod/fnf_eden` (249 new files), `External Scripts/` (7), `Kit Mission Files/` (3).

### 15.1 Directory-level diff

| v3.6.9 | v4.7.0 | Note |
|---|---|---|
| `FNF_MissionTemplate.VR/` — 421 files | `FNF_Mission_Template.VR/` — 155 files | Full replacement; renamed |
| ├ `config.sqf` (177 lines) | *(gone)* | Central config file abolished |
| ├ `configGuide.txt` (219 lines) | *(gone)* | In-repo documentation abolished |
| ├ `cba_settings.sqf` | *(gone from mission)* | Moved to `server_mod/cba_settings_userconfig/` |
| ├ `modes/` (15 files), `mode_config/` (9) | *(gone)* | Replaced by objective modules |
| ├ `missionSpecials/` (6) | *(gone)* | Ambient airdrop/artillery removed |
| ├ `description/` (202 files, incl. `KITS/` 159) | `Description/` (5 files) | Kits left the mission entirely |
| ├ `description.ext` + `description_SA.ext` | `Description.ext` (18 lines) | Two variants → one |
| ├ `mission.sqm` + `mission_SA_Modern.sqm` + `mission_SA_VN.sqm` | `mission.sqm` | Three starter files → one |
| ├ `client/` (151), `server/` (28) | `Client/` (116), `Server/` (30) | Rewritten, re-cased |
| *(none)* | `init3DEN.sqf` (319 lines) | **New concept** |
| *(none)* | `Version.txt` | v3 had `version.txt` (lower-case) |
| `server_mods/{common,early,late}_mod/` | `server_mod/{cba_settings_userconfig,fnf_server_vars,python_files}/` | 3-way split collapsed to one |
| `tools/` (export_kits, screenshot_kits, portableScreenshotsSystem, postProcessingForScreenshotKits) | *(gone)* | Kit-screenshot pipeline deleted |
| *(none)* | `External Scripts/` (7) | **New concept** |
| *(none)* | `Kit Mission Files/` (3 `.VR`) | **New concept** |
| `client_mod/fnf_eden/config.cpp` — 1 file, ~62 lines, 3 boolean object attributes, **zero modules** | `client_mod/fnf_eden/` — 250 files, 32 MB, 22 modules + 85 kit compositions + 36 system/objective compositions | **The entire v4 authoring surface** |
| `client_mod/fnf_setup/` (player prefs mod: 2 CBA settings, 5 keybinds) | *(gone)* | |
| `client_mod/{fnf_media, fnf_patches, fnf_vehicles, fnf_vn, fnf_ww2}` | *(gone; `fnf_vn`/`fnf_ww2_*` demoted to `Optionals/`)* | |
| *(none)* | `client_mod/{fnf_ace, fnf_hats, fnf_mapColors, fnf_medicalChanges, fnf_reSkins, fnf_sounds}` | |

### 15.2 The five mechanism swaps

The v3 mission maker used **five disjoint mechanisms**. v4 collapses all five into **one**: place a module, set its attributes, drag sync links.

**(a) Central SQF config file → per-module Eden attributes.**
v3: `config.sqf` (177 lines) defined ~21 live globals — `fnf_gameMode`, `fnf_defendingSide`, `fnf_attackingSide`, `fnf_vnArtillerySide`, `fnf_SWRadioForAll`, `fnf_enemyStartVisible`, `fnf_maxViewDistance`, `fnf_fortifyPoints`, `fnf_fortifyStyle`, `fnf_magnifiedOptics`, `fnf_isNightMission`, `fnf_addNVG`, `fnf_bluforUniform`/`Gear`, `fnf_opforUniform`/`Gear`, `fnf_indforUniform`/`Gear`, `fnf_bluAT`/`redAT`/`grnAT`, `fnf_showAlliedFactions`, `fnf_debug` — plus 4 briefing strings and **12 dead `…AuxRole` settings explicitly marked `//*NOT USED*`**. Values were C-preprocessor macros from `description/configDefs.hpp`, so `fnf_gameMode = destroy;` is a bare identifier, not a string.
v4: 62 live Eden attribute instances across 22 placeable modules (46 distinct property names), each typed (`NUMBER`/`STRING`/`BOOL`), each with a display name, tooltip and default, edited in Eden's attribute panel.

**(b) Nine fixed game modes → seven composable objective modules.**
v3 modes (`configGuide.txt:4-17`): **ATK/DEF** — `adSector`, `assassin`, `captureTheFlag`, `destroy`, `rush`, `uplink`; **NEUTRAL** — `connection`, `neutralSector`, `scavHunt`. One per mission, selected by `fnf_gameMode`, implemented in `modes/<name>/<name>_{client,server}.sqf` and parameterised by `mode_config/<name>.sqf` — files whose contents were `#include`d (textually pasted) into both the mode implementation and the briefing generator. Configuring a mode meant **editing SQF source**.
v4: no mode variable at all. Place any mix of `destroyObj`, `sectorCaptureObj`, `terminalObj`, `sectorHoldObj`, `assassinObj`, `stealObj`, `escortObj`, any number of times, on any sides, optionally sequenced. Rough lineage: `destroy`→Destroy, `assassin`→Assassin, `adSector`/`neutralSector`→Capture Sector, `uplink`→Terminal, `captureTheFlag`/`scavHunt`→Steal, `rush`→Sequential Planner + N objectives. **`connection` has no v4 equivalent** (`INFERRED:` from mode-name/objective-name comparison — the v4 code contains no analogue and no commit references it).

**(c) Config-class kits + slot tagging → Eden compositions with baked inventories.**
v3: kits were `.hpp` config classes shipped **inside each mission's `description.ext` include tree** — `description/cfgFNFLoadouts.hpp` including 69 `KITS/GEAR/*.hpp` and 89 `KITS/UNIFORMS/*.hpp` under `CfgFNFLoadouts >> UNIFORMS|GEAR >> <KIT> >> <ROLE>`, selected mission-wide by six globals (`fnf_bluforUniform`, `fnf_bluforGear`, …), and bound to a slot by typing `this setVariable ["fnfLoadout","<ROLE>"]` into that unit's Eden **init field**. A 33-file `client/loadout/` subsystem (`fn_applyLoadout`, `fn_givePrimaryWeapon`, `fn_giveNVG`, `fn_giveRadios`, `fn_setFace`, `fn_prepOpticsSelector`, …) applied it at runtime. Vehicle loadouts were another 24 `.hpp` files plus `cfgFNFVehicleLoadouts.hpp`.
v4: 85 draggable Eden compositions whose units carry literal `class Inventory` blocks. **No runtime loadout application exists.** The migration path is `External Scripts/ExportLoadoutFromOldFramework.sqf`, which calls the *v3* `fnf_loadout_fnc_applyLoadout` for every `fnf_loadout_roles` entry and clipboards the resulting `getUnitLoadout` arrays — literally a v3→v4 loadout exporter shipped in the v4 repo.

**(d) ORBAT config tree → role-description strings.**
v3: `description/cfgFNFORBAT.hpp`, **974 lines** of `CfgFNFORBAT` classes with `id`, `idType`, `side`, `size`, `type`, `insignia`, `commander`, `tags[]`, `text`, `textShort`, `description`, `assets[]` and nested `subordinates[]` — the BIS ORBAT-viewer schema, hand-edited per mission. Group IDs were set by a dedicated `client/briefing/fn_setGroupIDs.sqf`.
v4: the ORBAT *is* the set of placed groups; the only metadata is `description="Role@Group"` on each unit, and group IDs are derived at export (`init3DEN.sqf:153-178`). The ORBAT tab renders live from `allGroups` + `roleDescription`.

**(e) Hardcoded entity names + regex marker conventions → module attributes.**
v3 bound systems to **specific Eden object names**: `destroy_obj_1`, `term1`, `ctf_flagPole`, `fnf_sec1`, `fnf_sector1`, `HVT_1`, `scav_obj_1`, `west_safeZone_marker_1`, `west_safeZone_flag_1`, `fnf_briefingTable_west`, `zoneTrigger`. Polygon zones used fixed regex prefixes `fnf_custom_safeZone_<side>_<n>_marker_<v>` and `fnf_custom_zoneBoundary_1_marker_<v>`. `config.sqf:41-43` warned: *"DO NOT DELETE ANY OF THE OTHER TEMPLATE OBJECTIVE OBJECTS — they will be deleted automatically if not in use."*
v4: no reserved names anywhere. Prefixes are per-module `Edit` attributes, and objectives bind to *whatever object you sync*.

### 15.3 Concepts that exist only in v3

- **`fnf_gameMode`** and the whole mode/mode_config architecture.
- **Sustained Assault ("SA") mode** — a second, parallel mission shape shipped as *extra starter files*: `mission_SA_Modern.sqm`, `mission_SA_VN.sqm`, `description_SA.ext`, `definitions_SA.hpp`, `fn_teleportActions_SA.sqf`. Selecting it was a **file-renaming ritual** documented in `config.sqf:19-34`:
  > *"1a. if using SOG Prairie Fire, rename `mission_SA_VN.sqm` to `mission.sqm` … 2. rename `description_SA.ext` to `description.ext` … 4. delete `description_normal.ext` and whichever missionX.sqm files you didn't use (for filesize purposes)"* — and it **"requires advance permission from Missions Team Lead"**.
- **`missionSpecials/`** — ambient airdrop and ambient artillery (`fn_ambientAirdrop.sqf`, `fn_ambientArtillery.sqf`, plus box/plane/smoke helpers).
- **Prairie Fire / Vietnam as a first-class mission axis** — `fnf_vnArtillerySide`, `cfgVNArtillery.hpp`, `client_mod/fnf_vn`, `mission_SA_VN.sqm`. In v4 the VN content is `Optionals/fnf_vn` and every Vietnam kit registration is commented out.
- **`fnf_magnifiedOptics`** (`0`/`1`/`-1`: marksmen-only / everyone / forced ironsights), **`fnf_isNightMission`** (`-1` auto-detect / `0` / `1`), **`fnf_addNVG`** (side array), **`fnf_SWRadioForAll`**, **`fnf_enemyStartVisible`**, **`fnf_showAlliedFactions`**, **`fnf_fortifyStyle`** (5 values incl. `"Modern"` auto-detect and the VN-era `"NVA"`/`"MACV"` sets), **`fnf_bluAT`/`redAT`/`grnAT`** MAT selection, and the 12 dead `…AuxRole` knobs.
- **`configGuide.txt`** — 219 lines of in-repo documentation.
- **`tools/`** — the kit-screenshot pipeline: `export_kits.sqf`, `screenshot_kits.sqf` (315 lines), `portableScreenshotsSystem/` (+ `kitHash.txt`), and PowerShell post-processing (`enhance-images.ps1`, `square-and-label-for-addon.ps1`).
- **Three-way server mod split** — `common_mod` / `early_mod` / `late_mod`, the latter two each carrying their own `cba_settings.sqf` (`INFERRED:` early- vs late-war rulesets, from the names plus `client_mod/fnf_logos/data/{early,late}.paa` in v3).
- **`client_mod/fnf_setup`** — player-preference mod with CBA settings `fnf_pref_loadoutInterface` / `fnf_pref_spectatorInterface` and five keybinds (`fnf_key_hideUI`, `_openLoadoutFleximenu`, `_missionInfoPanel`, `_adminPanel`, `_spectatorMute`).
- **Briefing tables** (`client/briefing/table/`, `fnf_briefingTable_west`) and **news/articles** (`fn_news.sqf`, `articles/fn_NewYear2022.sqf`).
- **`fnf_patches`** — unit insignia / staff patch system.
- **Discord webhooks** — `server/webhook/fn_webhook_roundStart.sqf` / `fn_webhook_roundEnd.sqf`.

### 15.4 Concepts that exist only in v4

- **Eden modules.** 22 placeable `fnf_module_*` classes with 45 typed attributes. v3 had **zero**.
- **Eden compositions as content.** 85 kits + 34 system/objective presets, registered through `Cfg3DEN >> Compositions`. v3 had none.
- **Sync-as-configuration.** The polymorphic `synchronizedObjects` protocol (§3 step 9).
- **`init3DEN.sqf` export pipeline** — JIP sync transfer, derived group IDs, marker alpha toggling, vehicle-inventory clearing, paste auto-increment, empty-layer GC.
- **Eden menu-bar tools** — Generate Lobby Description, Spawn Custom Sided Kit.
- **`External Scripts/`** — ORBAT rebuilder, bulk composition re-exporter, v3 loadout exporter, the 1 604-line Python composition migrator, and three armour-mod code generators.
- **`Kit Mission Files/`** — dedicated kit-authoring Eden workspaces.
- **Sequential objectives** (`fnf_module_sequentialObjectivePlanner`) and paired attacker/defender objective modules.
- **Reinsert** (flare-called squad reinsertion) and the three-way `deathMode`; per-player life counters.
- **Selectors** — modular in-mission loadout choice with persisted per-UID selection and safe-start-only switching.
- **Personal rearm**, **asset restrictions**, **mobile spawn points**, **simulation control**, **hiding zones**, **teleport poles as a module**, **non-restrictive safe zones**.
- **In-game mission review** → Google Sheets via Pythia, and the **New Player Experience** gated on the `Version.txt` semver stamp.
- **Polygon zone shading** (`fn_triangulatePolygon`, `fn_invertPolygon`, `fn_shadeZone`) and **`fnf_mapColors`**.
- **Automatic map marking of editor-placed objects** with per-object opt-out.
- **Limited vs full spectator** tiers with configurable visibility.
- **Two objective types with no v3 mode ancestor:** **Hold Sector** (points-per-second accumulation to a completion threshold) and **Escort**.

### 15.5 What got easier

1. **You stop editing source.** v3's workflow was: rename `.sqm`/`.ext` files, edit `config.sqf` globals, edit `mode_config/<mode>.sqf` snippets, type `this setVariable ["fnfLoadout","SL"]` into unit init fields, and name objects exactly `destroy_obj_1`. v4's is: drag compositions, tick checkboxes, draw sync links. The typed attribute panel replaces free-form SQF.
2. **Kits became one drag.** v3: pick a uniform class and a gear class from a 219-line text file, then tag every slot's init field with a role string, and hope the 33-file runtime loadout builder assembles it correctly. v4: drag `US Army [2020]` onto the map and 77 fully-equipped slots with selectors and a kit-info module appear.
3. **Missions can have more than one objective, of mixed types, with different owners, in sequence.** v3 was strictly one mode per mission. This is the single largest expressive gain.
4. **Objectives bind to any object.** v3 required the template's pre-named objective objects and warned against deleting them; v4 syncs to whatever you place.
5. **Zones are arbitrary polygons with named prefixes** instead of fixed regex conventions with hardcoded side/index slots.
6. **The briefing writes itself.** v3 had a 24-file `client/briefing/` subsystem with per-mode `#include`s and hand-maintained `cfgFNFORBAT.hpp`; v4 derives ORBAT, kit imagery, vehicle stat cards and objective descriptions at runtime.
7. **Export is a validated build step.** v3 had no `init3DEN.sqf` at all — group IDs were set at runtime, and there was no export-time structural check. v4 warns about duplicate and malformed group names *before* you ship.
8. **Death handling became a dropdown.** v3's alternative to 1-life ("Sustained Assault") was a file-renaming procedure requiring team-lead permission; v4 is `Death Mode: reinsert | onelife | respawn`.
9. **Mission makers get feedback.** The review pipeline has no v3 counterpart.
10. **Re-siding a faction is a menu item** rather than hand-rewriting a kit config.

### 15.6 What got harder

1. **The documentation went away.** v3 shipped `configGuide.txt` (219 lines) *inside the mission folder*, and `README.md` stated the contract outright — *"All configuration changes should be made in `config.sqf`"*. v4's README points at a Google Doc, and the in-repo guidance is two `Comment` entities. A v3 mission maker could read the whole config surface in one file; a v4 mission maker must discover 22 modules and their sync semantics.
2. **Nothing is greppable any more.** v3's config was plain text in the mission folder — you could diff it, review it in a PR, or fix it in a text editor. v4's entire authored content lives in a **binarized `mission.sqm`**. You cannot code-review a v4 mission, diff two versions meaningfully, or script a bulk change without Eden. This is precisely why `fnf_transform.py` had to be written: 63 kit files needed an ORBAT change and the only tractable route was 1 604 lines of Python doing text surgery on `.sqe` config.
3. **Configuration is scattered.** One `config.sqf` became 22 modules whose attributes are only visible when the right module is selected. There is no "show me every setting in this mission" view. Some settings (`Fortify Points`) are on Init, some (`Death Mode`) on Misc Options, some (`Time until Zone is Deleted`) on each Safe Zone — and safe-start duration is *implicitly* the maximum across all of them.
4. **Correctness now depends on invisible graph edges.** A missing sync link silently disables an entire objective, and the diagnostic is gated behind a Debug checkbox that ships off. In v3 a missing setting was a visible blank in a text file.
5. **Objective identity is implicit and positional.** v3's `destroy_obj_1` was explicit. v4's "Objective 1" is whatever sorts first by X/Y/Z, and attacker/defender pairing is inferred from a shared sync target.
6. **The paired-module convention is undocumented and unenforced.** Nothing validates that each objective has both halves; the only tell is the lobby-description tool dividing by 2.
7. **Custom task text and completion scripting regressed to init fields.** v4 reads nine `fnf_customObjective*` variables plus `fnf_codeOnCompletion` that have no Eden attribute at all.
8. **Lost knobs.** `fnf_magnifiedOptics`, `fnf_isNightMission`, `fnf_addNVG`, `fnf_SWRadioForAll`, `fnf_enemyStartVisible`, `fnf_showAlliedFactions`, and `fnf_fortifyStyle`'s VN-era sets have no v4 equivalent. v4's fortify catalogue is two hardcoded arrays in `Server/fn_initFortify.sqf`; v3's was a five-value string with map-aware auto-detection.
9. **The kit library is 32 MB of generated data with an incomplete source.** Four of the seven kit-authoring workspaces were deleted (`578ba60e`), and the survivors are binarized. v3's kits were 158 readable `.hpp` files.
10. **Twenty kits are switched off** while still shipping their data (`loadouts.hpp:577-775`).
11. **Vietnam and WW2 were demoted.** v3 treated Prairie Fire as a first-class axis with its own starter `.sqm`, artillery config and client mod; v4 moves the content to `Optionals/` and comments out the registrations.
12. **Ambient systems were dropped** — no v4 equivalent of `missionSpecials/` airdrops or artillery.
13. **The mission-maker feedback loop for kit imagery got worse.** v3 had a whole screenshot pipeline (`tools/screenshot_kits.sqf` + `portableScreenshotsSystem` + PowerShell post-processing) producing canonical kit images; v4 samples three random live players at runtime.

### 15.7 Within-v4 evolution (v4.0.0 → v4.7.0, 350 commits)

The one structurally significant change inside the v4 era is the **ORBAT flattening**:

| | v4.0.0 (`git show 'v4.0.0:…/USArmy[2020]/composition.sqe'`) | v4.7.0 |
|---|---|---|
| Playable slots | 87 | 77 |
| Structure | **Company**: `Company HQ` + `Alpha`/`Bravo`/`Charlie`/`Delta` platoons, each with Platoon Leader, Platoon Sergeant, Marksman, Medic and two squads (`Alpha 1`, `Alpha 2`, …) of SL/AR/**Asst. AR**/LAT/Grenadier/CE/**Ammo Bearer**; plus `Echo`, `Golf 1/2`, `Hotel 1/2` | **Platoon**: `Command HQ` + `Alpha`…`Echo` single squads + `Mike` (MAT) + `Golf 1/2` + `Hotel 1/2` + **`Xray`** (sniper) + **`Lima`** (sappers) + **`Sierra`** (UAV) + **`India`** (mortar) |
| Element count | 18 named groups | 15 named groups |

Executed in two commits: `f401c2dd` (2025-01-29) "Changed: ORBAT Updated for all kits" — 128 files, +543 432 / −1 547 382 — and `778f2333` (2026-03-30) "Updated: New ORBAT partly implemented" — 63 files, +322 176 / −191 893, the commit that **added `External Scripts/fnf_transform.py`**. Roles `Asst. Automatic Rifleman`, `Ammo Bearer`, `Platoon Sergeant`, `Platoon Leader`, `Company Commander`, `Company Sergeant` and the `Foxtrot` squad were removed; `Breacher`, `Scout`, `Sapper`, `Systems Specialist`, `Mortar Gunner`, `Assistant Gunner`, `Sniper`, `Spotter` were added. `0a2f104c` "Fixed: TFAR values correctly set" followed.

Other notable within-era additions (from `git log --oneline v4.0.0..v4.7.0`): Misc Options + Respawn Position modules (`c9e69ea1`), Mobile Spawn Points (`f8257b1d`, per-player at `5f7a0255`), Asset Restrictions (`a9c69816`), Personal Rearm (`8930aff5`, after `dfc03889` "Added: Start of personal rearm (BROKEN)"), non-restrictive safe zones (`a2f50c2c`), simulation control (`b696fd6e`), custom teleport-pole names (`09cc2122`), commander review score (`582a053a`), integrated end-of-mission review time (`8d8d791c`), "Vastly more map colours" (`8ebeffc6`), and multiple FNF Info articles (`f53c3885`, `b60902fa`). Selectors were rewritten at least twice (`a0779a3d` "Changed: Started re-write of selectors (again)"). Removals: Staff Tools folder (`f871ed5c`), most kit mission files (`578ba60e`), the 4.3.0 explainer system (`cc29841b`).

The commit histogram — **157 `Fixed:` vs 78 `Added:`** — is itself evidence: two thirds of post-launch effort went into stabilising the module/sync model rather than extending it.

---

## What a web-based 2D mission editor should steal

1. **Model the mission as a typed graph, not a form.** FNF's single best idea is `(module, attributes) + typed sync edges`, where the edge's meaning is derived from the endpoint's type. A web editor can do this *far* better than Eden: named, validated, visible edges with an inspector that says "this Destroy Objective is missing a Side" instead of a `systemChat` line behind a debug flag.
2. **Ship a first-class content library with migrations.** 85 kits, each a complete ORBAT with baked loadouts, draggable in one action — plus a stable unit key (`Squad_index_Role`) that survives re-siding, and a migration tool for when the ORBAT schema changes. Treat mission content as a versioned database, not as files.
3. **Make export a compiler with a diagnostics panel.** `init3DEN.sqf` proves the value of a build step that derives data (group IDs from role strings), works around runtime limits, and validates. Do it in the browser, surface it as a list of errors and warnings, and make it always-on rather than debug-gated.
4. **Derive documentation.** Auto-generate the briefing, the ORBAT view, the vehicle stat cards and the lobby/summary string from the authored graph. Authors should write four paragraphs, not forty.
5. **Polygon zones as a single primitive with real editing.** One zone type — an ordered vertex list — reused for play area, safe zones, hiding zones, sectors and drop-offs, with colour as a first-class property. But give it what FNF cannot: click-to-place vertices, drag to reorder, live shading preview, and an inverted-fill mode for the play boundary.
6. **Pair objectives explicitly.** FNF's attacker/defender pairing is real design insight — every objective reads differently to each side — but it's encoded as "two modules that happen to share a target". Make it one object with per-side framing.
7. **Close the feedback loop.** Rate-the-mission and rate-the-command, tied to the mission and its author, is cheap to build and clearly changes what gets made.
8. **Text-first storage.** Everything painful about v4 authoring traces to a binarized `mission.sqm`: no diffing, no review, no scripted bulk edits, and a 1 604-line Python migrator as the escape hatch. A JSON mission document is already the right call — keep it, and expose bulk operations natively so nobody ever has to write `fnf_transform.py`.
