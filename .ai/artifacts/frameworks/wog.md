# WOG — Weekly Open Games (Arma 3 mission framework)

> Reverse-engineered from shipped binaries and 171 real missions. **There is no
> repo, no README and no upstream documentation for this framework.** Everything
> below is derived from the artefacts named in the Source inventory. Inference is
> labelled `INFERRED:`.

## Method note (needed to verify the citations)

Two decoding steps were performed; both are reproducible and neither guesses:

1. **De-rapification.** 107 `.bin` files are binarized (rapified) configs. I wrote
   a de-rapifier for the documented `\0raP` format
   (`/tmp/claude-1000/-home-Samuel/429c40b7-ef6b-4d3b-8b75-4dac381575e0/scratchpad/derap.py`).
   **76 of 107 decoded** into readable config text. The remaining 31 are *not*
   rapified configs (`Texheaders.bin`, `stringtable.bin`, and mojibake-named blobs)
   and **remain opaque — binarized**.
   Citations to decoded configs use the **class path**
   (e.g. `wog3_3den/config.bin → Display3DEN >> ... >> WOG3_CountSlots`) rather than
   a line number, because line numbers belong to my output, not the shipped file.
2. **Mission SQM decoding.** 86 of 171 `mission.sqm` are rapified; 85 are plaintext.
   All 171 were rendered readable and machine-parsed
   (`scratchpad/orbat.py`, `scratchpad/wmt.py`). **This matters: an early
   plaintext-only grep reported "wog3_presets is used by 0 missions", which was
   wrong** — the two missions that use it have binarized SQMs. Corpus claims below
   are computed over all 171 decoded SQMs.

`.sqf` / `.hpp` / `.cpp` / `.xml` citations are exact `file:line`.

---

## Source inventory

### Addons read

| Addon | Format read | What it yielded |
|---|---|---|
| `wog3_3den` | 20 `.sqf` (all readable) + `config.bin` (de-rapified) | **The entire Eden editor extension** — menu strip, context menu, mission event handlers, the one custom Eden attribute. Section 11. |
| `wog3_presets` | `Config.cpp`, `resource/CfgModules.h`, 5 `.sqf`, `stringtable.xml`, `XEH_preInit.sqf` | Force-package picker module: commander chooses 1 of N vehicle sets, which then spawn at markers. Sections 7, 10. |
| `wog3_arsenal` | `CfgFunctions.hpp`, 5 `.sqf`, `tmp.txt`, `config.bin` (de-rapified) | Arsenal IMPORT/EXPORT buttons and three loadout export formats. Section 5. |
| `wog3_inventory` | `fn_medicalEquipment.sqf`, `config.bin` (de-rapified) | Forced standard medical kit for every player; gear/join-group ACE actions. Section 8. |
| `wog3_admin` | 8 `.sqf`, `config.bin` (de-rapified) | In-game admin chat commands, UID allowlist. Section 11. |
| `wog3_tfar` | `XEH_postInitClient.sqf`, `config.bin` (de-rapified) | Automatic long-range radio for group leaders. Sections 10, 12. |
| `wog3_hardfreeze` | `XEH_postInit.sqf`, `config.bin` (de-rapified) | Player-count-scaled start freeze, TeamSpeak gate. Section 8. |
| `wog3_mod` | `XEH_postInit.sqf`, `config.bin` (de-rapified) | Global runtime hygiene (saving off, radio off, map click off). Section 10. |
| `wog3_ace_settings` | `ACE_Settings.hpp` (991 lines), `CfgWeapons.hpp` | House ACE repair/medical timings. Section 8. |
| `wog3_artui` | `config.bin` (de-rapified) | Mortars get range tables + maptools in cargo; `artilleryScanner = 0`. Section 10. |
| `wog3_usmc` | 14 `.hpp` + `Config.bin` (de-rapified, 3070 lines) | A shipped playable faction with 28 Eden group presets. Section 4. |
| `wog3_rearm` | `Config.cpp` | `transportAmmo = 0` on supply trucks/boxes. Section 10. |
| `wog3_disable_vehicle_lock` | `config.cpp` | Removes Tab-lock from all vehicles except AA. Section 10. |
| `wog3_c_heavyweapons` | `config.bin` (de-rapified) | Tank reload sound only. Cosmetic. |
| `wog3_rhs_*` (6 addons) | `config.bin` (de-rapified) + `.hpp` | Vehicle sensor/repair/refuel/uniform balance patches. Not authoring-facing. |
| `wog_main` | attempted | **Opaque — deliberately obfuscated.** See "What could not be read". |
| `wog_*` (33 tweak addons) | spot-checked | Weapon/ammo/vehicle balance. Not authoring-facing; not documented here. |

### Missions read

**All 171 mission PBOs were extracted and machine-analysed** (every `mission.sqm`
parsed into a class tree; corpus-wide greps over every file). **13 were additionally
read file-by-file:**

| Mission | Lineage | What it yielded |
|---|---|---|
| `wog_160_odins_sword_13.cup_chernarus_A3` | WOG-native | The canonical WOG mission: 1-line `description.ext`, `init.sqf`, `initServer.sqf`, `briefing.sqf`, `tpl/<faction>/<role>.sqf` loadouts |
| `wog_185_operation_magnolia_12.sara_dbe1` | WOG-native | `Equipment/<FACTION>/<ROLE>.sqf` loadout variant naming |
| `wog_147_papers_please_11.WL_Rosche` | WOG-native | `CfgUnitInsignia` in `description.ext`; ACE cargo preloading in `init.sqf` |
| `wog_78_krater_17.ProvingGrounds_PMC` | WOG-native | `class Params`, stringtable, "Weekly Open Games" info text |
| `wog_120_rostok_dm_14.VTF_Korsac` | WOG-native | Deathmatch outlier: 1 group, 211 markers, `respawnOnStart = 1` |
| `wog_45_extraction_10.lingor3` | WOG-native | Minimal mission: 2-line `init.sqf`, no `description.ext` |
| `BattleofDinant1914v4FINAL.SWU_Ardennes_1940` | OFCRA/OMTK | Full `omtk` toolkit tree + 100-line `class Params` |
| `DienBienPhu.csj_lowlands` | OFCRA/OMTK | Identical `description.ext` header block — copy-paste proof |
| `TowerDefense.Altis` | OFCRA/OMTK | Same header; ships `LICENSE`, `CHANGELOG.md`, `README.md` |
| `Congo_Romanian_Mercs_28.pja312` | third-party | 5-file mission; `zeusTracer.sqf` |
| `AngolisDefensev3.OPTRE_Kholo` | third-party | SQM `Attributes` shape (where `isPlayable`/`description` live) |
| `MSN_Charlotte_SWPMC_OP_1V8.tem_kujari` | third-party | 2-file mission — everything in the SQM |
| `Nam_Five_Off_SOG.Cam_Lao_Nam` | third-party | `CfgPatches` inlined into `description.ext` |

### What could not be read

- **`wog_main` — opaque, obfuscated.** Its PBO header contains mojibake filenames
  (`Чіяг?ї`, `уР?�ф`) confirmed present *in the archive index itself*, not introduced
  by extraction — `unpbo.py --list` on `pbo/wog_main.pbo` shows them, while
  `wog3_presets.pbo` lists clean names. `wog_main/config.cpp` is a single
  `#include "Чіяг?ї"` and the included files are byte-scrambled. The same scrambling
  affects `wog_logo`, `wog_gas`, `wog_surfaces`, `wog_airSound_fix`,
  `wog_bwa3Tweaks`, `wog_rhs_c_weapons`, `wog_quotation_fix`, `wog_manpads`,
  `wog_baf_c_vehicles`, `wog_cup_tweaks`. **I do not guess their contents.**
- **`wmt_main` is entirely absent from the corpus** — not in `extracted/`, not in
  `pbo/`. It is a hard `requiredAddons` dependency of `wog3_presets`
  (`wog3_presets/Config.cpp:18`) and `wog3_disable_vehicle_lock`
  (`config.cpp:12`), and supplies `WMT_fnc_CreateLocalMarker`, `WMT_pub_frzState`,
  `WMT_fnc_BriefingMap`, `WMT_fnc_ShowTaskNotification` and **all `WMT_*` Eden
  modules**. This is the single biggest gap: *the mission-maker-facing module set
  lives in an addon I was not given.* Section 7 reconstructs its parameter surface
  from mission usage — parameter **names and values are hard evidence**, parameter
  **semantics are `INFERRED:`**.
- **`wog3_fraction` and `wog3_lav25_t` do not exist** in this corpus (named in the
  task brief but absent from both `extracted/` and `pbo/`).
- **21 `.sqfc` (compiled SQF) — opaque, but harmless:** every one of the 21 has a
  readable `.sqf` twin at the same path (verified: 0 orphans). Nothing is lost.
- **`stringtable.bin`** in `wog3_3den` and `wog3_hardfreeze` — opaque. Minor loss:
  the 3den menu labels are hardcoded Russian literals in `config.bin`, not
  stringtable keys.

---

## 1. Identity

**WOG — "Weekly Open Games"**, a large Russian-language Arma 3 milsim community
running weekly public games at ~120–185 players per session.

- Home page `https://wogames.info/`, present as `authorUrl` in most `CfgPatches`
  (e.g. `wog3_admin/config.bin → CfgPatches >> wog3_admin >> authorUrl`).
