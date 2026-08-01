# Framework synthesis — FNF · OFCRA · WOG → TBD-Reforger Mission Creator

**Written 2026-08-01.** Synthesis of the five analyses in [`frameworks/`](frameworks/) (~6,300 lines)
against the target: TBD-Reforger's web-based 2D mission editor for **Arma Reforger**
(Rust/Leptos, wgpu, `yrs` document core, compiler emitting mission JSON to an Enfusion mod).

**All three communities are Arma 3.** Nothing here is a port. Every recommendation is
*"this authoring model teaches X"*, never *"copy this code"*.

## How to read the citations

| Form | Means |
|---|---|
| `wog.md §11` | A section of one of the five analyses. This is the unit of citation. |
| `fn_addZone.sqf:25-33` | An underlying source line, quoted **only** where the exact text is load-bearing. Always reached via the analysis that read it. |
| **`INFERRED:`** | The analysis labelled this as inference, not evidence. Carried forward, never laundered. |
| `editor_ops.rs:2275-2731` | Verified directly against TBD's tree during this synthesis. |

**Evidence weighting, per [`frameworks/README.md`](frameworks/README.md).** `fnf_v3.md` carries
**zero** `INFERRED:` markers across 1,198 lines (vs `wog.md` 17, `ofcra_omtk.md` 8, `fnf_v4.md` 5,
`fnf_tooling.md` 3) and its author spawned three sub-agents at ~2× any other's token spend. Where
v3 and v4 conflict, **v4 wins**: it owns the delta by design, had `git` access to both trees, and
labels its inferences. One such conflict is adjudicated explicitly in §A.7 and again in §D.1.

**Corpus caveat, per `README.md`.** Of `wog.md`'s 171 missions, **78 are WOG-native, 33 are imported
OFCRA missions, 60 are third-party**. Any corpus-wide statistic below is over a mixed population
unless it says "WOG-native". The 33 OFCRA missions are an independent cross-check on
`ofcra_omtk.md`, and they are used as one in §A.2.

---

# Executive summary — the five decisions

### 1. Objectives become typed, placed, per-side entities with one uniform attribute spine

