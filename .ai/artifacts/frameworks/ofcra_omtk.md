# OFCRA `omtk` — OFCRA Mission ToolKit v2.13.7

Forensic analysis for TBD-Reforger. Source: `/run/media/system/Disk_2/tbd-framework-analysis/ofcra/omtk/`
(read-only clone of <https://github.com/ofcrav2/omtk>, HEAD `7e1ebe8b6a28ed42712dd3bee4862b83ccfa2b09`,
commit message `V2.13.7`, authored `Fri Jul 3 19:43:04 2026 +0200` by `LeoRebu`).

**Clone depth caveat:** `git rev-list --count HEAD` = **1** and `git tag` is empty — this is a
single-commit shallow clone. No history, no blame, no tags. Every dated claim below therefore comes
from `CHANGELOG.md` or a module `README.md` "last modification date" row, not from git.

**Language note:** OFCRA is French (Organisation Française de Combat Réaliste sur ArmA). The
framework was translated to English at v2.3.0 — `CHANGELOG.md:245` "`~ Switch to English`". Only
7 French lines survive in the SQF (see §14). There is **no `stringtable.xml`** anywhere in the repo
(`find . -iname 'stringtable*'` → empty); all user-facing strings are inline literals.

---

## Source inventory

| Path | Lines | What I got from it |
|---|---|---|
| `README.md` | 178 | Stated purpose, dependency claims, 9-step install/usage procedure, lobby-parameter catalogue (stale vs `description.ext`) |
| `CHANGELOG.md` | 264 | Version/date spine 2016-08→2026-07, feature archaeology, the `omtk-loadouts.exe` v1.0.1 note |
| `LICENSE` | 14 | WTFPL v2 |
| `.gitignore` | 2 | `**/*~`, `*.bak` — no build artefacts ignored ⇒ nothing is generated in-tree |
| `description.ext` | 278 | The whole lobby-parameter surface (29 classes), respawn/chat/debug config, `CfgFunctions`, `CfgCommunicationMenu`, dialog includes |
| `init.sqf` | 103 | The mission-maker's hand-edited config block: objectives array, paradrop restrictions, heli unlock, admin UIDs, comm menus |
| `customScripts.sqf` | 51 | The maker's own-code hook + three commented worked examples (flags, hostage capture, `Put` handler) |
| `onPlayerKilled.sqf` | 17 | EG Spectator entry, side-vs-all spectator gating |
| `onPlayerRespawn.sqf` | 3 | Spectator teardown + loadout restore |
| `scripts utiles.txt` | 45 | French "useful scripts" scratchpad — side-local markers, Eden object-ID toggle, manual scoreboard/warmup force |
| `omtk/version.sqf` | 3 | `OMTK_VERSION = "2.13.7"` |
| `omtk/load_modules.sqf` | 61 | The module dispatch table — exactly which param gates which module, and the warm-up/post-warm-up split |
| `omtk/library.sqf` | 530 | Core function library: logging, teleport, vehicle lock, sim control, view distance, weapon safety, per-player admin actions |
| `omtk/briefing.sqf` | 103 | **The house rules, verbatim** + diary tabs (credits, donations, timings, uniforms, rules) |
| `omtk/fn_rosterBriefing.sqf` | 113 | Auto-generated Team Roster diary tab; the `Role@Squad` `roleDescription` convention |
| `omtk/fn_inventoryBriefing.sqf` | 163 | Auto-generated "Squad loadout" diary tab with per-man weapon/magazine/uniform icons |
| `omtk/table_forum.sqf` | 114 | **Slot-list JSON exporter** (French comments) — side → squads → roles, to clipboard |
| `omtk/score_board/README.md` | 241 | The objective DSL specification, 3-faction capzone semantics, timed-objective callbacks |
| `omtk/score_board/main.sqf` | 335 | Objective compilation, side duplication, timed-objective scheduler, end-of-mission timer, heli unlock |
| `omtk/score_board/library.sqf` | 548 | Score computation, all objective evaluators (`omtk_isAlive`, `omtk_isInArea`), flag API, winner determination, stats hooks |
| `omtk/warm_up/README.md` | 88 | Warm-up feature list; documents `OMTK_WU_CHIEF_CLASSES` (which no longer exists) |
| `omtk/warm_up/main.sqf` | 208 | Warm-up procedure, engine freeze, sim disable, per-client timer GUI, admin player-count ping |
| `omtk/warm_up/library.sqf` | 114 | Restriction trigger, teleport-back, spawn-zone marker, end-of-warm-up teardown, `omtk_wu_fn_launch_game` |
| `omtk/dynamic_startup/README.md` | 29 | **Stale** — documents a `launch_mode` module that no longer exists |
| `omtk/dynamic_startup/main.sqf` | 25 | Mode dispatch (markers / interactive) |
| `omtk/dynamic_startup/markers.sqf` | 200 | The marker-driven generator: spawn/respawn/capzone/flag markers → triggers + objectives |
| `omtk/dynamic_startup/markers_doc.sqf` | 23 | The in-game diary cheat-sheet for marker names |
| `omtk/dynamic_startup/interactive.sqf` | 23 | Chief-class gate for the interactive startup dialog |
| `omtk/uniform_lock/{README.md,main.sqf,lock.sqf,wwyw.sqf}` | 17/9/37/141 | The two uniform regimes: hard lock (IDC 6331 disable) vs pierremgi "Wear What You Want" |
| `omtk/rambo_warn/{README.md,main.sqf}` | 40/148 | **The lonewolf detector** — the numeric encoding of OFCRA's cohesion rule |
| `omtk/radio_lock/{README.md,main.sqf}` | 35/57 | TFAR encryption-code side check on `Take` |
| `omtk/respawn_mode/main.sqf` | 12 | Respawn timer / immortal mode |
| `omtk/ia_manager/{README.md,main.sqf}` | 40/59 | AI skill nerf table, playable-AI freeze, `addRating 1000000` |
| `omtk/difficulty_check/{README.md,main.sqf}` | 29/8 | `difficulty < 3` ⇒ warn |
| `omtk/kill_logger/{README.md,main.sqf}` | 32/37 | MPKilled/MPHit RPT logging, AI renaming to `bot_N`, ACE last-damage-source resolution |
| `omtk/zeus_admins/{README.md,main.sqf}` | 28/62 | 10 pre-created curator modules assigned by UID index |
| `omtk/tactical_paradrop/{README.md,main.sqf}` | 72/113 | Map-click paradrop with time window + circular allowed zones |
| `omtk/map_exploration/main.sqf` | 83 | Briefing/recon mode: teleport-on-click, spawn vehicles, reset daytime |
| `omtk/vehicles_thermalimaging/{README.md,main.sqf}` | 28/12 | Global `disableTIEquipment` |
| `omtk/view_distance/main.sqf` | 26 | Client view-distance enforcement loop (credited to FNF's Mallen) |
| `omtk/3rd-parties/README.md` | 26 | Empty extension point |
| `script_library/Readme.txt` | 1 | "Useful scripts for the edition of missions with the OMTK." |
| `script_library/switch_objective.txt` | 152 | Copy-paste recipe: contestable objective with hold-actions + flag API |
| `script_library/update_markers.txt` | 90 | Copy-paste recipe: side-local vehicle tracking markers |
| `script_library/limit_distance.txt` | 32 | Copy-paste recipe: drone leash (destroy beyond 500 m / 200 m AGL) |
| `script_library/receive_intel.txt` | 35 | Copy-paste recipe: hold-action that appends a diary record |
| `script_library/play_sound.txt` | 53 | Copy-paste recipe: `CfgSounds` + broadcast hold-action |
| `images/{blue,red,green}.jpg` | — | Read as images: **placeholder silhouettes** the maker replaces with side uniform photos |
| `images/{bluefor,redfor,greenfor}.jpg` | — | Read as images: scoreboard faction flags (`bluefor.jpg` = US flag) |
| `omtk/ui/pauseScreenMenu.sqf` | 796 | **Delegated read** — the in-game admin/referee console |
| `omtk/ui/defines.hpp` | 256 | **Delegated read** — UI base classes |
| `omtk/score_board/dialog_scoreboard.hpp` | 252 | **Delegated read** — scoreboard dialog |
| `omtk/score_board/dialog_action_progress.hpp` | 50 | **Delegated read** — hold-action progress dialog |
| `omtk/score_board/loadBoard.sqf` / `loadBoard_ms.sqf` | 33/36 | **Delegated read** — 2-faction / 3-faction scoreboard population |
| `omtk/dynamic_startup/loadPanel.sqf` | 248 | **Delegated read** — interactive spawn/vehicle picker |
| `omtk/dynamic_startup/dialog_interactive_startup.hpp` | 136 | **Delegated read** — that dialog's layout |
| `omtk/warm_up/dialog_timer.hpp` | 68 | **Delegated read** — warm-up countdown HUD |
| `omtk-loadouts/omtk-loadouts.exe` | — | **Delegated read** — Windows binary; see §5/§11 for what could and could not be determined |
| `omtk-loadouts/infantry/*.yml` (52 files) | ~500 ea. | **Delegated read** — the loadout corpus + `{bluefor,redfor}-classes.yml` |
| `omtk-loadouts/vehicles/{bluefor,redfor}-cargos.yml` | 467 ea. | **Delegated read** — vehicle cargo definitions |
| `omtk-loadouts/infantry/README.md` | — | **Delegated read** |
| **Not present** | — | No `mission.sqm`, no `.pbo`, no `stringtable.xml`, no `omtk/wiki/` (README links to `omtk/wiki/img/*` resolve to nothing in this checkout), no CI config, no test directory |

"Delegated read" = read in full by a sub-agent under the same evidence standard; those findings are
attributed inline in §5, §7, §11 and §12.

---

## 1. Identity

| Field | Value | Evidence |
|---|---|---|
| Name | OFCRA Mission ToolKit (OMTK) | `README.md:2` |
| Version | **2.13.7** | `omtk/version.sqf:1` `OMTK_VERSION = "2.13.7";` |
| Release date | **2026-07-03** | `CHANGELOG.md:9` `### V2.13.7 - 2026-07-03`; matches commit date |
| First changelog entry | 2016-08-07 (v2.2.0) | `CHANGELOG.md:252` — so the project predates the changelog; ~10 years of entries |
| Licence | **WTFPL v2** ("DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE") | `LICENSE:1-13` |
| Game | **Arma 3** (SQF, `description.ext`, Eden editor) | `README.md:56` "Create an empty mission with Eden editor" |
| Author org | OFCRA — <http://ofcrav2.org> | `README.md:4`; `description.ext:2` `author = "OFCRA";` |

### What ships as what

**OMTK is not a mod. It is a mission-folder overlay.** There is no `config.cpp`, no `CfgPatches`, no
`.pbo`, no addon source anywhere in the repo. The install procedure is literally "unzip into your
mission folder next to `mission.sqm`":

> `README.md:56-59`
> ```
> 1. Create an empty mission with Eden editor (_load all the required @mods_) and save it in not-binarized format
> 2. Download the [lastest OMTK version on github](https://github.com/ofcrav2/omtk/archive/master.zip).
> 3. Unzip the archive content inside your empty mission folder (_should be something like My Documents\Arma 3\missions\your_mission_name_) aside the file mission.sqm
> 4. Edit the 5-first lines in description.ext file
> ```

So **every mission carries its own private copy of the entire framework**. There is no shared
runtime, no versioning across missions, and no way to patch a framework bug in a mission that has
already been PBO'd. `README.md:64` even instructs the maker to *delete* two of the shipped folders
before playing (see §3 step 9).

### Runtime dependencies — README claim vs. what the code actually calls

`README.md:8` claims:

> "the OMTK relies on *@RHSmod* only, it is plain Arma 3 scripting (even no *@CBA_A3*)"

and `README.md:10` says ACE is optional ("If you use *@ACEmod*, the OMTK is compatible"). **Both
statements are contradicted by the source at HEAD:**

- **CBA is called.** `omtk/ui/pauseScreenMenu.sqf:543`
  `["ocap_exportData", ["Mission is OVER"]] remoteExec ['CBA_fnc_serverEvent', 2];` — the only CBA
  reference, on the admin "Export Ocap/Stats" button.
- **ACE is called unguarded in `rambo_warn`.** `omtk/rambo_warn/main.sqf:56`
  `private _SelfAlive = player call ace_common_fnc_isAwake;` and `:66` for each squad-mate, with no
  `isNil` guard and no `omtk_is_using_ACEmod` check — unlike `dynamic_startup/interactive.sqf:16`
  and `tactical_paradrop/main.sqf:6`, which *do* branch on ACE. `INFERRED:` with the Rambo module
  enabled and no ACE loaded, this loop errors every tick.
- **ACE is also called in** `omtk/library.sqf:409` (`ace_medical_treatment_fnc_fullHealLocal`, the
  admin heal button), `omtk/kill_logger/main.sqf:16,23` (`ace_medical_lastdamageSource`),
  `omtk/view_distance/main.sqf:14` (`"ace_spectator_virtual"`), `omtk/rambo_warn/main.sqf:59,69`
  (`ace_captives_isHandcuffed`).
- **RHS is hard-coded** in `omtk/dynamic_startup/markers.sqf:10,64` (`rhs_Flag_Russia_F`) and
  `omtk/map_exploration/main.sqf:67,70,75,78` (`RHS_UH60M_d`, `rhsusf_m1025_w`,
  `RHS_Mi8mt_Cargo_vvsc`, `rhs_uaz_open_MSV_01`).
- **TFAR** is required for `radio_lock` to do anything — it reads the config property
  `tf_encryptionCode` (`omtk/radio_lock/main.sqf:23`) and compares against
  `tf_west_radio_code` / `tf_east_radio_code` / `tf_independent_radio_code` (`:29,36,43`).
- **Optional integrations, both feature-detected:** OCAP (`omtk/score_board/main.sqf:67`
  `isClass(configFile >> "CfgPatches" >> "ocap")`) and STATSLOGGER
  (`omtk/score_board/library.sqf:134`).

### Repo layout

```
/                          mission-root files the maker edits or that Arma loads directly
  description.ext          lobby params + dialog includes + CfgFunctions      (EDIT)
  init.sqf                 the mission-maker's config block                    (EDIT)
  customScripts.sqf        the maker's own code                                (EDIT)
  onPlayerKilled.sqf       spectator entry                                     (framework)
  onPlayerRespawn.sqf      spectator exit + loadout restore                    (framework)
  loadscreen.jpg           loading image                                       (REPLACE)
  images/                  blue/red/green.jpg = uniform photos    (REPLACE)
                           bluefor/redfor/greenfor.jpg = scoreboard flags (145x103, optional)
  omtk/                    the framework itself — one folder per module, each with main.sqf
  omtk-loadouts/           YAML loadout corpus + omtk-loadouts.exe   (DELETE before play)
  script_library/          copy-paste recipe .txt files             (DELETE before play)
  scripts utiles.txt       French scratchpad, never loaded
  CHANGELOG.md README.md LICENSE
```

Module folders under `omtk/`: `3rd-parties`, `difficulty_check`, `dynamic_startup`, `ia_manager`,
`kill_logger`, `map_exploration`, `radio_lock`, `rambo_warn`, `respawn_mode`, `score_board`,
`tactical_paradrop`, `ui`, `uniform_lock`, `vehicles_thermalimaging`, `view_distance`, `warm_up`,
`zeus_admins` — 17 folders. **Thirteen** of the seventeen carry a `README.md` with a standard
**"Data card"** table (`folder name` / `last modification date` / `Ojective` [sic] / `Default` /
`Extra Parameters`), e.g. `omtk/score_board/README.md:5-11`. That is the framework's own
documentation convention. The four without one are `map_exploration`, `respawn_mode`, `ui` and
`view_distance`.

---

## 2. Mission file layout

A playable OFCRA mission is an Arma 3 mission folder containing:

| File | Origin | Hand-authored? |
|---|---|---|
| `mission.sqm` | **Eden editor** — not in this repo | Yes, entirely, in Eden |
| `description.ext` | Shipped by OMTK | **Partially** — `README.md:59` "Edit the 5-first lines"; the rest (params, includes) is framework |
| `init.sqf` | Shipped by OMTK | **Yes** — this is the mission's config file (§10) |
| `customScripts.sqf` | Shipped by OMTK, empty | **Yes** — the maker's escape hatch |
| `onPlayerKilled.sqf`, `onPlayerRespawn.sqf` | Framework | No |
| `loadscreen.jpg` | Placeholder | Replace (`README.md:60`) |
| `images/blue.jpg`, `red.jpg`, `green.jpg` | Placeholder silhouettes | Replace with real uniform photos |
| `images/bluefor.jpg`, `redfor.jpg`, `greenfor.jpg` | US / (Russian) / (green) flags | Optional replace, 145×103 px (`omtk/score_board/README.md:225`) |
| `omtk/**` | Framework | No |
| `omtk-loadouts/**`, `script_library/**` | Authoring aids | **Deleted before play** (`README.md:64`) |

**Nothing is generated into the mission folder.** `.gitignore` contains only `**/*~` and `*.bak`.
There is no build step, no preprocessor pass, no packer script in the repo. The only file that could
be called a compiler output is the clipboard JSON from `omtk/table_forum.sqf` (§11), which goes to an
external mission manager, not into the mission.

The first five lines the maker edits (`description.ext:1-5`):

```
onLoadName = "MISSION NAME";		// SHOWN AT THE VERY TOP
author = "OFCRA"; 					// SHOWN JUST BELOW LOADNAME
loadScreen = "loadscreen.jpg";		// SHOWN IN THE MIDDLE
onLoadMission = "www.ofcrav2.org";	// SHOWN AT THE BOTTOM
briefingName = "Nom de la mission"; // Name of the mission, shown on the list of missions on the server interface
```

(`briefingName`'s comment is one of the seven surviving French fragments — "Name of the mission".)

**Load order**, read from `init.sqf` and `omtk/load_modules.sqf`:

1. `description.ext` `CfgFunctions` runs three `preInit = 1` libraries — `omtk/library.sqf`,
   `omtk/score_board/library.sqf`, `omtk/warm_up/library.sqf` (`description.ext:259-277`). This is
   why `omtk_log` is callable on `init.sqf:4`.
2. `init.sqf` sets the maker's globals, then `execVM "customScripts.sqf"` (`init.sqf:28`), then
   `execVM "omtk\load_modules.sqf"` (`init.sqf:89`).
3. On clients only (`init.sqf:91`): `briefing.sqf`, `fn_inventoryBriefing.sqf`,
   `fn_rosterBriefing.sqf`, then `sleep 1` and **save the starting loadout**
   (`init.sqf:99-100` `loadout = getUnitLoadout player; player setVariable ["playerLoadout", loadout];`).
4. `load_modules.sqf` gates each module on its lobby parameter, then hands off to warm-up; the
   warm-up's end calls `omtk_load_post_warmup` (`omtk/warm_up/library.sqf:100`), which starts
   thermal-imaging, tactical paradrop and **the scoreboard** (`omtk/load_modules.sqf:25-29`).

That last point is a real design decision: **the mission clock does not start until the warm-up
ends.** `score_board/main.sqf` is not even loaded during warm-up.

---

## 3. Authoring workflow — blank page to playable mission

This is reconstructed from `README.md:53-67` (the stated procedure) plus what each referenced file
actually requires. Steps marked **[README]** are stated in the README; the rest are what the code
forces you to do.

### Step 1 — Build the mission in Eden **[README:56]**

Create an empty mission in the Eden 3D editor with all required mods loaded, and **save it
non-binarized**. Non-binarized is not decoration: `omtk/dynamic_startup/markers.sqf:117` reads
`markerText` off `allMapMarkers` at runtime, and objectives address units by their Eden *Variable
Name* as strings (`omtk/score_board/library.sqf:299` `missionNamespace getVariable [_x, objNull]`).

### Step 2 — Drop OMTK into the folder **[README:57-58]**

Download the master zip, unzip **alongside** `mission.sqm`. No merge, no injection — the framework
is additive files plus the three files it owns (`description.ext`, `init.sqf`,
`onPlayerKilled/Respawn.sqf`).

### Step 3 — Edit the top of `description.ext` **[README:59]**

Five lines: `onLoadName`, `author`, `loadScreen`, `onLoadMission`, `briefingName`
(`description.ext:1-5`).

### Step 4 — Replace `loadscreen.jpg` **[README:60]** and the uniform images

`omtk/briefing.sqf:69-72` embeds `images\blue.jpg` and `images\red.jpg` into a **"Uniforms"** diary
tab, and `:75-80` adds `images\green.jpg` when the three-faction parameter is on. I read all three
shipped files: they are **flat coloured soldier silhouettes**, i.e. placeholders. The maker is
expected to substitute screenshots of the actual side uniforms so players can identify friend from
foe — which is load-bearing, because uniform theft is a bannable rule (§12).

### Step 5 — Place your units, vehicles and objects in Eden **[README:61]**

Nothing OMTK-specific except the naming conventions the framework will later read (§4, §9):

- **Slot naming**: put `Role@SquadName` in each playable unit's *Role Description* field
  (`omtk/fn_rosterBriefing.sqf:84-88`, `omtk/table_forum.sqf:85-88`).
- **Variable Names** on anything an objective will reference (VIPs, bridges, laptops).
- **Trigger names** for INSIDE/OUTSIDE objective zones.
- **`heli01`…`heli04`** for helicopters you want time-locked (`init.sqf:25`), and lock them in Eden.
- **Close every unused playable slot** — `omtk/briefing.sqf:92` "NO AI units. Please close the
  unused slots".

### Step 6 — Author loadouts

The README does not describe this step at all; it only says to *delete* `omtk-loadouts/` at the end.
See §5 for the actual pipeline.

### Step 7 — Write the objectives table in `init.sqf` **[README:62]**

Fill `OMTK_SB_LIST_OBJECTIFS` (`init.sqf:31-33`, empty by default) using the DSL documented in
`omtk/score_board/README.md`. Each row is
`[points, side, type, label, <type-specific params…>]` (`omtk/score_board/README.md:83`). The README
is explicit about the syntax trap:

> `omtk/score_board/README.md:79` — "REMEMBER: no spaces between the lines and commas at the end of
> every line EXCEPT the last one."

That sentence is the clearest single indictment of hand-edited-array authoring in the whole corpus.

### Step 8 — Optional: extra `init.sqf` knobs

Paradrop restriction circles and delays (`init.sqf:7-17`), the mission-duration override
(`init.sqf:20`, commented out), helicopter unlock list and delay (`init.sqf:25-26`), timed-objective
callbacks (`init.sqf:41`), interactive-startup order-of-battle arrays (`init.sqf:43-47`),
`setTerrainGrid 3.125` (`init.sqf:50`), and the **admin Steam64 UID list** (`init.sqf:79`). Full
enumeration in §10.

### Step 9 — Optional: custom scripting

`customScripts.sqf` runs on clients (`customScripts.sqf:1` `if (hasInterface) then {`) and ships
three commented worked examples: setting objective flags (`:9-13`), taking control of an AI hostage
(`:17-34`), and reacting to an item being placed in a container via a `Put` event handler (`:38-48`).
The `script_library/*.txt` recipes are designed to be pasted here — each one opens with
`/* IMPORTANT: READ THIS` and ends with a `/* ---------------- STARTCODE ----------------- */`
fenced block (e.g. `script_library/switch_objective.txt:1,17`).

### Step 10 — Delete `omtk-loadouts/` and `script_library/` **[README:64]**

> `README.md:64` — "9. Delete folders .\omtk-loadouts and .\script_library"

These are authoring-time-only assets (~26 000 lines of YAML plus a Windows executable) that would
otherwise bloat the PBO. **This is manual and unenforced** — there is no packaging script that would
do it for you, and no validator that warns if you forget.

### Step 11 — Test / brief / play

`README.md:66-67`: the same PBO serves three uses — team training (respawn on), in-game briefing on
a live server (Map exploration on), and the real TvT match. The mode is chosen **in the server lobby
at launch time**, not baked into the mission. That is the single most important structural idea in
OMTK: **one artefact, many run modes, selected at launch** (§10).

### What the workflow is missing

There is no step that validates anything. No linter, no schema, no dry-run. A malformed
`OMTK_SB_LIST_OBJECTIFS` row fails at runtime with
`["unkown objective type","ERROR",true] call omtk_log` (`omtk/score_board/library.sqf:198` — note
the typo "unkown", which is in the shipped source) or
`["unknown side for objective creation","ERROR",true]` (`omtk/score_board/main.sqf:147`), i.e. a
line in the RPT and a systemChat message *during the match*.

---

## 4. Slotting / ORBAT model

**OMTK has no slot data model.** Slots are Eden playable units, full stop. What OMTK adds is a set
of *conventions* for how you fill in Eden's existing fields, plus three readers that turn those
conventions into player-facing artefacts.

### Sides

Three, hard-coded to Arma's engine sides throughout:

| OMTK name | Arma side | Colour used in the roster |
|---|---|---|
| `BLUEFOR` | `west` | `#0066CC` (`omtk/fn_rosterBriefing.sqf:95`) |
| `REDFOR` | `east` | `#990000` (`:93`) |
| `GREENFOR` | `resistance` | `#339900` (`:99`) |
| — | `civilian` | `#990099` (`:102`) — roster only, not a playable side |

`omtk/library.sqf:115-123` (`omtk_get_side`) maps only `east`→`"redfor"` and `west`→`"bluefor"` and
logs an error for anything else — so **GREENFOR is a second-class citizen**: it works in the
scoreboard and roster but not in the side-keyed helpers used by paradrop
(`omtk/tactical_paradrop/main.sqf:31`) or the interactive startup
(`omtk/dynamic_startup/interactive.sqf:12`).

Three-faction play is a single lobby switch, `OMTK_MODULE_MEXICAN_STANDOFF`
(`description.ext:143-149`, title "Three factions game", default off). It changes the briefing
uniform tab (`omtk/briefing.sqf:67-81`), the scoreboard dialog
(`omtk/score_board/library.sqf:4-10`, `ScoreBoard` vs `ScoreBoard_MS`) and the winner calculation
(`:97-132`).

### Groups and the `Role@Squad` convention

Squads are Eden groups. Their display name comes from `groupID` **unless** the unit's *Role
Description* contains an `@`, in which case everything after the `@` overrides the group name:

> `omtk/fn_rosterBriefing.sqf:83-89`
> ```sqf
> if(_newGrp != _oldGrp) then {
>     _nbr = (roleDescription _x) find "@";
>     if (_nbr < 0) then {
>         _strGrp = "<br/>" + (groupID(group _x)) + "<br/>";
>     } else {
>         _strGrp = "<br/>" + ((roleDescription _x) select [_nbr + 1]) + "<br/>";
>     };
> ```

and the part **before** the `@` is the role label:

> `omtk/fn_rosterBriefing.sqf:71-81` — role falls back to the unit class `displayName` from
> `CfgVehicles`, is overridden by `roleDescription`, and is truncated at the `@`.

So the authoring convention is: **Role Description = `Squad Leader@Alpha 1-1`**. Arma already shows
the `X@Y` form in its lobby slot list; OMTK reuses it as its ORBAT data format. There is no separate
ORBAT file.

### Rank

Rendered from the Eden rank field into six abbreviations (`omtk/fn_rosterBriefing.sqf:43-68`):
`Pvt.` / `Cpl.` / `Sgt.` / `Lt.` / `Cpt.` / `Maj.` / `Col.` for `rankID` 0–6, defaulting to `Pvt.`.

### The three ORBAT readers

1. **Team Roster diary tab** — `omtk/fn_rosterBriefing.sqf`. Runs on every client, lists **only the
   player's own side** (`:108` `forEach (_unitsArr select {side _x == side player})`), grouped, with
   rank + player name + role. Defaults exclude AI (`:16` `params [["_includeAI",false],…]`, `:33`
   `playableUnits`).
2. **Squad loadout diary tab** — `omtk/fn_inventoryBriefing.sqf`. Runs for `units group player` only
   (`:153`), numbering each man, tagging the leader (`:40` `if(leader _unit == _unit) then {" (Squad Leader)"}`),
   and rendering weapon / attachment / magazine / uniform icons pulled from
   `CfgWeapons >> picture` (`:60`), `CfgMagazines >> picture` (`:99`) and `CfgVehicles >> picture`
   for backpacks (`:117`). Magazines are de-duplicated and shown as `image ×N` (`:135`).
3. **Slot-list JSON export** — `omtk/table_forum.sqf`, an authoring/admin tool (§11).

### How a player picks a slot

Through Arma's stock multiplayer lobby. OMTK does not replace it. What OMTK adds is
**rules about which slot you may take**, enforced socially rather than in code
(`omtk/briefing.sqf:99`, quoted in §12): a squad's non-rifleman slots may not be taken unless the
squad leader slot is taken.

The one exception is `dynamic_startup` "interactive" mode, where a **chief class** unlocks a dialog:

> `omtk/dynamic_startup/interactive.sqf:5`
> ```sqf
> OMTK_DS_CHIEF_CLASSES = ["B_officer_F", "O_officer_F", "B_Soldier_SL_F", "O_Soldier_SL_F"];
> ```

`:15` gates the dialog on `_class in OMTK_DS_CHIEF_CLASSES`. Note these are **vanilla Arma
classnames** — an RHS-unit mission (which is OFCRA's norm, per `README.md:8`) would have no unit
matching this list, so the interactive dialog would never appear. `INFERRED:` this is one reason the
mode is parked under "OLD PARAMETERS" in the lobby (`description.ext:192-205`).

---

## 5. Loadouts / arsenal

This is the most interesting subsystem in OMTK and the one the README says least about. It is an
**offline mission compiler**: YAML in, `mission.sqm` `Inventory` attributes out, zero runtime
footprint.

*(This section is from a delegated full read of `omtk-loadouts/**`, including offline extraction of
the executable. Nothing on Disk_2 was modified and the binary was not executed.)*

### 5.1 `omtk-loadouts.exe` — what it actually is

Measured statically, not inferred: PE32 i386 console binary, 3 256 345 bytes, 9 sections, `.text`
only 18.6 KB, importing **only** `KERNEL32.dll` (44 fns) and `msvcrt.dll` (22 fns). The import set
(`GetTempPathA`, `CreateFileMappingA`, `MapViewOfFile`, `SetEnvironmentVariableA`, `CreateProcessA`,
`WaitForSingleObject`, `RemoveDirectoryA`…) is a dropper's, not a parser's. `strings` yields **zero**
hits for `yml`, `yaml`, `loadout`, `sqf`, `sqm` or `usage`.

It is an **Ocra 1.3.6 self-extracting stub wrapping a Ruby 2.2.0 application** — final 4 bytes
`41 B6 BA 4E` preceded by the overlay offset DWORD `0x00009a00`; the 3 216 921-byte overlay begins
with Ocra opcode `4` (`OP_DECOMPRESS_LZMA`) and an LZMA-alone stream that decompresses to
13 326 664 bytes containing **574 files**, including `bin/ruby.exe`, `libyaml-0-2.dll`, and gems
`liquid-3.0.6`, `ocra-1.3.6`, `rake-10.5.0`, **`sqm2json-0.0.3`**, `thor-0.19.1`.

Recovered bootstrap: `POSTPROC '|\bin\ruby.exe' 'ruby.exe "|\src\bin\omtk-loadouts\omtk-loadouts.rb"'`.
Recovered entry point (whole file, 94 bytes):

```ruby
require File.expand_path('../../../lib/omtk.rb', __FILE__)
OMTK::Command::MainCli.start(ARGV)
```

`src/lib/omtk/version.rb` → `VERSION = '1.0.1'`, matching `CHANGELOG.md:213`
"`~ [omtk-loadouts] upgrade omtk-loadouts.exe to v1.0.1`" (v2.3.3, 2016-09-28). **So the loadout
toolchain has not been touched in ten years** while the SQF framework advanced from 2.3.3 to 2.13.7.

**The CLI** (Thor, `src/lib/omtk/command/main_cli.rb`) — this is the authoring interface nobody
documented in-repo:

| Command | Description (verbatim) |
|---|---|
| `version` | `print tool version and misc. information` |
| `test` | `test if the mission.sqm file and all loadouts .yml files can be parsed successfully, but DOES NOT MODIFY anything` |
| `insert <target>` | `insert loadouts in mission.sqm file about the specified <target>, among {infantry\|vehicles\|all}` |
| `restore` | `restore the original mission file, deleting any changes` |

Options: `--mission-directory/-m` (default `..`), `--classes-bluefor/-cb`, `--classes-redfor/-cr`,
`--infantry-bluefor/-ib`, `--infantry-redfor/-ir`, `--vehicles-bluefor/-vb`, `--vehicles-redfor/-vr`,
`--debug/-d`. Banner: `author: galevsky@gmail.com`.

**Supported SQM versions are hard-coded to `[12, 51, 52]`** (`sqm2json-0.0.3/lib/sqm2json/version.rb:5`),
with a soft warning otherwise: *"SQM version vN in '<file>' is not officially supported (v[12, 51, 52])"* /
*"The loadouts insertion may work but it is not sure !!!"*. `INFERRED:` modern Arma 3 saves a higher
SQM version, so a 2026 mission almost certainly trips this warning.

**`omtk-loadouts/infantry/README.md` is a 1-byte file containing a single newline** — an empty
placeholder. There is no in-repo documentation of the loadout format at all.

### 5.2 The loadout YAML schema

Top level is a flat map of `role-name → gear map`, with one reserved key `default`. No version, no
header, no metadata — **the filename is the only identity**. Each role is **deep-merged onto
`default`**; nested maps recurse, arrays concatenate and are then coalesced (duplicate quantities
summed, re-emitted sorted as `"Nx item"`).

Field set (occurrence counts measured over all 57 files):

```
<role>/
  weapons/
    primary/    name · optics · muzzle · underBarrel · magazine
    launcher/   name · magazine · optics
    handgun/    name · magazine
  uniform/      name · magazines[] · items[]
  vest/         name · magazines[] · items[]
  backpack/     name · magazines[] · items[]      (explicit-null 579×)
  binocular · map · compass · watch · radio · goggles · headgear      (scalars)
```

Two supported-but-unused features: a `face` key (read by the tool, written as an Eden
`CustomAttributes` identity override — **0 occurrences**), and an `"a||b||c"` random-alternative
scalar syntax picked at random per unit — **0 occurrences of `||` in the whole corpus**.

**Value grammar** — one regex does everything:

```ruby
/^((?<quantity>[0-9]+)x )?((?<rounds>[0-9]+)#)?(?<item>.*)$/
```

So `"8x 30#rhs_30Rnd_545x39_7N10_plum_AK"` = **8 magazines each loaded with 30 rounds**. Bare
`ACE_EarPlugs` = quantity 1. **An empty value is meaningful**: `optics:` with nothing after it parses
as YAML `nil` and *suppresses* the inherited default. That is the framework's balance verb (§5.6).

Representative `default` block, verbatim
(`omtk-loadouts/infantry/redfor-loadouts-ru-flora.yml:1-39`):

```yaml
  1  default:
  2    weapons:
  3      primary:
  4        name: rhs_weap_ak74m_fullplum
  5        optics: rhs_acc_1p63
  6        muzzle: rhs_acc_dtk
  7        underBarrel:
  8        magazine: 30#rhs_30Rnd_545x39_7N10_plum_AK
  9      launcher:
 10        name:
 11        optics:
 12      handgun:
 13        name:
 14        magazine:
 15    uniform:
 16      name: rhs_uniform_vdv_flora
 17      magazines:
 18        - rhs_mag_rgd5
 19        - SmokeShell
 20      items:
 21        - 8x ACE_fieldDressing
 22        - 4x ACE_morphine
 23        - ACE_EarPlugs
 24        - ACE_CableTie
 25        - 4x ACE_splint
 26        - 2x ACE_epinephrine
 27        - 2x ACE_tourniquet
 28    vest:
 29     name: rhs_6b23_rifleman
 30    backpack:
 31      name: rhs_sidor
 32      magazines:
 33      items:
 34    binocular:
 35    map: ItemMap
 36    compass: ItemCompass
 37    radio: ItemRadio
 38    watch: TFAR_microdagr
 39    headgear: rhs_6b26_ess
```

and a role override on top of it (`:42-67`, the side commander `cdc`) which swaps the 1P63 red dot
for a 1P29 4× optic, adds a pistol, map tools, a `TFAR_mr3000_rhs` long-range radio backpack,
binoculars and a VDV beret.

### 5.3 `bluefor-classes.yml` / `redfor-classes.yml` — the join table

A "class" here binds an **OMTK role id** to a **stock Arma 3 unit classname**:

```yaml
  1  cdc:
  2    class: B_officer_F	              # NATO\Men\Officer
  3    description: Side leader
  4    rank: COLONEL
```
(`omtk-loadouts/infantry/bluefor-classes.yml:1-4`)

The join **runs backwards**. The maker places a stock Arma unit in Eden; the tool reads its `type`
from the SQM and does a reverse lookup by `class` value, returning the role key, which then indexes
the loadouts file:

```ruby
def get_MTK_class(arma3ClassName, side)
  klasses = conf["classes-#{get_color(side)}".to_sym]
  klass = klasses.select{ |k,v| v['class'] == arma3ClassName }
```

> **The Arma unit type is a token, not a unit choice.** `B_diver_TL_F` (a *diver*) means "crew
> chief"; `B_recon_LAT_F` means "HMG gun-bag carrier". The mission maker is placing *symbols*.

Side effects the tool applies while walking the SQM: sets `Attributes[:rank]` (default `PRIVATE`),
sets `Attributes[:description]` from the class file, **re-sorts every group's units by rank
descending** so leaders sit at the top of the lobby slot list, and force-writes
`(group this) setGroupId ["<CALLSIGN>"];` into the unit init from the group's Eden `groupID`.
A unit whose init contains `this setVariable ["omtk_ignore", 1];` is **skipped entirely**.

### 5.4 The 28 roles

Identical key set in both class files and in **all 57 loadout files** (verified programmatically).
Three ids are French abbreviations:

| id | Expansion | Role | BLUEFOR class | REDFOR class | rank |
|---|---|---|---|---|---|
| `cdc` | *commandant de compagnie* | Side leader | `B_officer_F` | `O_officer_F` | COLONEL |
| `cdg` | *commandant de groupe* | Squad leader | `B_Soldier_SL_F` | `O_Soldier_SL_F` | MAJOR |
| `cde` | *commandant d'équipe* | Team leader | `B_Soldier_TL_F` | `O_Soldier_TL_F` | CAPTAIN |
| `medic` | | Combat life saver | `B_medic_F` | `O_medic_F` | |
| `grenadier` / `grenadier_assistant` | | GL + ammo bearer | `B_Soldier_GL_F` / `B_Soldier_A_F` | `O_…` | |
| `autorifleman` / `autorifleman_assistant` | | LMG + assistant | `B_soldier_AR_F` / `B_soldier_AAR_F` | `O_…` | |
| `gunner` / `gunner_assistant` | | MMG/GPMG + assistant | `B_support_GMG_F` / `B_support_AMG_F` | `O_…` | |
| `antitank` / `antitank_assistant` | | Reloadable AT + assistant | `B_soldier_AT_F` / `B_soldier_AAT_F` | `O_…` | |
| `antitank_light` | | Disposable AT | `B_soldier_LAT_F` | `O_Soldier_LAT_F` | |
| `antipersonnel_light` | | Disposable HE | `B_G_Soldier_F` (FIA) | `O_soldierU_F` | |
| `marksman` / `sniper` / `spotter` | | DMR / sniper / spotter | `B_soldier_M_F` / `B_sniper_F` / `B_spotter_F` | `O_…` | |
| `explosive_specialist` | | Demo | `B_soldier_exp_F` | `O_soldier_exp_F` | |
| `rifleman` | | Rifleman | `B_Soldier_F` | `O_Soldier_F` | |
| `driver` | | **Crew chief** | `B_diver_TL_F` | `O_diver_TL_F` | MAJOR |
| `ground_crew` / `pilot` / `air_crew` | | Crew / pilot / air crew | `B_crew_F` / `B_Helipilot_F` / `B_helicrew_F` | `O_…` | pilot MAJOR |
| `op_drone` | *opérateur drone* | UAV operator | `B_soldier_UAV_F` | `O_soldier_UAV_F` | |
| `op_radio` | *opérateur radio* | Radio operator | `B_recon_JTAC_F` | `O_recon_JTAC_F` | |
| `recon` | | Recon rifleman | `B_recon_M_F` | `O_recon_M_F` | |
| `turret_gun_carrier` / `turret_tripod_carrier` | | HMG gun bag / tripod bag | `B_recon_LAT_F` / `B_recon_F` | `O_…` | |

### 5.5 The 57 faction files

**28 BLUEFOR + 29 REDFOR**, plus the two class files. Convention:
`{side}-loadouts-{faction}[-{camo|era}][-version-{weapon|letter}|-sans-ws-uniforms].yml`.

- `-version-<weapon>` = weapon-system swap (`-version-rpk`, `-version-m249`, `-version-m27IAR`,
  `-version-ak12`, `-version-m4`, `-version-m16`)
- `-version-a` / `-version-b` = whole-kit alternative (only `onu`)
- `-sans-ws-uniforms` = French *sans*, "without" — a **mod-dependency reduction** variant that
  swaps third-party clothing for RHS/RHSGREF equivalents, weapons untouched
- `insurgé` = French "insurgent"; `moderne` = "modern"; `ONU` = French for **UN**;
  `moutain` is a misspelling of "mountain"

BLUEFOR groups: US modern (OCP/UCP × M4/M16), USMC (90s, desert/MARPAT × M249/M27 IAR), US 2035
(+ Apex), European (BAF DPM, German Flecktarn, French CE, Danish, Finnish G3 ×2, Swedish ×2, Swiss,
Turkish), other (Aussie, Altis Lizard, CDF ×2, merc NATO, ONU A/B).
REDFOR groups: Russian (flora ×2, mountain ×2, summer modern ×2, VSR, Soviet 90s), CSAT/AAF 2035,
insurgent/irregular (insurgé est ×3, Taliban, FIA ×2, merc est), generic armies (green ×2, desert,
urban, Takistan, ch-vert), PLA ×2, other (GAF, SAF, Iran DPM).

**No GREENFOR loadout file exists**, although the tool defines the side constant
(`bluefor→WEST`, `redfor→EAST`, `greenfor→RESISTANCE`).

**Trap:** the CLI defaults are `./infantry/bluefor-loadouts.yml` and `./infantry/redfor-loadouts.yml`
— **neither exists**. `CHANGELOG.md:218-219` records the renames that broke them. The maker must
pass `-ib`/`-ir` explicitly, and nothing tells them so.

### 5.6 Vehicle cargos

A "cargo" is a vehicle's pre-stocked resupply pool, not a unit loadout. Both files have the same
14 keys: `common` plus `car`, `truck`, `apc`, `mrap`, `ifv`, `mbt`, `artillery`, `anti-aircraft`,
`heli_transport`, `heli_attack`, `plane_transport`, `plane_attack`, `boat`.

```yaml
  1  common:
  2    cargo:
  3      weapons:
  4      ammo:
  5      items:
  6        - 5x ACE_fieldDressing
  7        - ACE_epinephrine
  8        - ACE_morphine
  9      backpacks:
 12  car:
 13    classes:
 14      - rhsusf_m1025_d
```
(`omtk-loadouts/vehicles/bluefor-cargos.yml:1-14`)

Only entities whose SQM side is `EMPTY` are considered. `common.cargo` is concatenated onto the
type's cargo and emitted positionally as `weapons, ammo, items, backpacks`, injected as an Eden
custom attribute `ammoBox` with expression `'[_this,_value] call bis_fnc_initAmmoBox;'`.

> **Hard rule:** any `EMPTY` vehicle whose classname is **not** listed in either file has its cargo
> **forcibly emptied**. That is deliberate — an unregistered vehicle is a zero-supply vehicle. The
> escape hatch is again `this setVariable ["omtk_ignore", 1];` in the Eden init.

`backpacks:` is declared in both `common` blocks and **never populated by any vehicle type**.

### 5.7 Balance rules encoded in the loadout data

These are enforced by data, not by script — and they are the real OFCRA rulebook.

1. **Night vision is prohibited to infantry, absolutely.** Zero NVG classes in any of the 57
   infantry files. Every NVG in the corpus (`ACE_NVG_Gen4`) sits in a *vehicle*, tiered by platform:
   50 in a C-130, 15 in a truck or transport helo, 10 in an APC/MRAP/IFV, 8 in a car or boat, 3 in
   an MBT or SPG, 2 in an attack aircraft. **If you want NVGs you must go to a vehicle and draw
   them.**
2. **Binoculars are gated.** `default.binocular` is explicitly null in **57/57** files. Granted only
   to `cdc`, `cdg`, `cde`, `marksman`, `sniper`, `spotter`, `recon`, `op_drone` (and `driver`, 56/57).
3. **Long-range radios are gated to four roles.** TFAR backpack radios go only to `cdc`, `cdg`,
   `op_radio` (50/57 each) and `spotter` (39/57). `cde` **never** gets one (0/57). Crew, pilots and
   air crew have `backpack:` explicitly nulled in **57/57**, so they physically cannot carry one.
   Everyone gets a personal `ItemRadio` and a `TFAR_microdagr` as their watch (57/57).
4. **One launcher per launcher role, three spare rockets, three different warheads.** `antitank`
   carries a reloadable launcher in 57/57 with 3 spares in 48 files / 2 in 7. Example
   (`redfor-loadouts-ru-flora.yml:225-236`): RPG-7 + `PG7VL` tandem HEAT loaded, spares
   `PG7VL` + `PG7VR` + `OG7V` frag — you cannot stockpile one warhead type.
   `antitank_light` / `antipersonnel_light` get **disposable** tubes with `backpack:` nulled
   (49/57 and 50/57) — one shot, no bag, no second tube. AP vs AT is enforced by warhead
   (`rshg2` thermobaric / `M136_hedp` vs `rpg26` / `M136`).
5. **Rifle ammunition is capped and side-asymmetric.** `rifleman` vest magazines: **12× in 32 files**
   (Western 5.56/7.62 NATO), **8× in 10** (Russian regulars), **7× in 5** and **5–6× in 3**
   (insurgents/mercs). Support roles get a flat 8×.
6. **Belt-fed ammo is split across the gun team.** `autorifleman` = 1 loaded + 1 vest + 2 pack;
   `autorifleman_assistant` = 3 more belts + 8 rifle mags + an entrenching tool. Same for the MMG
   pair. A static HMG costs two rifles: `turret_gun_carrier` carries the gun bag,
   `turret_tripod_carrier` the tripod, and neither gets a launcher.
7. **Optics are role-gated.** The base soldier's optic is explicitly null in 18/57 files and a
   non-magnified red dot in the rest. Magnified optics appear only as role overrides —
   `cdc`/`cdg` 1P29, `gunner` 1P78, `marksman` PSO-1M2, `sniper` DH 5-20×56, `recon` PSO-1M21.
   Crew get a short carbine with `optics:` explicitly blanked — deliberately bad.
8. **A universal ACE medical baseline.** In 43/57 files `default.uniform.items` is byte-identical:
   8× field dressing, 4× morphine, ear plugs, cable tie, 4× splint, 2× epinephrine, 2× tourniquet.
   The medic multiplier is equally standardised — **57/57** give the medic 25× morphine,
   25× epinephrine, 10× splint, plus 40–50× field dressing and 6–10× blood IV.
9. **Two grenades, in the uniform, for everyone** — one frag + one smoke in 55/57 files.
10. **Per-side vehicle-supply asymmetry.** REDFOR `common` adds 2 frags + 2 smokes to *every*
    vehicle; BLUEFOR's is empty. BLUEFOR is the **only** side stocking laser/IR designators
    (`rhsusf_acc_anpeq15`, 8–50 per vehicle) — REDFOR gets zero anywhere. BLUEFOR trucks stock demo
    charges, satchels, a defusal kit and a clacker; REDFOR trucks stock none of those. BLUEFOR boats
    carry 8 NVGs, REDFOR boats 4.
11. **Explicit-null is used 579 times** to strip the default rucksack — crew 57/57, riflemen 47/57,
    marksman 47, sniper 52, recon 56. This is the mechanism that stops non-designated roles carrying
    extra ammo or a radio.

### 5.8 Variant diffs — what `-version-rpk` really changes

`diff -u redfor-loadouts-ru-flora.yml redfor-loadouts-ru-flora-version-rpk.yml` → **exactly two
hunks**, 497→500 lines. Uniforms, helmets, vests, rifles, launchers, medic supplies and radios are
byte-identical. Only the four automatic-weapon roles change:

```diff
 autorifleman:
   weapons:
     primary:
-      name: rhs_weap_pkm
-      magazine: 100#rhs_100Rnd_762x54mmR
+      name: hlc_rifle_rpk74n
+      magazine: 45#hlc_45Rnd_545x39_m_rpk
+      optics:
+      muzzle:
```

The two **added empty keys** are load-bearing — they null the inherited 1P63 optic and DTK muzzle
device, which the RPK cannot mount. Net effect: the squad automatic weapon moves from a belt-fed
7.62×54R PKM (400 rounds in 4 belts) to a magazine-fed 5.45 RPK-74N (450 rounds in 9 magazines,
sharing the riflemen's ammunition class), and the MMG's belts are upgraded ball → 7N26 AP as
compensation. **`-version-rpk` is a squad-automatic-weapon doctrine switch**, not a cosmetic one.

By contrast `-version-m4` vs `-version-m16` differs by **a single line** (an assistant's belt type),
i.e. the advertised rifle swap is missing — an unfinished variant.

### 5.9 Defects in the loadout data

Real bugs where the author's intent is silently discarded, all verified against the consuming Ruby:

- **Six keys mis-indented to column 0**, becoming orphan top-level entries that nothing reads:
  `bluefor-loadouts-cdf-camo.yml:127,128,150`, `bluefor-loadouts-cdf-camo-version-rpk.yml:126,148`,
  `redfor-loadouts-gaf.yml:123,144`. Those `cde`s get no binoculars and wear the default helmet; one
  classname is also misspelled (`hsgref_` for `rhsgref_`).
- **Five `medic` blocks put `name`/`magazine` directly under `weapons:`** instead of
  `weapons: primary:`, so the intended rifle is dropped —
  `redfor-loadouts-insurgé-est.yml:128-130` and three siblings, `redfor-loadouts-saf.yml:141-142`.
- **Five `sniper` blocks nest `headgear` under `backpack:`**, discarding the hat *and* accidentally
  un-clearing the rucksack — `bluefor-loadouts-us-ocp-version-m4.yml:340-341` and four siblings.
- **Two vehicle types have empty `classes:` lists** so their cargo can never fire:
  `bluefor-cargos.yml:276` (`anti-aircraft`) and `redfor-cargos.yml:417` (`plane_transport`).

### 5.10 There is no in-game arsenal

No Virtual Arsenal, no crate GUI, no gear-selection screen. A player's kit is fixed at slot
selection. The only kit mutation at runtime is the framework's own snapshot/restore (§8) and the
admin "reset inventory" button (§11). Uniform swapping is separately regulated (§12).

### 5.11 Confirmed: no runtime YAML consumer

- `.yml` / `.yaml` appear in **no** `.sqf` or `.ext` in the repo.
- `getUnitLoadout`/`setUnitLoadout` appear only in respawn plumbing (`init.sqf:99-100`,
  `onPlayerRespawn.sqf:2-3`, `omtk/library.sqf:382,389,400-401`) and re-apply whatever the engine
  handed the player from the SQM.
- `OMTK_LOADOUT` exists only in one dead comment, `init.sqf:66`.
- The tool *does* contain a Liquid template that would emit
  `omtk/infantry_loadouts/generated_loadouts.sqf`, but **both call sites are commented out** in
  `main_cli.rb:76,87`, no `--script` option is declared, and the module directory it targets does not
  exist. Dead code. A `Roster` Thor subcommand exists but is never registered and calls an undefined
  class.

---

## 6. Briefing / intel

OMTK's briefing is **entirely diary-record based** — Arma's map-screen "Briefing" notebook — and it
is **mostly auto-generated**. `omtk/briefing.sqf` is run on clients only (`init.sqf:91-93`).

### Tabs the framework creates, in order

| Tab | Source | Content |
|---|---|---|
| *(objective tasks)* | `omtk/briefing.sqf:5-33` | One `createSimpleTask` per objective in `OMTK_SB_LIST_OBJECTIFS`, filtered by side, with `setSimpleTaskDescription [ "<points> points", <label>, "Texte 3" ]` (`:16`). `"Texte 3"` is an untranslated French placeholder still shipping. |
| **Crédits** | `:61` | `"Mission réalisée avec l'OMTK"` — *"Mission made with the OMTK"* |
| **Donations** | `:63` | Link to ofcrav2.org donate page |
| **Mission Timings** | `:65` | Start and end clock times, green `size='30'` |
| **Uniforms** / **UNIFORMS** | `:69-81` | `images\blue.jpg` + `images\red.jpg` (+ `green.jpg` if three-faction) at `width='200'` |
| **Rules** | `:84-103` | The OFCRA rulebook, verbatim — see §12 |
| **Team Roster** | `omtk/fn_rosterBriefing.sqf:111` | Own side's full ORBAT |
| **Squad loadout (`<group>`)** | `omtk/fn_inventoryBriefing.sqf:161` | Own squad's per-man kit with icons |

`INFERRED:` (engine behaviour, not verified in-game) because `createDiaryRecord` prepends, the
records created last — Team Roster and Squad loadout — sit above the Rules tab, and the Rules tab
sits above everything created before it in `briefing.sqf`. Either way, the rules are near the top of
the map-screen notebook rather than buried.

### What the mission maker writes

Nothing, by default. There is **no `briefing` field, no situation/mission/execution template, and no
markdown**. The maker's only supported hook for narrative intel is to append diary records from
`customScripts.sqf`. The shipped recipe is `script_library/receive_intel.txt`, a `BIS_fnc_holdActionAdd`
on an object named `Intel` that grants a diary record on completion:

> `script_library/receive_intel.txt:10-16`
> ```sqf
> getIntel = {
> 	_side = _this select 0;
> 	if (side player == _side) then {
> 		player createDiaryRecord ["Diary", ["LOCATION", "The thing is at coordinates 015 085"]];
> 		titleText ["Intel added in 'Briefing'", "PLAIN DOWN"];
> 	};
> };
> ```

That is the entire intel model: **intel is a diary record you unlock by standing next to a laptop for
10 seconds** (`:30` — action duration 10 s, `:23` — condition `_this distance _target < 3 && side _this == west`).

### Markers

Map markers are placed in Eden and are **explicitly legal to use in play** — `omtk/briefing.sqf:95`
"Markers on map are authorized". Two marker facilities exist beyond stock Arma:

- **Side-local markers** — not a module, just a recipe in the French scratchpad
  `scripts utiles.txt:5-22`: hide every marker with `setMarkerAlphaLocal 0`, then re-show the ones
  for the player's own side. Marker names in the example are `spawnru/spawnus/mkus/mkru/…`.
- **Live vehicle-tracking markers** — `script_library/update_markers.txt`. A side leader triggers a
  20-second hold action on an object named `Updater` (or an ACE self-action) and the markers
  `Veh1_Mrkr`, `Veh2_Mrkr`… snap to the current positions of `Veh1`, `Veh2`…
  (`script_library/update_markers.txt:12-25`), locally, for that side only (`:19`
  `if ( side player == _side )`). The recipe ships with 2 vehicles wired and 2 commented out
  (`:16-17,22-23`) and tells the maker to uncomment lines to scale up.

`INFERRED:` that second recipe is an unusually good piece of milsim design — it makes recon a
*resource* (someone must physically go to the command laptop and spend 20 seconds) rather than an
always-on GPS feed. It is also, tellingly, not a module: it is a text file you paste.

### Sound / broadcast

`script_library/play_sound.txt` — a `CfgSounds` block for `description.ext` plus a one-shot
broadcast hold-action on an object named `Controller`, playing through an object named `loudspeaker`
via `say3D` (`:31`). The comment is candid about its own quality: `:2` "500 is supposedly the
distance, but i don't know which one is the correct one so just change both."

---

## 7. Objectives / game modes

### The mode model

There is **one game mode**, declared `gameType=TDM` in `description.ext:20`, with
`minPlayers = 1; maxPlayers = 200;` (`:21-22`). Everything else is a *run mode* selected in the
lobby (§10) — training, briefing/recon, or match — over the same mission file.

Win condition is **points at a fixed wall-clock deadline**. There is no sudden death, no early win,
no round system, no attacker/defender asymmetry beyond what the maker encodes in objectives.

### The objective DSL

Objectives live in one global array in `init.sqf`. Row shape
(`omtk/score_board/README.md:83`):

```
[points, side, objective_type, objective_label, specific_parameters…]
```

- **points** — integer, "can be negative to behave like a penalty" (`omtk/score_board/README.md:87`)
- **side** — one of `"BLUEFOR"`, `"REDFOR"`, `"GREENFOR"`, `"BLUEFOR+REDFOR"`, `"BLUEFOR+GREENFOR"`,
  `"REDFOR+GREENFOR"`, `"BLUEFOR+REDFOR+GREENFOR"` (`:88`; implemented at
  `omtk/score_board/main.sqf:84-149`). The combined forms **duplicate the objective**, one copy per
  side, so a "capture zone" is authored once and scored per side.
  `omtk/score_board/README.md:180` warns: "The order MUST be the one shown here. Inverting green
  with red or blue would break it." — and that is true of the code: `main.sqf` matches the literal
  string.
- **objective_type** — the README lists `SURVIVAL | DESTRUCTION | INSIDE | OUTSIDE | ACTION | FLAG`
  (`:89`). The evaluator in `omtk/score_board/library.sqf:150-200` additionally implements
  `TRIGGER` (an empty stub, `:171-173`) and the four timed variants `T_SURVIVAL`, `T_DESTRUCTION`,
  `T_INSIDE`, `T_OUTSIDE` (`:186-195`). `ACTION_DISPUTEE` appears as an empty `case` in
  `omtk/score_board/main.sqf:318-320` — French for "contested action", dead code.

#### Subject selector `[MODE, VALUES]`

Used by SURVIVAL / DESTRUCTION / INSIDE / OUTSIDE. Modes (`omtk/score_board/README.md:100`, implemented
in `omtk_isAlive` `library.sqf:394-468` and `omtk_isInArea` `:261-350`):

| MODE | VALUES | Meaning |
|---|---|---|
| `"BLUEFOR"` / `"REDFOR"` / `"GREENFOR"` | number | at least N of that side survive / are in the zone (`library.sqf:396-413`, `:262-273`) |
| `"DIFF"` | number | that side has ≥N **more** than *each* other side (`library.sqf:282-290`) |
| `"LIST"` | array of Variable Names (or map-object IDs) | **all** listed objects must satisfy the condition — "there is no OR condition" (`README.md:109`); implemented as an AND fold at `library.sqf:309-314` and `:436-441` |
| `"OMTK_ID"` | array of numbers | documented at `README.md:112-115` as matching `this setVariable['OMTK_ID',12345];` |
| `"MT_ID"` | array | undocumented; `omtk_isInArea` only (`library.sqf:326-345`) |

> **Confirmed defect.** `omtk/score_board/README.md:113` instructs the maker to write
> `this setVariable['OMTK_ID',12345];` in the unit's init field, but the evaluator reads a
> *different* variable: `omtk/score_board/library.sqf:447` `_id = _x getVariable ["mt_id", ""];`.
> The `MT_ID` branch in `omtk_isInArea` (`:329`) reads `mt_id` too. As shipped, the documented
> `OMTK_ID` workflow cannot match anything.
>
> **Second defect in the same branch.** `library.sqf:443-462`: `_res` starts `true`; for every unit
> whose id matches, `if (_r) then {_res = false;};` where `_r = alive _target` — and the mode
> inversion is commented out on `:453` (`//if (_mode < 1) then { _r = !_r; };`). So `OMTK_ID`
> behaves as DESTRUCTION regardless of whether the objective was declared `SURVIVAL` or
> `DESTRUCTION`.

#### The three-faction capzone rule

`"DIFF"` is evaluated against **both** rivals simultaneously (`library.sqf:282-290`), so a side wins
a contested zone only by out-numbering *everyone*. The README spells out the truth table
(`omtk/score_board/README.md:186-199`), including the honest admission that a two-way
`REDFOR+GREENFOR` capzone gives `NOBODY WINS` in a case where "technically green should win" (`:198`).

#### FLAG objectives — the scripting escape hatch

`[points, side, "FLAG", label, [[flagNumber, initialState], …]]`. The maker flips them from anywhere:

> `omtk/score_board/README.md:171-172`
> ```sqf
> [1, true] call omtk_setFlagResult; // set flag 1 to true
> [2, false] call omtk_setFlagResult; // set flag 2 to false
> ```

`omtk_setFlagResult` (`library.sqf:504-512`) writes into the public `sb_f` array and
`publicVariableServer`s it — the v2.10.9 changelog entry records the bug this fixed
(`CHANGELOG.md:58` "flag objectives would not be synced across client causing issues if multiple
flag objectives"). Flag numbers are array indices, documented range **1–10**
(`omtk/score_board/README.md:133`).

`script_library/switch_objective.txt` is the canonical FLAG recipe: a laptop object named `Obj` with
two side-gated 10-second hold actions; whoever completes theirs sets flag 1 true / flag 2 false or
vice versa (`:28-29,43-44`), and two coloured airport lights named `BLight_1` / `RLight_1` are
teleported to physically show ownership (`:25-26`), with the red one parked "far away outside the
playable area" at start (`:5`). It also ships a harder variant requiring a `jamTruck` or `jamBackup`
vehicle within 50 m to even attempt the capture (`:113-116`).

#### Timed objectives

`[points, side, "T_…", label, [flagNumber, minutes], …]` (`omtk/score_board/README.md:131`). A
scheduler on the server (`omtk/score_board/main.sqf:188-242`) polls every 5 s and, when
`dayTime >= gameStart + minutes/60`, evaluates the objective once and latches the result into the
flag. Semantics are "true at the checkpoint, true forever" — `README.md:135`: "If he dies after this
time has elapsed, the objective will still be considered completed (VIP saved)." Result is broadcast
to everyone at the moment it fires:

> `omtk/score_board/library.sqf:371,374`
> ```sqf
> ("[OMTK] OBJ " + _label + " COMPLETED BY " + _sideStr + ".") remoteExecCall ["systemChat"];
> ("[OMTK] OBJ " + _label + " FAILED BY " + _sideStr + ".") remoteExecCall ["systemChat"];
> ```

Duplicated `BLUEFOR+REDFOR` timed capzones get the mirror flag at `flagNumber + 10`
(`omtk/score_board/main.sqf:107-111`), which is why flag numbers are capped at 10.

**Timed-objective callbacks** (v2.13.6, `CHANGELOG.md:13`) let the maker run arbitrary server code
when a timed objective fires — `OMTK_TIMED_OBJECTIFS_CALLBACKS set [1, { … }]`, dispatched at
`omtk/score_board/main.sqf:227-235`. `init.sqf:40` warns: "The code defined in here is executed on
server only, make sure to account for locality!"

#### ACTION objectives

`[points, side, "ACTION", label, targetObjectOrVariableName, duration, code]`. Clients of the owning
side get an `addAction` on the target (`omtk/score_board/main.sqf:306-316`); completing it shows a
10-step progress dialog (`library.sqf:526-536`) and sets the objective result directly. Note
`library.sqf:536` `// TODO Display progress bar using _dur, then..` — the `_dur` parameter is
accepted but ignored; the bar is always 10 seconds.

### Scoring and end of mission

`omtk/score_board/main.sqf:38-70` spawns the master timer: a `"20 Minutes Left"` hint
(`:48`), then at `_gameEnd` it calls `omtk_sb_compute_scoreboard`, waits 2 s, calls
`omtk_sb_start_mission_end`, waits 10 s and triggers the OCAP export if OCAP is loaded (`:67-69`).
There is explicit handling for a mission that crosses midnight (`:51-61`).

Winner determination (`omtk/score_board/library.sqf:96-132`) is strict-greater-than on total points,
otherwise `"DRAW"`. In the three-faction branch, a case where one side ties another but beats the
third falls through to a comment that ships in production:

> `omtk/score_board/library.sqf:127` — `// Too lazy to implement the else, let's hope it won't happen`

**Survivor counting is broken.** `omtk/score_board/library.sqf:37-54` iterates `allPlayers`,
computes `_dmg = damage _x` on `:41` — and then never uses it, testing `damage player` instead on
`:44`, `:47` and `:50`:

```sqf
_dmg = damage _x;

if(_side==east) then {
    if ((damage player) < 0.975) then { [omtk_sb_redfor_survivors, _name] call BIS_fnc_arrayPush; };
```

This runs on the server, where `player` is not the unit being tested. Survivor lists feed the
`BLUEFOR`/`REDFOR`/`GREENFOR`/`DIFF` "supremacy" objectives (`library.sqf:396-419`), so the defect
propagates into scoring. The 0.975 threshold itself is a deliberate rule —
`CHANGELOG.md:209`: "survivors in objectives are now restricted to players (no IA) whose life is
below 0.975 (unconscious are not survivors anymore)".

---

## 8. Respawn / tickets / medical / revive

### The default: one life, then spectate

`description.ext:10-11`:

```
respawn = "BASE";
respawnDelay = 999999;
```

with `respawnOnStart = -1;` (`:7`), `Debriefing = 0;` (`:6`) and `Saving = 0;` (`:8`).
`corpseLimit = 999; wreckLimit = 999;` (`:12-13`) keep bodies and wrecks on the field for the whole
match — added in v2.7.3 (`CHANGELOG.md:169`).

On death, `onPlayerKilled.sqf` puts the player into **BIS EG Spectator** with every feature enabled:

> `onPlayerKilled.sqf:10`
> ```sqf
> ["Initialize", [player, [], true, true, true, true, true, true, true, true]] call BIS_fnc_EGSpectator;
> ```

and the `OMTK_MODULE_SPECTATOR` parameter (`description.ext:213-219`, values `all` / `team`) chooses
between an unrestricted spectator (empty side filter) and one restricted to the dead player's own
side (`onPlayerKilled.sqf:12-16`). Free camera is enabled globally in `init.sqf:64`
(`RscSpectator_allowFreeCam = true;`). `onPlayerKilled.sqf:4-7` re-enables user input first, with
the comment `// Re-enable input if player is dead (acebug workaround)`.

### Respawn as a *training* switch

`OMTK_MODULE_RESPAWN_MODE` (`description.ext:206-212`) offers
`no-respawn | 3 s | 30 s | 1 min | 1 min 30 s | 2 min | 3 min | immortal`, values
`{999999,3,30,60,90,120,180,-1}`, **default 999999 (no respawn)**. `omtk/load_modules.sqf:44` only
loads the module when the value is `< 999999`, and `omtk/respawn_mode/main.sqf:5-10`:

```sqf
if (_value > 0) then {
  setPlayerRespawnTime (_value);
} else {
  { _x allowDamage false; } forEach allUnits;
  { _x allowDamage false; } foreach vehicles;
};
```

So "immortal" (`-1`) is not a respawn setting at all — it disables damage on every unit and vehicle.
`README.md:96` explains the intent: "useful to let your invitees test the @mods and check that they
can connect to your server".

### Loadout on respawn

`init.sqf:99-100` snapshots `getUnitLoadout player` one second after mission start into the unit
variable `playerLoadout`; `onPlayerRespawn.sqf:2-3` restores it:

```sqf
loadout = player getVariable ["playerLoadout", 0];
player setUnitLoadout [loadout, true];
```

The same snapshot backs two admin buttons — `omtk_respawn_unit` (`omtk/library.sqf:378-394`, which
temporarily sets `setPlayerRespawnTime 2`, sleeps 3, restores 9999, then re-applies the loadout) and
`omtk_reset_unit` (`:396-403`, restore kit without dying).

### Medical and revive — none

**OMTK ships no medical system, no revive, no bleed-out, no tickets.** It defers entirely to ACE and
only *reads* ACE state:

- `omtk/library.sqf:409` — the admin full-heal button calls `ace_medical_treatment_fnc_fullHealLocal`
- `omtk/kill_logger/main.sqf:16,23` — resolves a self-kill to the real killer via
  `ace_medical_lastdamageSource`
- `omtk/rambo_warn/main.sqf:56,66` — treats incapacitated players as "not alive" via
  `ace_common_fnc_isAwake`
- `omtk/score_board/library.sqf:44` — the 0.975 damage threshold is the framework's own proxy for
  "unconscious does not count as a survivor"

**There is no ticket system of any kind.** Searching the corpus for ticket/reinforcement vocabulary
returns nothing. Attrition is expressed instead through *objectives*: the `["BLUEFOR", 5]` /
`["DIFF", 2]` supremacy objectives (`omtk/score_board/README.md:102-107`) are how OFCRA turns
casualties into score.

---

## 9. Zones / areas / triggers / play area

### There is no play-area / boundary system

No map-edge enforcement, no "you are leaving the battlefield" countdown, no AO polygon. The only
positional restrictions in the framework are the four below, and only the first is general.

### 9.1 Warm-up leash — per-player, radius from spawn

The strongest spatial rule in OMTK, and it is temporary. Each client, at warm-up start, records
`omtk_wu_spawn_location = getPos player` (`omtk/warm_up/main.sqf:85`) and builds a **local** trigger
around it:

> `omtk/warm_up/library.sqf:21-30`
> ```sqf
> omtk_wu_restrict_area_trigger = createTrigger ["EmptyDetector", omtk_wu_spawn_location, false];
> omtk_wu_restrict_area_trigger setTriggerArea [omtk_wu_radius, omtk_wu_radius, 0, false];
> omtk_wu_restrict_area_trigger setTriggerActivation [format["%1", side player], "NOT PRESENT", true];	// probably useless
> _trg_out_action = "['Leaving spawn location', 'INFO'] call omtk_log;
> hint 'GO BACK TO YOUR POSITION!';
> [omtk_wu_move_player_at_spawn_if_required, [], 5] call KK_fnc_setTimeout;";
> omtk_wu_restrict_area_trigger setTriggerStatements ["player in thisList || vehicle player in thisList", "hintSilent '';", _trg_out_action];
> ```

Five seconds after leaving, if still outside, the player is teleported back and it is logged as a
**cheat**:

> `omtk/warm_up/library.sqf:5-6`
> ```sqf
> ["teleport player '" + name player + "' back to his initial position", 'CHEAT', true] call omtk_log;
> ```

Radius is `OMTK_MODULE_WARM_UP_DISTANCE` (`description.ext:94-100`), 10 m…500 m, **default 150 m**.
`OMTK_MODULE_WARM_UP_MARKER` (`:101-107`, default on) draws the leash as a client-local orange
`ELLIPSE` marker named `SpawnZone` (`omtk/warm_up/library.sqf:11-17`) so players can see it.

### 9.2 Paradrop allowed zones — circles in `init.sqf`

`OMTK_TP_BLUEFOR_RESTRICTIONS` / `OMTK_TP_REDFOR_RESTRICTIONS` are arrays of
`[x, y, radius_in_m]` (`init.sqf:7-13`, empty by default). The union of circles is the allowed drop
area; empty array means anywhere:

> `omtk/tactical_paradrop/main.sqf:33-41`
> ```sqf
> if (count _restrictions < 1) then {
>     _result = true;
> } else {
>     {
>         _restriction = _x;
>         _distance = [(_restriction select 0), (_restriction select 1)] distance2D (_this select 0);
>         _result = _result || (_distance <= (_restriction select 2));
>     } forEach _restrictions;
> };
> ```

Illegal click → `hint "Forbidden zone, try again !"` (`:61`), and the player may retry inside the
time window. Note the README (`omtk/tactical_paradrop/README.md:70`) quotes a *different* message
("Unathorized area, jump somewhere else !") than the code.

### 9.3 Objective zones — Eden triggers, referenced by name

INSIDE/OUTSIDE objectives take a zone as a **string** (`omtk/score_board/README.md:124`: "zone has
to be the name of the trigger you want the subject to be inside (or outside)… Has to be between
quotation marks"). The code passes it straight through:

- `omtk/score_board/library.sqf:228` — `_r = (position _x) inArea _areaName;` (DIFF / side counting)
- `omtk/score_board/library.sqf:302` — `_r = (position _target) inArea _area;` (LIST)

`INFERRED:` (not verified in-engine) Arma's `inArea` treats a String argument as a **marker** name,
not a trigger name, so the README's "name of the trigger" instruction is at best ambiguous. The
authors evidently wrestled with this — two commented-out alternatives using
`BIS_fnc_inTrigger` survive at `library.sqf:301` and `:320`. Meanwhile `dynamic_startup` passes an
actual trigger **object** (`markers.sqf:43` `_obj = [_points, "BLUEFOR+REDFOR", "INSIDE", …, _trg, ["DIFF", 1]]`),
which `inArea` does accept.

Zone occupancy only counts **living** units — `library.sqf:229` `if (_r and alive _x)`, added in
v2.10.5 (`CHANGELOG.md:86` "INSIDE objective now only counts ALIVE members").

### 9.4 Drone leash — a copy-paste recipe, not a module

`script_library/limit_distance.txt:14-26` destroys a vehicle that strays too far from an anchor
object or flies too high, with a warning band first:

```sqf
if (_dist > 500 || _alt > 200 ) then {
    Veh1 setDamage 1;
};
if ( _dist > 350 || _alt > 120 ) then {
    hintSilent format["The vehicle is close to max distance - current distance %1 meters and height %2", _dist, _alt];
};
```

Objects named `Veh1` (the leashed vehicle), `Orig` (the anchor) and `DroneOperator` (who gets the
warning). This is how OFCRA caps UAV reach: **hard-kill at 500 m / 200 m AGL**.

### 9.5 Marker-driven zone generation (`dynamic_startup` markers mode)

The one place where OMTK *creates* zones. On mission start the server walks `allMapMarkers` and
switches on the marker's **text**:

> `omtk/dynamic_startup/markers.sqf:117,122` — `_mtext = markerText _x; … switch(_mtext) do {`

| Marker text | Effect | Code |
|---|---|---|
| `b_spawn` / `r_spawn` / `g_spawn` | Create side flag + `BIS_fnc_addRespawnPosition`, and mass-teleport that side there | `markers.sqf:123-137`, `:5-19` |
| `b_respawn` / `r_respawn` / `g_respawn` | Extra respawn position only | `:170-181`, `:22-24` |
| `cap_20` / `cap_50` / `cap_100` / `cap_150` / `cap_200` | Create a black `ELLIPSE` marker + `EmptyDetector` trigger of radius = diameter/2, and push a `["BLUEFOR+REDFOR","INSIDE",…,["DIFF",1]]` objective worth **radius/5** points | `:138-157`, `:27-46` |
| `b_obj` / `r_obj` / `g_obj` | Spawn a side flag and push a **3-point** `ACTION` objective | `:158-169`, `:55-82` |

The in-game cheat sheet (`omtk/dynamic_startup/markers_doc.sqf:2-22`) states "Points = diameter / 10
(100 m => 10 pts)" and "Points equal 3 per flag", which matches the code
(`markers.sqf:28-29` `_radius = (_this select 1)/2; _points = _radius/5;` and `:78` `[3, _side_name, "ACTION", …]`).

> **Doc/code drift.** `markers_doc.sqf:2` says "Supported **marker's names**", but the dispatcher
> matches `markerText` — the label, not the marker's variable name. The cheat sheet also omits
> `g_spawn` / `g_obj` / `g_respawn`, which the code supports.

Teleport-to-base is `omtk_mkd_mass_teleport` (`omtk/library.sqf:74-88`), which fans units out in a
spiral: 8 units per ring at 45° intervals, ring radius growing by 2 m each lap.

### 9.6 Vehicle locking as a soft zone

`omtk_lock_vehicles` / `omtk_unlock_vehicles` (`omtk/library.sqf:131-164`) lock the **driver seat
only** — `_x lockDriver true` plus `enableCopilot false` — deliberately, per `CHANGELOG.md:187`:
"vehicle are no more completly locked. Only driver is (so you can take stuff and board)". Timed
helicopter release is configured in `init.sqf:22-26` and executed by
`omtk/score_board/main.sqf:18-35`, unlocking every name in `OMTK_SB_UNLOCK_HELI_VARS` after
`OMTK_SB_UNLOCK_HELI_TIME` seconds **measured from the end of warm-up**, announced to everyone:
`("Locked Vehicles have been Unlocked (if any)") remoteExecCall ["systemChat"];` (`:29`).

---

## 10. Configuration surface

Exhaustive. Six layers: lobby parameters, `description.ext` scalars, `init.sqf` globals, in-source
constants, Eden naming conventions, and the loadout-toolchain CLI.

### 10.1 Lobby parameters — `description.ext:37-241`

**29 `class Params` entries, of which 4 are visual separators**, leaving **25 real knobs**. The
lobby is grouped into four bands by those separators: *FIRST CLASS PARAMETERS*, *MODULE SETTINGS*,
*EXTRA PARAMETERS*, *OLD PARAMETERS*.

| # | Class | Title | Options (texts → values) | Default | Consumed at |
|---|---|---|---|---|---|
| — | `OMTK_MODULE_SEPARATOR_0` | `----- FIRST CLASS PARAMETERS ---…` | separator | 0 | — |
| 1 | `OMTK_MODULE_WARM_UP` | Warm-up | off/10 s/30 s/1 min/1 min 30 s/2 min/3 min/5 min/8 min/10 min/15 min/20 min → 0,10,30,60,90,120,180,300,480,600,900,1200 | **480 (8 min)** | `load_modules.sqf:5,32`; `warm_up/main.sqf:35` |
| 2 | `OMTK_MODULE_SCORE_BOARD` | Scoreboard | off/15 min/30 min/45 min/1 h/1 h 15/1 h 30/1 h 45/2 h/2 h 30 → 0,900,1800,2700,3600,4500,5400,6300,7200,9000 | **5400 (1 h 30)** | `load_modules.sqf:28`; `score_board/main.sqf:13` |
| 3 | `OMTK_MODULE_VIEW_DISTANCE` | View distance | 500…8000 m → 500,1000,1500,2000,2500,3000,4000,5000,6000,8000 (no `texts[]`) | **3000** | `view_distance/main.sqf:7`; `library.sqf:296,326,448` |
| 4 | `OMTK_MODULE_MAP_EXPLORATION` | Map exploration | off/on → 0,1 | **0** | `load_modules.sqf:43` |
| 5 | `OMTK_MODULE_DISABLE_PLAYABLE_AI` | Disable playable AI | no/yes → 0,1 | **"1"** | `ia_manager/main.sqf:18`; `warm_up/library.sqf:94` |
| 6 | `OMTK_MODULE_RAMBO_DIST` | Rambo Script | disabled/strict (210\|615)/normal (230\|645)/loose (250\|675) → 0,5,15,25 | **"0"** | `load_modules.sqf:47`; `rambo_warn/main.sqf:23,27-28` |
| — | `OMTK_MODULE_SEPARATOR_1` | `----- MODULE SETTINGS ---…` | separator | 0 | — |
| 7 | `OMTK_MODULE_WARM_UP_DISTANCE` | Warm-up: Zone restriction size | 10/30/50/100/150/200/300/400/500 m | **150** | `warm_up/main.sqf:36` |
| 8 | `OMTK_MODULE_WARM_UP_MARKER` | Warm-up: Zone restriction marker | off/on → 0,1 | **1** | `warm_up/main.sqf:40`; `warm_up/library.sqf:32,79` |
| 9 | `OMTK_MODULE_WARM_UP_SAFETY` | Warm-up: Gun safety | off/on → 0,1 | **0** | `warm_up/main.sqf:39,91`; `warm_up/library.sqf:66` |
| 10 | `OMTK_MODULE_RAMBO_WARN` | Rambo Script: Give Warning | off/on → 0,1 | **"1"** | `rambo_warn/main.sqf:24,32` |
| 11 | `OMTK_MODULE_RAMBO_INTERVAL` | Rambo Script: Frequency (lower = more perf usage for clients) | 5s/10s/20s/30s/60s → 5,10,20,30,60 | **"10"** | `rambo_warn/main.sqf:25,30` |
| — | `OMTK_MODULE_SEPARATOR_2` | `----- EXTRA PARAMETERS ---…` | separator | 0 | — |
| 12 | `OMTK_MODULE_ARTY_COMPUTER` | Artillery Computer | off/on → 0,1 | **"0"** | `load_modules.sqf:49` — `enableEngineArtillery false` when **on** (see §14) |
| 13 | `OMTK_MODULE_MEXICAN_STANDOFF` | Three factions game | no/yes → 0,1 | **"0"** | `briefing.sqf:67`; `score_board/library.sqf:4,97` |
| 14 | `OMTK_MODULE_RADIO_LOCK` | Radio lock | off/on → 0,1 | **1** | `load_modules.sqf:45` |
| 15 | `OMTK_MODULE_VEHICLES_THERMALIMAGING` | Vehicles thermal imaging | off/on → 0,1 | **0** | `load_modules.sqf:26` — inverted: TI is *disabled* when the value is `< 1` |
| 16 | `OMTK_MODULE_DIFFICULTY_CHECK` | Difficulty check | off/on → 0,1 | **1** | `load_modules.sqf:41` |
| 17 | `OMTK_MODULE_KILL_LOGGER` | Kill logger | off/on → 0,1 | **1** | `load_modules.sqf:46` |
| 18 | `OMTK_MODULE_ZEUS_ADMINS` | Zeus for admins | no/yes → 0,1 | **1** | `load_modules.sqf:48` |
| 19 | `OMTK_MODULE_UNIFORM` | Uniform Management | Locked (old method)/Free (new method) → 0,1 | **1** | `uniform_lock/main.sqf:1` |
| — | `OMTK_MODULE_SEPARATOR_3` | `----- OLD PARAMETERS ---…` | separator | 0 | — |
| 20 | `OMTK_MODULE_DYNAMIC_STARTUP` | Dynamic startup | off/markers/interactive → 0,1,2 | **0** | `load_modules.sqf:17,55`; `dynamic_startup/main.sqf:11` |
| 21 | `OMTK_MODULE_RESPAWN_MODE` | Respawn | no-respawn/3 s/30 s/1 min/1 min 30 s/2 min/3 min/immortal → 999999,3,30,60,90,120,180,-1 | **999999** | `load_modules.sqf:44`; `respawn_mode/main.sqf:3` |
| 22 | `OMTK_MODULE_SPECTATOR` | Spectator | all/team → 0,1 | **"0"** | `onPlayerKilled.sqf:1,9` |
| 23 | `OMTK_MODULE_TACTICAL_PARADROP` | Tactical paradrop | off/BLUEFOR only/REDFOR only/BLUEFOR + REDFOR → 0,1,2,3 | **0** | `load_modules.sqf:27`; `tactical_paradrop/main.sqf:74` |
| 24 | `OMTK_MODULE_TACTICAL_PARADROP_ALTITUDE` | Tactical paradrop: altitude | 300/500/1000/1500/2000/2500/3000/4000/5000 m | **3000** | `tactical_paradrop/main.sqf:58` |
| 25 | `OMTK_MODULE_TACTICAL_PARADROP_TIME_LIMIT` | Tactical paradrop: timeframe | 1/2/3/5/10/15/20/30 min/unlimited → 1,2,3,5,10,15,20,30,9999 | **3** | `tactical_paradrop/main.sqf:8,56` |

Note several `default = "1"` / `"0"` values are **quoted strings** where sibling entries use bare
numbers (`description.ext:78,85,120,127,141,148,218`) — Arma tolerates it, but it is inconsistent.

**The README's parameter list is stale.** `README.md:69-172` documents warm-up options up to
"45 min / 1 h" and a 30 min default, scoreboard up to 3 h with a 2 h default, warm-up distances up
to 2000 m, and paradrop altitudes that omit 4000 m. None of those match `description.ext` at HEAD.
It also documents `IA skills`, `Kill logger`, `Radio lock` and `Tactical paradrop` sections that no
longer correspond to the current grouping, and omits **eleven** parameters that do exist:
`DISABLE_PLAYABLE_AI`, `RAMBO_DIST`, `RAMBO_WARN`, `RAMBO_INTERVAL`, `WARM_UP_MARKER`,
`WARM_UP_SAFETY`, `ARTY_COMPUTER`, `MEXICAN_STANDOFF`, `ZEUS_ADMINS`, `UNIFORM`, `SPECTATOR`.

### 10.2 `description.ext` scalars the maker touches

| Line | Setting | Value | Note |
|---|---|---|---|
| 1 | `onLoadName` | `"MISSION NAME"` | **EDIT** |
| 2 | `author` | `"OFCRA"` | **EDIT** |
| 3 | `loadScreen` | `"loadscreen.jpg"` | **EDIT** |
| 4 | `onLoadMission` | `"www.ofcrav2.org"` | **EDIT** |
| 5 | `briefingName` | `"Nom de la mission"` | **EDIT** |
| 6 | `Debriefing` | `0` | vanilla debrief off |
| 7 | `respawnOnStart` | `-1` | no respawn at start |
| 8 | `Saving` | `0` | saves disabled |
| 10 | `respawn` | `"BASE"` | |
| 11 | `respawnDelay` | `999999` | |
| 12 | `corpseLimit` | `999` | bodies persist |
| 13 | `wreckLimit` | `999` | wrecks persist |
| 15 | `enableDebugConsole` | `1` | "server admin only" per `README.md:25` |
| 16 | `disableChannels[]` | `{0,5,6}` | Global, Direct, System chat off (`README.md:24`) |
| 20 | `Header/gameType` | `TDM` | |
| 21-22 | `Header/minPlayers`, `maxPlayers` | `1`, `200` | |
| 35 | `onPauseScript` | `"omtk\ui\pauseScreenMenu.sqf"` | the admin console |

### 10.3 `init.sqf` globals — the real mission config file

| Line | Variable | Default | Meaning |
|---|---|---|---|
| 7-9 | `OMTK_TP_BLUEFOR_RESTRICTIONS` | `[]` | Allowed paradrop circles `[x, y, radius_m]` |
| 11-13 | `OMTK_TP_REDFOR_RESTRICTIONS` | `[]` | same for REDFOR |
| 16 | `OMTK_TP_BLUEFOR_DELAY` | `0` | Seconds after start before BLUEFOR may paradrop |
| 17 | `OMTK_TP_REDFOR_DELAY` | `0` | same for REDFOR |
| 20 | `OMTK_SB_MISSION_DURATION_OVERRIDE` | *commented out* | `[hours, minutes, seconds]` — **does not work on the server, see §14** |
| 25 | `OMTK_SB_UNLOCK_HELI_VARS` | `["heli01","heli02","heli03","heli04"]` | Eden variable names of helicopters to time-unlock |
| 26 | `OMTK_SB_UNLOCK_HELI_TIME` | `600` | Seconds after warm-up end |
| 31-33 | `OMTK_SB_LIST_OBJECTIFS` | `[]` | **The objectives table** (§7) |
| 41 | `OMTK_TIMED_OBJECTIFS_CALLBACKS` | `[]` | Index = flag number → server-side code block |
| 43-44 | `OMTK_LM_BLUEFOR_OB` | `[]` | Interactive-startup vehicle order-of-battle |
| 46-47 | `OMTK_LM_REDFOR_OB` | `[]` | same for REDFOR |
| 50 | `setTerrainGrid 3.125` | — | Highest terrain mesh quality (`README.md:26`) |
| 53-56 | `OMTK_WARMUP_MENU` | **commented out** | The dead "side is ready" comm menu (§14) |
| 58-61 | `OMTK_MARKERS_MENU` | active | Comm-menu entry calling `omtk_ds_process_markers_mode` |
| 64 | `RscSpectator_allowFreeCam` | `true` | Free spectator camera |
| 79 | `admin_uids` | 7 Steam64 IDs | **The referee whitelist**, `publicVariable`d |
| 85-86 | `onPlayerConnected` / `onPlayerDisconnected` | — | Audit log of every join/leave |

`init.sqf:69-77` carries a commented roster mapping each UID to a nickname (Manchot, Flip4flap,
PHK4900, Nasa, Daedalus, MrWhite350, Rigel) and the constraint *"Max 10 admins (or change
missionCurators array in zeus_admins\main.sqf)"* — which matches the 10 pre-created curator modules
at `omtk/zeus_admins/main.sqf:33-44`.

### 10.4 Constants edited in place, in module source

| Location | Constant | Value |
|---|---|---|
| `omtk/dynamic_startup/interactive.sqf:1` | `OMTK_DS_CHOSEN_SPAWN_FOR_PLAYER` | `0` |
| `:2` | `OMTK_DS_VEHICLES_MAX_NB` | `-1` (unlimited) |
| `:3` | `OMTK_DS_VEHICLES_MAX_NB_PER_GROUP` | `-1` |
| `:4` | `OMTK_DS_CHOSEN_VEHICLES` | `[]` |
| `:5` | `OMTK_DS_CHIEF_CLASSES` | `["B_officer_F","O_officer_F","B_Soldier_SL_F","O_Soldier_SL_F"]` |
| `omtk/ia_manager/main.sqf:6-15` | AI skill table | `aimingAccuracy .1`, `aimingShake .1`, `aimingSpeed .1`, `endurance .2`, `spotDistance .3`, `spotTime .4`, `courage .4`, `reloadSpeed .4`, `commanding .2`, `general .2` — documented as maker-editable at `omtk/ia_manager/README.md:37-40` |
| `omtk/warm_up/README.md:88` | `OMTK_WU_CHIEF_CLASSES` | **documented but does not exist in the code** (§14) |

### 10.5 Eden-side conventions (the "config" that lives in `mission.sqm`)

| Convention | Where read |
|---|---|
| `Role@SquadName` in *Role Description* | `fn_rosterBriefing.sqf:84-88`, `table_forum.sqf:85-88` |
| Group `groupID` = callsign | `fn_rosterBriefing.sqf:86`; the loadout tool force-writes it into unit inits |
| Unit / object *Variable Name* referenced by objectives | `score_board/library.sqf:299,427` |
| Trigger name as an objective zone string | `score_board/library.sqf:228,302` |
| `this setVariable ['OMTK_ID', N];` in unit init | documented `score_board/README.md:113` — **broken, code reads `mt_id`** |
| `this setVariable ["omtk_ignore", 1];` in unit or vehicle init | loadout tool: skip this entity entirely |
| Helicopter variable names `heli01`…`heli04`, locked in Eden | `init.sqf:23-25` |
| Marker **text** `b_spawn`/`r_spawn`/`g_spawn`/`*_respawn`/`cap_20…200`/`b_obj`/`r_obj`/`g_obj` | `dynamic_startup/markers.sqf:117-183` |
| Recipe object names: `Obj`, `BLight_1`, `RLight_1`, `Obj_Mrkr`, `jamTruck`, `jamBackup`, `Updater`, `Veh1…VehN`, `Veh1_Mrkr…`, `Intel`, `loudspeaker`, `Controller`, `DroneOperator`, `Orig` | `script_library/*.txt` |
| `bluefor_spawn_N` / `redfor_spawn_N`, `omtk_bluefor_spawn` / `omtk_redfor_spawn` | interactive startup (§11) |

### 10.6 Loadout toolchain configuration

Six YAML paths passed as CLI flags (`--classes-bluefor/-cb`, `--classes-redfor/-cr`,
`--infantry-bluefor/-ib`, `--infantry-redfor/-ir`, `--vehicles-bluefor/-vb`, `--vehicles-redfor/-vr`),
plus `--mission-directory/-m` and `--debug/-d`. See §5.1. Within the YAML, every key listed in §5.2
is a knob, and **explicit null is itself a setting**.

---

## 11. Tooling

There is **no CI, no test suite, no linter, no schema and no validator** in this repository —
`find` shows no `.github/`, no `.gitlab-ci.yml`, no `tests/`, no `Makefile`. `.gitignore` contains
two lines. The only automated quality gate in the entire toolchain is the loadout tool's `test`
subcommand.

What does exist is five genuine tools, three of them authoring-time and two in-game.

### 11.1 `omtk-loadouts.exe` — the offline mission compiler

Covered in §5.1. The relevant point for tooling: it has a **dry-run mode**.

> `test` — *"test if the mission.sqm file and all loadouts .yml files can be parsed successfully,
> but DOES NOT MODIFY anything"*

and a **`restore`** subcommand that reverts `mission.sqm` from the `mission.sqm.orig` backup it makes
on first run. It validates the SQM version against a hard-coded `[12, 51, 52]` and emits typed
errors: `"invalid location, '<abs>/mission.sqm' mission file missing"`,
`"invalid location, '<abs>/omtk-loadouts' OMTK data missing"`,
`"Class not found: '<arma3class>' in side '<colour>'"`,
`"Undefined infantry for class '<role>' in side '<colour>'"`,
`"original mission back-up already exists: <file>"`.

That is the closest thing OMTK has to a mission validator, and it only checks loadouts. Because it
is a Windows `.exe` (with a 64-bit Ruby inside a 32-bit stub), it is **Windows-only** in practice.

### 11.2 `omtk/table_forum.sqf` — the slot-list JSON exporter

The single most directly relevant tool for a web mission editor. Run in-game from the admin console
(`omtk/ui/pauseScreenMenu.sqf:701` `_handle = execVM "omtk\table_forum.sqf";`, button labelled
`"export list"`), it walks `playableUnits`, preserves **side order and group order of first
appearance**, and emits JSON to the clipboard:

```
[{"side":"BLUFOR","squads":[{"squadName":"Alpha","roles":[{"role":"Squad Leader"},…]},…]},…]
```

Built at `omtk/table_forum.sqf:60,76,95` and delivered by `:111-113`:

```sqf
forceUnicode 1;
copyToClipboard _json;
hint "JSON généré et copié dans le presse-papiers.";
```
*("JSON generated and copied to the clipboard.")*

Role names come from `roleDescription` truncated at `@`, falling back to the unit class `displayName`
(`:84-91`); empty group names become `Group_<index>` (`:72-74`). The header comment is French:
`:4` `Génère un JSON des squads et rôles (tous côtés, tous types d'unités)` — *"Generates a JSON of
the squads and roles (all sides, all unit types)"*, authored by "Manchot".

This replaced an HTML forum-table exporter in v2.13.4 — `CHANGELOG.md:19`:
`~ [ui] Changed "export list" button to produce the json file for the new mission manager instead of
the old forum html code`. **OFCRA has an external "mission manager" that ingests this JSON.** It is
not in this repository, but its existence and its input format are documented by that changelog line
and this script. The v2.10.4/v2.10.5 entries (`CHANGELOG.md:90,85`) record the older workflow: the
maker pasted a generated slot table into the forum thread announcing the mission.

### 11.3 `script_library/` — a copy-paste recipe library

Five recipes, each a `.txt` with a mandatory preamble and a delimited code block
(`/* IMPORTANT: READ THIS … */` then `/* ---------------- STARTCODE ---------------- */`). Covered
in §6, §7 and §9.4. This is "tooling" in the sense that it is the maker's pattern library, and it is
explicitly deleted before shipping.

### 11.4 The in-game admin / referee console

`description.ext:35` `onPauseScript = "omtk\ui\pauseScreenMenu.sqf";` — pressing **ESC** builds the
console imperatively via `ctrlCreate` (there is no `.hpp` for it; it is rebuilt from scratch on every
ESC press). **34 controls: 4 for everyone, 30 admin-side** (23 action buttons, 2 filter edit boxes,
2 player listboxes, 3 decorative).

**The gate**, verbatim (`omtk/ui/pauseScreenMenu.sqf:112-113`):

```sqf
admin_uids = missionNamespace getVariable ["admin_uids", 0];
if !(serverCommandAvailable "#kick" or _uid in admin_uids) exitwith {};
```

Two routes: engine-level server admin, **or** membership of the static Steam64 whitelist in
`init.sqf:79`. Lines 4–99 draw for everyone; 115–796 for admins only. There is **no per-button
re-check**.

**Everyone gets four controls:**

| Line | Label | Effect |
|---|---|---|
| `:12` | `Fix uniform Bug` | Client-side inventory launder — saves vest/uniform contents, removes and `forceAddUniform`s them back |
| `:67` | `Short View distance` | `[3] call omtk_set_viewDistance` → quarter of max, **capped 500 m** |
| `:81` | `Medium View distance` | `[2]` → half of max |
| `:95` | `Long View distance` | `[1]` → full max |

**Target selection** is by **player-name string**, not UID: two listboxes (IDC 10000 "WHO", 10001
"WHERE") populated per side with `===blufor===` / `===opfor===` / `===independent===` header rows,
two filter edit boxes (10002/10003), and a 0.2 s refresh loop (`:705-796`). The handler reads the
selected display text and `remoteExec`s it to **all** machines; each client self-filters with
`if (_name == name player)`. Two players with the same name are both hit.

**The 23 admin actions:**

| Line | Label | Invokes | Scope |
|---|---|---|---|
| `:209` | Teleport player | `omtk_teleport_unit` | selected → selected |
| `:233` | Warn lonewolf | `omtk_warn_unit` | selected |
| `:253` | Toggle Safety | `omtk_toggle_safety_unit` | selected |
| `:273` | Respawn | `omtk_respawn_unit` | selected |
| `:293` | Full heal | `omtk_heal_unit` (ACE) | selected |
| `:313` | Reset Inventory | `omtk_reset_unit` | selected |
| `:434` `:449` `:463` `:479` | Set MAX / 1÷2 / 1÷4 / 1÷8 VD | `omtk_set_viewDistance` | **all players** |
| `:508` | End Warm-up | `omtk_wu_fn_launch_game` | server |
| `:523` | Show Scoreboard | `omtk_sb_compute_scoreboard` + `omtk_sb_start_mission_end` | server → all |
| `:539` | Export Ocap/Stats | `statslogger_fnc_export` + `CBA_fnc_serverEvent "ocap_exportData"` | server |
| `:555` | Remove AIs | `omtk_delete_playableAiUnits` | all |
| `:570` | Freeze AIs | `omtk_disable_aiBehaviour` | server |
| `:587` | Enable Dmg ALL | `omtk_enable_playerDamage` | all |
| `:602` `:617` | Enable / Disable Safety ALL | `omtk_enable_safety` / `omtk_disable_safety` | all |
| `:633` | DISABLE Sim ALL | `omtk_sim_disablePlayerSim` + `omtk_sim_disableVehicleSim` | all, **JIP-persistent** |
| `:649` | Enable Sim ALL | `omtk_sim_enablePlayerSim 'all'` + `omtk_sim_enableVehicleSim` | all |
| `:667` `:681` | Show Player Count / Time Left | `omtk_show_player_count` / `omtk_show_time_left` | self, systemChat |
| `:699` | export list | `execVM "omtk\table_forum.sqf"` | self, clipboard |

Two of these deserve emphasis because they are *referee* tools, not admin conveniences:

- **"DISABLE Sim ALL" is a match pause.** `omtk_sim_disablePlayerSim` (`omtk/library.sqf:261-283`)
  freezes every player, forces view distance to 200 m and turns weapon safety on, showing
  `- PLAYER SIM DISABLED and WEAPON SAFETY ENABLED -` / *"Please hold until weapon safety is removed
  while the server takes a breather"*. `:272` exempts the logged-in admin
  (`if (call BIS_fnc_admin != 2)`) so the referee can still move. Un-pausing releases players on a
  **random 1–11 s stagger** (`:298 private _randRelease = (random 10) + 1;`) so nobody gains a
  reaction-time edge from the resume.
- **"End Warm-up" does not end it, it shortens it** — `omtk/warm_up/library.sqf:110`
  `o_wse set [1, (dayTime + 0.002777)];` ≈ 10 seconds, `publicVariable`d, so the on-screen countdown
  visibly jumps.

**Weapon "safety" is not an engine lock.** It is
`player addAction ["Weapon safety on", {hintSilent "Safety On";}, [], 0, false, false, "DefaultAction", ""]`
(`omtk/library.sqf:352,368,425`) — binding to the `DefaultAction` keybind hijacks the fire key. Soft
and defeatable, but sufficient as a referee signal.

**Dead UI:** `pauseScreenMenu.sqf:329-401` still builds per-side simulation controls
("Enable Blue Sim", "Enable Red Sim", "Enable Green Sim", "Enable Vics Sim") whose receiver
(`library.sqf:289-316`) still honours the per-side path — but the block sits above the drawn region
and only the "ALL" variants are reachable.

**Security posture:** the gate is a **UI-drawing gate only**. Every admin power is a client-issued
`remoteExec`; there is no server-side authorisation on any of them, and no `CfgRemoteExec` whitelist
in this repository. `INFERRED:` protection therefore depends entirely on the server's own
`CfgRemoteExec` configuration, which OFCRA presumably ships separately.

### 11.5 Zeus for referees

`omtk/zeus_admins/main.sqf` pre-creates **ten** `ModuleCurator_F` logics on the server (`:33-44`) and
assigns curator *n* to the *n*-th UID in `admin_uids` (`:53-58`), with
`setVariable ["Addons",3,true]` — `:16` `//3: allow all addons with proper use of CfgPatches`. So
every whitelisted referee has full Zeus over every loaded mod, at all times, by default
(`OMTK_MODULE_ZEUS_ADMINS` default 1). `omtk/zeus_admins/README.md:16` states the intent: "the
logged admin no longer has to bear all the work/responsibilities."

### 11.6 Logging as the audit tool

`omtk_log` (`omtk/library.sqf:1-9`) writes `[OMTK] <TAG>: <message>` to the RPT, optionally also to
systemChat. The tag vocabulary is the audit schema: `DEBUG`, `INFO`, `WARNING`, `ERROR`,
**`CHEAT`**, `CONNECT`, `DISCONNECT`, `OBJECTIVE`. `README.md:51` documents the format. Every
module brackets itself with `"<module> start"` / `"<module> end"` DEBUG lines, which makes the RPT a
readable execution trace.

The `CHEAT` tag is used for exactly three things: warm-up leash violations
(`omtk/warm_up/library.sqf:5`), radio theft (`omtk/radio_lock/main.sqf:50`) and lonewolf
infringements (`omtk/rambo_warn/main.sqf:127`). Kill/hit logging is separate
(`omtk/kill_logger/main.sqf:28,32`), and renames AI to `bot_1`, `bot_2`… first (`:6-7`) so the log
distinguishes players from bots. It also resolves ACE self-kills back to the real killer by waiting
up to 10 s for `ace_medical_lastdamageSource` (`:14-23`).

### 11.7 External integrations (feature-detected, not shipped)

| System | Detection | Use |
|---|---|---|
| **OCAP** (after-action replay) | `isClass(configFile >> "CfgPatches" >> "ocap")` — `omtk/score_board/main.sqf:67` | Auto-export 10 s after mission end |
| **STATSLOGGER** | `isClass(configFile >> "CfgPatches" >> "STATSLOGGER")` — `omtk/score_board/library.sqf:134` | `[_winner, blueScore, redScore] remoteExec ["statslogger_fnc_mission_end", 2]` then `statslogger_fnc_export` |
| **External mission manager** | — | Consumes `table_forum.sqf` JSON (`CHANGELOG.md:19`) |

The STATSLOGGER hook is the tell that **OFCRA keeps a persistent league table across missions**: the
framework computes a winner string (`"WEST"` / `"EAST"` / `"GREEN"` / `"DRAW"` / `"NA"`) and both
side scores specifically to hand off to it.

---

## 12. Conventions and house rules encoded in the framework

This is where OMTK differs most from every other Arma framework: **the rulebook ships inside the
mission and is shown to the player on the map screen**, and a subset of it is enforced in code with
a dedicated `CHEAT` log tag.

### 12.1 The rulebook, verbatim

`omtk/briefing.sqf:84-103` creates a diary record titled **"Rules"**. It is already in English (it
was translated at v2.3.0). Reproduced in full, one bullet per source clause:

> **General rules of our games:**
> - No technical support will be provided after 21h00 on the mission evening
> - Stealing radios is prohibited
> - Stealing uniforms (hats, clothes, vest) is prohibited
> - You can only use TFAR to communicate ingame. Any other way of communicating is strictly
>   forbidden (that includes steam messages)
> - The ingame chat is only allowed for technical issues.
> - Respect the hierarchy, orders from your superiors and the chain of command
> - NO AI units. Please close the unused slots
> - Stealing vehicles : it will be specified on each topic and mission rules specifically, what
>   vehicles can be stolen.
> - Running over an enemy intentionally with your vehicle is not allowed. Similarly, using an aerial
>   vehicle to kill players on the ground is strictly prohibited if it causes the vehicle to crash.
>   The same goes for hoisted vehicles. For example, using a helicopter's minigun to kill a player is
>   allowed. Crashing the helicopter in order to eliminate the player or his vehicle is strictly
>   prohibited. Casting off a boat in order to achieve a similar result is also prohibited.
> - Markers on map are authorized
> - The mission maker is authorised to create more rules according to their desires.
> - You are not allowed to keep and AI at the start of the game in order to kill it and take its
>   equipment
> - It is mandatory to use the vehicles allocated to your squad in the slot list by the editor. If
>   the vehicles haven't been assigned by the editor, this responsibility lies in the hands of the
>   side leader.
> - It is mandatory to at least have a squad leader to take the others slots in the squad, except
>   for the rifleman (and potential other slots that you've been given permission to take by OFCRA
>   staff)
> - Lonewolfing is strictly forbidden. We consider as lonewoling any person who is too far away from
>   their squad. In precise terms, the space between the more distant members of the squad should not
>   be more than 200 meters.
> - The rule above does not apply to side leaders when they are alone in their squad. If they have
>   one or more squad mates, the rule then applies to him and his teamates.
> - When vehicles are attached to a squad, the maximum distance they can get away from is 600 meters.
>   The calculation of the distance is the same way you would do it for infantery: between the two
>   players which are further away from each other (in this case a vehicle and an infantry member. If
>   the crew has to leave the vehicle for any reason, they have to regroup with their squad or any
>   allied forces as soon as they can. They are not allowed to seek contact.

Note the last clause: **"They are not allowed to seek contact."** OFCRA regulates not just position
but *intent*.

### 12.2 Which rules are enforced in code

| Rule | Enforcement | Mechanism |
|---|---|---|
| **Lonewolfing ≤ 200 m infantry / 600 m vehicle** | **Detect + warn + log** | `omtk/rambo_warn/main.sqf` — see 12.3 |
| **Stealing radios is prohibited** | **Hard block** | `omtk/radio_lock/main.sqf:24-53` — on `Take`, look up `tf_encryptionCode`; if it isn't your side's code, `removeItem` and log `CHEAT` |
| **Stealing uniforms is prohibited** | **Hard block (legacy mode)** | `omtk/uniform_lock/lock.sqf` — see 12.4 |
| **NO AI units / close unused slots** | **Automatic deletion** | `OMTK_MODULE_DISABLE_PLAYABLE_AI` (default **yes**): `omtk_delete_playableAiUnits` (`library.sqf:166-172`) deletes every non-player playable unit **at the end of warm-up** (`warm_up/library.sqf:94-96`) |
| **Don't farm AI for their gear** | **Pre-emptive** | Same deletion, plus `ia_manager/main.sqf:20-33` freezes playable AI (`disableAI` MOVE/TARGET/AUTOTARGET/FSM, `setBehaviour "CARELESS"`, `allowFleeing 0`, `setSpeaker "NoVoice"`) and makes them `allowDamage false` so they cannot be killed for loot |
| **In-game chat only for technical issues** | **Channels removed** | `description.ext:16` `disableChannels[]={0,5,6}` — Global, Direct and System |
| **Only TFAR for comms** | Partially | Radio lock protects the equipment; the rest is social |
| **Elite difficulty** | **Warn** | `difficulty_check/main.sqf:3-6`: `if (difficulty < 3)` → RPT `WARNING` + a `hint` |
| Chain of command | Not enforced | Social |
| Vehicle ramming / suicide-crash kills | Not enforced | Social |
| Squad-leader-first slotting | Not enforced | Social |

### 12.3 The lonewolf detector — a house rule turned into arithmetic

`omtk/rambo_warn/main.sqf` is the clearest example in any Arma framework of a *social* rule
compiled into code. Every client runs a loop over `units group player` and evaluates:

```sqf
private _AllowedInfantryDistance = 200+(200*_omtk_rw_distance/100);		//Infantry Lonewolfing: 100, 200, 300, 400, 500
private _AllowedVehicleDistance  = 600+(600*_omtk_rw_distance/200);		//Vehcicle Lonewolfing: 200, 400, 600, 800, 1000
private _IgnoreDistance			 = 1500;		//Ignore Distances further than
```
(`:27-29`)

With the lobby's three tolerance bands (`description.ext:83` `strict (210|615)`, `normal (230|645)`,
`loose (250|675)` → values 5, 15, 25) this yields infantry thresholds of **210 / 230 / 250 m** and
vehicle thresholds of **615 / 645 / 675 m** — i.e. the published 200 m / 600 m rule plus a 5 %,
15 % or 25 % margin of error. The 1500 m ignore band means a genuinely separated element stops
being nagged.

The exclusions encode the rulebook's exceptions precisely (`:74-82`): both parties must be alive
(via ACE `isAwake`, so *incapacitated* squadmates do not count), neither may be in a plane or
helicopter, neither may be handcuffed (ACE captives), and the buddy must be a real player.

The trip conditions (`:111`):

```sqf
if ( _BuddiesAlive == 0 || (_BuddiesToFar >= 2 ) ||	(_BuddiesToFar == 1 && _BuddiesAlive == 1 )	) then {
```

— i.e. you are flagged when everyone in your squad is down, **or** two or more mates are beyond the
threshold, **or** your single surviving mate is. Two distinct player-facing messages
(`:114`, `:118`): `"[LoneWolf] You lost your lads!"` and `"[LoneWolf] You're alone, join another
squad!"`. The second is doctrine, not just a warning: **if your squad is dead, attach yourself to
another squad.**

Every trip is reported to the server with the `CHEAT` tag:

```sqf
[_message2ToServer, 'CHEAT', false] remoteExecCall ["omtk_log",2,false];
```
(`:127`)

producing an RPT line of the form
`[LoneWolf] Player <name> got flagged for rambo infringment; _BuddiesTooFar: N _BuddiesAlive: M`.
There is a separate admin action, `omtk_warn_unit` (`omtk/library.sqf:346-358`), which flashes
`YOU ARE LONEWOLFING! RETURN TO THE REST OF YOUR SQUAD` in red at size 5 **and switches the
offender's weapon safety on for 11 seconds** — a referee can disarm you for breaking formation.

The module is candid about its own limits — `:36`
`private _WriteLocalRPT = true;	//Write in local rpt... could be exploited for cheating (notepad++ tail -f)`
and `:35` `private _ReportToServer = true;	//Snitches get Stiches?`.

### 12.4 Uniform regulation — two regimes, switchable

`OMTK_MODULE_UNIFORM` (`description.ext:185-191`) picks between:

- **`Locked (old method)`** (value 0) — `omtk/uniform_lock/lock.sqf` opens on the inventory display
  and keeps control IDC **6331** (the uniform slot) permanently disabled, re-adding the original
  uniform and its contents if anything changes:
  ```sqf
  while { !(isNull (findDisplay 602)) } do {
      // Keep the "uniform slot" control on lockdown. Else there are loop holes.
      ctrlEnable [6331, false];
  ```
  (`omtk/uniform_lock/lock.sqf:7-9`). You cannot take your uniform off, therefore you cannot take an
  enemy's.
- **`Free (new method)`** (value 1, **default**) — `omtk/uniform_lock/wwyw.sqf`, pierremgi's
  "Wear What You Want" script, which *allows* wearing any uniform including ones the engine would
  normally forbid by side (`:51` `!(player isUniformAllowed _selectedUnif)`), preserving inventory
  across the swap.

The two are opposites, and the default flipped to permissive in v2.13.0 (`CHANGELOG.md:50`
"`+ [uniform_lock] Allow players to pick up uniform even if not allowed (new parameter)`"). The
rulebook clause "Stealing uniforms … is prohibited" therefore became **socially** enforced rather
than mechanically, and the enforcement moved to the *uniform photograph* in the briefing (§3 step 4)
plus referee sanction.

### 12.5 What the framework forbids by omission

Read as design decisions, these are as informative as the rules:

- **No respawn.** One life is the default and everything else is labelled "useful for trainings"
  (`README.md:95`). Death moves you to spectator with the mission still running.
- **No revive, no medical, no tickets.** OMTK delegates to ACE and never models attrition as a
  resource (§8).
- **No in-game arsenal.** Your kit is decided by the mission maker at authoring time, in YAML, and
  baked into `mission.sqm` (§5). There is no crate, no loadout menu, no re-kitting.
- **No NVGs for infantry, anywhere, in any of 57 faction files.** Night vision exists only as a
  vehicle-borne resource (§5.7).
- **No laser designators for REDFOR** — zero `anpeq`-class items in `redfor-cargos.yml`.
- **No demolition charges or defusal kits for REDFOR** — BLUEFOR trucks stock them, REDFOR trucks do
  not.
- **No thermal imaging by default.** `OMTK_MODULE_VEHICLES_THERMALIMAGING` defaults to off and the
  module calls `disableTIEquipment true` on every vehicle (`library.sqf:413-419`), re-applying it
  whenever a static weapon is assembled (`vehicles_thermalimaging/main.sqf:8-10`) so a
  backpack-deployed drone cannot smuggle TI in.
- **No artillery computer by default** (`OMTK_MODULE_ARTY_COMPUTER`, default off) — indirect fire is
  meant to be computed by hand, which is why an unrelated *Mortar Calculator* is a first-class
  community tool.
- **No unregistered vehicle supply.** Any empty vehicle whose classname is not in the cargo YAML has
  its inventory **forcibly emptied** by the loadout tool (§5.6).
- **No AI in the field.** Playable AI is deleted at the end of warm-up by default; AI that must exist
  is skill-nerfed to 0.1–0.4 across every sub-skill.
- **No AI retaliation against friendly fire.** `ia_manager/main.sqf:56` gives every player
  `addRating 1000000`, documented at `omtk/ia_manager/README.md:18`: "in case of team-kill, IA units
  automatically engage the author. It is artificially disabled by adding a huge Rating to each human
  player."
- **No free-roaming during the briefing window.** The warm-up leash teleports you back and logs it as
  a cheat (§9.1); vehicle engines are frozen by an `Engine` event handler that switches them off
  again (`warm_up/main.sqf:97-108`); fuel used to be drained instead
  (`omtk/warm_up/README.md:19,33`).
- **No shooting during warm-up** if `OMTK_MODULE_WARM_UP_SAFETY` is on — implemented as an
  un-removable `addAction` named "Weapon safety on" (`library.sqf:421-430`).

### 12.6 What the framework forbids the *mission maker*

- **You cannot ship a mission with a private framework fork and expect support** — every mission
  embeds a full copy of OMTK, so version drift is the maker's problem.
- **You cannot express OR conditions in an objective.** `LIST` is an AND fold; the README states it
  outright (`score_board/README.md:109` "there is no OR condition").
- **You cannot invent a side combination.** Only the seven literal strings in
  `score_board/main.sqf:84-149` are accepted, in a fixed order
  (`score_board/README.md:180` "The order MUST be the one shown here").
- **You cannot use more than 10 flag numbers**, because timed capzone mirrors occupy `n+10`
  (`score_board/main.sqf:109`).
- **You cannot exceed 10 admins** without editing `zeus_admins/main.sqf` (`init.sqf:69`).
- **You cannot use a non-vanilla unit class as a "chief"** for interactive startup — the class list
  is vanilla-only (`dynamic_startup/interactive.sqf:5`).
- **You cannot place a unit type the loadout tool does not know** and expect it to be kitted; it
  errors with `"Class not found: '<class>' in side '<color>'"`.
- **You cannot binarize the mission.** `README.md:56` requires non-binarized save, and the loadout
  tool parses `mission.sqm` as text.

### 12.7 The referee is part of the ruleset

Most Arma frameworks treat admin tools as an afterthought. OMTK treats the referee as a first-class
role with a graduated sanction ladder, all reachable from the ESC menu (§11.4):

1. **Warn** — `Warn lonewolf` flashes a red full-screen message *and* locks the offender's weapon for
   11 seconds (`omtk/library.sqf:346-358`).
2. **Disarm** — `Toggle Safety` locks a named player's weapon indefinitely with the message
   *"THE ADMINS HAVE ENABLED SAFETY ON YOUR WEAPONS — You won't be able to shoot until the admins
   re-enable it"* (`omtk/library.sqf:367`).
3. **Reposition** — `Teleport player` moves a player to another named player
   (`omtk/library.sqf:336-344`) — used to put a stray back with their squad.
4. **Repair** — `Reset Inventory` restores the mission-start loadout
   (`omtk/library.sqf:396-403`); `Full heal`; `Respawn` for a player killed by a bug.
5. **Pause the match** — `DISABLE Sim ALL`, with the referee exempt and a randomised staggered
   restart (§11.4).
6. **Cut the clock** — `End Warm-up`; `Show Scoreboard` to force the end-of-mission display.

Plus permanent Zeus for up to ten whitelisted UIDs, and a `CHEAT`-tagged RPT trail that survives the
match for post-hoc adjudication. `omtk/briefing.sqf:96` is explicit that this is a human system:
"The mission maker is authorised to create more rules according to their desires."

---

## 13. What this framework does better than anyone else

**1. The rulebook ships with the mission and is enforced with graded sanctions.**
No other Arma framework in this comparison set puts its community's competitive rules into a diary
tab (`omtk/briefing.sqf:84-103`), then compiles the two most abusable ones — squad cohesion and radio
theft — into runtime detectors with a dedicated `CHEAT` log tag, then gives referees an in-game
ladder from "flash a warning" through "lock this player's weapon" to "freeze the entire match with a
randomised restart". The lonewolf rule in particular is a *social* norm ("the space between the more
distant members of the squad should not be more than 200 meters") turned into arithmetic with three
selectable tolerance bands (210/230/250 m infantry, 615/645/675 m vehicle) and correct exceptions for
incapacitated, handcuffed and airborne teammates. That is a design achievement, not a feature.

**2. Loadouts are a compiled artefact with inheritance, not a hand-edited blob.**
`default` + per-role deep merge + the `"8x 30#classname"` quantity/rounds grammar + **explicit-null
as the mechanism for removing an inherited item** is a genuinely good little language. It made 57
faction kits × 28 roles maintainable, and it made *balance policy* legible as data: you can grep the
corpus and prove "no infantry NVGs anywhere", "binoculars to eight roles", "one launcher and three
mixed warheads", "twelve magazines West, eight East, five to seven insurgent". Crucially it compiles
**into `mission.sqm`** and leaves zero runtime footprint — the authoring artefact is deleted before
the mission ships.

**3. One mission file, many run modes, chosen in the lobby.**
The same PBO is a training server (respawn 30 s), a briefing room (map exploration: click to
teleport, spawn a helicopter, reset the time of day), a mod-check sandbox (immortal), and the
competitive match — decided at launch by two dropdowns, with the mission clock deliberately not
starting until warm-up ends. `README.md:67` states it plainly. That is a much better answer to
"how do we rehearse" than shipping three mission variants.

Honourable mentions: the **warm-up as a first-class phase** (immortality, engine freeze, a visible
leash circle, reduced view distance during load, a countdown HUD, then a full-view-distance raise
15 s before go); and the fact that `table_forum.sqf` exists at all — OFCRA understood a decade ago
that the ORBAT needs to leave the game as structured data for the website.

---

## 14. Friction and known complaints

Evidence-ordered: defects verified in source, then dead features, then documentation drift, then what
the comments admit.

### 14.1 Confirmed defects that affect mission makers

| Defect | Evidence |
|---|---|
| **`OMTK_SB_MISSION_DURATION_OVERRIDE` does not work on the server.** `score_board/main.sqf:7` tests `!isNil "_mission_duration_override"` on a *local* never assigned in that scope (first assigned at `:260`, inside the later `hasInterface` block), so the test is always false and the server always uses the lobby value — while `briefing.sqf:43-48` and `main.sqf:260-269` *display* the override to clients. A documented knob, silently ignored. | `score_board/main.sqf:7,260`; `briefing.sqf:43` |
| **Survivor counting reads the wrong unit.** `_dmg = damage _x;` is computed and discarded; all three tests use `damage player`. Feeds every supremacy/DIFF objective. | `score_board/library.sqf:41,44,47,50` |
| **The documented `OMTK_ID` objective mode cannot match.** README says `setVariable['OMTK_ID',…]`; the code reads `mt_id`. Its mode inversion is also commented out, so it always behaves as DESTRUCTION. | `score_board/README.md:113` vs `library.sqf:447,453` |
| **Scoreboard rows desynchronise from scores if any objective is worth 0 points.** The display loop skips zero-point rows while the scoring loop advances its index unconditionally. | `loadBoard.sqf:11` vs `score_board/library.sqf:71-75` |
| **"DISABLE Sim ALL" is JIP-persistent and never cleared.** `remoteExec[…, 0, true]` queues a persistent JIP entry that "Enable Sim ALL" does not remove. `INFERRED:` every subsequent joiner is frozen on connect for the rest of the mission. | `pauseScreenMenu.sqf:636,652` |
| **`admin_uids` defaults to the scalar `0`**, so `_uid in 0` is a type error rather than a clean deny if the `publicVariable` has not landed. | `pauseScreenMenu.sqf:112`, `library.sqf:516`, `warm_up/main.sqf:186` |
| **`idd 1777` is declared twice.** | `dialog_interactive_startup.hpp:3`, `dialog_action_progress.hpp:3` |
| **The advertised admin player-count ticker never runs** — `while { time < 1}`, and `time` exceeds 1 within the first second, contradicting `CHANGELOG.md:39`. | `warm_up/main.sqf:190` |
| **GREENFOR is never teleported to base in markers mode** (`_Greens` filled, `_GreenUnits` teleported), and `case "g_obj"` passes 2 arguments to a function that dereferences `_this select 2`. | `dynamic_startup/markers.sqf:98,103,168,80` |
| **Off-by-one in the interactive vehicle picker** — `for "_i" from 0 to (lbSize 1502)` iterates one past the end, causing a `parseNumber ""` → 0 that reads a non-existent third element of the OB header row. | `dynamic_startup/loadPanel.sqf:54,90` |
| **A live debug line ships:** `systemChat "test";` inside the `WeaponAssembled` handler. | `warm_up/main.sqf:115` |
| **28 controls share IDC `1201`** in the admin menu. | `pauseScreenMenu.sqf` |
| **The loadout CLI's default file paths do not exist** — renamed in 2016, defaults never updated. | §5.5, `CHANGELOG.md:218-219` |
| **~11 silent data bugs in the loadout corpus** — mis-indented keys, `weapons:` missing its `primary:` level, `headgear` nested under `backpack`, two vehicle types with empty `classes:`. | §5.9 |
| **`OMTK_MODULE_ARTY_COMPUTER` is inverted or mislabelled.** The parameter is titled "Artillery Computer" with `off/on → 0,1`, but `load_modules.sqf:49` runs `enableEngineArtillery false` when the value is **> 0** — i.e. selecting "on" *disables* the artillery computer. | `description.ext:136-142`, `load_modules.sqf:49` |

### 14.2 Dead features still visible to the maker

- **"Side is ready" is gone.** `description.ext:244-249` declares `OMTK_END_WARMUP_COM_MENU` pointing
  at `#USER:OMTK_WARMUP_MENU`; `init.sqf:52-56` has that menu **commented out**; and
  `omtk_wu_set_ready` — referenced in `warm_up/main.sqf:26` and described in
  `warm_up/README.md:30` — is **defined nowhere** (grep for `omtk_wu_set_ready =` returns nothing).
  The documented "both sides declare ready, warm-up ends early" feature does not exist at HEAD; only
  the admin's "End Warm-up" button remains.
- **`OMTK_WU_CHIEF_CLASSES` does not exist** despite `warm_up/README.md:84-88` telling the maker to
  customise it.
- **Per-side simulation control** is built but unreachable (`pauseScreenMenu.sqf:329-401`); the
  receiver still honours the per-side path.
- **`OMTK_DS_CHOSEN_SPAWN_FOR_PLAYER`, `OMTK_DS_VEHICLES_MAX_NB`,
  `OMTK_DS_VEHICLES_MAX_NB_PER_GROUP`, `OMTK_DS_CHOSEN_VEHICLES`** are declared at
  `dynamic_startup/interactive.sqf:1-4` and never read.
- **`ACTION_DISPUTEE`** — an empty `case` (`score_board/main.sqf:318-320`).
- **`TRIGGER`** objective type — an empty `case` (`score_board/library.sqf:171-173`).
- **The `--script` SQF-generation path and the `Roster` subcommand** in the loadout tool are
  commented out / unregistered (§5.11).
- **The damage-immunity detector** was added in v2.13.3 and its driver loop removed in v2.13.5
  (`CHANGELOG.md:22,16`); the function survives, the loop is commented out
  (`score_board/main.sqf:325-332`).
- **`OMTK_MODULE_UNIFORM`'s "Locked (old method)"** is explicitly labelled old in the lobby itself.

### 14.3 Documentation drift

- `omtk/dynamic_startup/README.md` still documents a **`launch_mode` module** with modes
  `standard / campaign / markers / test / briefing`, none of which exist. Data-card date
  **2015-08-01**. The `OMTK_LM_` global prefix is a fossil of the same rename.
- `README.md:69-172` misstates most parameter ranges and defaults and omits **eleven** live
  parameters (§10.1).
- `README.md:8,10` claims no CBA and optional ACE; both are contradicted by the code (§1).
- `markers_doc.sqf:2` says "marker's **names**"; the code matches `markerText`. It also omits the
  three GREENFOR marker texts the code supports.
- `tactical_paradrop/README.md:70` quotes a rejection message the code does not emit
  ("Unathorized area, jump somewhere else !" vs the actual "Forbidden zone, try again !").
- `README.md` and `score_board/README.md:223` link to `omtk/wiki/img/*`; **there is no `omtk/wiki/`
  directory in this checkout**.
- All **thirteen** module data cards carry the same typo, `Ojective` (verified: `grep -l Ojective
  omtk/*/README.md` returns 13 of 13).
- `briefing.sqf:16` still emits the placeholder `"Texte 3"` as an objective's third description field.
- `score_board/library.sqf:198` logs `"unkown objective type"`.
- `omtk-loadouts/infantry/README.md` is a 1-byte file containing only a newline.

### 14.4 What the comments admit

The source is unusually candid. Verbatim:

- `score_board/library.sqf:127` — `// Too lazy to implement the else, let's hope it won't happen`
  (three-faction tie handling)
- `warm_up/main.sqf:95` and `warm_up/library.sqf:70` — `// Vehicle freeze, taken from ilbinek's IMF
  because i give up on life` / `// Vehicle unfreeze, taken from ilbinek's IMF because i give up on
  life`
- `zeus_admins/main.sqf:21` — `// I've read online that this is needed`
- `zeus_admins/main.sqf:4` — `// This should work JIP/rejoin too but i've got no idea.`
- `warm_up/library.sqf:24` — `setTriggerActivation […]	// probably useless`
- `rambo_warn/main.sqf:36` — `//Write in local rpt... could be exploited for cheating (notepad++
  tail -f)` — a known, unfixed cheat vector
- `rambo_warn/main.sqf:35` — `//Snitches get Stiches?`
- `rambo_warn/main.sqf:5-13` — a seven-line TODO block including `// How to define Parameter?`,
  `// Detect active "Ramboing"`, `// To Check: What about drones?`, `// Multi Language...`
- `rambo_warn/main.sqf:37,140` — `//Lot of stupid stuff, can be removed later`, `//Debug Thingy;
  remove later`
- `score_board/library.sqf:536` — `// TODO Display progress bar using _dur, then..`
- `score_board/library.sqf:38` — `_name = name _x; // test if name is OK TODO`
- `fn_inventoryBriefing.sqf:1-4` — `Newly added logs will likely clog up the client's logs. Remove
  the logs in _addExtPAA and _addToArray functions to lighten the load.`
- `score_board/README.md:175` — `This works on timed objectives too, but why would you do it, if
  their entire point is so that you don't have to do it??????`
- `uniform_lock/lock.sqf:8` — `// Keep the "uniform slot" control on lockdown. Else there are loop
  holes.`
- `library.sqf:318` — `// Has to be remotely called on clients from server - DEPRECATED`
- `score_board/main.sqf:8-9` — `// Changed variable to be as optimized as possible (at the cost of
  readability)`

### 14.5 Structural friction, ordered by how much it hurts a mission maker

1. **Authoring is hand-edited SQF array literals with no validation.** The objectives table is a
   nested array of positional fields whose meaning changes by objective type, with a README that has
   to warn about trailing commas (`score_board/README.md:79`). A typo surfaces as an RPT line and a
   systemChat message *during the match*.
2. **Every mission carries a private copy of the framework.** No shared runtime, no upgrade path, no
   way to hotfix a shipped PBO.
3. **The loadout toolchain is a decade-old Windows-only executable** (Ruby 2.2 inside an Ocra stub,
   version 1.0.1, 2016) whose SQM-version whitelist `[12,51,52]` predates a decade of Arma patches,
   whose default file paths are broken, and whose only documentation lives inside the binary.
4. **The two dynamic-startup modes are parked under "OLD PARAMETERS"**, default off, with a stale
   README, a vanilla-classname gate an RHS mission cannot satisfy, and known argument bugs. The
   generative marker workflow — the most editor-like idea in the framework — is effectively
   abandonware.
5. **No packaging step.** The maker must remember to delete `omtk-loadouts/` and `script_library/`
   by hand (`README.md:64`); nothing checks, and nothing warns.
6. **Referee actions are unauthenticated on the wire.** The gate is a UI-drawing gate; every action
   is a client-issued `remoteExec` targeting players by *display name*. Protection depends entirely
   on a `CfgRemoteExec` whitelist that is not in this repository.
7. **Three-faction support is bolted on.** GREENFOR works in the scoreboard and roster but not in
   `omtk_get_side`, not in paradrop, not in interactive startup, has no loadout file, and its tie
   case is an admitted TODO.
8. **Nothing tells you the mission is misconfigured until it is running.** There is no validator, no
   CI, no test harness, no schema — the only dry-run in the whole toolchain is
   `omtk-loadouts.exe test`, and it only checks loadouts.