- The name is spelled out in a shipped mission:
  `wog_78_krater_17.ProvingGrounds_PMC/init.sqf:14` —
  `["Weekly Open Games", "Кратер"] spawn BIS_fnc_infoText;`
- Authors named in configs: **Ezhuk** (most `wog3_*` runtime addons), **Kato**,
  **`[KND] Liquid`**, **`[GHOST]Mr.Purple`** (`wog3_3den/config.bin → CfgPatches >>
  wog3_3den >> author[]`), **`[GRU]Vincen`** (vehicle balance addons).
- It is **not a mission template**. There is no "copy this folder and start" skeleton.
  WOG is **a mod-side framework**: a set of always-loaded addons that impose
  behaviour on any mission, plus one Eden editor extension. The mission maker
  authors an ordinary Arma 3 Eden scenario; WOG's addons supply the game mode,
  briefing map, medical standardisation, radios and admin tooling from outside the
  mission file.
- **Scale is the design driver.** `wog3_hardfreeze/functions/XEH_postInit.sqf:16-19`
  branches on `>160`, `>130`, `>100` players; `:35` gates a whole subsystem on
  `> 70` players. Corpus median is 137 playable slots.

## 2. Mission file layout

There is **no enforced layout**. A WOG mission is a stock Arma 3 mission folder.
Measured across all 171:

| Root file | Missions carrying it (of 171) | Of the 78 WOG-native |
|---|---|---|
| `mission.sqm` | 171 | 78 |
| `init.sqf` | 115 | **78 (100%)** |
| `briefing.sqf` | 76 | **76 (97%)** |
| `description.ext` | 116 | **25 (32%)** |
| `initServer.sqf` | 76 | 22 |
| `initPlayerLocal.sqf` | 52 | 5 |
| `onPlayerKilled.sqf` / `onPlayerRespawn.sqf` | 34 / 34 | few |
| `zeusTracer.sqf` | 47 | — |
| `stringtable.xml` | 11 | 1 |

The striking result: **WOG-native missions overwhelmingly ship no
`description.ext`.** Of the 25 that do, 8 are zero-length and one is a single line
(`wog_160_odins_sword_13.../description.ext:1` = `overrideHazeQuality = 2;`).
By contrast the 91 non-WOG-native `description.ext` files have a median of 135 lines.

`INFERRED:` this is the framework's central structural fact — WOG moved mission
configuration **out of `description.ext` and into Eden-placed modules**
(`WMT_Main`, `WMT_Time`), so there is nothing left for `description.ext` to hold.

Loadout scripts are the only substantial convention, and there are two spellings:

- `tpl/<faction>/<role>.sqf` (+ `tpl/<faction>/veh/<vehicle>.sqf`) — e.g.
  `wog_160_odins_sword_13.cup_chernarus_A3/tpl/cdfass/{sl,tl,mg,at,me,rm,ar,gr,sn,sp,as}.sqf`
- `Equipment/<FACTION>/<ROLE>.sqf` — e.g.
  `wog_185_operation_magnolia_12.sara_dbe1/Equipment/SLA/{SL,TL,MG,AT,MED,RIF,...}.sqf`

Corpus-wide file-name frequency confirms the role vocabulary is stable:
`main.sqf` 497, `med.sqf` 136, `tl.sqf` 130, `sl.sqf` 130, `at.sqf` 101, `mg.sqf` 99,
`rm.sqf` 52, `crew.sqf` 50, `cr.sqf` 44, `rf.sqf` 43, `pl.sqf` 43, `gl.sqf` 42,
`pul.sqf` 41, `me.sqf` 38, `mk.sqf` 37, `ko.sqf` 37, `box.sqf` 36.

## 3. Authoring workflow

Reconstructed from the 3den extension's own affordances plus what the corpus shows
makers actually produced. Ordered as a maker would perform it.

1. **Open Eden with the WOG mod set loaded.** `wog3_3den` requires
   `3den`, `wog3_arsenal`, `ace_repair` (`wog3_3den/config.bin → CfgPatches`).
2. **Create the scenario.** On `OnMissionNew`, WOG stamps ~20 mission attributes
   automatically, including the mission name **pre-filled in house format**:
   `wog3_3den/functions/fn_onMissionNewEH.sqf:3` —
   `['Scenario', 'IntelBriefingName', 'WOG 160 Best Mission 1.0']`, and an overview
   text that literally tells the maker the required shape of a WOG mission:
   `:20` — `'attack fraction (side color, attack) vs defense fraction (side color, defense)'`.
   This is the framework teaching its own genre in a form field.
3. **Name the mission `wog_<slots>_<name>_<version>.<terrain>`.** Enforced by
   convention, not code — but 73 of 78 WOG-native missions (94%) have a filename
   slot count that **exactly equals** the parsed playable-slot count (see §15).
4. **Place `WMT_Main` and `WMT_Time`.** These are the game-mode and timing modules
   (78 and 77 of 78 WOG-native missions respectively). They carry the settings that
   in other frameworks would live in `description.ext` — view distance, disabled
   channels, name tags, thermals, auto-medicine, loss coefficients (§7, §10).
5. **Build the ORBAT in Eden.** Place groups, set every slot playable
   (`Tools ▸ WOG 3den Tools ▸ Сделать все слоты игровыми` runs
   `fn_setallplayable.sqf`, which sets `ControlMP` on **all** entities at once).
6. **Write slot descriptions as `Role@Group`.** The text before `@` is the role label,
   the text after is the squad grouping (§4).
7. **Count slots to hit the target.** `Tools ▸ Подсчитать количество игровых слотов`
   → `fn_countplayableslots.sqf:8` prints `WEST = %1 EAST = %2 GUER = %3 CIV = %4`
   plus `Всего слотов` ("total slots"). This is the tool that makes step 3's
   filename convention checkable.
8. **Kit each slot via the Arsenal.** Open Arsenal on a unit, build the loadout,
   press **`WOG: EXPORT`** → an imperative SQF script lands on the clipboard. Paste
   into `tpl/<faction>/<role>.sqf`. Put `this call compile preprocessFileLineNumbers "tpl\<faction>\<role>.sqf"`
   in the unit's init field. `WOG: IMPORT` reads the clipboard back onto a unit, so
   the round-trip is editable (§5).
9. **Fill vehicle/crate cargo,** then right-click ▸ `Занести содержимое Ammobox в
   буфер обмена` to dump that cargo as an SQF template (`fn_copyammotosqf.sqf`), which
   emits a file header naming its own destination: `tpl\<name>.sqf` (`:11`).
10. **Set ACE medic/engineer flags.** `Tools ▸ Настройки ACE Медик & Инженер по
    умолчанию` sets `ace_isMedic` and `ace_isEngineer` to `-1` on **every** entity
    (`fn_acesetdefaultmedeng.sqf:2`) — a reset-to-default sweep.
11. **Validate.** Two checkers exist, both clipboard-output, neither wired to a menu
    (§11.1): `fn_check_weapon.sqf` (ammo counts, missing vest/uniform, wrong-side
    radios) and `fn_check_lr.sqf` (long-range radio present, correct side, no
    duplicates, one per group leader).
12. **Write `briefing.sqf` by hand** as a sequence of `player createDiaryRecord`
    calls, switched on `side player`, using the canonical section titles (§6).
13. **Write `init.sqf`.** In 54 of 78 WOG-native missions the first line is exactly
    `[] execVM "briefing.sqf";`. Add `[] call WMT_fnc_BriefingMap;` (31 missions) and,
    very commonly, `wog3_no_auto_long_range_radio = true;` (74 of 171 missions).
14. **Save.** `OnMissionSave` runs a slot-description auto-tagger that appends
    ` | Med` / ` | Eng` to the role label of ACE medics/engineers
    (`fn_onMissionSaveEH.sqf`). **This is broken — see §14.**

`INFERRED:` steps 5–9 are the reconstructed order; the tools exist and the corpus
shows their outputs, but nothing in the source dictates sequence.

## 4. Slotting / ORBAT model

The ORBAT model is **stock Arma 3 Eden**: groups contain units, a unit becomes a slot
when `isPlayable = 1` is set in its `Attributes` sub-class. WOG adds three things.

**(a) The `Role@Group` description convention.** Slot descriptions are written
`"<role label>@<group label>"`. Evidence that WOG's own tooling parses it:
`wog3_3den/functions/fn_onMissionSaveEH.sqf:3` —
`private _desc = (_x get3DENAttribute "description") select 0 splitString "@";`
— then it edits only `_desc select 0` and rejoins with `"@"` (`:7-8`).
Corpus confirms the shape is universal, e.g. `description="1: Squad Leader@Team 1"`
(21 missions), `description="1: Officer@Command Team"` (25), `description="Zeus@Azus"` (33).
The leading `"1: "`, `"2: "` … numbers the position within the group.
`INFERRED:` the `@` split is Arma 3's own MP-lobby role/squad separator; I cannot
verify engine rendering from these files, only that WOG's tooling round-trips it.

**(b) Automatic Med/Eng advertisement in the slot list.** On save, WOG appends
` | Med` and/or ` | Eng` to the role label of any unit ACE considers a medic or
engineer (`fn_onMissionSaveEH.sqf:6-13`), so players choosing slots in the lobby can
see which are medics without opening the briefing. **The intent is excellent; the
implementation is broken (§14) and the corpus contains zero instances of its output.**
One mission hand-writes the uppercase form instead (`| MED` 29×, `| ENG` 30×, both in
a single Star-Wars-themed mission).

