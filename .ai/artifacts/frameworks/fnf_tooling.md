# FNF side repos — MissionAnalyzer (validator) + DTAS-Altis (shipped mission)

**Agent scope:** the two FNF repos that are *not* the framework. Sibling agents own
[`fnf_v3.md`](fnf_v3.md) and [`fnf_v4.md`](fnf_v4.md); this file cites `FNF-v3.6.9/` and
`FNF-v4.7.0/` **only** to resolve references (identifier renames), never to document them.

**Why this file matters:** the Analyzer is the only artefact in the whole FNF corpus that states,
in executable form, *what a wrong mission looks like*. Its rule set is the closest thing FNF has to
a spec for mission validity — and therefore a candidate feature list for **live validation in
TBD's Mission Creator**. Part 2 (DTAS) is the ground truth for what the framework actually
produces once a human has authored against it.

---

## Source inventory

Everything below was read or queried directly. **Disk_2 was treated as read-only; nothing on it was
modified** — all git commands were read-only (`log`, `show`, `ls-tree`, `ls-files`, `rev-list`).

### FNF-MissionAnalyzer

| File | Size / lines | How read | Notes |
|---|---|---|---|
| `.../fnf/FNF-MissionAnalyzer/AnalyzeSQM.ps1` | 2381 lines / 72,589 B | Full read, lines 1–2283 | Lines 2284–2381 are an Authenticode `# SIG #` block — base64, not code, not analysed |
| `.../FNF-MissionAnalyzer/README.md` | 35 lines | Full read | Usage + prerequisites |
| `.../FNF-MissionAnalyzer/parseSqm.py` | 19 lines | Full read | The entire Python side of the tool |
| `.../FNF-MissionAnalyzer/fnfCfgExportDB.db` | 6,692,864 B | **Binary SQLite** — schema + row counts queried read-only (`mode=ro`) | Not readable as text; summarised from `sqlite_master` + `count(*)`, not guessed |
| `.../FNF-MissionAnalyzer/v1.4.1` | 0 B | `ls` | Zero-byte version marker (`AnalyzeSQM.ps1:2215`) |
| `.../FNF-MissionAnalyzer/.git` | — | `git log`, `git ls-files`, `git rev-list`, `git show -s` | **Shallow clone** (`.git/shallow` present, `rev-list --count HEAD` = 1) |

### FNF-DTAS-Altis

| File | Size / lines | How read | Notes |
|---|---|---|---|
| `.../fnf/FNF-DTAS-Altis/README.md` | 76 lines | Full read | Provenance, mods, gameplay flow |
| `.../FNF-DTAS-Altis/changelog.md` | 55 lines | Full read | v5.1 + v5.0 only |
| `.../FNF-DTAS-Altis/.gitignore` | 2 patterns | Full read | `*.bak`, `*.pbo` |
| `.../FNF_DTAS_Altis.Altis/mission.sqm` | 155,704 B | **Unbinarized** (`head -c 8` → `version=`); censused by `grep -c` / `grep -oE` + sampled | Entity counts, marker list, one group block read verbatim; not read end-to-end |
| `.../FNF_DTAS_Altis.Altis/description.ext` | 285 lines | Structural outline + `class params` block read verbatim (`:116–273`) | |
| `.../FNF_DTAS_Altis.Altis/init.sqf` | 95 lines | Full read | |
| `.../FNF_DTAS_Altis.Altis/roundserver.sqf` | 671 lines | Targeted reads: `:49–50`, `:99–112`, `:157–167` | Objective generator + marker discovery verified verbatim |
| `.../FNF_DTAS_Altis.Altis/{preinit,capture,timerupdateclient,endhandler}.sqf` | — | Targeted reads of the cited line ranges | All four citations verified verbatim |
| `.../FNF_DTAS_Altis.Altis/islandspecific.{hpp,sqf}` | 2 lines each | Full read | The entire per-terrain surface |
| `.../FNF_DTAS_Altis.Altis/cba_settings.sqf` | 431 lines | `grep -c` counts + first 25 settings | 353 assignments / 271 forced |
| `.../FNF_DTAS_Altis.Altis/f/loadout/cfgLoadouts.hpp` | 197 lines | First 40 lines read; enum counts by grep | |
| `.../FNF_DTAS_Altis.Altis/f/loadout/defineclasses.sqf` | 139 lines | First 45 lines read | |
| Both `.Altis` trees | 159 files each | `find`, `diff -rq`, extension histogram | Variant delta = 18 files, 0 "Only in" |
| `.../FNF-DTAS-Altis/.git` | — | `git log`, `git rev-list` | **Shallow clone**, 1 commit `975c983` (2021-10-15) |

**Not read:** the remaining ~110 `.sqf` files (`roundclient.sqf`, `functions.sqf`, `f/spect/*`,
`f/loadout/{uniform,weapon}loadouts/*`, `QS_icons.sqf`) were located and line-counted but not read
in full. Claims about them are grep- or diff-derived and labelled as such. The 12 `.paa` files are
binary textures and were not opened.

### Consulted only, to resolve references (sibling agents own these)

| Source | How used |
|---|---|
| `.../fnf/FNF-v3.6.9/FNF_MissionTemplate.VR/config.sqf` | Read — proves the `phx_` → `fnf_` rename (27 `fnf_*` settings, 0 `phx_*`) |
| `.../fnf/FNF-v3.6.9/`, `.../FNF-v4.7.0/` (trees) | `grep -rl` / `find` / `comm` only — identifier-presence counts, binarization checks |
| `.../fnf/FNF-full/` (full clone, 1635 commits) | `git ls-tree`, `git show <commit>:<path>`, `cmp` — establishes that DTAS's `f/loadout/` is FNF's 2019 module (commit `0b32a3a6`) |

# Part 1 — FNF-MissionAnalyzer

## 1.1 What it is

| Property | Value | Evidence |
|---|---|---|
| **Purpose** | Vetting tool: "allows both the mission review team of Friday Night Fight staff and the mission creator to vet a mission and see if it meets the standards for submission" | `AnalyzeSQM.ps1:12` |
| **Language** | PowerShell 5.1 (Windows) + a 19-line Python 3 shim | `README.md:13`, `parseSqm.py` |
| **How it runs** | **Interactive desktop only.** "Left-click to select the AnalyzeSQM script in Windows Explorer, then right-click and select 'Run with Powershell'" | `README.md:17`, `AnalyzeSQM.ps1:24` |
| **Not CI** | It blocks on `$Host.UI.PromptForChoice(...)` and `Read-Host` before doing anything | `AnalyzeSQM.ps1:2229–2246` |
| **Input** | Absolute path to an **unbinarized** `mission.sqm`; sibling `config.sqf` is mandatory | `AnalyzeSQM.ps1:2246`, `:304`, `:314` |
| **Output** | One `Summary_<MissionName>.html` per mission (or `Mission_<n>.html` + `index.html` in Multi mode) written to the **script's** directory | `AnalyzeSQM.ps1:2004–2008`, `:2202` |
| **Third-party deps** | `PSSQLite` PowerShell module; `armaclass` Python package; Python 3.4+ | `README.md:6–11`, `AnalyzeSQM.ps1:2223–2224`, `:362–366` |
| **Licence** | **None.** `git ls-files` returns exactly 5 files; there is no `LICENSE`, and `README.md` contains no licence statement | `git ls-files` → `AnalyzeSQM.ps1 README.md fnfCfgExportDB.db parseSqm.py v1.4.1` |
| **Maintenance state** | **Dormant.** `main` HEAD is `3ee5b17` "Merge branch 'develop' into main", authored **2021-03-31**, by `Indigo <indifox926@gmail.com>` | `git show -s HEAD` |
| **Version** | `v1.4.1`, encoded as a zero-byte marker file that the script globs for at startup | `AnalyzeSQM.ps1:2215` — `Get-ChildItem -File \| Where-Object { $_.Name -match '^v\d.\d.\d$' }` |

**Caveat on maintenance state:** the clone is shallow (depth 1), so I can see only the tip commit,
not the full history. The claim "unmaintained" rests on tip-of-`main` being dated 2021-03-31 in a
clone taken 2026-08-01 — i.e. no commit to `main` in ~5 years.

### Two operating modes

* **Single** (`AnalyzeSQM.ps1:2244–2252`) — prompt for one `mission.sqm`, emit one HTML page.
* **Multi** (`AnalyzeSQM.ps1:2254–2270`) — the vetting team's weekly playlist build. Recurses for
  `*.sqm`, then loops **exactly six times** (`for ($i = 1; $i -lt 7; $i++)`), each iteration
  running Single with a `MissionNumber`, and writes an `index.html` with a hard-coded sidebar:
  `EU Mission 1..3` / `NA Mission 1..3` (`AnalyzeSQM.ps1:2176–2181`, mirrored per-page at
  `:1675–1680`). The six-mission playlist shape is **baked into the tool**, not configured.

### It mutates its input — flag this before reusing anything

The "analyzer" **rewrites `mission.sqm` in place**, twice:

```powershell
# AnalyzeSQM.ps1:330-332
[System.IO.File]::WriteAllLines(
    $FilePathSQM,
    (Get-Content $FilePathSQM | Out-String))
```

…re-encoding it as UTF-8-no-BOM, and again at `:341–343` to strip a leading block above the
`version=N;` line (a prefix "added by Mikero's derapping tools", `:335`). A validator that edits the
artefact it is validating is a hazard, and worth *not* copying.