WOG and FNF v4 arrived at the same answer independently, from opposite starting points (`wog.md §7`,
`fnf_v4.md §7`). WOG's `WMT_Task_Point` is the best-evidenced objective design in the corpus —
**166 instances across 73 missions** — and its parameter set is richer than TBD's shipped
`zoneRules`. Take the missing parameters: attacker count, defender count, starting owner, advantage
percentage, lock, auto-lose, and **min/max height so a zone is a volume, not a disc**.
Take FNF v4's per-side framing — every objective reads differently to attacker and defender — but
collapse it into **one entity with two framings**, not v4's two-modules-that-share-a-target
(`fnf_v4.md §14.4`, whose own steal-list says the same at `fnf_v4.md` §"What a web editor should
steal" #6). Make `condition` an advanced, optional field: WOG's scripting hatch is `true` in
**165 of 166** real uses (`wog.md §15.5`).

### 2. Live validation is a ticket group in its own right, and every rule ships with a test that makes it fire

**Four of four frameworks have no CI, no linter, no schema validation and no test harness**
(`fnf_v3.md §11`, `fnf_v4.md §11`, `ofcra_omtk.md §11`, `wog.md §11.3`). The two artefacts that *do*
validate are **both broken in the same way, and neither community appears to know**: FNF's Analyzer
runs 14 of its 27 checks — the objective-existence rules D1/D2 have been dead for ~5 years behind a
typo (`$MarkObjs`) and a bad truthiness guard (`fnf_tooling.md §1.3 C`) — and WOG's Med/Eng tagger
has a JavaScript `/g` flag pasted into an SQF regex, so its strip branch is dead code and its append
branch duplicates on every save (`wog.md §14.1`). TBD has no in-editor validation at all today; its
only mission-level check is a **server-side** zone scan that returns HTTP 400 at save
(`apps/website/api/src/contract/validate.rs`) — which is precisely the post-hoc shape FNF's Analyzer
has. Move it forward, build the four primitives, and make every rule fail on demand.

### 3. Generate, don't validate

Four of four ship a *derived-summary-to-clipboard* step as the last authoring action: FNF v3's lobby
text generator reading actual placed vehicles inside each safe zone (`fnf_v3.md §3 step 12`), FNF
v4's **Generate Lobby Description** (`fnf_v4.md §3 step 13`), OFCRA's `table_forum.sqf` slot-list
JSON export (`ofcra_omtk.md §11.2`), WOG's slot counter that produced a **94%-consistent
community-wide filename convention out of a 10-line function** (`wog.md §13.2`). The Analyzer proves
the negative case against itself: its regexes for mission name and lobby line are the two rules
whose own fix-it examples **fail those same regexes** (`fnf_tooling.md §1.3`, verified test table).
If the tool that owns the rule cannot hand-write a conforming example, no maker will.

### 4. Role, squad, rank and traits stay separate typed fields — derive display strings, never store them

`Role@Group` is the strongest convergence in the corpus: **five artefacts, four communities**
(`fnf_v3.md §4` `Role@Callsign`, `fnf_v4.md §4` `Role@Group`, `ofcra_omtk.md §4` `Role@SquadName`,
`wog.md §4` `Role@Group`, plus the Analyzer's own split at `fnf_tooling.md §1.2`). Every one of them
diagnoses it identically — two structured fields crammed into Eden's single free-text
`description` because there is nowhere else to put them. **TBD already has this right**
(`editor_ops.rs:120-132`, `orbat_manager.rs:754-766`, `mission.schema.json#/$defs/slot`). The live
decision is the *second-order* one: every framework then string-mangled a **third** axis into the
same field — WOG appends ` | Med` / ` | Eng` and prefixes `"1: "`, FNF v4 pads group names with
trailing spaces to disambiguate sides (`init3DEN.sqf:154`). Derive those at render time. WOG's
tagger is the cautionary tale: it *mutates the stored string*, and that is exactly where its bug
lives.

### 5. Four things all of them do that TBD should deliberately not do

| Not this | Evidence |
|---|---|
| **A play-time arsenal** | 4/4 reject it. `fnf_v3.md §5` "There is no arsenal"; `fnf_v4.md §5` "no ACE Arsenal box in the FNF flow"; `ofcra_omtk.md §5.10` "no Virtual Arsenal, no crate GUI, no gear-selection screen"; `wog.md §5` "does not ship a runtime virtual arsenal". The arsenal is an **authoring** tool. TBD's already is — keep it that way. |
| **A ticket system** | 4/4 absent. `ofcra_omtk.md §8` "no ticket system of any kind"; `fnf_v4.md §8` "Not applicable"; `wog.md §8` "No tickets"; FNF v3 uses `BIS_fnc_respawnTickets` only as an abstract point store (`fnf_v3.md §7`). TBD's `settings.respawn` enum still offers `tickets`, read by nothing — delete it or mark it unimplemented. |
| **Semantic editor layers** | FNF v3 used Eden layers as the game-mode selector and `fnf_v3.md §13.1` recommends copying it "wholesale". **Its own authors deleted it**, and `fnf_v4.md §15.5.3` calls the replacement "the single largest expressive gain". Adjudicated in §D.1. |
| **Positional / derived identity** | FNF v4 sorts objective modules by X/Y/Z to number them — *move a module and its number silently changes* (`fnf_v4.md §14.4`). TBD has stable CRDT ids and a `slot.uid` alongside a derived `slot.id`. Never regress that. |

### The v3 → v4 arc — the honest verdict

**It validates TBD's storage and compile architecture and warns hard about TBD's diagnostics gap.**
v4 delivered a real capability that v3 could not express — many objectives, mixed types, per-side,
sequenced — and paid for it with 350 commits of stabilisation, a lost documentation surface, and a
class of failure that did not previously exist: *silent, invisible, untyped edges*. The 157 `Fixed:`
vs 78 `Added:` figure is real but **cannot carry the weight of proving the graph model expensive**
— there is no v3 control group, commit counts are not effort, and the taxonomy is self-assigned by
one author. What *does* carry weight is the seven specific defects clustered on the sync mechanism,
every one of which is an **editor-affordance** failure rather than a data-model failure. Full
treatment in **Part B**.

### What this synthesis does not cover

Runtime and social enforcement — OFCRA's lonewolf detector, radio-theft blocking, the referee
sanction ladder (`ofcra_omtk.md §12`), WOG's hard freeze (`wog.md §8`), FNF's 15 restriction scripts
(`fnf_v3.md §12`). These are mod/server territory. They are the richest material in the corpus and
**none of it belongs in an editor ticket**. Where a piece of it does surface in the editor — the
house-rules briefing section — it is called out in §A.6.

---

# Part A — the convergence and divergence map

Fourteen dimensions, in the analyses' own schema order. Each is: **where they agree** (convergence
across independent communities is the strongest evidence available), **where they diverge and
whether that is taste or error**, and **which mechanism wins**.

## A.1 Identity — where the framework/content seam sits

**Convergence.** All four separate framework code from authored content. That is the only
agreement; they put the seam in four different places.

| | Framework lives in | Authored content lives in | What it costs |
|---|---|---|---|
| FNF v3 | client mod (16 addons) + server mods; **all game logic in the mission folder** | `config.sqf` + `mission.sqm` + `mode_config/*.sqf` | **Three hand-maintained copies** of a 516-line settings file, already divergent in seven values (`fnf_v3.md §14`) |
| FNF v4 | `client_mod/fnf_eden` — **250 files, 32 MB, the entire authoring surface** | `mission.sqm` only, **binarized** | Un-diffable, un-reviewable content; a **1,604-line Python migrator** as the only escape hatch (`fnf_v4.md §15.6.2`) |
| OFCRA | **nothing** — the whole framework is unzipped into each mission folder | `init.sqf` + `description.ext` + `mission.sqm` | Every mission carries a private copy: no shared runtime, **no way to patch a framework bug in a shipped PBO** (`ofcra_omtk.md §1`, §14.5.2) |
| WOG | **everything** — always-loaded addons; there is no mission template at all | `mission.sqm` + a ~19-line `init.sqf` | The maker **cannot read the definition of the modules they configure**: `wmt_main` is absent from the corpus, `wog_main` is deliberately obfuscated with scrambled PBO filenames (`wog.md §14.3`) |

**Divergence: one of them is right?** No — all four are wrong in the same way. The failure is always
**at the seam**, and it is always a *version* failure: which framework version is this mission built
against, and can I still change it?

**Verdict.** TBD's seam is already the best-shaped one available: framework in a mod + server, content
in a versioned server-side document with an explicit `schemaVersion` (`"1.2"` at T-092.1). The
transferable requirement is not the seam, it is **the migration path**. FNF wrote `fnf_transform.py`
*after* it needed it, doing text surgery on 63 kit files. TBD's migration is a server-side batch over
rows in a database. Build the first document migration before the first schema break, not after.

## A.2 Mission layout — where configuration lives

**The strongest structural convergence in the corpus, and it has a direction.**

| Era | Mechanism | Evidence |
|---|---|---|
| Before | A **text config file** the maker edits | FNF v3: `config.sqf`, 24 variables, "All configuration changes should be made in config.sqf" (`fnf_v3.md §3 step 0`, §10.A). OFCRA: `init.sqf` globals + a 100-line `class Params` block (`ofcra_omtk.md §10.3`) |
| After | **Typed attributes on placed objects** | FNF v4: `Description.ext` is 18 lines with no mission-specific content; the maker never edits it (`fnf_v4.md §2`). WOG: **median `description.ext` for WOG-native missions is 0 lines**; 55 of 171 have none, 8 of the 25 that do are empty (`wog.md §15.3`) |

FNF crossed this line deliberately and documented the crossing. WOG arrived there and `wog.md §2`
labels the reason **`INFERRED:`** — "WOG moved mission configuration out of `description.ext` and
into Eden-placed modules, so there is nothing left for `description.ext` to hold." The inference is
well-supported by the measurement (median 0 vs 135 lines for non-WOG missions) but it is an
inference and is carried as one.

**The cross-check.** OFCRA is the one that stayed on text config, and the 33 OFCRA missions inside
WOG's corpus confirm it independently — they are the ones carrying the ~100-line `class Params`
block (`wog.md §15.1`). And `ofcra_omtk.md §14.5` ranks it as friction **#1**: "Authoring is
hand-edited SQF array literals with no validation… a README that has to warn about trailing commas"
(`score_board/README.md:79`, quoted verbatim: *"REMEMBER: no spaces between the lines and commas at
the end of every line EXCEPT the last one"*). A typo surfaces as an RPT line **during the match**.

**The divergence that matters, and it is not taste.** FNF v4 names its own regression at
`fnf_v4.md §15.6.3`: *"One `config.sqf` became 22 modules whose attributes are only visible when the
right module is selected. There is no 'show me every setting in this mission' view."* Safe-start
duration is *implicitly* the maximum `fnf_timeZoneIsDeleted` across all safe-zone modules — a global
with no global home.

**Verdict.** Typed fields win outright. **But the aggregation v3 gave for free must be rebuilt
explicitly.** TBD's Mission Settings dialog (`eden_chrome.rs:3771-3906`) currently holds terrain,
time, weather, the three `flow` durations and JIP — the mission-wide settings. The moment a setting
moves onto a placed entity (zone rules already have), the same scattering starts. Recommendation in
§C.2: a **read-only aggregated settings view with diff-from-default**, listing every authored
setting in the document regardless of which entity owns it.

## A.3 Authoring workflow

**Convergence 1 — nobody has a scaffolding tool.** `fnf_v3.md §3 step 0`: "There is no scaffolding
tool, no generator, no CLI." OFCRA's step 1 is "create an empty mission in Eden"
(`ofcra_omtk.md §3`). WOG has no template at all (`wog.md §1`). FNF v4's is "copy the template
folder" (`fnf_v4.md §3 step 1`). 4/4.

**Convergence 2 — the last authoring action is always *generate a summary from ground truth*.**
Detailed in Decision 3 above. 4/4, and it is the single most directly copyable workflow step in the
corpus.

**Convergence 3 — three of four ship in-editor bulk operations.** FNF v4's `init3DEN.sqf` +
two menu-bar tools (`fnf_v4.md §11`); WOG's six-entry `Tools ▸ WOG 3den Tools` menu plus two
context submenus (`wog.md §11.1`); FNF v3 has the in-game lobby generator and admin menu but nothing
in-editor. OFCRA has **no in-editor tooling whatsoever** — its authoring aids are five `.txt`
copy-paste recipe files the maker is told to delete before shipping (`ofcra_omtk.md §11.3`).

**Divergence — subtractive vs additive, and it is genre, not error.**

- **Subtractive**: FNF v3 ships 267 playable units, nine game modes pre-placed, and 28 in-map
  Comment objects saying which to delete ("BLUFOR, delete if not using", "Delete this group if your
  mission doesn't need a MAT team") — `fnf_v3.md §3 steps 3–6`. FNF v4 is subtractive at the *kit*
  level: drag one composition, get 77 fully-equipped slots, delete the groups you don't want
  (`fnf_v4.md §3 steps 5–6`).
- **Additive**: WOG and OFCRA start from an empty Eden scenario.

Both work. FNF's subtractive model buys a maker a working mission in minutes and costs the rule
*"DO NOT DELETE ANY OF THE OTHER TEMPLATE OBJECTIVE OBJECTS"* (`config.sqf:43-44`). The additive
model costs a blank page.

**Verdict.** Additive baseline (TBD's), plus **compositions/templates as the subtractive escape**
— which is FNF v4's #3 strength (`fnf_v4.md §13.3`) and WOG's 28 shipped Eden group presets
(`wog.md §4c`). Ranked in §C.3. Note that Eden's own **F2 tab is Compositions**, holding `Fire Team`
/ `Rifle Squad` templates — one of six top-level asset tabs
(`eden_screenshots/README.md`, batch 06 supersedes batch 05).

## A.4 Slotting / ORBAT — the headline convergence

**Five artefacts, four communities, one convention.**

| Analysis | Form | Parser | Consumer |
|---|---|---|---|
| `fnf_v3.md §4` | `"Squad Leader@Alpha 1"` | `fn_serverInit.sqf:197-199` | Org label in the Discord teamkill embed; SHQ-aux config lookup |
| `fnf_v4.md §4` | `"Platoon Command@Command HQ"` (leaders only) | `init3DEN.sqf:159-177` | **Three** consumers: export→`groupID`, live ORBAT tab, kit re-siding key |
| `ofcra_omtk.md §4` | `"Squad Leader@Alpha 1-1"` | `fn_rosterBriefing.sqf:83-89` | Roster tab; the `@` half **overrides** `groupID` |
| `wog.md §4` | `"1: Squad Leader@Team 1"` | `fn_onMissionSaveEH.sqf:3` | Slot list; the auto-tagger round-trips it |
| `fnf_tooling.md §1.2` | `"Vehicle Commander@Golf 1"` | `AnalyzeSQM.ps1:448-449` | Three of the Analyzer's nine live rules (R3/R4/R5) |

All four diagnose it identically. `fnf_tooling.md §1.2` states it plainly: *"a string-packed relation
because Eden has no field for it — exactly the kind of thing a purpose-built editor gets to model as
real structure (role ref + squad ref) instead of parsing back out of a free-text box"*, and its
reuse verdict marks the packing an **anti-pattern to avoid**.

**The second-order finding — the packed string wants to be six fields.** Each framework's *extension*
of the convention reveals an axis it had nowhere else to put:

| Axis | Who mangled it in | Where |
|---|---|---|
| Role **label** (display) | all four | before the `@` |
| Role **key** (machine) | FNF v3 kept it **separate** — `this setVariable ["fnfLoadout","SL"]`, a 29-entry vocabulary | `fnf_v3.md §4` points 3–4 |
| Group / callsign | all four | after the `@` |
| **Side** | FNF v4 pads group names with `""` / `" "` / `"  "` per side to force uniqueness | `fnf_v4.md §12.2`, `init3DEN.sqf:154` |
| **Index in group** | WOG prefixes `"1: "`, `"2: "` | `wog.md §4a` |
| **Traits** (medic / engineer) | WOG appends ` \| Med` / ` \| Eng` on save | `wog.md §4b` |
| Rank | OFCRA reads Eden's rank field — got it free | `ofcra_omtk.md §4` |

FNF v3 is the interesting one: it already knew the **display string and the machine role are
different things** and kept both. v4 collapsed them into the string alone and lost the ORBAT tree
entirely — a 974-line `CfgFNFORBAT.hpp` replaced by "the ORBAT *is* the set of placed groups"
(`fnf_v4.md §15.2d`).

**Divergence — where the ORBAT is declared.** FNF v3 declares the same tree **three times, in three
formats, kept in sync by hand** (`cfgFNFORBAT.hpp`, the runtime table in `fn_setGroupIDs.sqf`, and
`mission.sqm` itself) — `fnf_v3.md §4`. That is straightforwardly worse than v4's single source, and
v4's collapse to one source is right even though the ORBAT-tree metadata it lost was real.

**Verdict.** TBD already models role / squad / callsign / rank / tag as separate typed fields
(`SlotAttrs { id, x, y, z, rotation, stance, role, tag, squad }` at `editor_ops.rs:120-132`;
`OrbatSlotDetail { role, tag, callsign, rank, index, squad_id, … }` at `editor_ops.rs:1167-1179`;
`slot { faction, groupCallsign, role, kit, uid, … }` in `mission.schema.json`). **Keep it. Add the
role-key/role-label distinction if it is not already explicit, derive trait badges at render time,
and generate any packed string only at compile.**

**Scale target from the corpus** (`wog.md §15.4`): 19,627 slots over 171 missions — min 9, p25 37,
**median 137**, max 324; ~32 groups per mission, **~6 players per group**; side balance
West 52% / East 42% / Independent 6%. TBD's editor is already validated to ~367k entities, so slot
*count* is not the constraint — **slot list legibility at 137 rows across 32 groups** is.

## A.5 Loadouts / arsenal

**Convergence — 4/4: there is no player-facing arsenal.** Quoted in Decision 5. This is the clearest
"do not build" in the corpus.

**The middle path, and it is a real convergence too.** Two of four ship a **bounded, author-defined,
pre-start choice**, and both gate it to safe start:

- FNF v3's **Gear Selector** — an ACE self-interaction menu whose children are exactly the
  `weaponChoices[]` / `explosiveChoices[]` / `grenadeChoices[]` of the player's role class plus the
  optic tier the role earns. Available **only while safe start is running** (`fnf_v3.md §5`).
- FNF v4's **Selectors** — `fnf_module_selectorHost` (name + type: Item/Optic/Primary/Launcher/
  Handgun) + N `fnf_module_selectorOption`, each synced to the physical container holding that
  option's gear. Selection persists per player UID; *"once safe start has ended you will no longer be
  able to change your selection"* (`fnf_v4.md §5`).

Not a free arsenal — a **constrained menu the mission maker authors**. Worth recording as a future
capability; not ranked for this program.

**Divergence — four genuinely different loadout models.**

| Model | Mechanism | The good part | The bill |
|---|---|---|---|
| **FNF v3** two-axis config matrix | 93 uniform sets × 63 gear sets, 25 role subclasses each by inheritance, reset between files by a 228-line `#undef` header (`fnf_v3.md §5`) | Two string edits change a mission's entire visual and armament identity | Config-file only; no GUI; **39 of 63 documented pairs have no preview images and 5 image folders are orphaned** (`fnf_v3.md §14`) |
| **FNF v4** baked Eden inventories | 85 compositions, each ~12–15k lines, literal `class Inventory` per unit; **no runtime application at all** (`fnf_v4.md §5`) | One drag → 77 equipped slots | 32 MB of generated data with an **incomplete source** (4 of 7 kit workspaces deleted, survivors binarized); **20 of 85 kits switched off** while still shipping their data; ORBAT change required a 1,604-line Python migrator (`fnf_v4.md §14.9-14.11`) |
| **OFCRA** offline YAML compiler | `default` + per-role **deep merge**; `"8x 30#classname"` = 8 mags of 30 rounds; **explicit-null strips an inherited item** (used 579×); compiled into `mission.sqm`, zero runtime footprint (`ofcra_omtk.md §5.2`) | **Balance policy becomes queryable data** | The compiler is a 2016 Windows-only Ruby-in-Ocra binary with an SQM whitelist of `[12,51,52]` and default file paths that **do not exist** (`ofcra_omtk.md §5.1`, §5.5) |
| **WOG** arsenal round-trip | GUI arsenal → `WOG: EXPORT` → clipboard → `tpl/<faction>/<role>.sqf`; **`WOG: IMPORT` reads it back**; three export dialects (`wog.md §5`) | The only **round-trip** in the corpus | Clipboard as transport; loadouts are imperative SQF |

**Which wins.** **OFCRA's inheritance grammar + WOG's round-trip.** Reasoning, in order:

1. **v4's baked model is the worst on the evidence.** It produced the most tooling debt of any
   mechanism in the corpus and its own analysis lists three separate friction entries about it.
2. **OFCRA's inheritance is the only model that made balance legible.** `ofcra_omtk.md §13.2`:
   *"you can grep the corpus and prove 'no infantry NVGs anywhere', 'binoculars to eight roles',
   'one launcher and three mixed warheads', 'twelve magazines West, eight East, five to seven
   insurgent'."* §5.7 enumerates eleven such rules **enforced by data, not by script**. That is the
   killer property: the loadout corpus *is* the rulebook, and it is queryable.
3. **WOG's round-trip is the interaction the other three lack.** `wog.md §13.3`: *"Most frameworks
   give you export only."*

**The cost, stated honestly.** OFCRA's inheritance is also where its bugs live —
`ofcra_omtk.md §5.9`: six keys mis-indented to column 0 and silently ignored; five `medic` blocks
missing the `primary:` level so the intended rifle is dropped; five `sniper` blocks nesting
`headgear` under `backpack:`, discarding the hat **and** accidentally un-clearing the rucksack. Every
one of those is a *schema* error — four defect classes, summarised at `ofcra_omtk.md §14.1` as
*"~11 silent data bugs in the loadout corpus"*. **A typed editor makes every one of them
unrepresentable** — which is the argument for doing inheritance in the editor rather than in YAML,
not an argument against inheritance.

**Where TBD stands.** Loadouts are **per-slot inline** (`slot.loadout` = `{gear, cargo[]}`,
T-068.11); the faction library's "Apply Template" *populates* a slot and then **each slot owns its
copy** (`arsenal_rules.rs`, `arsenal.rs`). Export exists (`download_json("loadout-export.json")`,
`arsenal.rs:1066`); **import does not**. So TBD has half of WOG's round-trip and none of OFCRA's
inheritance — and "apply a template, then it diverges" is exactly the drift DTAS demonstrates
(§A.14).

## A.6 Briefing / intel

**Convergence — 3/4 generate the briefing, and the fourth's makers hand-rolled the same template 80+
times.**

| | Maker writes | Framework derives |
|---|---|---|
| FNF v3 | **four HTML strings** (Background / World Info / Notes / Rules) | ORBAT with live randomised TFAR frequencies; a per-side asset inventory down to individual turret magazine counts (23 KB of SQF); the full kit breakdown with photographs; weather and astronomy; mode rules; overtime conditions; **seven physical briefing tables spawned in a 50 m radius** around the one desk the maker placed (`fnf_v3.md §6`, §13.3) |
| FNF v4 | **four `EditMulti5` fields** (Notes / AO / Background / Mission Rules), each emitted only if non-empty | Mission details; per-side vehicle stat cards with turret-by-turret breakdown and a cargo icon grid; live ORBAT refreshed every 2 s; 8 hardcoded house-rules records (`fnf_v4.md §6`) |
| OFCRA | **nothing, by default** | Objective tasks, credits, donations, timings, uniform photos, the rulebook, Team Roster, per-squad loadout with weapon/magazine/uniform icons (`ofcra_omtk.md §6`) |
| WOG | **all of it, by hand in SQF** — the exception | nothing (`wog.md §6`) |

But WOG's makers converged on a fixed section vocabulary anyway, counted across all `briefing.sqf`:
`Задачи` (Tasks) **80**, `Вводная` (Situation) **55**, `Условности` (Conventions) **42**,
`Формы сторон` (Uniforms of the sides) **44 records**, `Легенда` (Backstory) 11
(`wog.md §6`). Eighty hand-written instances of the same template is convergence by another route.

**Two mechanisms worth taking specifically.**

1. **A uniform-recognition section with images — invented independently by two communities.**
   WOG: 44 records, paired with shipped images (`brf/uniform_b.paa`, `brf/uniform_r.paa`).
   OFCRA: `images/blue.jpg` / `red.jpg` / `green.jpg` are **placeholder silhouettes the maker is
   required to replace**, and it is load-bearing because uniform theft is a bannable rule
   (`ofcra_omtk.md §3 step 4`, §12.1). `wog.md §13.7`: *"a real milsim problem at scale, given a
   first-class slot in the house template."* This is not an obvious feature and two communities
   arrived at it.
2. **Briefing text hyperlinks to map markers** — WOG only, but pervasive:
   `<marker name='east_ifv'>БМД</marker>`, `<marker name='s_0'>Солнечном</marker>`; clicking the word
   centres the map (`wog.md §6`). In a web editor where the briefing and the map are the same
   application, this is nearly free. **TBD's schema already has the hook**:
   `briefing.markers[]` with a **closed 64-value icon vocabulary** in `mission.schema.json`.

**Divergence — per-side briefings.** WOG (`switch (side player)`) and OFCRA (side-filtered tasks,
own-side-only roster) have them. `fnf_v4.md §6` states the absence outright: *"Not found: there is no
per-side briefing text, no per-group orders field, no map-marker-based intel authoring, no image
attachment attribute, and no rich-text editor."* That is a gap in v4, not a design choice — 2/4 have
it and the third names its lack. **Per-side wins.** TBD's schema is already correct here:
`briefings` is an object keyed by faction, each `{situation, mission, execution, markers[]}`.

**The anti-pattern.** FNF v4's kit tab **samples up to three random live players** and waits until
>50% of a side's slots are filled before rendering (`fnf_v4.md §14.8`): *"Authors cannot preview it
and it differs run to run."* **Derive from authored data, never from runtime sampling.**

**House rules as a briefing section.** FNF v4 ships ~90 lines of community rules as 8 hardcoded diary
records in *every* mission (`fnf_v4.md §6`); OFCRA ships its whole rulebook as a `Rules` diary tab
(`ofcra_omtk.md §12.1`); WOG's makers wrote a `Условности` section 42 times by hand. 3/4.
**Platform-provided rules section + a per-mission extension field** is the right shape and it is
cheap.

## A.7 Objectives / game modes

**The biggest architectural divergence, and it resolves cleanly.**

| | Model | Expressiveness |
|---|---|---|
| FNF v3 | **Nine fixed modes, one per mission.** `fnf_gameMode = destroy;` selects one; the mode's objects are pre-placed in named Eden layers and the framework **deletes the layers you did not pick** (`fnf_v3.md §3 step 3`, §7). Objective counts hard-capped at 3; object names are fixed literals | One mode, ≤3 objectives |
| FNF v4 | **Seven composable objective modules**, any mix, any count, any side, optionally sequenced. No mode variable at all (`fnf_v4.md §7`) | Arbitrary |
| OFCRA | **One mode** (points at a wall-clock deadline) + an objective DSL as a nested SQF array literal `[points, side, type, label, params…]`; 6 documented types, plus `TRIGGER` and `ACTION_DISPUTEE` shipping as **empty `case` stubs** (`ofcra_omtk.md §7`) | Arbitrary, but hand-edited arrays |
| WOG | **Objectives are Eden modules with a uniform schema.** Every `WMT_Task_*` shares `_Condition / _Winner / _Message / _Notice / _Count` (`wog.md §7`, §13.1) | Arbitrary |

**Convergence 1 — v4 and WOG independently arrived at "an objective is a typed, placed, parameterised
entity with a uniform attribute spine."** Two communities, opposite starting points: FNF from a
scripted-mode ancestor it rewrote away, WOG apparently from scratch. `wog.md §13.1`: a
capture-and-hold mission needs **zero lines of script** — *"That is why the median WOG
`description.ext` is empty."*

**Convergence 2 — all four are attack/defend at heart.**

- FNF v3: the ATK/DEF vs NEUTRAL split is enforced in code — `fn_setupGame.sqf` `exitWith`s with a
  red on-screen error if `sideEmpty` is in the wrong slot (`fnf_v3.md §7`).
- FNF v4: every objective type carries an attack value **and** a defend value —
  `des`/`pro`, `cap`/`def`, `hck`/`def`, `elm`/`pro`, `stl`/`kep`, `esc`/`des` (`fnf_v4.md §7`).
- WOG: `fn_onMissionNewEH.sqf:20` puts *"attack fraction (side color, attack) vs defense fraction
  (side color, defense)"* **into the default overview text of every new mission** — the framework
  teaching its own genre in a form field (`wog.md §3 step 2`) — borne out by the 52/42 side split.
- OFCRA: the DSL's `side` field, with combined forms duplicating the objective per side, and
  contested capzones evaluated with `DIFF` against **both** rivals simultaneously
  (`ofcra_omtk.md §7`).

**The mechanism that wins: WOG's spine + FNF v4's per-side framing, merged into one entity.**

`WMT_Task_Point`'s parameter set, with the observed value distributions that prove real use
(166 instances / 73 missions, `wog.md §7`):

| Parameter | Observed | vs TBD's shipped `zoneRules` |
|---|---|---|
| `_CaptureCount` | `4`×128, `1`×16, `2`×6, `3`×5 | **missing** — TBD has only `contestable: bool` |
| `_DefCount` | `3`×129, `1`×17, `2`×8, `0`×5 | **missing** |
| `_Timer` | `60`×51, `30`×39, `120`×27, `90`×15 | ✅ `captureSeconds` |
| `_AdvantagePercent` | `2`×72, `0`×67, `4`×14, `3`×13 | **missing** |
| `_MinHeight` / `_MaxHeight` | `-5` **invariant ×166** / `30`×137, `15`×16, `20`×13 | **missing — zones are 2D in TBD** |
| `_Owner` (starting owner) | `1`×70, `0`×65, `2`×30 | **missing** |
| `_Lock`, `_AutoLose`, `_EasyCapture` | `1`×136 / `-1`×156 / `1`×150 | **missing** |
| `_Marker`, `_MarkerText`, `_Message` | free text | partial (`label`) |
| `_Condition` | **`true` in 165 of 166** | n/a |
| — | — | TBD **has and WOG lacks**: `neutralizeSeconds`, `onEmpty` (hold/decay) + `decayRate`, `pauseOnEnemy` vs `resetOnEnemy`, `requireHolderPresent` |

**Carry the uncertainty forward.** `wog.md §7` marks the *semantics* `INFERRED:` — "a zone is
captured when `CaptureCount` attackers are inside the marker's area between `MinHeight` and
`MaxHeight` for `Timer` seconds while fewer than `DefCount` defenders contest it." The **parameter
names and the observed values are hard evidence**; the reading of what they do is inference,
because `wmt_main` is absent from the corpus entirely. So: take the parameter *set* with confidence;
treat the exact semantics as a design starting point to be decided by TBD, not a specification.

The `MinHeight = -5` / `MaxHeight = 30` pair is the standout. Invariant `-5` across all 166
instances, `30` in 137 of them — `wog.md §7` reads it as deliberately **including basements and
excluding aircraft**. TBD already has a DEM and a Z axis (`terrainZ`, T-091.2), so a volume is close
to free.

**FNF v4's pairing insight, and why to implement it differently.** The design insight is real —
*every objective reads differently to each side*, and v4 generates genuinely different task text,
different titles, and separate "My Tasks" / "Ally Tasks" trees from it (`fnf_v4.md §7`). The
implementation is bad in four documented ways (`fnf_v4.md §14.4`, §15.6.5-6):
identity is positional (sorted by zero-padded X/Y/Z, so moving a module renumbers it); two modules
are "the same objective" only because they happen to sync to the same object or share a prefix
string; **nothing validates that both halves exist**; and the only tell is the lobby tool dividing
every objective count by 2 — *"Forget the second module and the lobby description's `/2` arithmetic
produces a fractional count."* `fnf_v4.md`'s own steal-list says it: **"Make it one object with
per-side framing."**

**The escape hatch loses.** WOG: `_Condition` is `true` in 165/166 `Task_Point`, 71/71
`CapturePoint`, 14/14 `Destroy`, 7/7 `Compose` (`wog.md §15.5`). FNF v4's *only* scripting hook,
`fnf_codeOnCompletion`, has **no Eden attribute at all** and must be typed as
`this setVariable [...]` into a module's Init field — one of eleven such undeclared variables
(`fnf_v4.md §10.13`, §14.2; the "settable only via Init field" reading is marked `INFERRED:`).
**Ship the declarative parameters first-class; put any condition/hook behind advanced disclosure.**

**And FNF v3's mode-as-layer?** Adjudicated in §D.1. Short version: the transferable idea is not
"layers select modes" — it is **conditional inclusion**, a subtree that compiles only when a variant
is selected. That generalises to OFCRA's run-mode parameters and DTAS's lobby-selected modes without
overloading layers.


## A.8 Respawn / tickets / medical

**The strongest single-value convergence in the corpus: one life, then spectate.**

| | Setting | Death path |
|---|---|---|
| FNF v3 | `respawn = 3; respawndelay = 99999; respawnTemplates[] = {};` plus the respawn button removed from the pause menu (`fnf_v3.md §8`) | ACE spectator after 3 s |
| FNF v4 | `respawn = 3; respawnButton = 0; respawndelay = 99999; respawnTemplates[] = {};` (`fnf_v4.md §8`) | Three death modes; `onelife` available, `reinsert` default |
| OFCRA | `respawn = "BASE"; respawnDelay = 999999; respawnOnStart = -1;` (`ofcra_omtk.md §8`) | BIS EG Spectator with every feature enabled |
| WOG | **77 of 78** WOG-native missions `respawn = 1` + a `Spectator` respawn template pushed by the addon (`wog.md §8`) | `INFERRED:` "die once, then spectate", corroborated by `ace_spectator_virtual` appearing 140× in 33 missions |

Three of four use the *same* sentinel idiom (99999 / 999999). And **no ticket system exists
anywhere**: `ofcra_omtk.md §8` "There is no ticket system of any kind"; `fnf_v4.md §8` "Not
applicable"; `wog.md §8` "No tickets"; FNF v3 uses `BIS_fnc_respawnTickets` only as an abstract
point store for two neutral modes, where the count only ever *increases* toward 100
(`fnf_v3.md §7`, §8).

**This validates a decision TBD already made independently.** `eden_chrome.rs:3916-3960`
(`SETTINGS_UNREAD_NOTE`) deliberately withholds respawn, spectator policy, night vision and
per-faction tickets from the Mission Settings dialog, on the stated grounds that *"TBD events are one
life"* and no mod script reads them. Four independent communities agree. **Corollary:** the
`settings.respawn` enum in `mission.schema.json` still offers `"tickets"`, which nothing implements
— that is precisely the dead-knob class the corpus is littered with (FNF v4's
`FNF_vehicleLoadouts_useDefault`, read by **nothing in the entire worktree**, `fnf_v4.md §5`;
OFCRA's `OMTK_SB_MISSION_DURATION_OVERRIDE`, which tests a local never assigned in that scope so the
server always ignores it, `ofcra_omtk.md §14.1`). Remove it or annotate it.

**Divergence — what replaces respawn.** FNF v4 has the richest answer and it is a genuinely designed
mechanism: **reinsert** — a squadmate fires `fnf_weap_reinsert_flare`, the projectile position 2 s
later becomes the insertion point, a helicopter fast-ropes in up to **4** dead squadmates in death
order; **one reinsert per squad ever**; calling it with nobody dead **burns it**; the window closes
N minutes after safe start; reinserted players receive the kit's "bare bones" loadout rather than
their original (`fnf_v4.md §8`). OFCRA's answer is orthogonal and also good: respawn is a **lobby
dropdown** — `no-respawn / 3 s / 30 s / 1 min / … / immortal` — so the same artefact is a training
server, a briefing room, a mod-check sandbox and the match (`ofcra_omtk.md §13.3`, §8).

**Medical — 4/4 take it away from the maker.** FNF v3 (~520 forced CBA settings, `fnf_v3.md §10.G`),
FNF v4 (**518** `force force` lines across 30 sections; *"A mission maker cannot change any medical
setting"*, `fnf_v4.md §8`, §10.10), WOG (a server-init function that **removes 25 named medical item
types and re-adds a fixed kit** to every player, with a medic's backpack cleared and refilled;
opt-out is one module checkbox, `wog.md §8`), OFCRA (delegates entirely to ACE and only reads its
state, `ofcra_omtk.md §8`).

**Verdict, with the counter-lesson attached.** Platform-layer standardisation is right — `wog.md
§13.6`: *"No mission can ship with a medic carrying two bandages."* But WOG also supplies the
warning that makes it actionable: **`wog3_no_auto_long_range_radio = true` appears in 74 of 171
missions — the single most-used WOG API in the entire corpus is the one that turns a WOG feature
off** (`wog.md §14.2`). Its own steal-list draws the conclusion: *"if 43% of missions disable your
default, the default is wrong — so make such toggles visible mission settings, not magic globals."*

**And a capability none of them had.** WOG could only discover that number because an outside analyst
machine-parsed 171 shipped PBOs. **TBD owns its mission corpus in a database.** It can instrument its
own defaults directly — "what fraction of missions override X" is a query. No Arma framework can do
this. Recommended in §C.3.

## A.9 Zones / areas / triggers

**Convergence 1 — triggers lose; named regions win.**

| | Trigger usage |
|---|---|
| WOG | **162 triggers across 171 missions — under 1 per mission**, against 8,628 markers (mean 50) and 2,396 layers. Zones are markers consumed by modules (`wog.md §9`) |
| FNF v4 | *"Not applicable in the FNF model. No FNF system reads Arma triggers; all area logic is `inPolygon` against marker-derived polygons evaluated client-side at 1 Hz."* (`fnf_v4.md §9`) |
| FNF v3 | Mixed — a named `zoneTrigger` `EmptyDetector` **or** a polygon of numbered markers; five distinct zone systems, all keyed on naming conventions the maker must obey exactly (`fnf_v3.md §9`) |
| OFCRA | Uses Eden triggers, referenced **by name string**, and `ofcra_omtk.md §9.3` flags `INFERRED:` that Arma's `inArea` treats a String as a **marker** name, not a trigger name — so the README's "name of the trigger" instruction is at best ambiguous, and two commented-out `BIS_fnc_inTrigger` alternatives survive in the source as evidence the authors wrestled with it |

2/4 abandoned triggers outright, 1 uses both, and the 1 that relies on them has a documented
ambiguity bug about it. **Zones are first-class geometry, not triggers.** TBD is already here.

**Convergence 2 — one polygon primitive, reused for every purpose.** `fnf_v4.md §9`: *"One zone
primitive: the marker polygon"* — play zone, safe zone, hiding zone, sector and steal/escort
drop-off are all the same object with a different **prefix string**. FNF v3 has the same idea in a
worse form: five regex-keyed systems (`^(west|east|guer)_safeZone_marker_\d+$`,
`^fnf_custom_zoneBoundary_1_marker_\d+$`, …) — `fnf_v3.md §9`.

`fnf_tooling.md §3.4` names the fix explicitly, from DTAS's `mrkZone*` convention: **"Give zones a
declared *purpose*, not a name prefix… typing them (`objective-spawn-region`, `safe-zone`,
`capture-ring`) makes the same affordance discoverable and checkable."**

**TBD already did this.** `mission.schema.json#/$defs/zone` carries `type` as a closed enum —
`spawn | objective_capture | objective_destroy | objective_hold_until | boundary | base_protection`
— and `eden_chrome.rs:3518-3535` reads those six values **out of the embedded schema at runtime**
rather than hardcoding them in the UI. Shapes are `circle {x,z,r}` or `polygon` (min 3 vertices);
there is no rectangle. Authoring lives in `editor_ops.rs:2275-2731` (`ZoneDraft`, `ZoneRow`). This is
the corpus's best zone idea, already shipped, and better than any of the four originals.

**What is still missing from it:** the volume (`MinHeight`/`MaxHeight`), the attacker/defender counts,
and the starting owner — §A.7.

**Convergence 3 — the play area is a first-class enforced zone in 3 of 4.** FNF v3: outside the area
and not in/under an aircraft, a 20-second countdown then the vehicle is neutralised and
`player setDamage 1` (`fnf_v3.md §9`). FNF v4: teleport back to last known in-zone position, with
**aircraft deliberately exempt** — the play-zone restriction group's air flag is `false`, which is
also why a player exiting an aircraft outside it is teleported back *possibly mid-air*
(`fnf_v4.md §9`). WOG: `WOG_IslandRestriction`, **34 instances in 34 missions**, with `_Timer`,
`_Height` and a Russian-named `_Otstup` ("margin") parameter (`wog.md §9`). **OFCRA has none at
all** — `ofcra_omtk.md §9`: *"There is no play-area / boundary system."* Its only spatial rule is a
temporary warm-up leash.

TBD's `zoneRules` already models this properly: `graceSeconds`, `warnEverySeconds`,
`penalty: none|warn|kill`. The **aircraft-exemption axis is the missing one**, and FNF v4 explains
exactly why it exists.

**The anti-pattern, named by FNF v4 about itself** (`fnf_v4.md §14.5`): *"Marker-polygon zones are
laborious and fragile. Every zone is N hand-placed markers that must be numbered contiguously from
1, ordered correctly (they become polygon vertices in placement order), and coloured to set the zone
colour. There is no in-editor preview of the resulting polygon, no vertex reordering, and no error
unless you fall below three."* The vertex-discovery method is worse than that description implies —
`fn_addZone.sqf:25-33` *attempts to create* `<prefix>1`, `<prefix>2`, … locally and treats a
creation failure as proof the vertex exists. And `fnf_module_stealObj` and `fnf_module_escortObj`
**share the default prefix `fnf_marker_steal_`**, so a mission with one of each silently collides.
Every one of those failures is deleted by click-to-place vertices, drag-to-reorder and live shading
— which TBD has.

## A.10 Configuration surface

Largely covered by §A.2. One distinct axis remains: **run-mode parameters chosen at launch, not
baked at authoring time.**

| | Parameter count | Role |
|---|---|---|
| OFCRA | **29 `class Params` entries, 4 of them visual separators → 25 real knobs**, grouped into four lobby bands (`ofcra_omtk.md §10.1`) | The mechanism behind "one artefact, many run modes" |
| DTAS | **17 active params, 3 commented out** (`fnf_tooling.md §2.4`) | Everything an operator tunes |
| FNF v3 | **4**, and they are chosen by the admin at mission start, not by the maker (`fnf_v3.md §10.E`) | Marginal |
| FNF v4 | **exactly one** — `PerformanceTweaks` (`fnf_v4.md §10.8`) | Vestigial |
| WOG | `class Params` is one of only **three** things WOG-native missions still use `description.ext` for (`wog.md §15.3`) | Rare |

**Divergence is genre, not error.** OFCRA runs one mission file as training / briefing / match, which
*requires* launch-time switches. FNF and WOG ship one-shot rounds.

`fnf_tooling.md §2.5 #2` and §3.4 make the case for modelling parameters as first-class document
objects (name, i18n-key title, value list, display list, default, consuming symbol). The
supporting evidence for *urgency*, though, is thin and self-defeating: **two of DTAS's shipped bugs
live in the parameter arrays** — a missing comma between `14` and `15` leaves `values[]` with 25
entries against `texts[]`'s 26, so the displayed capture radius stops matching the applied one from
that index on; and `AFKKillTime`'s 5th label reads `"60"` where the value is `70`
(`fnf_tooling.md §2.4`). Plus **29 call sites read `phx_loadout_*` params that `class params` never
declares**, so long-range radios and nav gear silently resolve to absent.

**Verdict.** A parameter concept is correct but **ranks low for TBD**: the heavy-use evidence is two
artefacts from one community (OFCRA authored `omtk`; DTAS is 2013 third-party code FNF adopted), and
TBD's genre is FNF/WOG's one-shot round, not OFCRA's multi-mode artefact. The genuinely good half —
**"one artefact, many run modes, selected at launch"** — is worth keeping in view for a training/
rehearsal mode later. The parameter *bugs* are an argument for the editor owning value/label pairing,
not for prioritising the feature.

## A.11 Tooling

**Convergence 1 — unanimous, and it is the corpus's defining absence.**

> `fnf_v3.md §11`: *"CI / validation / linting — **there is none**."* No workflows, no Makefile,
> no `*.yml`, no `*.sh`, no HEMTT config, no SQF linter config, no test harness, no `.pbo`.
> `fnf_v4.md §11`: *"No build pipeline in-tree."*
> `ofcra_omtk.md §11`: *"There is **no CI, no test suite, no linter, no schema and no validator**."*
> `wog.md §11.3`: *"No mission validator CLI, no build/packaging script, no linter, no test harness,
> no mission template generator, no web tooling, no schema files."*

4/4. Correctness rests on human vetting, formalised at most as a GitHub issue template
(`testing_required.md`, labels `dedi testing, local testing` — present in both FNF eras).

**Convergence 2 — the two artefacts that *do* validate are both broken, in the same shape, and
neither community knows.** This is the README's own cross-analysis finding and it is the most
important single fact in the corpus:

- **FNF-MissionAnalyzer**: 27 checks in source, **14 execute** (5 hard gates + 9 validations); 13
  are dead. `D1` (required core markers) is guarded by `if ($MarkObjs.name)` where `$MarkObjs` is a
  **typo** for `$MarkerObjs` and appears exactly once in the whole file, so it is `$null` and the
  loop never runs. `D2` (required core objects — the terminals, the destroy objectives, the
  flagpole) is guarded by `if ($ReqCoreObjs.name)` where `$ReqCoreObjs` is an **array of arrays**,
  which has no `.name`, so the guard is always false. `fnf_tooling.md §1.3 C`: *"the tool no longer
  checks that objectives exist at all… a five-year-old silent failure in a tool people trusted."*
- **WOG's Med/Eng auto-tagger**: `regexMatch ".*\| (Med|Eng).*/g"` — a JavaScript regex flag pasted
  into an SQF pattern string, where `/g` is **literal**. The pattern requires the description to end
  with the characters `/g`, so it never matches: the strip branch is unreachable dead code and the
  append branch has no idempotence guard, so it duplicates on every save. Corroborated by the
  corpus containing **zero** instances of its output across 171 missions (`wog.md §14.1`).

**Both are the same defect: a tool reports success over an input it never actually examined.**
The `README.md` names this as this repo's own recurring shape, and draws the only defensible
conclusion: *"a reason to build validation that can be **made to fail on demand**, not a reason to
copy either implementation."*

**Convergence 3 — 4/4 deliver tool output to the clipboard.** WOG (every 3den tool —
`wog.md §11.1`: *"There is not one tool that creates content"*), FNF v3's lobby generator, FNF v4's
Generate Lobby Description (clipboard **and** the `IntelOverviewText` attribute), OFCRA's
`table_forum.sqf` JSON. `wog.md §14.6` names the cost: *"a validator whose findings vanish if you
copy anything else."*

**Convergence 4 — the best tools are unreachable.** WOG's two mission validators — `fn_check_weapon`
(magazine minima, missing vest/uniform, wrong-side radios) and `fn_check_lr` (leader long-range
radio present, correct side, no duplicates) — have **no menu entry at all**; their usage is a
commented first line telling you to type a call into the debug console, and `fn_check_weapon`
requires four positional arguments with no defaults (`wog.md §14.5`). FNF's Analyzer commented out
its own **"Issues to Fix" rollup** inside an HTML comment, so a maker gets **no summary of failures**
and must expand each accordion hunting for orange headers (`fnf_tooling.md §1.3 D11`) —
*"The one aggregation the tool had, it disabled."*

**The bonus defect worth quoting because it is a UX lesson, not a bug.** The Analyzer's
role-description accordion **can never render green**: the header condition requires all four lists
non-empty, including the two *unlabelled* lists, so a mission where every callsign was done
correctly has zero unlabelled units and is therefore marked orange. *"Doing the job perfectly is
indistinguishable from doing it badly"* (`fnf_tooling.md §1.3`). **Warnings that fire on correct
input train people to ignore warnings.**

**The asset database.** `fnfCfgExportDB.db` — 6.4 MB SQLite, `assets` 4,528 rows,
`cfgVehiclesEmpty` 9,225, `equipment` 3,083, `weapons` 1,665, `magazines` 2,252 — is the closest
thing in the corpus to TBD's registry, and `fnf_tooling.md §1.6` says so: *"exactly TBD's
`registry_items` shape (T-068.2)"*. Its classification rule is **DB membership itself** (in `assets`
→ manned asset; in `cfgVehiclesEmpty` → structure/prop; in neither → unknown). It is also a dirty
export: **every value carries a trailing space**, which is why every query literal appends one.
Concept validates; artefact discards.

**Verdict.** Three requirements fall straight out:
1. **The issue panel is the product**, not a secondary surface (D11 revived).
2. **Every rule ships with a test that asserts the rule can fire** (D1/D2 and the Med/Eng tagger).
3. **Findings live in a persistent panel; the clipboard is an export, not a delivery mechanism.**

## A.12 Conventions and house rules the framework encodes

**Convergence — all four encode balance policy in data or config, never in prose.**

| | Mechanism |
|---|---|
| FNF v3 | The **client mod** makes it un-overridable: all 94 patched vests get a byte-identical protection profile (`armor=15` ×282 = 94×3, no outliers) so *a plate carrier and a chest rig stop the same round*; backpack capacity flat at `maximumLoad = 1000`; coloured smoke shrunk to a signal puff while white smoke is enlarged; hand grenades bypass ACE fragmentation, overriding the server-wide force (`fnf_v3.md §12`) |
| FNF v4 | **Codegen**: `GenerateHatMod.sqf` / `GenerateVestMod.sqf` / `GenerateBackpackMod.sqf` enumerate every headgear/vest/backpack in the loaded config tree, walk `HitpointsProtectionInfo`, and regenerate an override mod — *"rather than trusting each source mod's armour values"* (`fnf_v4.md §11`) |
| OFCRA | **In the loadout data**: no infantry NVGs in any of 57 files (NVGs exist only in vehicles, tiered 50/15/10/8/3/2 by platform); binoculars explicitly null in 57/57 defaults and granted to 8 roles; long-range radios to 4 roles, with crew/pilot backpacks nulled in 57/57 so they *physically cannot* carry one; rifle magazine caps 12 West / 8 Russian regular / 5–7 insurgent (`ofcra_omtk.md §5.7`) |
| WOG | Forced medical kit; whisper-by-default (`TF_speak_volume_level = "whispering"`, 5 m); no auto-rearm (`transportAmmo = 0` on every supply vehicle, forcing ACE cargo logistics); no Tab-lock except AA (`wog.md §12`) |

**OFCRA's is the most transferable, because it is the only one that is *queryable*.** In TBD, with a
registry and a database, "prove no infantry gets NVGs" is a query, not a grep.

**Divergence — how much the maker may override.** FNF v3/v4 allow almost nothing (518 forced
settings, the fortify catalogue hardcoded in SQF, ~90 lines of house rules in the briefing —
`fnf_v4.md §14.7`). OFCRA gives the maker the loadouts entirely. WOG gives one opt-out checkbox.
TBD is a *platform*, so the right shape is **community-level policy that a mission can diff
against, with the diff visible** — the "instrument your own defaults" point from §A.8.

**The uncomfortable convergence, recorded and then excluded.** All four regulate *player* behaviour,
and two compile social rules into arithmetic. OFCRA's lonewolf detector is the exemplar: a published
"200 m infantry / 600 m vehicle" rule turned into three selectable tolerance bands
(210/230/250 m and 615/645/675 m), with correct exceptions for incapacitated (via ACE `isAwake`),
handcuffed, and airborne teammates, a 1500 m ignore band, a `CHEAT`-tagged RPT trail, and a referee
ladder from *warn* (red flash + weapon safety on for 11 s) through *disarm* and *reposition* to
*pause the entire match with a randomised 1–11 s staggered restart* (`ofcra_omtk.md §12.3`, §12.7).
`ofcra_omtk.md §13.1` is right that this is *"a design achievement, not a feature"*. **It is also
entirely runtime/mod territory and belongs in no editor ticket.** The one piece that does surface in
the editor is the house-rules briefing section — §A.6.

## A.13 What each does best — the shortlist

Condensed from each analysis's §13, filtered to what an editor can act on.

| Framework | The one thing | Take it? |
|---|---|---|
| **FNF v3** | *"The briefing is almost entirely derived, not written"* — four prose strings in, ORBAT with live frequencies + per-side asset inventory down to turret magazine counts + kit photos + weather + mode rules out (`fnf_v3.md §13.3`) | **Yes** — §C.3 |
| **FNF v3** | In-editor documentation placed next to the thing it documents — 28 `Comment` objects, including a seven-paragraph tutorial on the polygon boundary scheme sitting in the map (`fnf_v3.md §13.2`) | **Yes** — §D.2 |
| **FNF v3** | Automatic map marking: any object over 1.5 m outside a safe zone drawn as a correctly-oriented rectangle matching its real bounding box, with a one-checkbox opt-out (`fnf_v3.md §13.4`; same feature survives into v4, `fnf_v4.md §13.6`) | **Already have** — TBD renders real world geometry |
| **FNF v4** | *"The export hook that makes Eden a compiler"* — `init3DEN.sqf` derives data, works around a runtime limit, validates, then **reverses every mutation and re-saves** so the author's file is untouched, wrapped in one undo step, and self-heals on next open (`fnf_v4.md §13.1`) | **Yes, the pattern** — §B.4 |
| **FNF v4** | The kit library as a first-class migratable asset — 85 compositions, a stable unit key (`Squad_index_Role`) that survives re-siding, a bulk re-export driver and a schema migrator (`fnf_v4.md §13.3`) | **Yes, later** — §C.3 |
| **FNF v4** | A closed feedback loop: rate-the-mission and rate-the-commanding sliders + free text, uploaded to a sheet keyed by mission and author (`fnf_v4.md §13.4`) | **Platform, not editor** — TBD already has events/missions in a DB |
| **OFCRA** | Loadouts as a compiled artefact with inheritance and **explicit-null as the removal verb** (`ofcra_omtk.md §13.2`) | **Yes** — §C.2 |
| **OFCRA** | One mission file, many run modes, chosen in the lobby (`ofcra_omtk.md §13.3`) | **Later** — §A.10 |
| **WOG** | The editor counts your slots and the count goes in the name — a 10-line function producing a **94%-consistent** community convention (`wog.md §13.2`) | **Yes** — §C.3 |
| **WOG** | Objectives as configured modules with a uniform schema and a rich capture model (`wog.md §13.1`) | **Yes** — §A.7 |
| **WOG** | Round-trippable loadout export **and import** through the arsenal GUI (`wog.md §13.3`) | **Yes** — §C.2 |
| **WOG** | Briefings hyperlink to map markers; a conventional section for **recognising friendly uniforms** (`wog.md §13.7`) | **Yes** — §C.3 |

## A.14 Friction — the anti-pattern register

The failures worth designing against, each evidenced, grouped by what causes them.

**Dead knobs — something is declared, looks live, and does nothing.** The corpus's signature defect.

| Defect | Source |
|---|---|
| `Distance To Disable` writes `fnf_distanceToDisable`; the runtime reads `fnf_distanceForDisable` — a genuine name mismatch, so the value always falls back to the hardcoded 250 | `fnf_v4.md §9`, §14.3 |
| `FNF Properties ▸ Use Default Loadout` is **read by nothing in the entire worktree** — a case-insensitive grep returns only its two declaration lines | `fnf_v4.md §5`, §14.3 |
| Eleven runtime variables read but never declared as attributes, including *all* custom objective titles/descriptions and the only scripting hook | `fnf_v4.md §10.13` |
| `OMTK_SB_MISSION_DURATION_OVERRIDE` tests a local never assigned in that scope, so the server always ignores it — while the briefing *displays* the override to clients | `ofcra_omtk.md §14.1` |
| The documented `OMTK_ID` objective mode reads `mt_id`, not `OMTK_ID`, and its mode inversion is commented out, so it always behaves as DESTRUCTION | `ofcra_omtk.md §7`, §14.1 |
| `nameLength` is a live, defaulted, lobby-visible parameter whose only consumer is 292 lines of dead code | `fnf_tooling.md §2.4` |
| 5 placeable FNF modules missing from `CfgPatches >> units[]` | `fnf_v4.md §10.14` (effect marked `INFERRED:`) |

**Documentation that contradicts the code.** FNF v3's `config.sqf` opens by telling the maker to
rename `mission_normal.sqm` and `description_normal.ext` — **neither file exists**, they were
renamed to become the defaults three minor versions earlier. *"The first thing a new mission maker
reads is wrong"* (`fnf_v3.md §14`). OFCRA's README misstates most parameter ranges and defaults and
**omits eleven live parameters**; its `dynamic_startup/README.md` still documents a `launch_mode`
module that does not exist; all thirteen module data cards carry the same typo, `Ojective`
(`ofcra_omtk.md §14.3`).

**Silent naming traps.** FNF v3's assassin mode looks up HVTs with `str _object`, which returns
Eden's *Variable Name* field — a maker who instead writes `init="HVT_1 = this;"` gets `objNull`,
`!alive objNull` is true, and **every HVT is auto-killed the moment safe start ends**
(`fnf_v3.md §14`). Typed references make this unrepresentable.

**Typos baked into the wire format.** `fnf_v4.md §14.12`: `Breifing` as a directory name, a module
classname (`fnf_module_breifingAssets`) and ~12 variable names; plus `Handeler`, `Zues`,
`Resinserts`, `Vincible`. WOG has `WMT_Main_DisableBreifingMarkerMove` and
`WMT_Main_IndetifyTheBody`, *"serialized into all 79 module instances and every mission file, so they
cannot be fixed without migrating the corpus"* (`wog.md §14.4`). **Field names are a wire contract.**

**Destructive bulk operations with the safety on the wrong ops.** `wog.md §14.7`: WOG's
mission-wide `Clear all vehicle storage` and `Default ACE medic & engineer settings` both operate on
`all3DENEntities select 0` with **no confirmation and no `collect3DENHistory` wrapper — neither is
undoable** — while `Clear objects' init` and `Set health to max`, which act only on the *selection*,
both confirm and both are undoable. *"The confirmations are on the safe operations and absent from
the dangerous ones."*

**The framework fighting the maker.** `wog.md §14.8`: WOG's `OnMissionLoad` handler silently rewrites
**21 mission attributes every time the mission is opened**, including `Respawn`, `SaveBinarized` and
`EnableDebugConsole`. A maker who deliberately changes any of them loses the change on next open,
with no notification. And it does not even work: `RespawnTemplates` is set in three places and is
**unset in all 171 shipped SQMs** (`wog.md §14.9`).

**Diagnostics off by default.** `fnf_v4.md §14.9`: every `DANGER:` / `WARNING:` in the framework is
gated behind `fnf_debug`, which defaults to `false` — *"so the default authoring experience is
silent failure. The template's own reminder comment tells you to turn it off before exporting —
meaning the shipped state is the un-diagnosable one."*

**Variants as tree copies, with no inheritance layer.** DTAS ships two variants with **159 identical
filenames**; `diff -rq` reports **zero** "Only in" entries and **18 files differing, of which only 4
are the intended knob**. Everything else is unpropagated drift: the WWII variant's `roundserver.sqf`
is a structurally older generation of the same function, and it carries a divergent loadout file its
own default never invokes (`fnf_tooling.md §2.1`). `fnf_tooling.md §3.4` calls this *"the strongest
evidence in either repo for TBD's base + sparse delta approach (T-110)"*. **17.8% of DTAS's SQF is
dead — 1,736 of 9,754 lines** — largely because the dominant authoring gesture is comment/uncomment,
which no validator can see (`fnf_tooling.md §2.5 #5`).

**Validating free text you could have generated.** The Analyzer's R1/R2. Already covered; it belongs
here too because it is the most self-evident failure in the corpus: the tool shows a failing maker
two example strings, and **both examples fail the rule the maker just failed**
(`fnf_tooling.md §1.3`, verified test table).

**A validator that mutates its input.** `AnalyzeSQM.ps1:330-332` and `:341-343` rewrite
`mission.sqm` in place, twice, before analysing it. `fnf_tooling.md §1.1`: *"A validator that edits
the artefact it is validating is a hazard, and worth **not** copying."*

---

# Part B — the arc that matters: FNF v3 → v4

## B.1 The facts, first

All from `fnf_v4.md §15.0` and §15.7, which had `git` access to the full 1,635-commit clone.

- `v3.6.9` = `285e4441`, **2023-09-12**. `v4.0.0` = `68e6d38d`, **2023-12-16**, subject
  *"Manual Merge of 4.0.0"*, and its parent list is exactly `285e4441`. **There is one commit
  between the two tags.**
- `git show --stat v4.0.0` → **1504 files changed, 1,045,833 insertions, 85,891 deletions.**
- `git diff --stat v3.6.9 v4.7.0` → **1672 files, 1,438,844 insertions, 91,349 deletions**, over
  **351 commits**, 350 of them post-4.0.0.
- **Nothing in v3's mission template survived.** The whole `FNF_MissionTemplate.VR/` tree (421 files)
  is deleted; `FNF_Mission_Template.VR/` (155 files) is created, along with
  `client_mod/fnf_eden` (249 new files), `External Scripts/` (7) and `Kit Mission Files/` (3).

v4 was **developed off-repo and landed as a single squashed drop**. This is not an evolution with a
migration path; it is a replacement with a compatibility script (`ExportLoadoutFromOldFramework.sqf`,
~35 lines, which calls the *v3* loadout function and clipboards the result).

**The five mechanism swaps** (`fnf_v4.md §15.2`), which are the substance of the arc:

| # | v3 | v4 |
|---|---|---|
| a | Central SQF config file (`config.sqf`, 177 lines, ~21 live globals + 12 dead `//*NOT USED*` ones) | **62 live typed Eden attribute instances across 22 placeable modules** (46 distinct property names) |
| b | **Nine fixed game modes**, one per mission, configured by editing SQF in `mode_config/<mode>.sqf` | **Seven composable objective modules**, any mix, any count, any side, optionally sequenced |
| c | Config-class kits (`CfgFNFLoadouts >> UNIFORMS\|GEAR >> <KIT> >> <ROLE>`, 158 readable `.hpp` files) + a 33-file runtime loadout builder + `this setVariable ["fnfLoadout","SL"]` per slot | **85 Eden compositions with baked `class Inventory` blocks; no runtime loadout application exists** |
| d | **974-line `CfgFNFORBAT.hpp`** — the BIS ORBAT-viewer schema, hand-edited per mission, plus a dedicated `fn_setGroupIDs.sqf` runtime table | The ORBAT **is** the set of placed groups; the only metadata is `description="Role@Group"` |
| e | Systems bound to **hardcoded entity names** (`destroy_obj_1`, `term1`, `ctf_flagPole`, `zoneTrigger`, `west_safeZone_flag_1`) and fixed regex marker prefixes | **No reserved names anywhere.** Prefixes are per-module attributes; objectives bind to whatever you sync |

**The trajectory in one line:** SQF globals → typed Eden attributes → a typed graph with sync-link
edges.

## B.2 What 157 `Fixed:` vs 78 `Added:` does and does not prove

The commit histogram across `v4.0.0..v4.7.0` (351 commits) is: **157 `Fixed:`, 78 `Added:`,
39 `Changed:`, 26 `Version Bump`, 15 `Updated:`, 11 `Removed:`, 7 `Multiple Changes`**
(`fnf_v4.md §12.26`). `fnf_v4.md §15.7` concludes: *"two thirds of post-launch effort went into
stabilising the module/sync model rather than extending it."*

**That conclusion is directionally fair and it does not survive as proof. Four reasons:**

1. **There is no v3 control group.** v3's changelog is 272 lines of prose (`v3.1.0`→`3.6.8`), not a
   tagged commit taxonomy, and its discipline had lapsed — `version.txt` says `3.6.9` while the
   newest changelog entry is `3.6.8`, and *"the last three entries are variations on '1. Misc bug
   fixes'"* (`fnf_v3.md §14`). We cannot compute v3's ratio, so we cannot say v4's is worse.
2. **Commit counts are not effort.** A `Fixed:` commit can be one character. Meanwhile the single
   largest piece of work in the v4 era — the ORBAT flattening — lands as a `Changed:` and an
   `Updated:` totalling **+865,608 / −1,739,275 lines across 191 files**, i.e. two commits carrying
   more change than the other 349 combined.
3. **The taxonomy is self-assigned by one author.** Nearly every function header in the tree carries
   `Author: Mallen` (`fnf_v4.md §1`). It is a personal convention, not a reviewed classification.
4. **A rewrite's first two years are supposed to be `Fixed:`-heavy.** 350 of the 351 commits are
   post-launch stabilisation of a codebase that landed as one squashed drop with no incremental
   review. The ratio is at least as consistent with "it shipped un-reviewed" as with "the model is
   expensive".

**What the number *is* good for:** it is a corroborating signal, not a load-bearing one. Cite it as
"the post-launch commit taxonomy is 2:1 fixes to additions", and put the weight on §B.3.

## B.3 The seven defects, and the one thing they have in common

Each is individually evidenced, and all seven cluster on exactly one mechanism — the sync graph.

| # | Defect | Source |
|---|---|---|
| 1 | **A missing sync link silently disables an entire objective**, and the diagnostic is gated behind a Debug checkbox that ships **off** | `fnf_v4.md §15.6.4`, §14.9 |
| 2 | **Objective identity is positional.** Numbering derives from sorting module world positions (X/Y/Z zero-padded to 6 digits and concatenated). *Move a module on the map and its number changes.* | `fnf_v4.md §14.4`, §3 step 10 |
| 3 | **The paired attacker/defender convention is undocumented and unenforced.** Two modules are "the same objective" only because they sync to the same object or share a prefix string; the only tell is the lobby tool dividing by 2 | `fnf_v4.md §15.6.6` |
| 4 | **Three confirmed dead knobs**, all attribute-plumbing failures (§A.14) | `fnf_v4.md §14.3` |
| 5 | **Eleven runtime variables read but never declared** — including *all* custom objective text and `fnf_codeOnCompletion`, the only scripting hook in the entire objective system | `fnf_v4.md §10.13`, §14.2 |
| 6 | **Five placeable modules missing from `CfgPatches >> units[]`** | `fnf_v4.md §10.14` |
| 7 | **Sync links do not survive JIP**, and the workaround is invasive: the export pipeline creates one throwaway `Logic` per playable unit, moves the unit's sync connections onto it, rewrites the unit's init field, exports, then reverses all of it. *"The exported `mission.sqm` differs structurally from the saved one."* | `fnf_v4.md §14.14`, §3 step 14 |

**Every one of these is an editor-affordance failure, not a data-model failure.** FNF built a typed
graph on a host that gives you: no edge labels, no edge inspector, no "what is this connected to"
view, no validation surface, no stable identity, and a diagnostics channel that ships disabled.

`fnf_v4.md`'s own steal-list says it in one sentence: *"A web editor can do this **far better** than
Eden: named, validated, visible edges with an inspector that says 'this Destroy Objective is missing
a Side' instead of a `systemChat` line behind a debug flag."*

So the honest generalisation is not *"the graph model is expensive"*. It is:

> **A graph whose edges are invisible, untyped, unvalidated and un-diagnosable is expensive. The
> cost is in the affordances, and every one of them is purchasable.**

## B.4 What the arc validates in TBD's architecture

| Validated | Evidence | TBD's position |
|---|---|---|
| **Text/structured storage over an opaque binary** | `fnf_v4.md §15.6.2` is unambiguous: *"Everything painful about v4 authoring traces to a binarized `mission.sqm`: no diffing, no review, no scripted bulk edits, and a 1,604-line Python migrator as the escape hatch."* Its steal-list #8 is literally **"Text-first storage… A JSON mission document is already the right call — keep it"** | `yrs` CRDT + JSON compile. **Correct. Do not add an opaque intermediate.** |
| **Typed attributes over a central config file** | `fnf_v4.md §15.5.1`: *"You stop editing source."* And the same finding independently at WOG (`wog.md §10`, §15.3) | Mission Settings + `zoneRules` + `slot` are typed. **Correct.** |
| **Export as a compiler** | `fnf_v4.md §13.1`: *"No other Arma framework I've seen treats the editor as a compilation front-end this deliberately."* It derives data, works around a runtime limit, validates, and reverses every mutation so the author's file is untouched | TBD compiles `mission.schema.json` and serves `/missions/:id/compiled`. **Correct — and TBD's compile is non-destructive by construction, which is the property `init3DEN.sqf` spends 150 lines buying back.** |
| **Composable objectives over fixed modes** | `fnf_v4.md §15.5.3`: *"Missions can have more than one objective, of mixed types, with different owners, in sequence. v3 was strictly one mode per mission. **This is the single largest expressive gain.**"* | TBD's zone `type` enum + `winConditions.endOn` are already composable in the contract. **Correct.** |
| **Stable identity, derived display key** | v4's positional numbering (§B.3 #2) is the counter-example | `slot.uid` is *"the editor document's slot id, carried verbatim"* alongside a **derived** `slot.id` = `{faction}:{groupCallsign}:{role}:{index}`. **Exactly right. Never regress it.** |
| **Workflow-only layers** | v3's semantic layers were deleted by their own authors (§D.1) | `outliner.rs:44-50` — layers are workflow folders and do not reach the compiler. **Correct.** |

## B.5 What the arc warns against, specifically

1. **Do not ship a connection/sync feature before the inspector, the validation and the
   "what-is-this-attached-to" view.** `gap_analysis.md` currently lists `CONN-SYNC-001` as a bare
   `missing` row with **no ticket and no notes**. Building the edges before the diagnostics is
   precisely the v4 mistake, and it is the one gap-table row this synthesis would put a warning
   label on.
2. **Do not derive identity or ordering from position.** Free for TBD; the thing v4 got most
   obviously wrong.
3. **Do not gate diagnostics behind a flag.** v4's shipped state is the un-diagnosable one
   (`fnf_v4.md §14.9`). TBD's `Ctrl+Alt+D` debug HUD is a *telemetry* toggle and that is fine;
   **mission-correctness diagnostics must be always-on.** (This is also the substance of draft
   T-635 — see §D.4.)
4. **Do not let a relationship exist that only one tool understands.** The `/2` arithmetic in the
   lobby generator is the sole enforcement of v4's central objective convention.
5. **Budget for the migration before the first schema break.** v4's ORBAT change needed 1,604 lines
   of Python doing text surgery. TBD's equivalent is a server-side batch — cheap **if the document
   is versioned and the migration harness exists**, expensive if invented under pressure.

## B.6 The verdict, without flattery

**The rewrite delivered.** `fnf_v4.md §15.5` lists ten things that got easier and the third is a
genuine capability that v3 could not express at all. Anyone claiming "the graph model
under-delivers" has to explain away that entry, and cannot.

**The rewrite also lost things that were not in the graph model's way**, and that is the part TBD
should read as a warning about *rewrites*, not about *graphs* — `fnf_v4.md §15.6`:
the in-repo documentation (219-line `configGuide.txt` → two `Comment` entities); greppability;
ambient systems; the kit-screenshot pipeline; seven maker-facing knobs with no successor; and
Vietnam/WW2 demoted to `Optionals/` with **20 of 85 kits commented out while still shipping 32 MB of
their data**.

**The accurate three-sentence verdict:**

> The v3→v4 arc **validates TBD's storage, compile and typed-attribute choices directly** — v4's
> single worst structural decision was binarized content, and TBD made the opposite one.
> It **does not validate the graph model on its own terms**: the model bought real expressiveness
> and then spent 350 commits and seven documented defect classes paying for invisible, untyped,
> unvalidated edges, and the 157:78 ratio is a corroborating signal rather than proof, because there
> is no v3 control group.
> The transferable conclusion is therefore **conditional**: TBD may build relationships between
> mission entities, but only behind the affordances Eden could not provide — visible typed edges, an
> inspector, always-on validation, and identity that never derives from position.


---

# Part C — what TBD should build, ranked

## C.0 Grounding — what the editor actually is today

Verified directly against the tree during this synthesis, because several of the corpus's best ideas
turn out to be **already in TBD's contract but not yet authorable in the editor** — which changes the
recommendation from "build a feature" to "expose a field".

| Surface | State | Evidence |
|---|---|---|
| Slot model | `role`, `tag`, `squad`, `callsign`, `rank`, `index` are **separate typed fields**; compiled `slot` has `faction / groupCallsign / role / kit / uid` + derived `id` | `editor_ops.rs:120-132`, `:1167-1179`; `orbat_manager.rs:754-766`; `mission.schema.json#/$defs/slot` |
| Editor layers | **Workflow-only folders**, never reach the compiler. The compiled `layers[]` field is a *different* concept — decoration-overlay aliases `^layer:[a-z0-9_]+$` | `outliner.rs:44-50`; `editor_ops.rs:819-862`; golden mission `bridgehead-at-levie.json` |
| Zones | **Typed 6-value `type` enum read out of the embedded schema at runtime**; circle or polygon (min 3 verts), no rectangle; rich `zoneRules` incl. `neutralizeSeconds`, `onEmpty`/`decayRate`, `pauseOnEnemy`/`resetOnEnemy`, and play-area `graceSeconds`/`penalty` | `eden_chrome.rs:3518-3535`; `editor_ops.rs:2275-2731`; `mission.schema.json#/$defs/zone`,`/zoneRules` |
| Markers | **Deliberate stub** — *"Marker placement lands in T-069"*. Schema has `briefing.markers[]` with a **closed 64-value icon vocabulary**; nothing in the editor writes it | `eden_chrome.rs:2986,3044,3337,4497-4500` |
| Briefing | **No authoring UI** (T-214 / T-418). Schema has `briefings` keyed by faction, each `{situation, mission, execution, markers[]}`. The `briefing` textarea in `event_manager.rs` is the *operation-level* prose field, a different thing | `mission_hydrate.rs:525`; `mission-editor-payload.schema.json:51` |
| Mission Settings | terrain, time, weather (4 presets), `timeLimitSeconds`, `briefingSeconds`, `safeStartSeconds`, JIP, + editor-visual toggles. Respawn / spectator / NVG / tickets **deliberately withheld** | `eden_chrome.rs:3771-3906`, `:3916-3960` |
| Loadouts | **Per-slot inline** `{gear, cargo[]}`. Faction-library "Apply Template" populates, then each slot owns its copy. **Export exists, import does not** | `arsenal.rs:489-596`, `:1049-1066`; `arsenal_rules.rs:592-694` |
| Validation | **None at mission level.** Only `arsenal_rules.rs:350-368` `validate_loadout()` (optic/magazine compat, stranded picks) inline in the Arsenal. Mission-level checking is a **server-side scan returning HTTP 400 at save** | `arsenal_rules.rs:350-368`, `:879`; `apps/website/api/src/contract/validate.rs` |
| Comments | **Do not exist** | confirmed absent |
| Registry | `GET /api/v1/registry`, paged, then held in a `thread_local!` SPA-session cache — **fully resident in memory** | `mission_editor.rs:36-93`, `:106-129` |

Two of those lines are the most important in this document:

- **TBD already has the corpus's single best zone idea** — a declared, closed, schema-driven zone
  *purpose* — which `fnf_tooling.md §3.4` recommends and which none of the four frameworks has.
- **TBD's only mission-level check is post-hoc, at a boundary, delivering an error code** — which is
  structurally the same shape as FNF's Analyzer. The maker can author something the server will
  reject, and finds out at save. That is the exact failure `fnf_tooling.md §3.1` says a live editor
  deletes.

## C.1 Already has — keep, and do not regress

| # | Thing | Convergence backing |
|---|---|---|
| K1 | **Structured, versioned, server-side mission document + JSON compile** | `fnf_v4.md §15.6.2` (binarization is v4's worst decision) + its steal-list #8 |
| K2 | **Role / squad / callsign / rank / tag as separate typed fields** | 5 artefacts, 4 communities (§A.4). `fnf_tooling.md §1.6` calls the packed string an anti-pattern by name |
| K3 | **Stable `uid` + derived `id`** | v4's positional identity is the counter-example (`fnf_v4.md §14.4`) |
| K4 | **Zones as typed-purpose geometry, enum read from the schema** | `fnf_tooling.md §3.4`; better than all four originals |
| K5 | **Arsenal as an authoring tool, not a play-time surface** | 4/4 (§A.5) |
| K6 | **One life; no tickets** | 4/4 (§A.8). TBD reached it independently at `eden_chrome.rs:3916-3960` |
| K7 | **Editor layers stay workflow-only** | v3's semantic layers deleted by their own authors (§D.1) |
| K8 | **Live asset registry, resident in memory** | `fnf_tooling.md §1.6` — same shape as `fnfCfgExportDB.db`, without the trailing-space rot |
| K9 | **Undo/redo across every editor operation** | WOG's two most destructive bulk ops are not undoable (`wog.md §14.7`) |
| K10 | **Closed vocabularies for marker icons and zone types** | WOG's open marker-type string produced `loc_Fuelstation` **1,803 times** as an accidental default nobody chose (`wog.md §9`, `INFERRED:`) |

## C.2 Has in a weaker form — upgrade

| # | Upgrade | From | Why it beats the alternative | Cost | Maps onto |
|---|---|---|---|---|---|
| U1 | **Zone volume + force counts + starting owner** — add `minHeight`/`maxHeight`, `attackerCount`, `defenderCount`, `startingOwner`, and optionally `lock`/`autoLose` to `zoneRules` | `wog.md §7` — 166 instances, `MinHeight=-5` invariant | TBD's `contestable: bool` is the degenerate 1-v-1 case of `_CaptureCount`/`_DefCount`. The volume is what excludes aircraft and includes basements. **Semantics are `INFERRED:` in `wog.md §7`** — take the parameter set, decide the semantics | Schema + settings UI is small; **the mod must read it** (`executor: workbench`) | `mission.schema.json#/$defs/zoneRules`, `editor_ops.rs:2275-2731` |
| U2 | **Loadout import — complete the round-trip** | `wog.md §13.3` — *"Most frameworks give you export only"* | TBD already emits `loadout-export.json`; ingesting the same document closes the loop with no new format | Small — one parser + one apply path | `arsenal.rs` (export at `:1049-1066`) |
| U3 | **Named loadout templates with inheritance** — base + per-role override + an explicit **remove-inherited** verb, attached to slots **by reference** | `ofcra_omtk.md §5.2`, §5.7, §13.2 | Today "Apply Template" copies and then diverges — the exact drift DTAS demonstrates (18 files differ, only 4 intentionally, `fnf_tooling.md §2.1`). Inheritance makes balance policy **queryable**: "prove no infantry gets NVGs" becomes a query. OFCRA's four classes of silent YAML defect (`ofcra_omtk.md §5.9`, ~11 affected roles per §14.1) are all schema errors a typed editor makes unrepresentable | Medium — new entity + resolution order + arsenal UI | `arsenal_rules.rs`, faction library |
| U4 | **Aggregated "every setting in this mission" view, with diff-from-default** | `fnf_v4.md §15.6.3` — the regression v4 named about itself | Prevents the scattering that typed-attributes-on-entities always causes, *before* TBD has enough entity-level settings to suffer it. `fnf_v4.md` has no equivalent view; v3's `config.sqf` gave it away for free | Small — read-only aggregation over the doc | Mission Settings dialog |
| U5 | **Derived trait badges on slot rows** (medic / engineer / radio operator / leader), computed at render | `wog.md §4b` + `wog.md` steal-list #3: *"derive the badges from the slot's actual traits at render time — never mutate the stored string"* | WOG's stored-mutation version is broken in three ways at once and has **zero** output across 171 missions (`wog.md §14.1`) | Small | ORBAT manager / outliner rows |
| U6 | **Aircraft exemption + enforcement policy on boundary zones** | `fnf_v4.md §9` (air flag `false` on the play-zone restriction group) | TBD has `graceSeconds`/`warnEverySeconds`/`penalty`; the vehicle-class axis is the missing one, and FNF documents exactly why | Small | `zoneRules` |
| U7 | **Compile becomes a compiler with a visible diagnostics result** | `fnf_v4.md §13.1` | TBD's compile already runs in a worker; surfacing its findings as structured diagnostics rather than a toast is the difference between a build step and a build *system* | Small–medium | `compiler`, top strip |

## C.3 Lacks entirely — build

| # | Build | Source | Why it beats the alternatives | Cost |
|---|---|---|---|---|
| B1 | **Live validation: a persistent issue panel + a rule engine with four primitives** — V1 required-entity presence, V2 cardinality, V3 per-object invariant, V4 field-shape-or-derivation | `fnf_tooling.md §3.2` (the four primitives cover **all 21** live-evaluable Analyzer rules), §3.3 (build order), plus `wog.md §13.4` (`fn_check_weapon`/`fn_check_lr`) | The corpus's unanimous gap (4/4 have no validation) *and* its unanimous failure (both existing validators silently dead). The panel must come **first**: FNF commented out its rollup and every other check became invisible (`fnf_tooling.md §1.3 D11`) | Medium. The doc is already reactive; the registry is already resident |
| B2 | **Slot census + generated mission summary** — a live per-side header badge, and a summary line composed from the document rather than typed | `wog.md §13.2` (10 lines → 94% convention), `fnf_v3.md §3 step 12`, `fnf_v4.md §3 step 13`, `ofcra_omtk.md §11.2` | 4/4 convergence. And `fnf_tooling.md §3.3` ranks *generating* the name and lobby line at #7–8 precisely because generating **deletes the rules** rather than enforcing them | **Small** — the data is already in the doc |
| B3 | **Editor comments / annotations** | `fnf_v3.md §13.2` (28 in-map comments incl. a 7-paragraph tutorial), `fnf_v4.md §2` (2 comments survived a total rewrite as the framework's *entire* in-repo onboarding), `fnf_v4.md §5` (a `Comment` labels every kit composition) | Editor-only entity, no compile path, reuses existing placement/layer/outliner machinery. See the honest caveat in §D.2 | **Small** |
| B4 | **Briefing authoring: per-faction, structured, with marker links and a uniform-recognition section** | 3/4 derive it (§A.6); WOG's makers hand-wrote the same sections 80+ times; **two communities independently invented the uniform-recognition section** (`wog.md §6`, `ofcra_omtk.md §3 step 4`) | The schema shape already exists (`briefings[faction] = {situation, mission, execution, markers[]}`). Marker links are nearly free in a web editor where briefing and map share a process. **Do not sample runtime state** — v4's kit tab is the anti-pattern (`fnf_v4.md §14.8`) | Medium; **depends on markers (T-069)** for the link half |
| B5 | **Objectives as first-class per-side entities** with the WOG spine, on top of the existing zone types | `wog.md §7`/§13.1 + `fnf_v4.md §7`/steal-list #6 | Two communities converged. The per-side framing is the real insight; one entity with two framings avoids all four of v4's pairing defects | **Large** — mod-side work; belongs in its own program (§D.5) |
| B6 | **Compositions / ORBAT templates** — save a selection, place it, re-side it | `fnf_v4.md §13.3` (85 kits, stable `Squad_index_Role` key surviving re-siding), `wog.md §4c` (28 shipped Eden group presets), Eden's own **F2 = Compositions** tab | The subtractive escape hatch from §A.3. **Check what T-180's ORBAT templates already cover before scoping** | Large — draft T-650 |
| B7 | **Default-override instrumentation** — measure what fraction of missions change each default | `wog.md §14.2` (74/171 missions disable one feature; *"if 43% of missions disable your default, the default is wrong"*) | **No Arma framework can do this**; TBD owns its corpus in a database. It converts a design argument into a query | Small — an analytics query over mission versions |
| B8 | **Conditional inclusion / variants** — a flag on any subtree that compiles only under a selected variant | The salvaged half of `fnf_v3.md §3 step 3` (§D.1), plus `fnf_tooling.md §3.4` on base + sparse delta (T-110) and DTAS's 18-file drift | Serves mode presets, day/night, and player-count variants without overloading layers | Large; ranks low |
| B9 | **Mission parameters** as first-class document objects | `fnf_tooling.md §2.5 #2`, §3.4 | Correct in principle; §A.10 argues the evidence is 2 artefacts from 1 community and the genre does not match. **Lowest rank; may be a "not now"** | Medium |

## C.4 Deliberately do not build

| Not this | Why | Evidence |
|---|---|---|
| **A play-time virtual arsenal** | 4/4 reject it; the arsenal is an authoring tool | §A.5 |
| **A ticket / attrition system** | 4/4 have none; TBD already decided one-life independently. **Remove or annotate the dead `settings.respawn: "tickets"` enum value** | §A.8 |
| **Semantic editor layers** | Deleted by their own authors; the salvageable idea is conditional inclusion (B8), not layer semantics | §D.1 |
| **Positional or derived identity/ordering** | v4's most obvious mistake, and free for TBD to avoid | `fnf_v4.md §14.4` |
| **Debug-gated correctness diagnostics** | v4's shipped state is the un-diagnosable one | `fnf_v4.md §14.9` |
| **Regex validation of free text you could generate** | The Analyzer's own fix-it examples fail its own rules | `fnf_tooling.md §1.3` |
| **A validator that mutates the document** | `AnalyzeSQM.ps1:330-332` rewrites `mission.sqm` before analysing it | `fnf_tooling.md §1.1` |
| **Un-confirmed, un-undoable mission-wide bulk operations** | WOG puts the confirmations on the *safe* ops and none on the destructive ones | `wog.md §14.7` |
| **Runtime/social enforcement in editor scope** — lonewolf detection, referee ladders, weapon safety | The corpus's best design work, and entirely mod/server territory | `ofcra_omtk.md §12` |
| **Warnings that fire on correct input** | The Analyzer's role accordion can never go green | `fnf_tooling.md §1.3` |
| **A "checks passed" state that was never watched fail** | Both existing validators in the corpus are silently dead | `README.md` cross-analysis caveat |

## C.5 The ranking, by value per effort

| Rank | Item | Value | Effort | Note |
|---:|---|---|---|---|
| 1 | **B1 — issue panel + rule engine + first rules** | Very high — 4/4 unanimous gap | Medium | Ships the panel *first*; each rule gets a fail-on-demand test |
| 2 | **B2 — slot census + generated summary** | High | **Small** | Data already present; 4/4 convergence |
| 3 | **B3 — editor comments** | Medium-high | **Small** | No compile path; reuses placement + layers |
| 4 | **U4 — aggregated settings view** | Medium-high | Small | Prevents v4's scattering *before* it happens |
| 5 | **U5 — derived trait badges** | Medium | Small | Also removes a whole bug class by construction |
| 6 | **U2 — loadout import** | Medium-high | Small | Closes WOG's exact round-trip |
| 7 | **U7 — compile diagnostics** | Medium-high | Small-medium | Feeds B1's panel |
| 8 | **U6 — boundary aircraft exemption** | Medium | Small | Needs a mod-side read |
| 9 | **B7 — default-override instrumentation** | Medium | Small | A capability no framework has |
| 10 | **U1 — zone volume + force counts** | High | Medium (mod-gated) | 166 real uses; semantics `INFERRED:` |
| 11 | **B4 — briefing authoring** | High | Medium | Blocked on markers (T-069) for links |
| 12 | **U3 — loadout templates + inheritance** | High | Medium | The balance-policy-as-data payoff |
| 13 | **B5 — objectives as per-side entities** | Very high | **Large** | Own program |
| 14 | **B6 — compositions** | High | Large | Check T-180 overlap first |
| 15 | **B8 — conditional inclusion / variants** | Medium | Large | Also serves T-110 |
| 16 | **B9 — mission parameters** | Low for this genre | Medium | Weakest evidence in the set |


---

# Part D — feedback into the ticket drafts

The 23 drafts (`T-631 … T-653`) were written **before** this research. Nothing below edits
[`editor_ui_ticket_drafts.md`](editor_ui_ticket_drafts.md); these are recommendations to apply.

## D.1 `LAYER-CREATE-001` — does the `tbd_only` classification need reversing?

**No. It needs a different correction, and the recommendation v3 attaches to it must be rejected.**

**The premise is right.** FNF v3 did use Eden layers as the game-mode selector. The template ships
**12 named layers**, seven of them `FNF Gamemode: <Mode>`, and `fn_setupGame.sqf:92-154` deletes
every object and marker belonging to the modes you did not pick —
`_test = (getMissionLayerEntities "FNF Gamemode: Destroy");`. The layer name is a **hard runtime
contract**, which is why `config.sqf:43-44` warns *"DO NOT DELETE ANY OF THE OTHER TEMPLATE
OBJECTIVE OBJECTS"* (`fnf_v3.md §3 step 3`).

**Three findings change what to do about it.**

**(a) The parity status is wrong, but not in the direction the question implies.** `tbd_only` asserts
"Eden has no equivalent". That is false — Eden has layers and **three of the four frameworks use
them**:

| | Layer usage |
|---|---|
| FNF v3 | **Semantic** — 12 layers, 7 of which are the mode selector (`fnf_v3.md §3 step 3`) |
| FNF v4 | **Structural** — every kit composition has exactly three layers named `Info` / `Selectors` / `Units`, blacklisted from re-export; and `init3DEN.sqf`'s `OnDeleteUnits` handler does **empty-layer garbage collection**, wrapped in `collect3DENHistory` (`fnf_v4.md §11`, §12.5) |
| WOG | **Organisational, and heavily** — **2,396 layers across 171 missions, a mean of 14 per mission**; `wog.md §4` reads it as *"makers organise heavily"* | 
| OFCRA | none |

TBD's Editor Layers are a *weaker* version of something Eden has, not a TBD invention.
**Recommendation: change parity `tbd_only` → `partial`**, and rewrite `gap_notes` from
"Editor Layers ≠ Eden layers" to something honest, e.g. *"Workflow folders only. Eden layers also
carry structural roles (composition sub-layers, empty-layer GC) and, in FNF v3, a runtime
game-mode-selector contract that FNF itself abandoned at v4."*

**(b) The recommendation attached to it should be rejected, and this is an explicit adjudication
between two analyses.** `fnf_v3.md §13.1` ranks semantic layers **#1** in its top-ten and says a web
editor *"should copy the idea wholesale: layers that carry semantics, not just visibility."*
`fnf_v4.md §15.2b` documents that FNF **deleted the entire mode architecture** — `modes/` (15 files)
and `mode_config/` (9) are gone — and `§15.5.3` calls the replacement *"the single largest expressive
gain."*

The v3 analysis could not know this: its own scope note says *"This document describes v3 on its own
terms. It makes no claims about v4 and draws no comparison to it."* But per the README's evidence
standard, v3 is also the analysis with **zero `INFERRED:` markers over 1,198 lines** and three
sub-agents, and §13.1 is an unsourced *recommendation* rather than a cited finding.
**v4 wins: it owns the diff, it has the outcome, and the outcome is unambiguous.** The mechanism was
abandoned by the community that invented it, in exchange for the largest capability gain in the
framework's history.

Note also what the mechanism cost while it lived, all from `fnf_v3.md §12`: objective counts hard-
capped at 3 (1–4 for one mode); object and marker names are fixed literals, not maker-chosen; and
the maker is forbidden from deleting content they are not using.

**(c) The salvageable idea is not layers — it is conditional inclusion.** What v3 actually built is
*"a subtree of the document that compiles only when a condition holds"*, implemented by overloading
the one grouping primitive Eden offered. Named properly, it generalises: mode presets, day/night
variants, player-count variants, and OFCRA's "one artefact, many run modes"
(`ofcra_omtk.md §13.3`). It is also the same shape as `T-110` base + sparse delta, which
`fnf_tooling.md §3.4` independently argues for from DTAS's 18-file variant drift.

**Recommended ticket:**

> **`T-654 — Conditional inclusion: variant-gated document subtrees`**
> An entity or subtree carries an optional variant predicate; the compiler emits it only when that
> variant is selected. Replaces FNF v3's layer-name-as-mode-selector without giving layers runtime
> semantics. Serves mode presets, day/night, player-count bands, and T-110's base+delta.
> `status: idea` — this is a design ticket first.

**Do not make Editor Layers semantic.** K7 in §C.1 stands.

## D.2 `PLACE-COMMENT-001` / T-651 — promote, and one correction to the record

**First, a correction.** The brief attributes to `fnf_v3.md` the phrase *"the single cheapest,
highest-value idea here"* about its 28 Comment objects. **That phrase does not appear in
`fnf_v3.md`.** Verified: the only occurrences of "cheapest" / "highest-value" anywhere in the five
analyses are in `fnf_tooling.md:403` (about the dead objective rule D2) and `fnf_tooling.md:861` (a
section heading). What `fnf_v3.md §13.2` actually says, ranked **#2** in its top-ten, is:

> *"In-editor documentation placed next to the thing it documents. 28 Eden `Comment` objects sit
> physically beside the objects they explain… **No other framework in this study puts its manual
> inside the map.**"*

The recommendation survives; the quote should not be repeated.

**Where the phrase actually came from, and why it matters.** It is not a misquote of either file —
it appears in **no artifact at all**. It was written by the `fnf_v3` agent in its *final chat
message* to the dispatcher ("FNF's Comment objects are the single cheapest, highest-value idea
here"), and was never carried into `fnf_v3.md`. The dispatcher then repeated it as if sourced from
the file, and it entered this brief with a citation it never had.

That is the same failure the operator flagged earlier in this program: **an agent's chat summary
contains claims its artifact does not, and those claims get treated as sourced because they arrived
alongside ones that were.** The corpus README exists because of it. This instance is worth keeping
on the record precisely because it happened *after* the lesson was written down, in a program that
had already committed to fixing it — the fix caught it, which is the system working, but only
because a later reader checked a quotation instead of inheriting it.

Practical rule this supports: **a claim is sourced when it is in an artifact with a citation.** An
agent's summary is a pointer to work, not evidence of it.

**The evidence, honestly weighted.**

| For | Against |
|---|---|
| FNF v3 ships **28** in-map comments, including a **seven-paragraph tutorial** on the polygon-boundary naming scheme (`mission.sqm:4093`) and per-object instructions like *"Place this in a flat area near the inner radius of the safe zone"* and *"Delete this group if your mission doesn't need a MAT team"* (`fnf_v3.md §3 step 3`) | **WOG and OFCRA have no comment equivalent** in their corpora. This is **one community across two eras**, not a four-way convergence, and must not be presented as one |
| FNF v4 **deleted everything else** — the 219-line `configGuide.txt`, the whole 421-file template — and what survived as onboarding is literally **two `Comment` entities**. `fnf_v4.md §2`: *"Those two comments are, effectively, the framework's entire in-repo onboarding."* And §14.1 ranks the loss of documentation as v4's **#1** friction | v3's analysis is the one with zero `INFERRED:` markers, so its top-ten ranking carries less weight than its citations do. The *citations* (28 comments, quoted text, line numbers) are solid; the ranking is opinion |
| FNF v4 also uses a `Comment` as the **label for every kit composition** (`fnf_v4.md §5`, `Info` layer) — comments double as structural annotations | |

**The value-per-effort argument stands on its own.** It is an editor-only entity with a position,
text and layer membership, and **no compile path** — the draft already says *"never compile into the
mission"*. Against T-644 (viewshed raster, which needs a colour language designed before the compute
can be built) or T-645 (placement helpers: 4 patterns + 6 align + 3 space + 6 orient +
drag-to-garrison), it is an order of magnitude cheaper.

**Recommendations:**

1. **Move it out of group F and up to the B/C band** — file it alongside T-638 rather than at
   T-651, i.e. it lands in the first half of the program. Since nothing is filed yet, this is a
   renumber, not a supersede.
2. **Add one scope line the draft lacks**, taken directly from v4's evidence: the **new-mission
   template should seed comments**, the way v4's `mission.sqm` seeds its two reminders. That is the
   half of the mechanism that made it survive a total rewrite, and it costs nothing.
3. **Keep the "never compiles" constraint explicit** — it is what keeps the ticket small.

## D.3 Validation — yes, it is a ticket group in its own right

`fnf_tooling.md` is a build-order document for exactly this and the drafts contain **zero validation
tickets across 23**. The three facts that make it group-sized rather than ticket-sized:

- **21 of the Analyzer's 27 checks are live-evaluable**, and they reduce to **four primitives**
  (`fnf_tooling.md §3.1`, §3.2). Four primitives is an engine, not a feature.
- **Nothing in the rule set is genuinely post-hoc.** `fnf_tooling.md §1.5`: *"Every check the
  Analyzer performs is a predicate over structure the editor already holds."*
- **TBD already has a post-hoc mission-level validator in the wrong place** — the server-side zone
  scan returning HTTP 400 at save. That is not a greenfield build; it is a relocation plus an engine.

Proposed group, continuing the drafts' A–H lettering as **I**:

| Id | Title | Notes |
|---|---|---|
| **T-655** | **Validation panel: persistent issue list with rollup** | Ships **first**. `fnf_tooling.md §3.3` ranks the rollup **#2 of 10** and the reason is decisive: FNF commented theirs out inside an HTML comment, so a maker gets no summary of failures and every other check became invisible (§1.3 D11). Click-to-select findings, not a clipboard dump (`wog.md §14.6`). Severity ladder with a legend — and no severity that fires on correct input (`fnf_tooling.md §1.3`) |
| **T-656** | **Rule engine: the four validation primitives + per-rule fail-on-demand tests** | V1 required-entity presence (conditional on mission shape, so the tool never needs FNF's *"the following missing items can be ignored"* disclaimer, `AnalyzeSQM.ps1:1886-1889`); V2 cardinality; V3 per-object invariant; V4 field-shape/derivation. **Acceptance mirrors T-631's**: every rule ships with a test that asserts the rule *can* fire. Both validators in the corpus are silently dead — `$MarkObjs` and the `/g` flag — and *"neither community appears to know"* (`README.md`) |
| **T-657** | **ORBAT and slot rules** | Every slot resolves a role and a squad; no default/empty identity fields; leaderless squads; duplicate callsigns within a side. Collapses the Analyzer's R3+R4+R5 — three overlapping rules with hardcoded name lists that exist **only because Eden packs role and callsign into one string** — into one query over TBD's typed fields. R3 is the one rule FNF rated **error** rather than warning. Revives D3–D8 (six commented-out per-squad coverage rules) as **one rule parameterised by squad template** (`fnf_tooling.md §3.3` ranks 3 and 9) |
| **T-658** | **Registry resolution: every placed asset resolves in the live catalogue** | Revives dead rule D13. `fnf_tooling.md §3.3` rank 6 marks this `on-save` *"only as the conservative default for a cold registry"* — **that caveat does not apply**: `mission_editor.rs:36-93` holds the catalogue in a `thread_local!` SPA-session cache, so this is **live**. Catches modset drift the moment an asset is placed |
| **T-659** | **Slot census badge + generated mission summary line** | The B2 item. Per-side live counts in the header (`WEST 78 · EAST 74 · IND 8 · TOTAL 160`) and a summary composed from the document. Replaces the Analyzer's R1 and R2 by **making the malformed state unreachable** rather than caught (`fnf_tooling.md §3.1` — the "should be unrepresentable" pile). WOG got a 94%-consistent community convention out of the counter alone |
| **T-660** | **Cargo and loadout policy rules** | Analyzer R9 (vehicle inventories match policy — PvP fairness) plus `wog.md`'s `fn_check_weapon` set: below-standard magazine counts, no vest, no uniform, missing map/compass/radio. TBD holds the cargo model already (T-068.15.1). Note FNF's version compares against a **serialised sentinel string** `[[[[],[]],…],false]` — the brittleness a typed model deletes |

**Ordering within the group:** T-655 → T-656 → T-659 → T-657 → T-658 → T-660. The panel and the
engine are prerequisites; T-659 is cheap and visible; the rule content follows.

**One cross-cutting requirement, stated once here:** validation is **always on**. Not behind a debug
flag (`fnf_v4.md §14.9`), not on save, not on export.

## D.4 Other changes to the 23

| # | Recommendation | Why |
|---|---|---|
| 1 | **Split T-641.** Spot heights (Eden parity, a render feature) and scale bar + edge grid labels (an operator addition that pairs with the ruler) have different acceptance criteria and different owners in the render stack. The draft already says *"Two halves."* | Hygiene |
| 2 | **Re-rank T-651 (comments) into the B/C band** | §D.2 |
| 3 | **Add group I (T-655…T-660)** | §D.3 |
| 4 | **Add T-654 (conditional inclusion) as `status: idea`** | §D.1 |
| 5 | **Check T-074 before folding `RIGHT-SUBMODE-001` into T-646.** T-074 is `cancelled` in the registry, and `gap_analysis.md` maps `RIGHT-SUBMODE-001` to it. T-646 would therefore **revive a deliberately cancelled ticket** without saying so | Avoids re-litigating a settled decision by accident |
| 6 | **T-650 (compositions) deserves a higher rank than second-to-last in F**, but scope it against T-180 first. It is FNF v4's #1 strength (`fnf_v4.md §13.3`), WOG ships 28 group presets (`wog.md §4c`), and Eden's **F2 tab is Compositions** — one of six top-level asset tabs (`eden_screenshots/README.md`, batch 06 supersedes batch 05). CLAUDE.md records T-180 as already shipping "templates/vehicles", so part of this may exist | High corpus value, unknown overlap |
| 7 | **T-635 (debug HUD) — keep the distinction explicit.** Gating *telemetry* behind `Ctrl+Alt+D` is right; the ticket should state that **mission-correctness diagnostics are never gated**, so the pattern is not later copied onto validation | `fnf_v4.md §14.9` |
| 8 | **T-645 (placement helpers) — add the confirmation/undo rule.** Bulk operations on a selection must confirm and must be undoable. WOG's own tools invert this and neither destructive bulk op is wrapped in undo | `wog.md §14.7` |
| 9 | **T-642 (ruler) — the draft's "removing `disabled` without the tool working is worse than the current state" is the corpus's own lesson**, and worth keeping as written. Cite it: FNF's `Use Default Loadout` checkbox is read by nothing in the entire worktree, and OFCRA's duration override is displayed to clients while the server ignores it | Reinforces an existing good instinct |
| 10 | **T-652 (rocks) — leave deferred.** No framework evidence bears on it | — |
| 11 | **T-653 (screenshot harness) — leave as `idea`.** All four frameworks' tooling is single-machine and undocumented (`fnf_v3.md §14`: a hardcoded `K:\SteamLibrary\…` path). Promoting the harness is the cheap counter-move | Weak but aligned |

**One row in `gap_analysis.md` that this research would flag beyond `LAYER-CREATE-001`:**
`CONN-SYNC-001` (entity sync) is a bare `missing` row with no ticket and no note. Per §B.5, it
should carry a note that FNF v4's entire defect cluster lives on exactly this mechanism, and that
the inspector + validation must precede the edges.

## D.5 What belongs in this program, and what needs a second one

The drafted program is titled *editor UI/UX*, and it is well-scoped as one. This synthesis's top
recommendations split cleanly:

**Belongs in the editor-UI program (add to T-631…T-653):**
group I validation (T-655–T-660), comments (re-ranked T-651), conditional inclusion as a design
ticket (T-654), the aggregated settings view (U4), derived trait badges (U5), loadout import (U2),
and compile diagnostics (U7). All are editor surfaces.

**Needs its own program — mission content, not editor chrome:**
objectives as per-side entities (B5), briefing authoring (B4), loadout templates with inheritance
(U3), zone volume + force counts (U1). Every one of these changes `mission.schema.json` and requires
**Enfusion mod work to consume it** — which means slices with `executor: workbench` or `human`, and
the executor gate in `CLAUDE.md` applies. Filing them into a UI/UX program would bloat it and stall
it on a different discipline.

**Recommendation:** file the drafts + group I as the editor-UI program, and open a second program
(mission-content) for B5 / B4 / U3 / U1 with its own ticket range and its own mod-side lane.

## D.6 Filing caveats

- **The registry's highest id is `T-626`**, and `T-627`–`T-630` are **absent from
  `.ai/tickets/registry.json`** while existing in shipped code and tests
  (`apps/website/api/tests/t630_map_assets_exempt.rs`, plus in-code tags in `mission_editor.rs`,
  `world_assets/*`, `crates/map-engine-render/src/engine.rs`). So the draft's "next free id is
  T-631" is **correct**, but the registry is stale by four tickets. Backfill before filing, or the
  program starts on a gap nobody can explain later.
- The drafts propose superseding **T-072 / T-073 / T-075 / T-077 / T-078**. All five are live rows
  (`queued` / `idea` / `deferred`). Supersede them in the registry rather than leaving duplicates —
  and note **T-074 is already `cancelled`** (§D.4 #5).
- Ticket ids **T-654 … T-660** are proposed here on the assumption T-631–T-653 are filed as drafted.

---

## Appendix — the convergence table

One row per claim, so it can be checked. **4/4** = all four frameworks; **5 artefacts** counts the
Analyzer separately.

| Claim | Strength | Sources |
|---|---|---|
| Role and group are packed into one free-text field because the editor offers nowhere else | **5 artefacts / 4 communities** | `fnf_v3.md §4`, `fnf_v4.md §4`, `ofcra_omtk.md §4`, `wog.md §4`, `fnf_tooling.md §1.2` |
| One life, then spectate; no ticket system | **4/4** | `fnf_v3.md §8`, `fnf_v4.md §8`, `ofcra_omtk.md §8`, `wog.md §8` |
| No player-facing arsenal; the arsenal is an authoring tool | **4/4** | `fnf_v3.md §5`, `fnf_v4.md §5`, `ofcra_omtk.md §5.10`, `wog.md §5` |
| No CI, no linter, no schema validation, no tests | **4/4** | `fnf_v3.md §11`, `fnf_v4.md §11`, `ofcra_omtk.md §11`, `wog.md §11.3` |
| Medical/ruleset is taken away from the mission maker | **4/4** | `fnf_v3.md §10.G`, `fnf_v4.md §10.10`, `ofcra_omtk.md §8`, `wog.md §8` |
| The last authoring step generates a summary from ground truth, to the clipboard | **4/4** | `fnf_v3.md §3 step 12`, `fnf_v4.md §3 step 13`, `ofcra_omtk.md §11.2`, `wog.md §11.1` |
| The briefing is mostly derived; the maker writes ~4 prose fields | **3/4** derive; the 4th's makers hand-rolled the same sections 80+× | `fnf_v3.md §6`, `fnf_v4.md §6`, `ofcra_omtk.md §6`, `wog.md §6` |
| Configuration moved from a text file to typed attributes on placed objects | **2 crossed it, 1 stayed and names it as friction #1** | `fnf_v4.md §15.2a`, `wog.md §2` (`INFERRED:`), `ofcra_omtk.md §14.5.1` |
| Objectives are typed, placed, parameterised entities with a uniform spine | **2/4, arrived at independently** | `fnf_v4.md §7`, `wog.md §7`/§13.1 |
| Every mission is attack/defend at heart | **4/4** | `fnf_v3.md §7`, `fnf_v4.md §7`, `ofcra_omtk.md §7`, `wog.md §3 step 2` |
| Triggers are abandoned in favour of named regions | **2/4 outright, 1 mixed, 1 with a documented ambiguity** | `fnf_v4.md §9`, `wog.md §9`, `fnf_v3.md §9`, `ofcra_omtk.md §9.3` (`INFERRED:`) |
| One polygon primitive reused for every zone purpose | **2/4** (the two that abandoned triggers) | `fnf_v4.md §9`, `fnf_v3.md §9` |
| A uniform-recognition briefing section with images | **2/4, invented independently** | `wog.md §6`, `ofcra_omtk.md §3 step 4` |
| Per-side briefings | **2/4 have it; 1 names its absence** | `wog.md §6`, `ofcra_omtk.md §6`, `fnf_v4.md §6` |
| Bounded, author-defined, safe-start-only loadout choice | **2/4** (both FNF eras) | `fnf_v3.md §5`, `fnf_v4.md §5` |
| The declarative parameters are used; the scripting escape hatch is not | **1 framework, but 165/166 instances** | `wog.md §15.5` |
| Editor comments as in-map documentation | **1 community, 2 eras** — *not* a convergence | `fnf_v3.md §13.2`, `fnf_v4.md §2` |
| Both existing validators are silently dead, in the same shape | **2 independent tools** | `fnf_tooling.md §1.3 C`, `wog.md §14.1`, `README.md` |