**(c) A shipped faction with Eden group presets.** `wog3_usmc` registers
`[WOG] USMC` (`CfgFactions.hpp:3-9`) and contributes **28 pre-built groups** to the
Eden group palette across four categories (`CfgGroups.hpp:5-11`), counted from the
de-rapified `Config.bin` tree: `group_usmc_wd` 12, `group_usmc_des` 12,
`group_marsoc_wd` 2, `group_marsoc_des` 2. Each group is a full unit list with
per-unit `vehicle`, `rank` and `position[]` (e.g. `group_usmc_wpsq` "Weapon squad",
`groups/usmc_wd.hpp:6-30`).
`wog3_3den/config_lop_fix/config.bin` additionally renames 14 LOP factions into a
consistent `[LOP] …` display scheme across West/East/Indep/Civ.

**Measured ORBAT shape (all 171 missions):**

- 19,627 playable slots total. min 9, p25 37, **median 137**, mean 115, max 324.
- Slots by side: **West 10,156 (52%) · East 8,301 (42%) · Independent 1,156 (6%) ·
  Civilian 14 (0%)**.
- Groups by side: East 2,478 · West 2,191 · Independent 762 · Civilian 79.
- Mean 14 editor **layers** per mission (2,396 total) — makers organise heavily.
- `groupID` is overridden 1,070 times (e.g. `Альфа 2-3`, `Blue Side Lead`).

`INFERRED:` the near-50/50 West/East slot split reflects the
"attack fraction vs defense fraction" genre that `fn_onMissionNewEH.sqf:20` prescribes.

## 5. Loadouts / arsenal

WOG does **not** ship a runtime virtual arsenal for players. `wog3_arsenal` is an
**authoring tool**: it bolts five buttons onto both the vanilla and the ACE arsenal
displays (`wog3_arsenal/config.bin → RscDisplayArsenal >> controls` and
`→ ace_arsenal_display >> controls`, identical sets):

| Button | Position | Calls |
|---|---|---|
| `WOG: Clean` | y 0.88, full width | `fn_arsenal_clean` — strips the unit bare (10 `remove*` calls) |
| `WOG: IMPORT` | x 0.335, y 0.91 | `fn_arsenal_import` |
| `WOG: Export to config` | x 0.505, y 0.91 | `fn_arsenal_export_to_config` |
| `WOG: EXPORT` | x 0.335, y 0.94 | `fn_arsenal_export` |
| `WOG: Export Loadout` | x 0.505, y 0.94 | `fn_exportLoadout` |

**The clipboard is the transport format.** `fn_arsenal_import.sqf:4-7`:

```sqf
private _code = copyFromClipboard;
if (_code isNotEqualTo "") then { _unit call compile _code; };
```

Three export dialects, deliberately different:

1. **`fn_arsenal_export.sqf` — imperative SQF, the round-trippable one.** Emits
   `if (not local _this) exitwith {};`, then eight `remove*` lines, then
   `forceAddUniform` / `addVest` / `addBackpack` / `addWeapon` /
   `addPrimaryWeaponItem` / `addItemToVest` … Repeats are collapsed into loops via
   `BIS_fnc_consolidateArray` (`:7-16`), producing
   `for '_i' from 1 to 5 do { _this addItemToVest '…';};`. This is exactly the form
   found in shipped `tpl/*.sqf` files — compare
   `wog_160_odins_sword_13.cup_chernarus_A3/tpl/cdfass/sl.sqf:1-28`. **This output
   is what `WOG: IMPORT` consumes, so a loadout can be pulled back into the Arsenal,
   edited, and re-exported.**
2. **`fn_exportLoadout.sqf` — the `setUnitLoadout` array form** (`:21-35`), a compact
   10-element array. Not consumable by IMPORT.
3. **`fn_arsenal_export_to_config.sqf` — a `CfgVehicles` class**, wrapping
   `BIS_fnc_exportInventory` output and additionally synthesising `respawnWeapons[]`,
   `respawnMagazines[]`, `respawnLinkedItems[]`, `respawnItems[]` from the
   corresponding non-respawn arrays (`:23-38`). For baking loadouts into an addon.

From Eden, right-click ▸ `Log ▸ Занести содержимое Arsenal в буфер обмена` runs
`fn_copyarsenaltosqf.sqf`, which loops selected `Man` objects and concatenates
`fn_arsenal_export` output for each, prefixed with `//<classname>` (`:8-9`).
The sibling `fn_copyammotosqf.sqf` does the same for vehicle/crate cargo, emitting
`clearWeaponCargoGlobal` + `add*CargoGlobal` pairs (`:12-29`).

**House loadout style, from the corpus.** `tpl/cdfass/sl.sqf` is representative:
strip first (`:3-10`), then randomise cosmetics so a squad does not look cloned —
`BIS_fnc_selectRandom` over weapon variants (`:12`), laser/light block (`:17`),
uniform (`:20`), and a `switch (selectRandom ["green","digital"])` for headgear
(`:32-43`). Magazines are added by counted loops (`:25-28`).

## 6. Briefing / intel

Briefings are **hand-written SQF**, not authored in Eden. 76 of 171 missions ship
`briefing.sqf` (76 of 78 WOG-native).

The house structure is a sequence of `player createDiaryRecord ["diary", [<title>, <html>, <icon>]]`
calls wrapped in `switch (side player)` so each side reads a different briefing —
`wog_160_odins_sword_13.cup_chernarus_A3/briefing.sqf:16-51`.
**No mission in the corpus calls `createDiarySubject`** — everything is written into
the stock `"diary"` subject.

Canonical section titles, counted over all `briefing.sqf`:

| Title (RU) | English | Count |
|---|---|---|
| `Задачи` | Tasks | 80 |
| `Вводная` | Situation / intro | 55 |
| `–––––––––––––––––––` | horizontal rule record | 48 + 18 |
| `Условности` | Conventions (house rules for this mission) | 42 |
| `Поставленная задача` | Assigned task | 28 |
| `Задача` | Task | 25 |
| `Формы сторон` | **Uniforms of the sides** | 18 (+8 `Форма сторон`) |
| `Доп.информация` / `Дополнительная информация` | Additional info | 16 + 13 |
| `Условия` | Conditions | 12 |
| `Условности \| Conventions` | bilingual variant | 11 |
| `Легенда` | Backstory | 11 |
| `Задача \| Task` | bilingual variant | 11 |
| `Форма обороны` / `Форма атаки` | Defender / attacker uniform | 9 / 9 |
| `Предупреждение` | Warning | 8 |

Two conventions are worth stealing:

- **A dedicated "uniform recognition" briefing section** (`Формы сторон` /
  `Форма атаки` / `Форма обороны`, 44 records total), usually paired with shipped
  images — `wog_160_odins_sword_13.../brf/uniform_b.paa`, `brf/uniform_r.paa`.
  At 137 players with mixed mod factions, telling friend from foe is a first-class
  briefing concern.
- **Briefing text hyperlinks to map markers.** `<marker name='east_ifv'>БМД</marker>`
  (`briefing.sqf:22`), `<marker name='s_0'>Солнечном</marker>` (`:28`),
  `<marker name='medS'>Электрозаводске</marker>` (`:36`). Clicking the word in the
  briefing centres the map on that marker. Used pervasively.

An **extended briefing** is a `WMT_Main` toggle — `WMT_Main_ExtendedBriefing`, set to
`1` in 78 of 79 module instances. `INFERRED:` this drives a WMT-supplied briefing
screen; the implementation is in the missing `wmt_main`.

`WMT_fnc_BriefingMap` is called from 31 missions' `init.sqf`.
`INFERRED:` it renders a pre-start map with the operation graphics.
`WMT_Main_DisableBreifingMarkerMove` (sic — misspelled in the shipped attribute
name) is present on all 79 instances, always `0`.

## 7. Objectives / game modes

**Objectives are Eden modules, not scripts.** This is WOG's most distinctive
authoring decision. All module types below are `WMT_*`, supplied by the **absent**
`wmt_main`; their parameter names and the values makers chose are recovered from the
171 SQMs. Parameter **semantics are `INFERRED:` from names and value distributions.**

### `WMT_Task_Point` — 166 instances in 73 missions (the dominant objective)

A capture-zone. Every instance carries all 15 parameters:

| Parameter | Observed values |
|---|---|
| `WMT_Task_Point_Marker` | a marker name (`zone_1`, `zone`, `z1`) — the zone's shape |
| `WMT_Task_Point_MarkerText` | display label (often empty; else e.g. `База`) |
| `WMT_Task_Point_Message` | capture announcement, e.g. `Орвич-Вонор захвачен` |
| `WMT_Task_Point_Owner` | `1` ×70, `0` ×65, `2` ×30, `4` ×1 — starting owner |
| `WMT_Task_Point_CaptureCount` | `4` ×128, `1` ×16, `2` ×6, `3` ×5 — attackers needed |
| `WMT_Task_Point_DefCount` | `3` ×129, `1` ×17, `2` ×8, `0` ×5 — defenders needed to hold |
| `WMT_Task_Point_AdvantagePercent` | `2` ×72, `0` ×67, `4` ×14, `3` ×13 |
| `WMT_Task_Point_Timer` | `60` ×51, `30` ×39, `120` ×27, `90` ×15 — seconds to capture |
| `WMT_Task_Point_MinHeight` | `-5` ×166 (invariant) |
| `WMT_Task_Point_MaxHeight` | `30` ×137, `15` ×16, `20` ×13 — **3D zone volume** |
| `WMT_Task_Point_EasyCapture` | `1` ×150, `0` ×16 |
| `WMT_Task_Point_Lock` | `1` ×136, `0` ×30 |
| `WMT_Task_Point_AutoLose` | `-1` ×156, `1` ×10 |
| `WMT_Task_Point_Notice` | `1` ×166 (invariant) |
| `WMT_Task_Point_Condition` | `true` ×165; one real expression: `pwz getVariable "WMT_PointOwner" == east \|\| pez getVariable "WMT_Point…` |