## 1.2 What it parses, and how

### `mission.sqm` — via `armaclass`, not a hand-rolled parser

The whole SQM parse is delegated. `parseSqm.py` is the complete Python side:

```python
# parseSqm.py:14-19
result = armaclass.parse(testmission, keep_order=True)
jsonresult = json.dumps(result)
f = open("sqmjson.txt", "w")
```

PowerShell shells out (`AnalyzeSQM.ps1:368`), reads `sqmjson.txt`, `ConvertFrom-Json`s it
(`:376`), and **deletes the temp file** (`:378`). So the reusable parser here is
[`armaclass`](https://github.com/overfl0/Armaclass) — an Arma class-definition → JSON parser — not
anything FNF wrote. `AnalyzeSQM.ps1` contains a fossil record of the *previous*, regex-based
approach in block comments (`:519–522`, `:773–790`, `:992–1005`, `:1041–1058`): line-grep for
`overviewText=`, `description=`, `type=`. That regex era was abandoned for a real parser — a
lesson worth inheriting.

**Entity traversal** (`:384–443`): the JSON is walked as `Mission.Entities.Item<N>`, switched on
`dataType` into seven buckets — `Group`, `Object`, `Layer`, `Marker`, `Logic`, `Trigger`,
`Comment` (`:398–408`). Layers are then flattened **one level deep** (`:410–435`) and units are
pulled out of groups by iterating `Item0..Item12` (`:438–443`).

> **INFERRED:** the flatten is single-level and the group-unit loop is capped at 13 (`-le 12`), so
> nested layers-within-layers and groups larger than 13 are silently under-counted. I did not find
> a recursive descent anywhere in the file.

**The `description` convention.** FNF encodes *two* fields in Eden's one `description` string,
split on `@`:

```powershell
# AnalyzeSQM.ps1:448-449
@{n = "unitName";  e = { ($_.Attributes.description -split '@')[0] } },
@{n = "groupName"; e = { ($_.Attributes.description -split '@')[1] } },
```

So `"Vehicle Commander@Golf 1"` means role = Vehicle Commander, callsign = Golf 1. This is a
**string-packed relation because Eden has no field for it** — exactly the kind of thing a purpose-built
editor gets to model as real structure (role ref + squad ref) instead of parsing back out of a
free-text box.

### `config.sqf` — a naive line grep, and it is stale

```powershell
# AnalyzeSQM.ps1:531-535
$ConfigSettingsRaw = ($FileContentConfig | Select-String -Pattern '^phx_' ...)
... "Name" = ($t[0].Trim() -replace 'phx_', '') ...
```

It takes any line starting `phx_`, splits on `=`, strips quotes and the trailing `;`. No
expression evaluation, no comment stripping beyond that, no `#include` following.

**This finds nothing on a modern FNF mission.** `grep -r 'phx_'` over `FNF-v3.6.9/` and
`FNF-v4.7.0/` returns **0 hits in both**; `FNF-v3.6.9/FNF_MissionTemplate.VR/config.sqf` declares
**27** settings, all prefixed `fnf_` (`config.sqf:9,11,13,15,40,46,47,50,54,60,61,69,77,87,95,104,
119,120,123,124,127,128,136,139,142,173,177`). The community renamed `phx_` → `fnf_` and the
Analyzer was never updated. Its "Config Settings" table therefore renders **empty** against v3.6.9+.

### The asset database — `fnfCfgExportDB.db`

A 6.4 MB SQLite dump of Arma + FNF-modset config, queried per placed object via `PSSQLite`:

```powershell
# AnalyzeSQM.ps1:1087-1088
... -Query "SELECT ... from assets where assets.ClassName='$($Vehicle.type) '"
... -Query "SELECT ... from cfgVehiclesEmpty where ClassName='$($Vehicle.type) '"
```

| Table | Rows | Columns |
|---|---:|---|
| `assets` | 4,528 | ClassName (PK), DisplayName, Side, Category, Subcategory, Scope, DLC, Weapons, Magazines, Items, Addons, Feature |
| `cfgVehiclesEmpty` | 9,225 | ClassName (PK), DisplayName, Side, Category, Subcategory, Scope, DLC, Addons, Feature |
| `equipment` | 3,083 | ClassName, DisplayName, Side, Category, Subcategory, Scope, DLC, Addons, Feature |
| `weapons` | 1,665 | Class, Name, InventoryDescription, Magazines, Accessories, UsedBy |
| `vehicleWeapons` | 442 | Class, Name, InventoryDescription, Magazines, UsedBy |
| `magazines` | 2,252 | Class, Name, InventoryDescription, Ammo, UsedBy |

*(row counts from `select count(*)`; schema from `sqlite_master`.)*

**The classification rule is the DB membership itself:** in `assets` → it is a manned asset
(vehicle/infantry); in `cfgVehiclesEmpty` → it is a structure/prop; in neither → unknown.

**Quirk worth knowing if you ever reuse this DB:** every value is stored with a **trailing space**
(sample row: `ClassName: 'ArtilleryTargetW '`, `Side: 'BLUFOR '`), which is why every query
literal in the script appends one (`'$($Vehicle.type) '`) and every comparison calls `.Trim()`
(`:1318`, `:1320`). It is a dirty export, not a designed schema.

**This is the closest thing FNF has to TBD's asset registry** — a flat, offline-queryable class →
{display name, side, category, subcategory, weapons} table. TBD already has `registry_items`
(`T-068.2`); the concept validates.

## 1.3 THE RULE SET

**Headline count: 27 distinct checks exist in the source. Only 14 actually execute — 5 hard gates
and 9 validations. The other 13 are dead** (commented out, or guarded by a condition that can never
be true).

Severity is read off the code, which has four levels:

| Severity | How it manifests | Example |
|---|---|---|
| **fatal** | `Write-Error`/red host text + `Pause` + `exit` — no report produced | `:304–308` |
| **error** | Red accordion `style="background-color:#990000"`, and the only entry that reaches `$NeedToFix` | `:1758`, `:836` |
| **warning** | Orange accordion `class="accordion issuebg"` (`#995c00`) + `.issuetxt` (`#ffa31a`) | `:1649`, `:1644` |
| **info** | Plain accordion, count only, no colour | `:1854` |

**Editor timing** column: `live` = evaluable on every keystroke/placement from editor state alone;
`on-save` = needs a whole-document pass; `post-hoc only` = needs something the editor cannot know.

### A. Preconditions (hard gates — abort before any analysis)

| # | Rule | What it checks | Why it matters | Severity | Implemented at | Editor timing |
|---|---|---|---|---|---|---|
| P1 | SQM path exists | `Test-Path $FilePathSQM` | Typo'd path wastes a vetting slot | fatal | `AnalyzeSQM.ps1:288–292` | **n/a** — an editor owns the document; there is no path to be wrong |
| P2 | **SQM is not binarized** | Any byte > 127 in the raw file ⇒ treated as binary; "Mission is binarized and cannot be read" | Binarized SQMs cannot be reviewed, diffed, or fixed by anyone but the author | fatal | `AnalyzeSQM.ps1:294–308` | **n/a** — an editor's own document is by definition readable. *This whole class of problem disappears when the editor is the source of truth.* |
| P3 | `config.sqf` present beside the SQM | `Test-Path "$FilePathMission\config.sqf"` | A mission with no framework config is not an FNF mission | fatal | `AnalyzeSQM.ps1:314–318` | **live** — equivalent: "mission has required settings block" |
| P4 | Python ≥ 3.4 available | Regex on `python -V` | Tool prerequisite | fatal | `AnalyzeSQM.ps1:348–360` | **n/a** — tool-environment, not mission validity |
| P5 | SQM parses cleanly | `$LASTEXITCODE -ne 0` after `parseSqm.py`; "Please ensure it hasn't been manually modified" | Hand-edited SQMs break the game, not just the tool | fatal | `AnalyzeSQM.ps1:369–373` | **n/a** — see P2 |

**P2 is worth dwelling on.** Two of five hard gates (P2, P5) exist *purely because the mission
format is a file that a third-party tool may have mangled*. A browser editor with its own document
model deletes both. That is a structural advantage, not a feature to port.

### B. Active validations (these actually fire)

| # | Rule | What it checks | Why it matters | Severity | Implemented at | Editor timing |
|---|---|---|---|---|---|---|
| R1 | **Mission name format** | `$SQMJson.sourceName` must match `^FNF_[A-z0-9]+_[A-z0-9\-]+_[A-z0-9]+_v[0-9]{1,2}_(EU\|NA\|ANY)` — i.e. `FNF_<author>_<title>_<gamemode>_v<n>_<region>` | The playlist, the server rotation and the vetting index all key off the filename; a malformed name breaks scheduling downstream of the maker | warning | pattern `AnalyzeSQM.ps1:545`; test `:546–550`; render `:1721–1734` | **live** — pure string check on a metadata field |
| R2 | **Mission description / lobby text format** | `Mission.Intel.overviewText` must match the `MODE(n) // ATK: … // BLU: … // OPF: …` pattern at `:553` | The lobby line is how players read the game mode, attacker/defender and each side's assets *before* joining | warning | pattern `AnalyzeSQM.ps1:553`; test `:557–561`; render `:1737–1750` | **live** — but see the defect note below; in an editor this should be *generated*, not regex-checked |
| R3 | **At least one special role has a customised callsign** | If neither `$LabeledSpecialUnitsGolfHotel` nor `$LabeledSpecialUnitsCharlie` is non-empty → "No properly-named GOLF or HOTEL units found!" | If nobody customised any vehicle-crew or AT-team callsign, the maker has not done the slotting pass at all. This is the only rule severe enough to get its own red header | **error** | `AnalyzeSQM.ps1:834–838`; the one live `$NeedToFix` push `:836`; render `:1757–1759` | **live** — derivable from the ORBAT model continuously |
| R4 | **Charlie 2 leader callsign customised** | Units whose `unitName` matches `Team Leader` while `groupName` is still the stock `'Charlie 2'` are listed as unlabeled | Charlie 2 is FNF's MAT (medium anti-tank) team. The stock callsign tells a player nothing; the maker is expected to replace it with something that identifies the team's actual job | warning | "labelled" `AnalyzeSQM.ps1:796`; "unlabelled" `:824–826`; render `:1769–1792` | **live** |
| R5 | **Golf / Hotel crew callsigns customised** | Units whose `unitName` ∈ {`Vehicle Platoon Leader`, `Vehicle Commander`, `Air Detachment Leader`, `Pilot`} (`:727–732`) while `groupName` is still a stock value ∈ {`Golf`,`Golf 1..4`,`Hotel`,`Hotel 1..4`} (`:733–744`) | Golf = vehicle platoon, Hotel = air. A player picking "Vehicle Commander@Golf 2" cannot tell *which vehicle* — the maker is expected to encode that in the callsign | warning | "labelled" `AnalyzeSQM.ps1:795`; "unlabelled" `:821–823`; render `:1794–1817` | **live** |
| R6 | **Required triggers present** | All five of `zoneTrigger`, `phx_sec1`, `phx_sec2`, `phx_sec3`, `ctf_attackTrig` exist as trigger names | These are the game-mode mechanics. Delete one and the mode silently does not run — this is the classic "played it, nothing happened" failure | warning | list `AnalyzeSQM.ps1:876–882`; loop `:883–893`; render `:1882–1900` | **live** — a "required framework entities" presence set is a continuous check |
| R7 | **≥ 6 ACE spectator slots** | `count($LogicObjs where type == 'ace_spectator_virtual') < 6` → one missing-object row per shortfall | Dead players and casters need somewhere to go; FNF mandates six | warning | `AnalyzeSQM.ps1:914–923` | **live** — a cardinality rule on one entity type |
| R8 | **≥ 1 Zeus / Game Master module** | `count($LogicObjs where type == 'ModuleCurator_F') < 1` | Without a curator the admin cannot intervene mid-game | warning | `AnalyzeSQM.ps1:925–932` | **live** — cardinality rule |
| R9 | **All vehicle inventories empty** | Any vehicle whose inventory `CustomAttributes.ammoBox` value ≠ `[[[[],[]],[[],[]],[[],[]],[[],[]]],false]` is listed | PvP fairness: a vehicle pre-loaded with gear is an unearned supply cache. The empty-inventory sentinel is a **literal string comparison** against Eden's serialised empty-container value | warning | sentinel `AnalyzeSQM.ps1:1123`/`:1144`; collection `:1449–1489`; render `:1837–1848` | **live** — per-object invariant, checkable the moment cargo is edited |

**Notes on the active rules — each is a defect a live editor would not have:**

* **R1 uses `[A-z]`, not `[A-Za-z]`.** The ASCII range `A`–`z` also contains `` [ \ ] ^ _ ` ``, so
  the "letters or digits" classes silently admit punctuation. Harmless here (it only makes the
  check *looser*), but it is why a regex is the wrong primitive for a naming convention that an
  editor could simply *construct*.
* **R2's own help text fails R2's own regex.** I ran both patterns against every example the repo
  ships (patterns transcribed verbatim from `:545` and `:553`; executed with Python `re` — the
  patterns use no .NET-specific constructs):

  | Example | Source | Result |
  |---|---|---|
  | `RUSH(2) // ATK: BLU 15% adv - DEF: OPF // BLU: … // OPF: …` | in-code comment `:554` | **match** |
  | `NSECTOR(3) // ATK: BLU OPF IND // BLU: … // OPF: … // IND: …` | in-code comment `:555` | **match** |
  | `NSECTOR(3) // ATK: BLU OPF // BLU: … // OPF: …` | in-code comment `:556` | **match** |
  | `Gamemode(x) // ATK: Faction1 X% advantage - DEF: Faction2 // …` | **HTML shown to the maker** `:1742` | **FAIL** |
  | `UPLINK(3) // ATK: BLUE 15% advantage - DEF: OPFOR // BLUE: … // OPFOR: …` | **HTML shown to the maker** `:1743` | **FAIL** |
  | `FNF_JohnDoe_9LivestoLive_destroy_v2_ANY.Altis` | HTML `:1726` (name rule R1) | match |
  | `FNF_Johnny_25minutes-to-wait_nsector_v1_EU.Malden` | HTML `:1727` (name rule R1) | match |

  The pattern requires `[A-Z]{3}` faction tokens and the literal `adv`; the help text says `BLUE`,
  `OPFOR`, `advantage`. **Both examples the tool shows a failing maker are themselves invalid under
  the rule it just failed them on.** Copying the fix-it example does not fix it. This is the single
  strongest argument in the repo for *generating* the string from structured fields rather than
  validating free text.
* **R3/R4/R5 check the *callsign*, not the role.** The intent is stated in the code comment:
  "Check to make sure non-default unit descriptions exist in C2/G/H groups and if they exist, make
  sure all are completed" (`AnalyzeSQM.ps1:793`). "Labelled" means `groupName` (the part after `@`)
  has been changed away from a stock value; "unlabelled" means it is still `Golf 1` / `Charlie 2`.
  So the rule is: *if you have a special role, its callsign must say something useful.*
* **R3/R4/R5's colour logic is inverted — a perfect mission can never go green.** The accordion
  header at `AnalyzeSQM.ps1:1760` reads:

  ```powershell
  } elseif (!$NonLabeledSpecialUnitsGolfHotel -or !$NonLabeledSpecialUnitsCharlie -or
            !$LabeledSpecialUnitsGolfHotel   -or !$LabeledSpecialUnitsCharlie) {
      Write-Output '<button class="accordion issuebg">…'   # orange
  } else { … goodbg … }                                     # green
  ```

  Green requires **all four** lists to be non-empty — including the two *unlabelled* lists. A
  mission where every special callsign has been done correctly has zero unlabelled units, so
  `!$NonLabeledSpecialUnitsGolfHotel` is true and it is marked **orange**. Doing the job perfectly
  is indistinguishable from doing it badly. (The per-row tables at `:1769` and `:1794` are still
  correct — they only print when there *are* unlabelled units — so the detail is right and only the
  header colour lies.)
* **R6's identifiers are the pre-rename generation.** `phx_sec1/2/3` returns 0 hits across both
  framework checkouts; `FNF-v3.6.9` uses `fnf_sec1/2/3` and `fnf_sector1..4`. The *rule* is still
  right; the *names* are five years stale. Same for R6's `zoneTrigger` / `ctf_attackTrig`, which
  do still exist in v3.6.9 (4 and 6 file-hits) but return 0 in v4.7.0 — though note v4.7.0's
  template `mission.sqm` is **binarized** (`head -c 8` → `\0raP`), so a 0 there is not proof of
  absence, only of unreadability. I am not claiming anything about v4.7.0's entity names.
* **R9's sentinel is a serialised-format literal.** Comparing against
  `[[[[],[]],[[],[]],[[],[]],[[],[]]],false]` is brittle in exactly the way a typed document model
  is not.

### C. Dead rules — present in source, never execute

These matter *more* than they look: they are the checks FNF **wanted** and lost to bit-rot. They
belong on the feature list even though the tool no longer runs them.

| # | Rule | What it would check | Why it is dead | Severity intended | Implemented at |
|---|---|---|---|---|---|
| D1 | **Required core markers** | `destroy_obj1Mark`, `destroy_obj2Mark`, `bluforSafeMarker`, `opforSafeMarker`, `indforSafeMarker` | Guarded by `if ($MarkObjs.name)` — **`$MarkObjs` is a typo** for `$MarkerObjs` and appears exactly once in the whole file, so it is `$null` and the loop never runs | warning | `AnalyzeSQM.ps1:895–912`; the typo at `:902` |
| D2 | **Required core objects** | `term1`/`term2`/`term3` (`Land_DataTerminal_01_F`), `destroy_obj1`/`destroy_obj2` (`Box_FIA_Ammo_F`), `ctf_flagPole` (`FlagPole_F`) | Guarded by `if ($ReqCoreObjs.name)` — `$ReqCoreObjs` is an **array of arrays**, which has no `.name` property, so the guard is always false | warning | `AnalyzeSQM.ps1:934–953`; the bad guard at `:943` |
| D3 | Charlie 2 has ≥ 2 labelled special units | "Missile Specialist in C2 needs role description set" | Inside a `<# … #>` block | warning | `AnalyzeSQM.ps1:840–845` |
| D4 | Golf Actual has a labelled Vehicle Platoon Leader | — | Inside `<# … #>` at `:846–867` | warning | `AnalyzeSQM.ps1:847–850` |
| D5 | Golf 1 has a labelled Vehicle Commander | — | ditto | warning | `AnalyzeSQM.ps1:851–854` |
| D6 | Golf 2 has a labelled Vehicle Commander | — | ditto | warning | `AnalyzeSQM.ps1:855–858` |
| D7 | Golf 3 has a labelled Vehicle Commander | — | ditto | warning | `AnalyzeSQM.ps1:859–862` |
| D8 | Golf 4 has a labelled Vehicle Commander | — | ditto | warning | `AnalyzeSQM.ps1:863–866` |
| D9 | Zeus module present (legacy impl.) | Superseded by R8 | Inside `<# … #>` at `:961–975` | warning | `AnalyzeSQM.ps1:962–967` |
| D10 | ≥ 6 spectator slots (legacy impl.) | Superseded by R7 | ditto | warning | `AnalyzeSQM.ps1:969–974` |
| D11 | **"Issues to Fix" rollup** | Render every `$NeedToFix` entry as a single list at the top of the report | The whole `<h2>Issues to Fix</h2>` block is inside an **HTML comment** `<!-- … -->` | — | `AnalyzeSQM.ps1:1694–1701` |
| D12 | Init-script listing | Surface every non-empty unit/object `init=` field for review | `$NonEmptyInits` is assigned **only inside** the block comment `1493–1498`, so the render guard at `:1935` is always false | info | `AnalyzeSQM.ps1:1493–1498`, `:1935–1944` |
| D13 | Unknown-object listing | List every classname found in neither DB table | `$AllUnknownsGrouped` is assigned only inside the block comment spanning `1235–1427`, so the panel at `:1958–1961` always renders empty | info | `AnalyzeSQM.ps1:1413–1427`, `:1958–1961` |

**D11 is the most consequential loss.** With the rollup commented out, a maker gets **no summary
of failures** — they must expand each accordion and notice which headers are orange or red. The
one aggregation the tool had, it disabled. Our editor should treat "the issue list" as the
primary surface, not a secondary one.

**D1 + D2 together mean the tool no longer checks that objectives exist at all.** The single most
important class of mission error — the destroy objective, the terminals, the flagpole, the safe-start
zones — is unverified because of a variable typo and a bad truthiness guard. Only the *triggers*
(R6) survive. This is a five-year-old silent failure in a tool people trusted.

### D. Informational extraction (reported, never judged)

Not rules, but they define the report's shape and are worth knowing:

| Output | Source |
|---|---|
| Config settings table (`phx_*`) | `AnalyzeSQM.ps1:530–535` → render `:1706` |
| Weather **start**: overcast, fog, fog decay, fog base, wind, rain, `rainForced` (bool), waves | `AnalyzeSQM.ps1:613–655` → render `:1711`. Rendered as a 10-cell ASCII bar via `Show-ResultAsTextBar` (`:571–584`) |
| Weather **forecast**: overcast, fog decay, wind, waves, lightning | `AnalyzeSQM.ps1:663–683` → render `:1714` |
| Usable vs **locked** vehicles, grouped by (type, init, textures), with resolved weapon display names | `AnalyzeSQM.ps1:1114–1156`, `:1176–1216` → render `:1854–1863` |
| All soldiers (side / unitName / groupName) | `:444–451` → render `:1867–1871` |
| Logic / Trigger / Marker object counts + lists | render `:1904–1926` |
| Structures & decorations (from `cfgVehiclesEmpty`) | `:1159–1169`, `:1219–1233` → render `:1949–1953` |

Weapon-name resolution filters a blocklist `rhs_wep_DummyLauncher|Horn|\'|DUKE|MASTERSAFE`
(`AnalyzeSQM.ps1:1094`) so the report shows real armament, not horns and dummy launchers.

> **Minor defect:** `$UnwantedWepsPattern` is assigned *inside* the multi-weapon branch (`:1094`)
> but read in the single-weapon `else` branch (`:1103`). Until some multi-weapon vehicle has been
> processed, it is `$null`, so the filter is a no-op for early single-weapon vehicles.

## 1.4 What it reports, and how a maker acts on it

A single self-contained HTML file — inline CSS (`:1511–1667`), 19 lines of vanilla JS for the
accordions (`:1979–1997`), dark theme, no external assets. Structure:

1. `<h1>` mission name, `<h3>` lobby description (`:1688–1691`)
2. ~~Issues to Fix~~ (D11 — commented out)
3. Config Settings table (empty in practice — §1.2)
4. Weather Start / Forecast tables
5. **Name and Description** — two accordions, green or orange, each with a "here is the correct format" panel
6. **Units and Assets** — role-description accordion (green / orange / red), vehicle-inventory accordion, then plain count accordions for Vehicles and Soldiers
7. **Logic / Triggers / Markers** — the Required Game Objects accordion + plain lists
8. **Other Information** — init scripts (dead), structures, unknown objects (dead)

**The action model is: scan for colour, expand, read the table, go fix it in Eden.** There is no
line reference, no jump-to, no machine-readable output. Green (`#006600`) = pass, orange
(`#995c00`) = fix this, red (`#990000`) = you have not done the work. The report also carries a
deliberate false-positive disclaimer:

```html
<!-- AnalyzeSQM.ps1:1886-1889 -->
<h4>The following missing items can be ignored:</h4>
<p>Safe Start markers for non-playing factions</p>
<p>Terminal 3 in 2-terminal game modes</p>
```

That disclaimer is itself a design signal: **the rule set has no notion of "which game mode is this
mission?"**, so it demands every objective object for every mode and then tells the human to
mentally filter. An editor that *knows* the selected game mode makes the requirement set
conditional and the disclaimer unnecessary.

## 1.5 Which rules could run live in a mission editor

| Rule | Timing | Justification |
|---|---|---|
| P1, P2, P4, P5 | **n/a** | Artefacts of "the mission is a file on disk that external tools touch". A browser editor with its own document model has no path, no binarization, no external parser, no Python. Do not port them; note that owning the document *deletes* them. |
| P3 (config present) | **live** | Restates as "the mission document has its required settings block populated" — a document-shape invariant. |
| R1 mission name | **live** | Pure predicate on one metadata string. Better: **generate** it from author + title + mode + version + region and make the malformed state unreachable. |
| R2 lobby description | **live** | Same, more so. Given the tool's own example fails its own regex (§1.3), the right move is a structured briefing header compiled to the string, with the free-text box removed. |
| R3 special roles labelled | **live** | Derivable from the ORBAT tree on every edit. TBD already models slots and roles as first-class (`T-180`), so this is a query, not a parse. |
| R4 Charlie 2 leaders | **live** | Per-slot predicate. |
| R5 Golf / Hotel crew roles | **live** | Per-slot predicate. |
| R6 required triggers | **live** | Set-membership over placed framework entities. Cheap to keep current incrementally. |
| R7 ≥ 6 spectator slots | **live** | Cardinality over one entity type; recompute on add/remove. |
| R8 ≥ 1 Zeus module | **live** | Cardinality. |
| R9 vehicle inventories empty | **live** | Per-object invariant; fires the instant cargo is edited. TBD's arsenal/cargo work (`T-068.15`) already owns the data. |
| D1 required markers | **live** | Same shape as R6 — a named-entity presence set. Should be **revived**, and made conditional on game mode (§1.4). |
| D2 required objectives | **live** | Same. This is the highest-value dead rule. |
| D3–D8 per-squad role coverage | **live** | Per-squad cardinality predicates over the ORBAT. |
| D9, D10 | **live** | Duplicates of R7/R8. |
| D11 issue rollup | **live** | Not a rule — an *aggregation*. Should be the editor's persistent validation panel. |
| D12 init-script listing | **on-save** | A whole-document sweep for arbitrary script fragments. Cheap enough per-document, pointless per-keystroke; and in TBD there is arguably no init-string field to sweep. |
| D13 unknown classnames | **on-save** *(live if the registry is resident)* | Requires a lookup against a 4.5k+13.7k-row asset registry. **Not post-hoc**: TBD already ships `GET /api/v1/registry` with ETag caching (`T-068.2`), so with the catalogue in memory this becomes live. Marked `on-save` only as the conservative default for a cold registry. |
| Weather / config / count tables | **on-save** | Reporting, not validation — belongs in a "mission summary" view, generated when the maker asks for it. |

**Nothing in this rule set is genuinely `post-hoc only`.** That is the finding. Every check the
Analyzer performs is a predicate over structure the editor already holds — presence, cardinality,
naming, or a per-object invariant. The *only* things that must stay post-hoc are things the
Analyzer does not do at all: playtest-derived facts (is the mission balanced? does the AI path?
does the objective actually reachable-by-attackers?), and cross-mission playlist concerns (are
three EU and three NA missions selected?). The four gates I marked `n/a` are not post-hoc either —
they simply cease to exist.

## 1.6 Reuse verdict — what is worth taking, concretely

| Artefact | Verdict | Reason |
|---|---|---|
| **The rule set** (§1.3) | **Take, all of it** | It is a real community's five-year-tested definition of mission validity. It is also the only such definition in the FNF corpus. |
| **`armaclass`** (the SQM parser) | Concept only | Arma 3 `.sqm` class syntax, not Reforger. The transferable lesson is *use a real parser, not line regexes* — the Analyzer's own commented-out regex era (§1.2) is the counter-example. |
| **`fnfCfgExportDB.db`** (the asset DB) | Concept only, and TBD already has it | Flat classname → {display, side, category, subcategory, weapons} is exactly TBD's `registry_items` shape (`T-068.2`). The FNF instance is Arma 3 content and a dirty export (trailing spaces on every value). |
| **The `role@callsign` packing** | **Anti-pattern to avoid** | An artefact of Eden having one free-text field. TBD models role and squad separately; do not reintroduce a packed string. |
| **The HTML report** | Concept only | Single-file, offline, accordion-per-section is a reasonable *export* format for a mission summary. It is the wrong shape for live validation, which needs a persistent panel, not a generated document. |
| **The PowerShell / Python / SQLite / Windows-Explorer toolchain** | **Discard entirely** | Interactive-only, Windows-only, unlicensed, five years dormant. |

---

---

# Part 2 — FNF-DTAS-Altis

## 2.0 Two corrections to the brief

1. **DTAS = "Dynamic Take And Secure", not "Destroy The Attackers' Spawn."** Line 3 of the repo's
   own README: `## **Dynamic Take and Secure**`, `_Inspired by DTAS (Dynamic Take And Secure) for
   Infiltration…_` (`FNF-DTAS-Altis/README.md:1–5`). It is an attack/defend round mode with a
   randomly relocating objective.
2. **It is not authored against the FNF mission template.** It is a 2013 third-party mission
   (Gal Zohar, Arma Israel) that FNF adopted and grafted its loadout module into — see §2.2.

## 2.1 What the game mode is, and how it works

Round-based attack/defend, one life per round, no AI, objective **repositioned by script**.

| Phase | Behaviour | Evidence |
|---|---|---|
| **Join** | Auto-assigned to a side; spawn in walled safe zones at opposite ends of the map; scroll-wheel crates give class loadouts; TFAR channels auto-assigned per group (≤3) and per side | `README.md:37–39`; `classmenu.sqf`, `classaction.sqf`; `functions/fnc_addradio.sqf`, `functions/fnc_handlegroups.sqf` |
| **Planning** | 60 s first round, 30 s after; objective location + this side's ATK/DEF role visible; ACE self-interaction offers a 1×–2× optic | `README.md:43`; params `FirstRoundSetupTime` = 60 / `SetupTime` = 30 (`description.ext:151–158`, `:144–150`); `optics.sqf` |
| **Insertion** | Attackers spawn near the objective in weighted-random vehicles; defenders spawn **on** the objective, preserving their base-relative formation offset | `README.md:48`, `:53`; `roundserver.sqf:6–18`; `roundclient.sqf:21–27`, `:357–360` |
| **Playing** | 10-min default timer; wipe the defenders or hold the capture ring. HUD = timer + capture bar | `README.md:55`; `TimeLimit` = 10 (`description.ext:126–132`); `hud_dialog.cpp`, `hud_update.sqf` |
| **Round end** | Four outcomes, coded `roundEnd` 1–4: attackers wiped / defenders wiped / zone captured / timer expired | `roundserver.sqf:584–642`, decoded in `roundendmsg.sqf:11–59` |
| **Heats** | Each location is played **twice**, sides swapped between heats — implemented as a `_changeAttackerSide` toggle that gates re-rolling the objective | `README.md:61`; `roundserver.sqf:668` |
| **Match end** | First to `maxScore` **with a 2-point lead** | `endhandler.sqf:29` — `waitUntil {((scoreW >= maxScore) && (scoreW > (scoreE + 1))) \|\| ((scoreE >= maxScore) && (scoreE > (scoreW + 1)))};` |

### The objective generator — and the one genuinely editor-facing hook

`roundserver.sqf:99–111` discovers objective-spawn regions by **string-prefix match over
`allMapMarkers`**, then area-weights them:

```sqf
_markerPrefix = "mrkZone";
{
	if ([_markerPrefix, _x] call BIS_fnc_inString) then {
		_area = ((markerSize _x) select 0) * ((markerSize _x) select 1);
		totalMarkerArea = totalMarkerArea + _area;
		markerAreaArray set [_j, [_x, _area]];
```

`preinit.sqf:3–21` hides everything matching the same prefix (`_x setMarkerAlpha 0`). **A maker
reshapes objective distribution purely by drawing more `mrkZone1`, `mrkZone2`, … rectangles in
Eden — no code change, no config entry.** This is a naming-convention-as-API, and it is the single
most editor-friendly authoring hook in the mission.

Placement then rejection-samples inside the chosen zone until the point is not water, is
≥ `minDist + 50` m from **both** respawn markers, and has 3–6 non-trivial buildings within 75 m:

```sqf
// roundserver.sqf:157-167
objPos = [_minX + random (_maxX - _minX), _minY + random (_maxY - _minY)];
_nearBuildings = nearestObjects [objPos, ["House_F"], 75, true] select {!(_x isKindOf "PowerLines_base_F" || ... || _x isKindOf "House_Small_F")};
(!(surfaceIsWater objPos))
(!(([objPos, (markerPos "respawn_west")] call fnc_airDistance) < (minDist + 50)))
```

matching `README.md:71` ("a bias toward locations with a moderate number of sizeable buildings").
The marker's `angle=` is **ignored** by the sampler — the effective region is the axis-aligned box.

### Capture is superlinear in attacker count and uncontested

```sqf
_r = ln (1 - minCapTime/maxCapTime);                                  // capture.sqf:31
capPercentage = capPercentage + ((1 - exp (_r*_count))/minCapTime)/2; // capture.sqf:51
```

With `minCapTime = 20; maxCapTime = 60;` (`init.sqf:9–10`): 60 s for a lone attacker, asymptoting
to 20 s for many. There is **no defender decrement** — the subtraction is commented out at
`capture.sqf:22`.

### The comeback timer is asymmetrically hidden

When the attackers are down to one player, ≤10 % alive, or ≤5 participants overall, the round timer
is clamped and the removed time stored in `fakeExtraDefenderTime` (`roundserver.sqf:571`,
`:614–622`). The client then adds it back **for defenders only**:

```sqf
// timerupdateclient.sqf:22-26
_actualDisplayedTime = _lastTime;
if (sidePlayer != attackerSide) then { _actualDisplayedTime = _lastTime + fakeExtraDefenderTime; };
```

The mode deliberately shows the two sides **different HUD state** to preserve the attackers'
illusion of force (`README.md:73`).

### The two variants

Both trees carry **exactly the same 159 files** — `diff -rq` reports **no "Only in" entries** and
only **18 files differ**:

```
ads/adboard.paa · ads/controls.hpp · description.ext · f/loadout/defineclasses.sqf
f/loadout/fn_loadout_checkLoadout.sqf · f/loadout/fn_loadout_set.sqf
f/loadout/units/{AT,CE,RAT}.sqf · f/loadout/weaponloadouts/{blufor,opfor}Loadout.sqf
flagmenu.sqf · functions/fnc_assigngear.sqf · functions.sqf · init.sqf
mission.sqm · roundclient.sqf · roundserver.sqf
```
*(method: `diff -rq FNF_DTAS_Altis.Altis FNF_DTAS_AltisWWII.Altis`)*

**Only 4 of those 18 diffs are the intended knob** — the four loadout-param defaults in
`description.ext:253,259,265,271` (`UNIFORM_MARPAT_WD`→`UNIFORM_USA`,
`UNIFORM_EMR_SUMMER`→`UNIFORM_GERMANY`, and the two weapon sets). Everything else is
**unpropagated drift**: WWII's `roundserver.sqf` is a structurally *older* generation of the same
function (single fixed `fow_v_willys_usmc` instead of the weighted vehicle table, no road-graph
placement), and WWII carries a divergent `opforLoadout.sqf` that its own `WEAPONS_GERMANY` default
never invokes. Mod requirements differ too: Standard needs CBA_A3 + RHS USAF + RHS AFRF + TFAR;
WWII needs IFA + FOW (`README.md:20–29`).

## 2.2 The framework/mission split — and where the FNF code actually is

### DTAS is an adopted codebase with one FNF module grafted in

The lineage is stated outright: 2013 Gal Zohar (Arma Israel), logo + Russian localisation by
Excess3 → 2016 Fritz → 2020 Martin → "2021: Modernized and tailored for the Friday Night Fight by
the FNF Technical Team" (`README.md:9–15`). Verified:

| Check | Result |
|---|---|
| DTAS paths shared with the **`FNF-v3.6.9` worktree** (any relative path) | **0** |
| DTAS paths shared with FNF's **initial commit `0b32a3a6`** ("init", Mjolnir64, 2019-08-24, in `FNF-full`) | **33** |
| Of those 33, byte-identical | **2** — `f/loadout/fn_loadout_handleClothing.sqf`, `f/loadout/readme.md` |
| `phx_fnc_loadout_handleGear` in `0b32a3a6:f/loadout/` | present |
| `phx_*` anywhere in `FNF-v3.6.9` / `FNF-v4.7.0` | **0** |
| `phx_*` in DTAS | **1,170 hits across 72 files**, all in the loadout subsystem |
| `fnf_*` in DTAS | **0** |
| `config.sqf` (FNF's mission-config file) in DTAS | **absent** |

*(method: `comm -12` over `git ls-tree -r --name-only 0b32a3a6 \| sort` vs
`find . -type f -printf '%P\n' \| sort`; `git show <commit>:<path> \| cmp -s -`; `grep -rc`.)*

**So `f/loadout/` is genuinely FNF framework code — FNF's *2019* loadout module.** By v3.6.9 FNF had
relocated it to `FNF_MissionTemplate.VR/client/loadout` and renamed the whole namespace `phx_` →
`fnf_`, which is why a path comparison against the v3.6.9 worktree finds nothing. DTAS froze the
old copy. Everything else that merely shares a *filename* (`init.sqf`, `briefing.sqf`,
`description.ext`, `cba_settings.sqf`, `mission.sqm`) has entirely different contents.

`f/spect/` is **not** FNF at all — its own header says
`// F3 - Spectator Script / Credits: Please see the F3 online manual` (`f/spect/fn_CamInit.sqf:1–2`),
i.e. Folk ARPS, DTAS's grandparent lineage.

**Consequence: the MissionAnalyzer cannot run on this mission.** Gate P3 (§1.3) hard-exits when
`config.sqf` is missing (`AnalyzeSQM.ps1:314–318`), and DTAS has none. FNF's validator cannot
validate FNF's own shipped game mode. The 1,170 `phx_` hits do confirm DTAS and the Analyzer are
contemporaries — both were left behind by the `phx_` → `fnf_` rename.

### Annotated layout (both variants identical: 159 files, 12 dirs)

**[FW]** framework · **[MODE]** DTAS game-mode code · **[INST]** per-instance authored · **[DEAD]** unreferenced

```
FNF_DTAS_Altis.Altis/     134 .sqf · 12 .paa · 8 .hpp · 1 each .sqm/.ext/.cpp/.xml/.md
├── mission.sqm           [INST] 155,704 B, UNBINARIZED (head = "version="), version=54, items=182
├── description.ext       [INST] 285 lines — Header, params, CfgNotifications, CfgDebriefing, CfgFunctions
├── islandspecific.hpp/.sqf [INST] 2 lines each — the ENTIRE per-terrain surface
├── cba_settings.sqf      [INST] 353 assignments, 271 forced — byte-identical between variants
├── stringtable.xml       [MODE] 222 keys, EN + 100 RU (the 2013 Excess3 localisation)
├── preinit.sqf/preinit2.sqf · init.sqf   [MODE] boot; params unpacked from paramsArray by index
├── roundserver.sqf (671 L) / roundclient.sqf (464 L)   [MODE] ← the game mode
├── capture.sqf · endhandler.sqf · roundendmsg.sqf · timerupdate{server,client}.sqf  [MODE]
├── hud_dialog.cpp · hud_create.sqf · hud_update.sqf    [MODE] 4-control HUD (idd 1000)
├── classmenu.sqf · classaction.sqf · flagmenu.sqf · pickspawnaction.sqf  [MODE]
├── briefing.sqf          [MODE] 9 createDiaryRecord calls, all localize + live param interpolation
├── QS_icons.sqf          [FW] 1,731 lines, shared with FNF (142 diff lines)
├── unitmarkers.sqf       [DEAD] 292 lines, zero references
├── spawnprotection.sqf   [DEAD] only ref is `//execVM` at init.sqf:63
├── {ready,preferdriving,choosetfrchannelmenu,canceltfrchannelmenu,settfrchannel,enablecommandchannel}
│                         [DEAD in base, LIVE in WWII] — toggled purely by //-commenting flagmenu.sqf:86,92-99
├── adminactions/         [MODE] 4 files, 6 lines total — force/pause/unpause round, relocate objective
├── functions/            [MODE] 9 fnc_*.sqf, 368 lines
├── ads/                  [INST] event advertising board — controls.hpp:19 hardcodes "FNF Titans v3" + a Google Doc URL
├── images/ · media/      [INST] 8 notification icons + logo + two side flags
└── f/
    ├── loadout/          [FW] FNF's 2019 module — 33 shared paths, 2 byte-identical
    │   ├── cfgLoadouts.hpp   197 L — 19 UNIFORM_*, 12 WEAPONS_*, 17 ROLE_* + ROLE_SPECTATOR 99
    │   ├── defineclasses.sqf [MODE] 139 L — DTAS-authored role menu, NOT in FNF
    │   ├── units/            17 role scripts
    │   ├── uniformloadouts/  26 files [MODE — DTAS refactor]
    │   └── weaponloadouts/   17 files [MODE — DTAS refactor]
    └── spect/            [DEAD] 18 files (2,237 lines; 15 are .sqf = 1,356 SQF lines) — F3 spectator, superseded by ACE
```

**17.8 % of the SQF is dead — 1,736 of 9,754 lines** (`find . -name '*.sqf' -exec cat {} + | wc -l`):
`f/spect/*.sqf` (1,356), `unitmarkers.sqf` (292, **zero references anywhere in the mission**),
`spawnprotection.sqf` (8, only reference is the commented `//execVM` at `init.sqf:63`), and the six
flagmenu-toggled scripts (80). `description.ext:277–279` comments out the F3 spectator's
`CfgFunctions` registration while `:275` still `#include`s its dialog classes.

## 2.3 What the maker actually authors — almost nothing in the .sqm

Census of `FNF_DTAS_Altis.Altis/mission.sqm` (`grep -c`; **identical in both variants**):

| Entity | Count | Detail |
|---|---:|---|
| `dataType="Group"` | **124** | one unit each — 62 West, 62 East |
| `isPlayable=1` | **124** | matches `maxPlayers = 124` (`description.ext:8`) exactly |
| `dataType="Object"` | 178 | 14 distinct classnames: 62 `O_Soldier_F`, 62 `B_Soldier_F`, 24 `Land_HBarrierBig_F`, 14 `B_supplyCrate_F`, 6 `Land_Noticeboard_F`, 4 `Land_Sign_WarningNoWeapon_F`, 2 each `SignAd_SponsorS_F` / `Land_CinderBlocks_F` / `Flag_White_F` |
| `dataType="Marker"` | **5** | see below |
| `dataType="Trigger"` | 2 | both `EmptyDetector`, `sizeA=sizeB=27`, resized at runtime to `capRad` |
| `dataType="Logic"` | 2 | `ModuleCurator_F` (Zeus, owner `#adminLogged`) + one bare `Logic` used only as a preinit hook |
| `dataType="Layer"` | 1 | `name="AdBoards"` |

**All five markers:**

| Name | Type | Purpose |
|---|---|---|
| `mrkObj1` | `mil_objective` icon | objective icon; moved at runtime |
| `mrkObj` | `ELLIPSE` | capture ring; resized to `capRad` (`roundserver.sqf:49`) |
| `respawn_west` / `respawn_east` | `Empty` | engine respawn + base anchor + `minDist` reference |
| `mrkZone0` | `RECTANGLE`, half-extents 14000 × 11000 | **the objective spawn region** — essentially all of Altis |

**Every one of the 124 playable units is a bare soldier** with the same description:

```
// mission.sqm:1704-1708
skill=0.2;
name="w24";
description="Soldier";
isPlayer=1;
isPlayable=1;
```

`grep -oE 'description="[^"]*"' mission.sqm | sort -u` → **one distinct value**, `"Soldier"`, ×124.
Names are positional only: `w0`–`w61`, `e0`–`e61`.

**There is no ORBAT in the mission file.** Squads are formed in-game via
`["Initialize"] call BIS_fnc_dynamicGroups` (`init.sqf:39`, `:73`); roles are picked from a crate
scroll-menu backed by `f/loadout/defineclasses.sqf`, which builds
`[displayName, ROLE_ID, onSelectCode]` triples:

```sqf
// f/loadout/defineclasses.sqf:3-21
aClasses = [ [ "Squad Lead", ROLE_SL, {} ]
	,[ "Team Lead",           ROLE_TL,  {} ]
	,[ "Combat Life Support", ROLE_CLS, {isMedic = true;} ] ...
```

11 attacker + 11 defender classes in the base variant; 9 + 9 in WWII.

**DTAS therefore structurally cannot satisfy MissionAnalyzer rules R3/R4/R5** (§1.3), which demand
customised `role@callsign` descriptions on Golf/Hotel/Charlie groups. It has no such groups and one
literal description.

**Only 7 distinct `init=` expressions in the whole SQM** (49 occurrences): `this allowDamage false`
×24 (barriers), `execVM "populateammocrate.sqf"` ×14 (crates), a `BIS_fnc_holdActionAdd` remoteExec
×6 (ad posters), `execVM "flagmenu.sqf"` ×2 (the two "yellow box" action hubs),
`setFlagTexture` ×2, and the single `preinit.sqf` hook.

**Metadata inconsistencies worth noting** (both are authored, both shipped): `description.ext:6–8`
declares `minPlayers = 4` while `mission.sqm:135–137` declares `minPlayers=2`; and
`mission.sqm:130` says `author="Gal Zohar, Fritz, Friday Night Fight"` while `description.ext:11`
says `author = $STR_GalZohar`. Also, `AddonsMetaData` lists only 10 addons and **includes neither
RHS, TFAR nor CBA** despite `README.md:20–24` requiring all three — because they are referenced
only from SQF strings, never from a placed entity.

## 2.4 Per-instance tunables

### `class params` — 17 active, 3 commented out (`description.ext:116–273`)

Chosen in the server lobby; unpacked **by index** into `missionNamespace` at `preinit2.sqf:9–14`.

| Param | Title | Values | Default | Consumed at |
|---|---|---|---|---|
| `MaxScore` | Best of X rounds | 2–13 (labels 3–25 = 2N−1) | **13** | `endhandler.sqf:29` |
| `TimeLimit` | Round time limit (min) | 1, 5–20 | **10** | `preinit2.sqf:16` (×60), `roundserver.sqf:523` |
| `LastPlayersCountdown` | Last-players countdown (s) | 90–300 step 30 | **120** | `roundserver.sqf:285`, `:614–619` |
| `SetupTime` | Planning time limit (s) | 20–240, `−1` = unlimited | **30** | `roundserver.sqf:215` |
| `FirstRoundSetupTime` | First-round planning (s) | `−2` = double, 40–480, `−1` = unlimited | **60** | `roundserver.sqf:30` |
| `DefaultAdminPaused` | Automatic admin pause | 0 Never / 1 First round / 2 Always | **0** | `roundserver.sqf:34–37`, `:207–211` |
| `AFKKillTime` | AFK kill time (s) | 30–3600 | **120** | `afkkiller.sqf:7`, `:29` |
| `nameLength` | Marker name length | 0 = none, 1–10, 15, 20 | **10** | consumed only by `unitmarkers.sqf` — **which is dead** |
| `OvercastParam` | Clouds | 0–10 → 0.0–1.0 | **3** | `weather.sqf:4`, `:19` |
| `FogParam` | Fog | 0–10 → 0.0–1.0 | **0** | `weather.sqf:3`, `:18` |
| `minDist` | Min objective distance from spawns (m) | 250–2000 | `DEFAULT_MINDIST` = **1000** | `roundserver.sqf:165–167`, `fnc_startpos.sqf:11–12`, `flagmenu.sqf:16` |
| `capRad` | Capture radius (m) | 5–100 (**malformed, see below**) | **25** | `roundserver.sqf:49–50`, `roundclient.sqf:43` |
| `trainingRound` | Training round (title **not** localised) | No / Yes | **0** | `roundserver.sqf:33`, `:569–574` |
| `phx_loadout_blufor_uniform` | BLUFOR uniform | 19 `UNIFORM_*` | `UNIFORM_MARPAT_WD` (WWII `UNIFORM_USA`) | `fnc_assigngear.sqf:22` |
| `phx_loadout_opfor_uniform` | OPFOR uniform | 19 `UNIFORM_*` | `UNIFORM_EMR_SUMMER` (WWII `UNIFORM_GERMANY`) | `fnc_assigngear.sqf:18` |
| `phx_loadout_blufor_weapons` | BLUFOR weapons (R, LMG, HMG, AT) | 12 `WEAPONS_*` | `WEAPONS_M4A1_BLOCK_M249_M240G_M136_GUST` | `fnc_assigngear.sqf:23` |
| `phx_loadout_opfor_weapons` | OPFOR weapons (R, LMG, HMG, AT) | 12 `WEAPONS_*` | `WEAPONS_AK74M_PKM_PKP_RPG7_RPG32` | `fnc_assigngear.sqf:19` |
| ~~`DefenderGearQuality`~~, ~~`AttackerFactionParam`~~, ~~`DefenderFactionParam`~~ | — | — | commented out at `:182–189`, `:205–228` | — |

**Two authored defects in the param data — both would be caught by a schema-aware editor:**

* `description.ext:235` — **missing comma**: `values[] = {5, …, 13, 14 15, 17, …}`. `values[]` has
  25 entries, `texts[]` has 26. From that index on, the displayed capture radius does not match the
  applied one.
* `description.ext:171–172` — `AFKKillTime`'s 5th label is `"60"` where the value is `70`.

The commented-out faction params' stringtable keys (`STR_IDF`, `STR_HamasIDF`,
`STR_TerroristsHLCAK`, …) still live in `stringtable.xml` — fossils of the 2013 Arma-Israel
original.

**29 call sites reference params that are never declared**, so they always fall back to their
default: `phx_loadout_{radio,map,gps,watch,compass,modSet,nightvision}`,
`phx_loadout_indfor_{uniform,weapons}`, `phx_loadout_{blufor,opfor,indfor,civ}_lr_radio`
(`fnc_assigngear.sqf:29–33`, `fn_loadout_set.sqf:12–16`, `:44`, `:64–65`,
`uniformloadouts/longRadio.sqf:4,10,16,22`, `weaponloadouts/opforLoadout.sqf:13`).
**INFERRED:** DTAS imported FNF's `f/loadout` module without importing that module's own param
declarations, so all long-range radios and nav gear silently resolve to absent. `init.sqf:11–12`
separately hardcodes `s_loadout_radio = 0; s_loadout_map = 0;`.

### Per-terrain knobs — three constants, two files

```c
// islandspecific.hpp        (#include'd by description.ext:1 and init.sqf:1)
#define DEFAULT_MINDIST 1000
#define DESERTCAMO true      // defined but never referenced anywhere
```
```sqf
// islandspecific.sqf        (init.sqf:4, under the comment "//No clue what this island specific shit is supposed to do")
deleteRadius = 2000;   // radius around which to delete objects at end of round
```

**That is the entire per-terrain surface.** Porting DTAS to a new island = new `mission.sqm`
(two safe zones + 124 slots + markers) plus three constants.

### Hard-coded script constants (`init.sqf:9–14`)

`minCapTime = 20; maxCapTime = 60; s_loadout_radio = 0; s_loadout_map = 0;
minDistFactors = [1, 0.7, 0.55, 1];`

### CBA / ACE settings — `cba_settings.sqf`

**353 assignments, 271 of them `force`d** (server-locked; 82 client-overridable), across 39
commented section headers spanning `ace_*` (285), TFAR (48) and Enhanced Movement Rework (20).
Loaded because `description.ext:285` sets `cba_settings_hasSettingsFile = 1`.
**Byte-identical between the two variants.**

### Runtime admin controls (no file editing)

On the base "yellow box" (`flagmenu.sqf:102–105`), gated by `serverCommandAvailable '#kick'`
(`functions.sqf:98–101`): Force Round Start, Pause / Unpause Round Start, and **Relocate Objective**
via map click (`adminObjPos` public variable → `roundserver.sqf:83–97`).

## 2.5 What DTAS reveals that the framework repos do not

1. **FNF has two authoring models, and they are opposites.** The framework model is *author the
   ORBAT in Eden* — hence the `role@callsign` convention and Analyzer rules R3–R5. DTAS's model is
   **author almost nothing; let the round script build the mission**: 124 identical soldier slots,
   5 markers, two prefab bases, and 134 `.sqf` files. An editor that only supports the first model
   cannot express the second.

2. **A "mission" can be a game *mode*, and its authoring surface is then parameters, not entities.**
   Everything a DTAS operator tunes is a lobby parameter (§2.4). This argues for a first-class
   **parameter-definition surface** in TBD's document model — name, title (i18n key), value list,
   display list, default — separate from placed content. It maps almost 1:1 onto Arma's
   `class params`, and it is exactly where two of DTAS's shipped bugs live.

3. **Naming-convention-as-API is the one thing that already works like an editor.** `mrkZone*`
   (§2.1) lets a maker reshape the whole objective distribution by drawing rectangles. This is the
   pattern worth generalising: **typed, named regions the runtime discovers**, rather than
   hard-coded positions. TBD already has zones and markers; the missing piece is a declared
   *contract* (`this zone is a "objective-spawn-region"`) instead of a string prefix.

4. **Variants are whole-tree copies, and they have already drifted.** 159 identical filenames,
   18 files differing, of which **only 4 are intentional** (§2.1). Improvements landed in the base
   variant — weighted vehicle table, road-graph placement, the Panzerfaust `isNil` guards — never
   reached WWII. There is no inheritance layer. This is the strongest evidence in either repo for
   TBD's **base + sparse delta** approach (`T-110`).

5. **The dominant authoring gesture is comment/uncomment, not configure.** Six top-level scripts
   ship in both variants and are wired in only one, purely by `//`-toggling `addAction` lines
   (`flagmenu.sqf:86`, `:92–99`). `spawnprotection.sqf` is disabled by a `//execVM` at
   `init.sqf:63`. Class-list differences are whole blocks deleted rather than flagged. An
   editor-based model represents these as booleans; here they are diffs — invisible to any
   validator, and the reason **17.8 % of the SQF (1,736 of 9,754 lines) is dead**.

6. **House naming rules are not universal, and a rule engine must know that.** `FNF_DTAS_Altis`
   **fails the MissionAnalyzer's own mission-name regex R1** (verified in §1.3's test table). The
   convention is real for weekly submitted missions and simply does not apply to shipped game
   modes. Any validation TBD builds needs a notion of *which rules apply to which kind of mission*.

7. **Localisation is near-total, including parameter labels** — 222 stringtable keys, essentially
   zero literal player-facing strings in SQF, and `title = $STR_BestOfX;` on params. The two
   exceptions (`trainingRound`'s `"Training Round"` and the four `"BLUFOR Uniform:"`-style loadout
   titles) are later additions that broke the convention. **Param titles should be i18n-key-typed
   in an editor model**, not free strings.

8. **The code is honest about being inherited, and that is what file-based authoring costs.**
   `init.sqf` carries `//No clue what this island specific shit is supposed to do` (`:3`),
   `// The variable this is waiting on is launched by a module...why is it done this way? I have
   no idea.` (`:16`), and `//Set the weather ... don't even know if this works` (`:65`). Four
   maintainers over eight years, and the current one cannot tell what parts of his own mission do.

9. **Versioning is hand-written prose and is already stale.** `changelog.md` has two entries
   (`v5.1` 2021-06-08, `v5.0` 2021-04-14), keyed to player-visible effect rather than files or
   commits, at the repo root for both variants. `.gitignore` is `*.bak` + `*.pbo` — the PBO is
   built locally, there is no CI, no build script, and (unlike
   `FNF-v3.6.9/FNF_MissionTemplate.VR/version.txt`) no version file. The newest changelog entry
   **predates HEAD** (`975c983`, 2021-10-15, "updates to vic spawning, player placement,
   simEnabled"), so the shipped code contains unchangelogged work.
   **INFERRED:** the changelog is written at release, not at commit.

10. **Event content is baked into mission config.** `ads/controls.hpp:19` hardcodes
    `"FNF Titans v3 … A 7v7 Team vs Team Tournament!"` with a live Google Docs URL, plus a 357 KB
    `adboard.paa`. A tournament advert is compiled into the mission PBO — which means re-releasing
    the mission to change a poster.

# Part 3 — What TBD's Mission Creator should take from this

## 3.1 The single structural insight

**FNF's validator exists because Eden cannot say no.** Every rule in §1.3 is a *post-hoc assertion
about a file* that a live editor could have made *impossible to violate*. Sort the 27 checks by
what an editor does with them and you get three piles, not one:

| Pile | Count | What the editor does |
|---|---:|---|
| **Ceases to exist** — artefacts of file-on-disk authoring | 4 (P1, P2, P4, P5) | Nothing. Owning the document deletes the failure mode. |
| **Should be unrepresentable** — the editor generates the value instead of checking it | 2 (R1 name, R2 lobby line) | Compose the string from structured fields. A malformed name/description becomes unreachable, not "caught". |
| **Genuine live validation** — real predicates over mission structure | 21 (P3, R3–R9, D1–D13) | Continuous evaluation against the document model, surfaced in an always-visible issue panel. Note D11 is an *aggregation* and D12/D13 are *listings* rather than pass/fail rules — they still belong in the editor, as the issue panel and an asset-resolution check. |

The second pile is the important one, and the Analyzer proves the point against itself: its own
fix-it examples for R1 and R2 do not satisfy R1 and R2 (§1.3). **If the tool that owns the rule
cannot write a conforming example by hand, no mission maker will either.** Generate, don't validate.

## 3.2 The four validation primitives that cover the entire rule set

Every live rule in §1.3 reduces to one of four shapes. Build these four and you have implemented
FNF's whole checklist, plus room for OFCRA's and WOG's:

| # | Primitive | Rules it covers | TBD implementation note |
|---|---|---|---|
| **V1** | **Required-entity presence** — "an entity named/typed X must exist" | R6, D1, D2 | The largest single win. Should be **conditional on game mode** so the tool never emits the "you can ignore these" disclaimer FNF ships (`AnalyzeSQM.ps1:1886–1889`). |
| **V2** | **Cardinality** — "there must be ≥ N of type X" | R7, R8, D3–D10 | Trivially incremental: recount on add/remove of one type. |
| **V3** | **Per-object invariant** — "every object of type X must satisfy P" | R9 (empty inventory) | Fires on edit of the object, not on a document sweep. |
| **V4** | **Field-shape / cross-field** — "field F must match/derive from G" | R1, R2, R3, R4, R5 | Prefer *derivation* over *validation* wherever the value has structure. |

## 3.3 Build order — the ten highest-value checks, in order

Ordered by (damage prevented) × (cheapness given what TBD already ships). TBD's existing
`T-180` ORBAT model, `T-068.2` registry and `T-068.15` cargo work already hold the data for
most of these.

| Rank | Check | FNF origin | Why it is first / why here |
|---:|---|---|---|
| 1 | **Required framework entities exist, conditional on game mode** | R6 + D1 + D2 (revived) | The failure it prevents is total: the mission loads and the game mode silently does not run. FNF's version is half dead (D1, D2) and mode-blind. This is where a live editor most obviously beats a validator. |
| 2 | **Persistent issue panel with a rollup** | D11 (revived) | Not a rule — the *delivery mechanism*. FNF commented theirs out and left makers hunting for orange accordions. Without this, every other check below is invisible. Cheap, and it is the thing that makes validation feel live. |
| 3 | **Every ORBAT slot has a resolved role and callsign** | R3 + R4 + R5 unified | FNF needs three overlapping rules and a hardcoded name list because Eden packs role and callsign into one `description` string. TBD models them separately (`T-180`), so this collapses to "no slot has an empty/default identity field" — and it is the only rule FNF rated **error**. |
| 4 | **Spectator / admin slot cardinality** | R7, R8 | Two-line rules that prevent an un-adminnable, un-spectatable session. Pure V2. |
| 5 | **Vehicle/container cargo matches policy** | R9 | PvP fairness. TBD already has the cargo model (`T-068.15.1`), so this is a predicate over data it holds, not a string compare against a serialised sentinel. |
| 6 | **Every placed asset resolves in the registry** | D13 (revived) | FNF's is dead code; the concept is sound and TBD is better positioned — `GET /api/v1/registry` with ETag caching means the catalogue is already resident, so unknown-classname detection is **live**, not post-hoc. Catches modset drift the moment an asset is placed. |
| 7 | **Mission name is generated, not typed** | R1 | Compose from author + title + mode + version + region. Deletes the rule. |
| 8 | **Lobby/briefing summary line is generated** | R2 | Same. Compose from game mode + attacker/defender + per-side asset summary — all of which the ORBAT and placement model already know. This also removes the drift that made FNF's own examples invalid. |
| 9 | **Per-squad role coverage** | D3–D8 (revived) | "A vehicle squad must have a commander", "an AT team must have a designated leader". FNF wrote six of these and commented out all six. In TBD they are one rule parameterised by squad template. |
| 10 | **Mode-consistency: attacker/defender sides are set and distinct** | *not in FNF* — implied by R2's `ATK:`/`DEF:` grammar and `fnf_attackingSide`/`fnf_defendingSide` (`FNF-v3.6.9/FNF_MissionTemplate.VR/config.sqf:46–47`) | The Analyzer only checks that the *description string* mentions attacker and defender; nothing checks the actual config. A live editor should validate the real setting, which is the whole point. |

## 3.4 What DTAS adds to the list

The Analyzer's rules all concern *placed content*. DTAS shows that a second whole class of mission
error lives in **parameters and settings**, which the Analyzer never looks at — and DTAS ships two
of them:

| Check | Origin | Shape |
|---|---|---|
| **Parameter value/label arrays are the same length** | `description.ext:235` — `capRad`'s `values[]` has 25 entries, `texts[]` has 26, because of a missing comma between `14` and `15`. Displayed radius ≠ applied radius from that index on | Schema validation on the parameter definition. **Unrepresentable** if the editor edits value/label as a single row. |
| **Every parameter a script reads is declared** | 29 call sites in DTAS read `phx_loadout_*` params that `class params` never declares, so long-range radios and nav gear silently resolve to absent (§2.4) | Cross-reference: declared params vs referenced params. |
| **Declared settings agree across files** | `description.ext:6–8` says `minPlayers = 4`; `mission.sqm:135–137` says `minPlayers=2` | Cross-field consistency — trivial once one document owns both. |
| **Parameter labels are localisation keys, not literals** | 222 stringtable keys and near-total localisation, broken by 5 later params using literal titles (§2.5 #7) | Type the field as an i18n key. |
| **No dead references** | `nameLength` is a live, defaulted, lobby-visible parameter whose only consumer (`unitmarkers.sqf`, 292 lines) is unreferenced dead code | Reachability check — the same "can this rule fire?" discipline as §3.5. |

Two of these five are the *same class of defect* as the Analyzer's own dead rules (§1.3 D1/D2):
something is declared, looks live, and does nothing. That is the characteristic failure of
file-based authoring, and it is what a single owned document model eliminates.

**And three non-check recommendations DTAS makes on its own:**

* **Model mission parameters as first-class document objects** (name, i18n title, value list,
  display list, default, and the script symbol that consumes them). It maps 1:1 onto Arma's
  `class params` and onto DTAS's 17 tunables, and it is where two shipped bugs live.
* **Give zones a declared *purpose*, not a name prefix.** `mrkZone*` (§2.1) is the best authoring
  hook in either repo and it is implemented as `BIS_fnc_inString` over `allMapMarkers`. TBD already
  has zones; typing them (`objective-spawn-region`, `safe-zone`, `capture-ring`) makes the same
  affordance discoverable and checkable.
* **Base + sparse delta, not tree copies.** The WWII variant is 159 identical filenames with 18
  differing, of which only 4 are intentional (§2.1). Everything else is drift that nothing detects.
  This is direct field evidence for `T-110`.

## 3.5 Things to explicitly *not* copy

* **Do not mutate the document while validating it** (`AnalyzeSQM.ps1:330–332`, `:341–343`).
* **Do not hardcode the deployment shape into the tool** — the six-mission EU/NA playlist is
  baked into `AnalyzeSQM.ps1` (`:2262`, `:2176–2181`). That belongs in data.
* **Do not validate free text with a regex when you can generate the text.** §1.3, R1/R2.
* **Do not let a check fail open.** D1 and D2 — the objective-existence rules, the most important
  in the set — have been silently dead for five years because of a typo (`$MarkObjs`) and a bad
  truthiness guard (`$ReqCoreObjs.name`). Any rule engine TBD builds should have a test per rule
  that asserts the rule *can* fire.
* **Do not colour-code without a legend, and do not invert your conditions.** The
  role-description accordion can never render green (§1.3), so a maker who does the work perfectly
  still sees a warning colour. Warnings that fire on correct input train people to ignore warnings.