`INFERRED:` a zone is captured when `CaptureCount` attackers are inside the marker's
area between `MinHeight` and `MaxHeight` for `Timer` seconds while fewer than
`DefCount` defenders contest it. The `MinHeight`/`MaxHeight` pair means **zones are
volumes, not 2D circles** — `-5` to `+30` deliberately includes basements and
excludes aircraft.

### `WMT_Task_CapturePoint` — 71 instances in 70 missions

The **win condition** that aggregates `WMT_Task_Point`s:
`_Count` (`1` ×29, `2` ×18, `3` ×14, `4` ×4) — how many points must be held;
`_Winner` (`0` ×35, `1` ×35, `2` ×1) — which side wins if satisfied;
`_Message` — victory text; `_Condition` — `true` on all 71.

### Other objective modules

| Module | Instances / missions | Parameters |
|---|---|---|
| `WMT_Task_Destroy` | 14 / 13 | `_Count` (`0` ×11 — `INFERRED:` 0 = "all"), `_Winner`, `_Message`, `_Notice`, `_Condition` |
| `WMT_Task_Compose` | 7 / 7 | `_Count`, `_Winner`, `_Message`, `_Condition` — `INFERRED:` composite of sub-tasks |
| `WMT_Task_Arrive` | 2 / 2 | `_Marker`, `_Count`, `_Winner`, `_Message`, `_Notice`, `_Condition` |
| `WMT_Task_VIP` | 1 / 1 | `_Marker`, `_ReturnTime` (15), `_Count`, `_Winner`, `_Message`, `_Notice`, `_Condition` |

Every task module shares the same five-parameter spine —
`_Condition`, `_Winner`, `_Message`, `_Notice`, `_Count` — which is a clean,
learnable schema.

### `wog3_presets` — the force-package picker (readable source; near-dead in practice)

The one objective-adjacent system whose source survives. Two Eden modules
(`wog3_presets/Config.cpp:49-69, 70-223`):

- **`wog3_presets_note`** (`Choice presets` / `Выбор условий`) — one `Name` argument.
  Synchronise it to exactly one man; that player becomes the chooser
  (`fn_moduleInit.sqf:4-6` rejects 0 or >1 synced units).
- **`wog3_presets_option`** (`Option` / `Вариант`) — **20 string slots** `Name1`…`Name20`,
  each documented in the stringtable as
  `"marker", "type vehicle", {code}` (`stringtable.xml:22-23`). Each option module
  is one selectable package; sync several to the note module.

Flow: the chooser gets a diary page listing each package with **`[Show]`** and
**`[Select]`** links (`fn_presetToDiary.sqf:25-26`). `Show` previews the package as
red local map markers (`fn_show.sqf:17`). `Select` is irreversible
(`fn_select.sqf:4-8` refuses once chosen), spawns the vehicles at the markers,
runs the per-entry `{code}`, and — if the freeze is still active — attaches EHs that
kill the engine and delete any round fired (`:41-61`).

**Usage: 6 `wog3_presets_option` + 2 `wog3_presets_note` instances, in 2 of 171
missions.** Effectively dead (§15).

### `WMT_StartPosition` — 24 instances in 13 missions

Player-chosen spawn. `_Positions` (comma-separated marker names, e.g.
`spawn_1, spawn_2, spawn_3, spawn_4`), `_Owner` (an object/slot name),
`_CenterObject`, `_MarkerSide`, `_Time` (`10` ×8, `30` ×5, `3` ×5, `5` ×4),
`_Text` (e.g. `Выбирайте стартовую позицию` / `Choose a spawn location`).
`INFERRED:` the commander picks the insertion point within `_Time` minutes.

### Genre

`fn_onMissionNewEH.sqf:20` states the intended genre outright:
**attacking faction vs defending faction**, capture-and-hold, one life, ~2 hours
(`WMT_Time_MissionTime` = `120` in 61 of 77 instances).

## 8. Respawn / tickets / medical / revive

**No tickets. No revive system of WOG's own. One life.**

**Respawn.** The 3den extension sets `['Multiplayer','Respawn',1]` and
`['Multiplayer','RespawnTemplates',['Spectator']]` in all three of its mission-attribute
functions (`fn_defaultsettings.sqf:5-7`, `fn_onMissionLoadEH.sqf:19-20`,
`fn_onMissionNewEH.sqf:22-23`). Measured in the SQMs:

- **77 of 78 WOG-native missions have `respawn = 1`**; 1 has `3`.
- Of the 93 non-WOG-native: 57 have `respawn = 3`, 34 unset, 2 have `1`.
- `respawnTemplates` is **unset in all 171 SQMs** — the 3den default does not persist
  into `ScenarioData`.
- `disabledAI` is unset in all 78 WOG-native missions; set to `1` in 55 others.

`INFERRED:` `respawn = 1` is Arma's BIRD template; combined with the Spectator
template pushed by the addon at runtime, the intended experience is **die once, then
spectate**. `ace_spectator_virtual` appears 140× in 33 missions, consistent with this.

**Medical is taken away from the mission maker.** `wog3_inventory` runs
`fn_medicalEquipment.sqf` as a `serverInit`
(`wog3_inventory/config.bin → Extended_PostInit_EventHandlers >> wog3_autoMedicalEquipment`).
It iterates `playableUnits` (`:71`) and **removes 25 named medical item types then
re-adds a fixed kit** (`:8-42`):

- Every player: 4 fieldDressing, 4 elasticBandage, 3 packingBandage, 3 quikclot,
  1 splint, 1 tourniquet, 1 morphine, 1 epinephrine — into the **uniform**.
- Every group leader additionally gets `ACE_MapTools` if missing (`:45-49`).
- Every ACE medic: backpack **cleared entirely** (`:53-55`) then filled with 10/10/10/10
  bandages, 4 adenosine, 6 morphine, 10 epinephrine, 5 tourniquets, 4 saline500,
  3 blood500, 2 blood250, 1 surgicalKit, 8 splints (`:56-68`).

Opt-out is a single global: `if ( !(missionNamespace getVariable ['wmt_param_AutoMedicine', true]) ) exitWith {};` (`:5`),
surfaced to the maker as the `WMT_Main_AutoMedicine` module attribute
(`1` in 70 of 79 instances, `0` in 9).

**ACE timings are standardised mod-side** in `wog3_ace_settings/ACE_Settings.hpp`
(991 lines): wheel replace/remove 80 s, misc repair / track work 120 s, full repair
300 s (`:3-23`); all bandage types 10 s, diagnose 5 s, check pulse 5 s, blood
pressure 10 s, check response 2 s, personal aid kit 20 s (`:28-59`). A large
`ACE_Medical_Advanced` wound table (`:61-…`) is **commented out**.
`CfgWeapons.hpp:5-10` sets `ACE_bodyBag` mass to 4.

**Hard freeze.** `wog3_hardfreeze` scales the pre-start hold to the player count
(`XEH_postInit.sqf:16-19`): `>160` players → +180 s, `>130` → +90 s, `>100` → +30 s,
and forces `WMT_pub_frzTimeLeft = 1500` if a freeze is active (`:8-10`). Above 70
players every client is `enableSimulation false` (`:37`) with weapons force-safed
each frame (`:46`), and is **not released until the TeamSpeak plugin is detected**
(`waitUntil { … [] call TFAR_fnc_isTeamSpeakPluginEnabled }`, `:57-61`), then after a
random 1–16 s stagger (`:65-66`) to avoid a simultaneous unfreeze spike.

## 9. Zones / areas / triggers / play area

**Triggers are barely used: 162 across all 171 missions — a mean of under 1 per
mission.** Compare 8,628 markers (mean 50) and 2,396 layers. Zones are expressed as
**markers consumed by modules**, not as trigger areas.

- `WMT_Task_Point_Marker` names a marker; `_MinHeight`/`_MaxHeight` turn it into a
  volume (§7).
- `WOG_IslandRestriction` — **34 instances in 34 missions** — is the play-area
  boundary module: `_Marker` (`gamezone` ×14, `zone` ×2, `tbd` ×2, empty ×4),
  `_Timer` (`15` ×23, `30` ×4, `10` ×3, `25` ×2), `_Height` (`50` ×29),
  `_Otstup` (Russian *отступ*, "margin/offset" — `3` ×28), `_Area` (`0` ×30, `1` ×4).
  A `wog_islandrestriction.pbo` exists but its config is opaque (`Texheaders.bin`
  only decodes; the main config is in the obfuscated set).
  `INFERRED:` leaving the marker for `_Timer` seconds kills or returns the player.
- Briefings back this up as a stated rule: `Запрещено пересекать границы игровой зоны`
  ("crossing the game-zone boundary is forbidden") —
  `wog_160_odins_sword_13.../briefing.sqf:30`.

**Top marker types across the corpus:** `loc_Fuelstation` 1,803 · (blank) 1,649 ·
`o_inf` 1,021 · `b_inf` 818 · `mil_dot` 628 · `Empty` 321 · `n_installation` 258 ·
`b_installation` 240 · `b_unknown` 199 · `mil_flag` 109.
`INFERRED:` the `loc_Fuelstation` count is anomalous and probably a default type
left unchanged on bulk-created markers rather than a deliberate choice.

Terrain editing is heavy: **`ModuleHideTerrainObjects_F` appears 3,053 times across
121 missions**, and `ModuleEditTerrainObject_F` 204 times in 16. Makers reshape the
map extensively. `trencher_main_Module_TrenchPiece` (167 in 9 missions) and
`acex_fortify_buildLocationModule` (12 in 2) add field fortification.

## 10. Configuration surface

Exhaustive, grouped by where the maker sets it.

### 10.1 `WMT_Main` module — 16 attributes (present on all 79 instances)

| Attribute | Observed values |
|---|---|
| `WMT_Main_AI` | `0` ×79 (invariant) |
| `WMT_Main_AutoMedicine` | `1` ×70, `0` ×9 |
| `WMT_Main_DisableBreifingMarkerMove` | `0` ×79 (invariant; name misspelled upstream) |
| `WMT_Main_DisableChannels` | `0,2,4,5` ×78, `0,2,4,6` ×1 |
| `WMT_Main_DisableFuelSt` | `0` ×75, `1` ×4 |
| `WMT_Main_ExtendedBriefing` | `1` ×78, `0` ×1 |
| `WMT_Main_GenerateFrequencies` | `1` ×75, `0` ×4 |
| `WMT_Main_HeavyLossesCoeff` | `0.1` ×36, `0.05` ×31, `0.075` ×6, … |
| `WMT_Main_IndetifyTheBody` | `1` ×78, `0` ×1 (name misspelled upstream) |
| `WMT_Main_MaxViewDistance` | `2500` ×50, `3000` ×14, `2000` ×8, `3500` ×2 |
| `WMT_Main_MaxViewDistanceTerrain` | `10000` ×55, `5000` ×10, `8000` ×3, `4500` ×2 |
| `WMT_Main_NameTag` | `1` ×77, `0` ×2 |
| `WMT_Main_SideChannelByLR` | `1` ×73, `0` ×6 |
| `WMT_Main_Statistic` | `1` ×79 (invariant) |
| `WMT_Main_TI` (thermal imaging) | `0` ×51, `2` ×19, `1` ×9 |
| `WMT_Main_TotalDominationCoeff` | `-1` ×72 (78 instances carry it), `4` ×2, `3` ×2 |

### 10.2 `WMT_Time` module — 6 attributes (all 77 instances)

`_MissionTime` (`120` ×61, `90` ×7, `100` ×5, `60` ×2) ·
`_PrepareTime` (`3` ×69, `5` ×6, `1` ×2) ·
`_StartZone` (`100` ×69, `50` ×3, `20` ×2, `200` ×2) ·
`_RemoveBots` (`10` ×69, `3` ×7, `5` ×1) ·
`_WinnerByTime` (`0` ×35, `1` ×32, `2` ×10) ·
`_WinnerByTimeText` (free text, e.g. `Атакующие силы отступают`).

### 10.3 Objective modules

Enumerated exhaustively in §7: `WMT_Task_Point` (15 attrs), `WMT_Task_CapturePoint`
(4), `WMT_Task_Destroy` (5), `WMT_Task_Compose` (4), `WMT_Task_Arrive` (6),
`WMT_Task_VIP` (7), `WMT_StartPosition` (6), `WOG_IslandRestriction` (5),
`wog3_presets_note` (1), `wog3_presets_option` (20).

### 10.4 Per-entity Eden attribute added by WOG — exactly one

`wog_editorLoadedJerryCans` (`wog3_3den/config.bin → Cfg3DEN >> Object >>
AttributeCategories >> ace_attributes >> Attributes`): displayName `Jerrycans`,
control `Edit`, `typeName = "NUMBER"`, `validate = "number"`, `defaultValue = "1"`,
`condition = "objectHasInventoryCargo"`, expression
`_this setVariable ['%s',_value];`. Consumed at runtime by
`wog3_3den/XEH_preInit.sqf:5-11`, which calls `ace_repair_fnc_addSpareParts` with
`Land_CanisterFuel_F`, defaulting to 1 with an explicit comment
`// must match eden attribute default` (`:7`). Registered on `Tank` and `Car` class
init (`:14-15`). **Used 373 times in the corpus** (`0` ×143, `2` ×87, `4` ×64).

### 10.5 Global script variables a maker sets in `init.sqf`

- `wog3_no_auto_long_range_radio = true;` — **74 of 171 missions**. Suppresses the
  automatic LR radio backpack (`wog3_tfar/functions/XEH_postInitClient.sqf:12`).
  Also readable per-unit via `setVariable` (`:14`).
- `wmt_param_AutoMedicine` — mission-namespace opt-out for forced medical
  (`wog3_inventory/functions/fn_medicalEquipment.sqf:5`).
- `WMT_global_EnableConsole` — array of Steam UIDs granted admin
  (`wog3_admin/functions/fn_admin_command.sqf:20`; commented example in
  `wog3_mod/XEH_postInit.sqf:2`).
- `WOG3_AllowChangeGroup` — per-unit, 20-second consent window for group joining
  (`wog3_inventory/config.bin → CAManBase >> ACE_SelfActions >> ACE_Equipment >> WOG3_AllowChangeGroup`).

### 10.6 Mission attributes force-set by the 3den extension

`fn_onMissionLoadEH.sqf` rewrites **21 attributes on every mission load** (:1-21):
Scenario `Briefing`, `Debriefing`, `Saving=false`, `ShowMap`, `ShowCompass`,
`ShowWatch`, `ShowGPS`, `ShowHUD`, `ShowUAVFeed`, `ForceRotorLibSimulation=false`,
`EnableDebugConsole=1`, `EnableTargetDebug=0`, `SaveBinarized=true`; Multiplayer
`GameType='Unknown'`, `MinPlayers=0`, `MaxPlayers=0`, `DisabledAI=false`,
`Respawn=1`, `RespawnTemplates=['Spectator']`, `SharedObjectives=0`.
`fn_defaultsettings.sqf` (manual, from the menu) sets a 7-attribute subset including
`IntelIndepAllegiance=[0,0]` under both `Multiplayer` and `Scenario`.

### 10.7 Mod-side behaviour with no mission-level switch

- `wog3_mod/XEH_postInit.sqf`: `disableRemoteSensors true` (`:5`),
  `enableSentences false` (`:6`), `enableSaving [false,false]` (`:9`),
  `onMapSingleClick` neutered (`:12`), AI conversations off (`:15-18`),
  `enableRadio false` (`:21`).
- `wog3_tfar/XEH_postInitClient.sqf`: `tf_no_auto_long_range_radio = true` (`:1`),
  `TF_give_personal_radio_to_regular_soldier = false` (`:2`), and after 5 s
  `TF_speak_volume_level = "whispering"`, `TF_speak_volume_meters = 5` (`:22-23`).
- `wog3_rearm/Config.cpp`: `transportAmmo = 0` on GAZ-66 ammo variants, M113 supply,
  LAV25, and NATO/East/Indep ammo boxes (`:20-51`) — **disables vanilla auto-rearm**,
  forcing ACE cargo logistics.
- `wog3_artui/config.bin`: mortars get `artilleryScanner = 0`,
  `transportMaxItems = 5`, and 2× range table + 2× maptools pre-loaded in cargo.
- `wog3_disable_vehicle_lock/config.cpp`: `allowTabLock = 0`, `canUseScanners = 0`,
  `irScanGround = -1` on `Tank_F` / `Wheeled_APC_F` / `RHS_M2A2_Base`, re-enabled
  only on AA variants (`:29-74`); Stinger reload rebalanced (`:83-86`).
- `wog3_inventory` ACE actions: `WOG3_Invetory` (sic) — open another player's gear at
  4 m if they are dead, unconscious, or you are their group leader; `WOG3_JoinInGroup`
  — join a consenting player's group, capped at 12 (`condition`/`statement` strings
  in `wog3_inventory/config.bin`).

### 10.8 Admin runtime commands

`wog3_admin/functions/fn_help.sqf:3-9` lists the chat commands:
`#lr` (give long-range radio), `#sw` (give short-wave radio), `#map` (give maps),
`#fe` (freeze-time enable), `#fd` (freeze-time disable).
Backed by `fn_give_radio_lr`, `fn_give_radio_sw`, `fn_fix_radio_sw`, `fn_give_map`,
`fn_hardfreez_enable`, `fn_hardfreez_disable`. Authorisation is
`getPlayerUID player in WMT_global_EnableConsole` **or** `serverCommandAvailable('#kick')`
(`fn_admin_command.sqf:20-21`); dispatch is `remoteExec` to `all` / `server` / `local`
(`:26-31`). Note the comment at `:23` — *"Compile code to avoide white list for
remoteExec and run localy"* — an explicit remoteExec-whitelist bypass.

## 11. Tooling

### 11.1 `wog3_3den` — the Eden editor extension (complete inventory)

This is WOG's answer to "what should the mission editor give the maker". Everything
it adds, and exactly where it appears.

**A. Menu strip — `Tools ▸ WOG 3den Tools`.**
Injected via `items[] += {"WOG3_ToolFolder"}` into the stock `Tools` menu
(`config.bin → Display3DEN >> Controls >> MenuStrip >> Items >> Tools`). The folder
is labelled **"WOG 3den Tools"** with the WOG logo (`\wog3_3den\Data\woglogo.paa`).
Six entries, in order:

| # | Label (RU) | English | Function | Behaviour |
|---|---|---|---|---|
| 1 | `Установить настройки по умолчанию` | Apply default settings | `fn_defaultsettings.sqf` | Sets 7 mission attributes: `IntelIndepAllegiance [0,0]` (×2), `RespawnTemplates ['Spectator']`, `saving false`, `SaveBinarized true`, `Respawn 1`, `EnableDebugConsole 1` |
| 2 | `Сделать все слоты игровыми` | Make all slots playable | `fn_setallplayable.sqf` | `set3DENAttributes [[all3DENEntities select 0,"ControlMP",true]]` — one call, every entity |
| 3 | `Подсчитать количество игровых слотов` | Count playable slots | `fn_countplayableslots.sqf` | `titleText` with per-side counts + `Всего слотов` total |
| 4 | `Очистить хранилище всей техники` | Clear all vehicle storage | `fn_clearvehinv.sqf` | Sets `ammoBox` to the empty literal on **every** entity mission-wide |
| 5 | `Настройки ACE Медик & Инженер по умолчанию` | Default ACE medic & engineer settings | `fn_acesetdefaultmedeng.sqf` | `ace_isMedic = -1`, `ace_isEngineer = -1` on all entities |
| 6 | `Показать на карте Object ID` | Show Object IDs on map | `fn_mapobjectid.sqf` | `do3DENAction "ToggleMapIDs"` |

**B. Right-click context menu.** WOG rewrites the whole `ContextMenu` `items[]` order
and inserts two submenus after `PlayAsEntity`
(`config.bin → Display3DEN >> ContextMenu >> Items`):

| Submenu | Entry | Icon / condition | Function |
|---|---|---|---|
| `Формации` (Formations) | `Колонна` (Column) | stock column icon, `conditionShow = "selected"` | `fn_column.sqf` — aligns selection nose-to-tail along Y using `boundingBoxReal`, copying rotation from the first object |
| | `Шеренга` (Line) | stock line icon, `"selected"` | `fn_line.sqf` — same along X |
| `Функции` (Functions) | `Очистить Init объектов` (Clear objects' init) | delete icon, `"selected"` | `fn_clearInit.sqf` — confirm dialog, then blanks `Init`, wrapped in `collect3DENHistory` (undoable) |
| | `Установить здоровье на макс.` (Set health to max) | support icon, `"selected"` | `fn_setFullHealth.sqf` — confirm dialog, `Health = 1`, undoable |
| | `Очистить снаряжение` (Clear equipment) | `"selected"` | `fn_clearUnitLoadout.sqf` — `setUnitLoadout (configFile >> "EmptyLoadout")` on `CAManBase`, then `save3DENInventory` |

Plus two entries appended to the stock `Log` submenu:

| Entry | `conditionShow` | Function |
|---|---|---|
| `Занести содержимое Ammobox в буфер обмена` | `hoverObject * (1 - hoverObjectBrain)` — a vehicle/crate, not a person | `fn_copyammotosqf.sqf` |
| `Занести содержимое Arsenal в буфер обмена` | `hoverObjectBrain * (1 - hoverObjectVehicle)` — a person on foot | `fn_copyarsenaltosqf.sqf` |

**C. Mission event handlers** (`config.bin → Cfg3DEN >> EventHandlers >> 3denProperAttributes`):
`OnMissionLoad` → `fn_onMissionLoadEH` (21 attributes, §10.6);
`OnMissionNew` → `fn_onMissionNewEH` (same set + house name/overview, guarded so it
only fires when both name and overview are empty, `:1`);
`OnMissionSave` → `fn_onMissionSaveEH` (Med/Eng slot tagging, §4/§14).

**D. Entity attribute:** `Jerrycans`, injected into the existing `ace_attributes`
category on any object with inventory cargo (§10.4).

**E. Two validators with no UI entry point.** Registered under `CfgFunctions >>
wog3_3den >> ForChecking` but absent from both menus — they must be invoked from the
debug console, and their own headers say so:

- `fn_check_lr.sqf:1` — `//call wog3_3den_fnc_check_lr;`
  Walks `allGroups`, temporarily removes and re-adds the leader's backpack to
  determine whether a valid TFAR long-range radio is present, and emits per-group
  flags to the clipboard: `WRONGLR` (radio encrypted for another side, via
  `tf_encryptionCode`, `:33-37`), `NOTLR`, `EXTRABP(...)`, `EXTRALR(...)`, `BP(...)`.
- `fn_check_weapon.sqf:1` — `//[4,4,2,2] call wog3_3den_fnc_check_weapon;`
  Takes minimum magazine counts `[primary, MG, secondary, handgun]` and reports every
  playable unit failing them, plus `NoPrimaryWeapon`, `NoVest`, `NoUniform`,
  `WrongItems` (missing map/compass/radio, or a radio whose side does not match the
  unit's — a hardcoded 6-entry TFAR radio→side table at `:47`). Disposable launchers
  are exempted (`:105`). Prints `All good!` when clean (`:138`).

**F. `fn_dumpconfig.sqf`** — a vendored copy of Denis Usenko's MIT `dumpConfig.sqf`
(header `:1-6`, "Modified by Kato for WOG3"), dumping any config branch to the
clipboard. Registered under `OthersFn`; no menu entry.

**G. `config_lop_fix/config.bin`** — renames 14 LOP factions to a consistent
`[LOP] …` scheme in the Eden group palette.

**Design summary:** every WOG 3den tool is either (i) a **mission-wide bulk
operation**, (ii) a **counter/validator whose output goes to the clipboard**, or
(iii) an **exporter to the clipboard**. There is not one tool that creates content.

### 11.2 Companion tooling the community relies on

- **3DEN Enhanced** — declared in `EditorData >> mods[]` of **58 of 171 missions**.
  Its attributes are all over the corpus: `ENH_disableAI_path` 9,859 uses,
  `ENH_aimingAccuracy` / `ENH_spotDistance` / `ENH_courage` / `ENH_allowFleeing`
  etc. 400 each, `ENH_markerShape` 486. WOG's own extension does not duplicate it.
- **`GF_deformString`** — `wog3_3den/XEH_preInit.sqf:18-19` shows a red
  **"DEFORMER NEEDED!"** title in Eden when a mission uses the GF Terrain Deformer
  but the addon is missing. A dependency-check nicety.
- **ACE Arsenal / vanilla Arsenal** — the loadout editor (§5).
- **`zeusTracer.sqf`** — byte-identical in all 47 missions carrying it
  (single MD5 `cf725dbd…`), authored by "417 (Coder) and Adrihado".

### 11.3 Not found

No mission validator CLI, no build/packaging script, no linter, no test harness, no
mission template generator, no web tooling, no schema files. Nothing outside Eden and
the in-game clipboard.

## 12. Conventions and house rules encoded in the framework

Ranked by how strongly the artefacts enforce them.

1. **Mission filename declares the slot count.**
   `wog_<slots>_<name>_<version>.<terrain>`, e.g. `wog_160_odins_sword_13.cup_chernarus_A3`.
   **73 of 78 (94%) match exactly; 76 of 78 (97%) within ±5.** The only outliers are
   `wog_147_papers_please_10/11.WL_Rosche` (declared 147, actual 153). The 3den
   extension pre-seeds this format as the mission name
   (`fn_onMissionNewEH.sqf:3`, `'WOG 160 Best Mission 1.0'`) and supplies the
   counter that checks it (`fn_countplayableslots.sqf`).
2. **Attack faction vs defence faction.** Written into the default overview text a
   maker sees on every new mission (`fn_onMissionNewEH.sqf:20`) and borne out by the
   52/42 West/East slot split.
3. **One life, then spectate** (§8). 77 of 78 WOG-native missions.
4. **Slot labels are `Role@Group`, and medics/engineers are advertised in the label**
   (§4).
5. **Medical loadout is not the maker's business** — forcibly standardised for all
   players (§8), opt-out via one module checkbox.
6. **Group leaders get a long-range radio automatically** — but **74 of 171 missions
   set `wog3_no_auto_long_range_radio = true`**, i.e. the single most-used WOG API in
   the entire corpus is the one that turns a WOG feature off (§14).
7. **Radios must match the wearer's side.** Both validators check
   `tf_encryptionCode` / a radio→side table (`fn_check_lr.sqf:33-37`,
   `fn_check_weapon.sqf:47`).
8. **Whisper by default.** Direct-speech volume forced to `"whispering"` / 5 m for
   every client (`wog3_tfar/XEH_postInitClient.sqf:22-23`).
9. **Channels are restricted.** `WMT_Main_DisableChannels = "0,2,4,5"` in 78 of 79
   instances; side channel gated on holding a long-range radio
   (`WMT_Main_SideChannelByLR = 1` in 73 of 79).
10. **No auto-rearm; ACE cargo logistics instead** (`wog3_rearm`, §10.7). Missions
    hand-load ammo in `initServer.sqf` —
    `wog_160_odins_sword_13.../initServer.sqf:1-3` loads 96 mortar boxes in a loop.
11. **No Tab-lock except for AA** (`wog3_disable_vehicle_lock`).
12. **Saving is disabled** three separate ways (3den attribute, `wog3_mod` runtime,
    and again in `fn_onMissionLoadEH`).
13. **Debug console left enabled** (`EnableDebugConsole = 1`) in the shipped defaults.
14. **Briefings name their own house rules** in a `Условности` ("conventions")
    section — 42 missions — and identify uniforms in `Формы сторон` — 44 records.
15. **Scenery is made invulnerable at scale**: `allowDamage = 0` appears **66,127
    times** across the corpus (vs `1` 2,909 times).

## 13. What this framework does better than anyone else

1. **Objectives are configured Eden modules with a uniform schema, not scripts.**
   Every `WMT_Task_*` shares `_Condition`, `_Winner`, `_Message`, `_Notice`, `_Count`,
   and `WMT_Task_Point` adds a rich, genuinely well-chosen capture model:
   attacker count, defender count, timer, advantage percentage, lock, auto-lose, and
   a **3D volume** via `MinHeight`/`MaxHeight`. 166 instances across 73 missions is
   real adoption, and it means a capture-and-hold mission needs **zero lines of
   script**. That is why the median WOG `description.ext` is empty.
2. **The editor counts your slots and puts the number in the filename.** A trivial
   tool (`fn_countplayableslots.sqf` is 10 lines) that produces a 94%-consistent
   community-wide naming convention, so a server admin reading a directory listing
   knows instantly whether a mission fits tonight's turnout. Tooling and convention
   co-designed.
3. **Loadout authoring is a real round-trip through a GUI.** `WOG: EXPORT` →
   clipboard → `tpl/<faction>/<role>.sqf`; `WOG: IMPORT` → clipboard → back into the
   Arsenal for editing. Three export dialects for three destinations (mission script,
   `setUnitLoadout` array, addon config class). Most frameworks give you export only.
4. **Validators that answer "is this mission shippable?"** `fn_check_weapon.sqf`
   and `fn_check_lr.sqf` check the things that actually ruin a 150-player game:
   a squad leader with no radio, a radio encrypted for the wrong side, a rifleman
   with no vest, four magazines when the standard is eight. Output goes to the
   clipboard so it can be pasted into a review thread.
5. **The framework designs for 150+ players explicitly.** Freeze time scales with
   headcount, simulation is disabled above 70 players, release is randomly staggered
   to avoid a thundering herd, and players are held until their TeamSpeak plugin is
   detected (`wog3_hardfreeze`). No sibling framework in this comparison has
   player-count-conditional startup logic.
6. **Standardising medical mod-side removes an entire class of maker error.** No
   mission can ship with a medic carrying two bandages.
7. **Briefings hyperlink to map markers** (`<marker name='…'>`), and there is a
   conventional briefing section for **recognising friendly uniforms** — a real
   milsim problem at scale, given a first-class slot in the house template.

## 14. Friction and known complaints

No issue tracker or forum was available, so these are defects and tensions **visible
in the artefacts**, not reported complaints. Each is evidenced.

1. **The Med/Eng slot auto-tagger is broken and will corrupt descriptions.**
   `fn_onMissionSaveEH.sqf:5`:
   ```sqf
   if !((_desc select 0) regexMatch ".*\| (Med|Eng).*/g") then {
   ```
   The trailing `/g` is a JavaScript regex-flag idiom pasted into an SQF pattern
   string, where it is **literal**. The pattern therefore requires the description to
   *end with the characters `/g`*, so it never matches. Consequences:
   (a) the `else` branch (`:14-23`), which strips ` | Med` / ` | Eng` when a unit
   stops being a medic, is **dead code that can never run**;
   (b) the `then` branch appends ` | Med` with **no idempotence guard** (`:7`), so
   every save appends another copy.
   `INFERRED:` this is consistent with the corpus containing **zero** occurrences of
   the generated ` | Med` / ` | Eng` form across 171 missions — makers either predate
   the feature or do not save with the addon loaded. The one mission that tags roles
   does it by hand in uppercase (`| MED`, `| ENG`).
2. **The most-used WOG API is the one that disables a WOG feature.**
   `wog3_no_auto_long_range_radio = true` appears in **74 of 171 missions** — more
   than any other `wog3_*` symbol (next is `wog3_mod` at 2). Setting it as a *global*
   disables the automatic leader radio for the entire mission
   (`wog3_tfar/XEH_postInitClient.sqf:12` tests `isNil`), so 43% of missions opt out
   wholesale. A feature that nearly half the corpus turns off is mis-defaulted.
3. **The core module set is unshippable/unauditable.** `wmt_main` — which owns every
   `WMT_*` module, the briefing map, task notifications and freeze state — is not
   distributed with the addon set examined here, and `wog_main` is **deliberately
   obfuscated** (scrambled filenames in the PBO header, byte-scrambled includes).
   A mission maker cannot read the definition of the modules they are configuring;
   neither can a reviewer.
4. **Two shipped attribute names are misspelled and permanent.**
   `WMT_Main_DisableBreifingMarkerMove` ("Breifing") and `WMT_Main_IndetifyTheBody`
   ("Indetify"), plus the ACE action class `WOG3_Invetory` ("Invetory"). These are
   serialized into all 79 module instances and every mission file, so they cannot be
   fixed without migrating the corpus.
5. **The two best tools are unreachable from the UI.** `fn_check_weapon` and
   `fn_check_lr` — the mission validators — have no menu entry; their usage is a
   commented first line telling you to type a call into the debug console, and
   `fn_check_weapon` requires four positional magazine-count arguments with no
   defaults. `fn_dumpconfig` likewise.
6. **All tool output goes to the clipboard or a `titleText`.** No report pane, no
   persistent log, no way to diff two runs. `fn_check_weapon.sqf:139` ends
   `copyToClipboard _txt;` — a validator whose findings vanish if you copy anything
   else.
7. **Destructive bulk operations act on the entire mission, not the selection.**
   `fn_clearvehinv.sqf:1` and `fn_acesetdefaultmedeng.sqf:2` both operate on
   `all3DENEntities select 0`. Neither shows a confirmation dialog — while
   `fn_clearInit` and `fn_setFullHealth`, which act only on the *selection*, both do
   (`fn_clearInit.sqf:11`, `fn_setFullHealth.sqf:10`). The confirmations are on the
   safe operations and absent from the dangerous ones. Neither bulk op is wrapped in
   `collect3DENHistory`, so **neither is undoable**.
8. **The framework fights the maker over mission attributes.** `OnMissionLoad`
   silently rewrites 21 mission attributes **every time the mission is opened**
   (`fn_onMissionLoadEH.sqf`), including `Respawn`, `SaveBinarized` and
   `EnableDebugConsole`. A maker who deliberately changes any of them loses the change
   on next open, with no notification.
9. **`RespawnTemplates` never persists.** The extension sets
   `['Multiplayer','RespawnTemplates',['Spectator']]` in three places, yet the
   attribute is **unset in all 171 shipped SQMs** — so the spectator-on-death
   behaviour depends on the addon being loaded at runtime rather than on the mission
   file being correct.
10. **Documentation is entirely absent and the UI is Russian-only.** Every menu label
    is a hardcoded Russian string literal (`stringtable.bin` is opaque and unused for
    the menus), so the toolset is inaccessible to non-Russian-speaking makers — while
    11 missions ship bilingual briefing sections (`Задача | Task`), showing the
    community does have non-Russian players.
11. **An explicit remoteExec-whitelist bypass ships in the admin tool**
    (`fn_admin_command.sqf:23-24`, comment and all), gated only by a UID array or
    `#kick` availability.

## 15. Evidence from the mission corpus

**Scope: all 171 missions extracted; all 171 `mission.sqm` decoded and parsed;
13 read file-by-file** (listed in the Source inventory).

### 15.1 The corpus is not one community's work

| Lineage | Missions | Signature |
|---|---|---|
| **WOG-native** | **78** | contains a `WMT_Main` module |
| **OFCRA / `omtk`** | **33** | ships an `omtk/` directory; `description.ext` header `onLoadMission = "www.ofcrav2.org"` |
| Neither | 60 | third-party / imported |

Other reference counts (missions containing the string, all files):
`ace_` 167 · `BIS_fnc_` 158 · `tf_` 96 · `TFAR` 89 · `WOG` 78 · `wog3_` 74 · `WMT_` 64.

**The OFCRA missions are a wholesale import of a rival framework.** `omtk` is a
mission-embedded toolkit of **16 module directories, 12 of which carry their own
`README.md`**: documented — `warm_up`, `score_board`, `dynamic_startup`,
`zeus_admins`, `kill_logger`, `radio_lock`, `rambo_warn`, `uniform_lock`,
`ia_manager`, `difficulty_check`, `tactical_paradrop`, `vehicles_thermalimaging`;
undocumented — `map_exploration`, `respawn_mode`, `view_distance`, `ui`
(plus a `3rd-parties/README.md` attribution notice). It configures itself
through a ~100-line `class Params` block of `OMTK_MODULE_*` entries
(`BattleofDinant1914v4FINAL.SWU_Ardennes_1940/description.ext:37-80`) —
the exact opposite of WOG's module-based approach. **This is the only documentation
in the entire corpus: 437 `readme.md` files, all of them OMTK's, none of them WOG's.**

### 15.2 Copy-paste lineage is measurable

- The OFCRA `description.ext` header block — down to the trailing comments
  `// SHOWN AT THE VERY TOP`, `// SHOWN JUST BELOW LOADNAME` — is byte-similar across
  `BattleofDinant1914v4FINAL`, `DienBienPhu`, and `TowerDefense`, changing only
  `onLoadName`, `author` and `briefingName`.
- `zeusTracer.sqf` is **byte-identical (one MD5) in all 47 missions** that ship it.
- The 33 `omtk` trees share identical per-module `README.md` files (33 identical
  copies each).
- Mission series are versioned by copying: `Congo_Romanian_Mercs_*` has 20 entries
  (4, 6, 9–21, 23, 25–28); `CoalitionOfTheWillingM01..M04` are 4 missions with
  **324 slots and 66 groups each** — the same ORBAT re-dressed four times.
- `wog_137_budweiser_10` and `_11` are the same mission (137 slots, 26 groups,
  146 markers) at two versions; likewise `wog_146_stilet_chast2_10`/`_11`,
  `wog_142_shooting_cans_10`/`_11`, `wog_147_papers_please_10`/`_11`.

### 15.3 How `description.ext` is really written

**Mostly, it isn't.** 55 of 171 missions have none at all; of the 78 WOG-native, only
25 have one and 8 of those are empty. Median length: **0 lines for WOG-native,
135 lines for everything else.**

When a WOG-native mission does use it, it is for the three things modules cannot do:
- `class Params` for mission-selectable options —
  `wog_78_krater_17.ProvingGrounds_PMC/description.ext:1-11` (a `timeOfDay` selector
  with a stringtable-localised title).
- `class CfgUnitInsignia` for unit patches —
  `wog_147_papers_please_11.WL_Rosche/description.ext:1-19`.
- One-line engine tweaks — `overrideHazeQuality = 2;`, `respawnOnStart = 1;`.

Non-WOG missions occasionally inline an entire `class CfgPatches` + `CfgWeapons`
override into `description.ext` (`Nam_Five_Off_SOG.Cam_Lao_Nam/description.ext:3-22`).

### 15.4 Typical slot counts and side balance

- **19,627 playable slots** across 171 missions. min 9 · p25 37 · **median 137** ·
  mean 115 · p75 157 · max 324.
- Largest: `CoalitionOfTheWillingM01..M04` at 324 each (OFCRA lineage);
  largest WOG-native: `wog_185_operation_magnolia_12.sara_dbe1` at 185.
- Side balance: **West 52% / East 42% / Independent 6% / Civilian 0.07%**.
- Groups: 5,510 total, mean 32 per mission; WOG-native missions cluster at 20–31
  groups for 120–185 slots, i.e. **~6 players per group**.
- Structural density per mission: 50 markers, 14 layers, **fewer than 1 trigger**,
  11 waypoints.

### 15.5 Which framework features are actually used, and which are dead

**Heavily used:**

| Feature | Adoption |
|---|---|
| `WMT_Main` | 78 of 78 WOG-native missions |
| `WMT_Time` | 77 |
| `WMT_Task_Point` | 166 instances / 73 missions |
| `WMT_Task_CapturePoint` | 71 / 70 |
| `wog3_no_auto_long_range_radio` | 74 of 171 missions |
| `WOG_IslandRestriction` | 34 / 34 |
| `wog_editorLoadedJerryCans` (WOG's one Eden attribute) | **373 uses** |
| `tpl/` or `Equipment/` loadout scripts | ~all WOG-native |
| Briefing `<marker name='…'>` links | pervasive |

**Marginal:** `WMT_StartPosition` 24 / 13 · `WMT_Task_Destroy` 14 / 13 ·
`WMT_Task_Compose` 7 / 7 · `WMT_Task_Arrive` 2 / 2 · `WMT_Task_VIP` 1 / 1.

**Effectively dead:**

- **`wog3_presets`** — 6 `wog3_presets_option` + 2 `wog3_presets_note` instances in
  **2 of 171 missions**, despite being one of only two WOG systems shipped with full
  readable source. (Where used, it is used properly: 12–18 of the 20 `NameN` slots
  filled with `"marker","class",{code}` triples.)
- **The Med/Eng slot auto-tagger** — 0 occurrences of its output corpus-wide (§14.1).
- **`WMT_Task_Point_Condition`** — `true` in 165 of 166 instances; the scripting
  escape hatch is essentially never used. Same for `WMT_Task_CapturePoint_Condition`
  (71/71 `true`), `_Destroy` (14/14), `_Compose` (7/7).
- **Triggers** — 162 across 171 missions.
- Several `WMT_Main` attributes are invariant across all 79 instances and are
  therefore settings nobody changes: `_AI` (0), `_Statistic` (1),
  `_DisableBreifingMarkerMove` (0); `WMT_Task_Point_MinHeight` (-5) and `_Notice` (1)
  likewise.

### 15.6 House patterns the mod source does not reveal

1. **The filename↔slot-count contract** (§12.1) — 94% exact. Invisible in the addon;
   only the 3den default mission name hints at it.
2. **`init.sqf` is a fixed 3-line preamble.** 54 of 78 WOG-native missions open with
   exactly `[] execVM "briefing.sqf";`; `[] call WMT_fnc_BriefingMap;` and
   `wog3_no_auto_long_range_radio = true;` follow. Median WOG-native `init.sqf` is
   **19 lines**.
3. **`initServer.sqf` exists almost solely to preload ACE cargo.**
   `wog_160_odins_sword_13.../initServer.sqf` is 11 lines, all
   `ace_cargo_fnc_loadItem` loops.
4. **Loadouts randomise cosmetics deliberately** so a 12-man squad is not visually
   cloned (`tpl/cdfass/sl.sqf:12,17,20,32-43`).
5. **Scenery invulnerability at scale** — `allowDamage = 0` **66,127 times**.
6. **Terrain is heavily rewritten** — `ModuleHideTerrainObjects_F` 3,053 times in 121
   missions.
7. **3DEN Enhanced is a de-facto dependency** — declared by 58 missions; its AI-skill
   attributes are set 400× each with two clear house presets (`0.3` aim / `0.9`
   awareness, or all `1`).
8. **A "Zeus" slot is a first-class ORBAT entry** — `description="1: Zeus@Azus"`
   appears in 33 missions; `ModuleCurator_F` in 89 of 171.
9. **Missions are shipped both binarized and not** — 86 vs 85 — despite
   `SaveBinarized = true` being force-set on every mission load, another sign the
   3den defaults are not actually reaching final saves (§14.9).

---

## What a web-based 2D mission editor should steal

Ordered by value to TBD-Reforger's Mission Creator.

1. **A live slot counter, per side, always visible** — and make the mission's
   declared player count a *field* that the editor validates against the real slot
   count. WOG got a 94%-consistent convention out of a 10-line function. In a web
   editor this is a header badge: `WEST 78 · EAST 74 · IND 8 · TOTAL 160 ✓ matches`.
2. **A structured objective schema instead of scripts.** Adopt the
   `_Condition / _Winner / _Message / _Notice / _Count` spine, and copy
   `WMT_Task_Point` wholesale: attacker count, defender count, capture timer,
   advantage percentage, lock, auto-lose, **and min/max height so a zone is a volume**.
   166 real uses prove it covers the genre. Make `Condition` an *optional advanced*
   field — 165 of 166 real uses left it at `true`.
3. **Role labels that carry structured qualifiers.** `Role@Group` plus automatic
   ` | Med` / ` | Eng` badges is the right idea, executed badly. In a web editor,
   derive the badges from the slot's actual traits at render time — never mutate the
   stored string. This removes WOG's §14.1 bug class entirely.
4. **A mission validator with a real results panel.** Port `fn_check_weapon` and
   `fn_check_lr`: slots with no vest/uniform, below-standard magazine counts, missing
   map/compass/radio, radios encrypted for the wrong side, group leaders without a
   long-range radio, duplicate radios. Show it as a click-to-select findings list, not
   a clipboard dump — that alone beats WOG's best tool.
5. **Round-trippable loadout export/import through the arsenal.** The web arsenal
   should emit and *re-ingest* the same format, so a loadout can be edited after it
   has been saved to a role. Offer the three destinations WOG offers: mission data,
   compact array, and reusable named template.
6. **A briefing editor with first-class map-marker links** (`<marker name='x'>`
   equivalent) and a **prescribed section template** — Situation, Tasks, Conventions,
   **Uniform recognition**, Additional info — with per-side variants. WOG's makers
   converged on exactly this by hand 80+ times; ship it as the default document.
7. **Bulk operations on selection, with undo and confirmation.** WOG's own tools show
   the failure mode: put confirmations and undo on the *destructive mission-wide* ops,
   not the safe selection-scoped ones.
8. **Formation helpers (column / line) on the right-click menu** — align selected
   entities nose-to-tail using bounding boxes. Cheap, and clearly used enough to earn
   a top-level context submenu.
9. **Standardise medical/comms kit at the platform layer, with a per-mission
   override toggle.** WOG's forced medical kit eliminates a whole class of error.
   But learn from `wog3_no_auto_long_range_radio`: if 43% of missions disable your
   default, the default is wrong — so make such toggles visible mission settings, not
   magic globals.
10. **Do not put mission configuration in a text file.** WOG's median native
    `description.ext` is empty because the settings moved into structured, inspectable
    module properties. A web editor should have no equivalent of `description.ext` at
    all — every setting a typed field in the mission document.
