# 3den Enhanced sweep — the catalogue triaged into parity rows

**Input:** [`../3den_enhanced_feature_catalogue.md`](../3den_enhanced_feature_catalogue.md) (869 lines, 484
table rows, mod v8.8.0 @ `73f6868`).
**Priority authority:** [`../framework_synthesis.md`](../framework_synthesis.md) Part C — its build list is
authoritative and is **not re-litigated** here. Where this sweep disagrees, §4.4 says so explicitly.
**Registry:** [`../../tickets/registry.json`](../../tickets/registry.json) (656 tickets) +
[`registry_write_log.md`](registry_write_log.md). Every top-level `T-` id below was read out of the
registry, not inferred. Dotted ids (`T-090.3.3`, `T-091.0`, `T-092.1`, `T-151.3`, `T-180.4`) are
**shipped slice tags** recorded in `CLAUDE.md` §Status, not registry rows — cited as provenance for a
capability, never as a destination for work.

**What this file is for.** The catalogue was researched thoroughly and then reached almost nothing —
zero citations in `gap_analysis.md`, two in the registry, one ticket (T-645) borrowing from it. 3den
Enhanced is the mod the Arma community actually runs on top of Eden, so its feature list is, by
construction, *what mission makers found missing*. That makes it the best available wishlist. It is
also ~250 features long, which is far too many to build — **the value of this sweep is the filter,
not the inventory.**

---

## 1. Method and numbering

### 1.1 Id scheme

Ids are `3DEN-<PREFIX>-<NNN>`, where the prefix is bound to the catalogue section the feature is
*defined* in — not the section it is cross-referenced from. The catalogue cross-references heavily
(§2, §9 and §10 are re-tabulations of §3–§8), so a feature gets **exactly one id at its definition
site** and the cross-reference sections are cited in `notes` rather than minting a second row.

| Prefix | Catalogue section | Scope |
|---|---|---|
| `3DEN-PLACE` | §2.4, §2.5, §3.2 | Placement Tools, align, space, orient, snap, garrison, patterns |
| `3DEN-MEAS` | §2.1 | Measure Distance and its adjacencies |
| `3DEN-TOOL` | §3.1 | Tools ▸ Utilities — the standalone dialogs |
| `3DEN-LOAD` | §3.3 | Loadout Tools + the Equipment Storage Editor |
| `3DEN-VEH` | §3.4 | Vehicle Customization + Pylons |
| `3DEN-DBG` | §3.5, §7.3 | Debug Tools and the 28 Debug Options |
| `3DEN-MISC` | §3.6 | Miscellaneous Tools — the batch toggles |
| `3DEN-LYR` | §3.7, §4.1, §8.5 | Layer commands and the default-layer mechanism |
| `3DEN-CTX` | §4 | Right-click context-menu additions |
| `3DEN-CHROME` | §5 | Status bar, panel tabs, scene overlays |
| `3DEN-KEY` | §6 | Keybinding *policy* only — bindings for features owned elsewhere are not re-minted |
| `3DEN-PREF` | §7.1, §7.2, §7.4 | The Preferences block |
| `3DEN-ATTR` | §8 | Attribute-window additions |
| `3DEN-CAM` | §10 | Camera / view / map tools **not already defined elsewhere** |
| `3DEN-QOL` | §11 | Behaviours with no menu entry |
| `3DEN-EXT` | §13 | Extensibility surfaces + companion addons |
| `3DEN-GONE` | §12 | Documented-but-removed. All `no` by construction — see §1.4 |

### 1.2 Collapse policy

The catalogue's own collapse is inherited verbatim, and stated so a later reader can reverse it:

- **145 event-handler code slots → 5 rows**: `3DEN-ATTR-007` object (73), `-046` mission global (42),
  `-047` mission server (9), `-048` music (2), `-058` group (19). 73+42+9+2+19 = 145. All `no`/`d`:
  TBD has no scripting layer for them to attach to.
- **16 briefing diary fields → 2 rows** (`3DEN-ATTR-043`, `-044`), split on where TBD's blocker is:
  the three sections TBD's schema already carries, and Signal, which it does not.
- **53 `remove_*` optional PBOs → 2 rows** (`3DEN-ATTR-066` the mechanism, `-067` the five stale ones,
  which are a separate finding).
- **28 Debug Options → 8 rows**, split where the verdict differs rather than by the mod's own four
  sub-headings — three of the Visualization options turn out to be things TBD already has.

Counted this way the table below is **245 rows** against the catalogue's stated "roughly 250 distinct
user-facing features". That agreement is a check, not a coincidence.

### 1.3 Column semantics

`3den_id | tbd_id | want | build_class | notes | ticket`

**`want`** replaces `parity`, because this is a wishlist and not a parity target. There is no
obligation to reach any of it.

| Value | Meaning |
|---|---|
| **have** | TBD already does this. Cited with `file:line` in the live editor — no exceptions |
| **want** | Worth building. A row scored `want` is one I would defend in review |
| **maybe** | Defensible either way. The `notes` cell must say what the argument turns on |
| **no** | Explicitly not wanted, with the reason. §5 is the scope boundary and matters as much as the yes list |

**`build_class`** — the same ladder `gap_analysis.md` uses for attribute rows:

| Class | Test |
|---|---|
| **a** | SPA-buildable today — no mission-contract blocker; editor-only, or the compiled schema already carries the key |
| **b** | Schema-blocked — `mission.schema.json` carries 25 `additionalProperties: false` and must be widened first |
| **c** | Mod-blocked — no concept exists in `apps/mod/tbd-framework`, or a runtime (AI, triggers, damage) must exist first |
| **d** | N/A for a 2D browser editor — a 3D-scene affordance, an A3-engine concept with no Enfusion analogue, or a scripting handle with no scripting layer |

`tbd_id` reuses the existing `gap_analysis.md` / `interactions.md` vocabulary where a row already
exists there; `—` where 3den Enhanced is adding something Eden itself does not have (which is most of
the table — that is the point of the mod).

### 1.4 The §12 caveat, honoured

The catalogue records that the mod's auto-generated GitHub wiki has **drifted from shipping source**
and still documents features removed from v8.8.0. All ten §12 features are minted as `3DEN-GONE-*`,
scored **`no` / `d`**, with the reason `not in shipping source (§12)`. None is triaged as available.
`3DEN-GONE-001` (Toggle Marker Alpha) is the only one whose *idea* is worth anything, and its row says
so without claiming the mod ships it.

### 1.5 Evidence standard

Every row cites a catalogue section. Every `have` cites `file:line` in the live editor. Inference is
prefixed `INFERRED:`. Two facts were verified rather than inherited, because both are load-bearing and
both are easy to get wrong:

- **TBD has no snapping.** `grep -rn snap apps/website/frontend/src --include=*.rs` returns 11 hits and
  **every one is `read_snapshot()` / `snapshot` prose** (`orbat_manager.rs:283`, `select_tool.rs:5`,
  `yrs_persist.rs:29`…). There is no grid, no surface snap, no alignment guide. `gap_analysis.md`
  scores `XFORM-SNAP-001` as `na` for a different and correct reason (the mod grounds every spawn by
  contract, T-092.1) — that reason covers surface snap only, not translation/rotation grids.
- **TBD's per-slot attribute surface is nine fields** — `read_attrs` / `SlotAttrs` at
  `editor_ops.rs:631-663` and `:122-132`: `id, x, y, z, rotation, stance, role, tag, squad`. So most
  attribute-level rows below have nothing to attach to yet, and that is why so many of §8 scores `no`
  rather than `want`.

---

## 2. The full table

### 2.1 `3DEN-PLACE` — placement, patterns, align, orient, garrison (§2.4, §2.5, §3.2)

The strongest area of the mod and the densest cluster of `want` in this sweep. **T-645 already covers
the spine** — patterns, align, space-equally, orient, garrison — so almost every row here **extends
T-645 rather than proposing a new slice**, per the brief. Rows that fall outside T-645's stated scope
say where they go instead.

| 3den_id | tbd_id | want | build_class | notes | ticket |
|---|---|---|---|---|---|
| 3DEN-PLACE-001 | — | want | a | §2.5/§3.2 — Placement Tools dialog: operates on the current selection and **re-lays it live as sliders move**. The live-preview contract is the borrow, not the dialog | T-645 |
| 3DEN-PLACE-002 | — | want | a | §2.5 — Circular pattern: Radius, Initial Angle, Central Angle; each entity rotated to face outward; Central Angle < 360 gives an arc. Named in T-645's summary | T-645 |
| 3DEN-PLACE-003 | — | want | a | §2.5 — Line pattern: Spacing + Direction, single file along a bearing | T-645 |
| 3DEN-PLACE-004 | — | want | a | §2.5 — Grid pattern: #Columns + Spacing X/Y, rows auto-derived from selection size | T-645 |
| 3DEN-PLACE-005 | — | want | a | §2.5 — Fill Area: random scatter inside an A×B box. The workhorse for populating a treeline or a compound | T-645 |
| 3DEN-PLACE-006 | — | want | a | §2.5 — Fill Area spawns a live **preview area** showing the bounds. In TBD this is a preview rectangle, not an entity — the zone-draw preview machinery already exists (`editor_ops.rs:2350` `begin_zone_draw`, `:2412` `zone_draw_pop_vertex`) | T-645 |
| 3DEN-PLACE-007 | — | want | a | §2.5 — Orientation: set one absolute facing across the whole selection | T-645 |
| 3DEN-PLACE-008 | — | want | a | §2.5/§3.2 — Randomise Direction (`CTRL+NUM3`). Cheap, and the single most-used anti-uniformity verb | T-645 |
| 3DEN-PLACE-009 | — | want | a | §3.2 — Reverse Direction, flip 180° (`CTRL+NUM7`) | T-645 |
| 3DEN-PLACE-010 | — | want | a | §3.2 — Orientate North/East/South/West, absolute 0/90/180/270. Four presets, one code path with -007 | T-645 |
| 3DEN-PLACE-011 | — | want | a | §2.5 — Pattern centre defaults to screen centre, with a crosshair button to re-centre. In a 2D editor screen centre **is** map centre, so this is nearly free | T-645 |
| 3DEN-PLACE-012 | — | maybe | a | §2.5 — Centre drawn every frame as a magenta `X`, in 3D **and** on the map. TBD needs one surface, not two; turns on whether the pattern gizmo gets its own render lane or reuses the zone-draft lane | T-645 |
| 3DEN-PLACE-013 | — | want | a | §2.5 — Every edit box nudges ±1 / ±0.1 / ±0.01 / ±0.001 on PageUp/PageDown by Ctrl/Alt/Shift. TBD's `number_field` (`attributes.rs:167-228`) commits on blur/Enter and has **no nudge at all** — this is a general numeric-field idiom, not a placement feature | new — E8 |
| 3DEN-PLACE-014 | — | no | d | §2.5 — Dialog hides the left panel while open and repositions when panel state changes. A workaround for a 3D editor whose dialogs float over the scene; TBD's dialogs are modal over a map that does not need to stay visible | — |
| 3DEN-PLACE-015 | — | want | a | §2.4/§3.2 — Align to X+ / X− (farthest east / west entity). `ENH_fnc_alignEntities` index 0, max/min — one function, six commands | T-645 |
| 3DEN-PLACE-016 | — | want | a | §2.4/§3.2 — Align to Y+ / Y− (farthest north / south), index 1 | T-645 |
| 3DEN-PLACE-017 | — | no | d | §2.4/§3.2 — Align to Z+ / Z− (highest / lowest). **No subject in TBD:** the mod grounds every spawn (`jsonY` → `GetSurfaceY`, `CAPSULE_GROUND_OFFSET_M = 0.0`, T-092.1) and the editor re-drops z to 0 on any x/y edit (`store.rs:1356-1358`, `// terrain-follow`). A Z-align would fight the contract | — |
| 3DEN-PLACE-018 | — | want | a | §2.4/§3.2 — Space along X-Axis, equal intervals (`ENH_fnc_spaceEqually`) | T-645 |
| 3DEN-PLACE-019 | — | want | a | §2.4/§3.2 — Space along Y-Axis | T-645 |
| 3DEN-PLACE-020 | — | no | d | §3.2 — Space along Z-Axis. Same reason as -017 | — |
| 3DEN-PLACE-021 | XFORM-SNAP-001 | no | d | §2.4 — Snap to Surface (`CTRL+SPACE`). Already scored `na` in `gap_analysis.md`: there is no un-snapped state to toggle. **Note this does not mean TBD has snapping** — it has no translation or rotation grid either, and that gap is real (see -022 note and T-648) | — |
| 3DEN-PLACE-022 | — | want | a | §2.5 — **Garrison**: drag a selection onto a building and it fills the building's interior positions. Named in T-645. `INFERRED:` TBD's world export carries measured building OBBs (T-090.3.3, **4,131** building instances; drawn and picked since T-151.3) but **no interior position list**, so positions must be synthesised from the OBB — scope that inside T-645 before committing to it | T-645 |
| 3DEN-PLACE-023 | — | maybe | a | §7.2 — Garrison pref: Create Layer (garrisoned units get their own layer). Turns on whether T-666 lands layer authoring first; without it a new layer has nowhere to be managed | T-645 |
| 3DEN-PLACE-024 | — | want | a | §7.2 — Garrison pref: Group Units (units in the same building are grouped). TBD has real squads (`editor_ops.rs:1293` `orbat_add_squad`), so this maps cleanly | T-645 |
| 3DEN-PLACE-025 | — | want | a | §7.2 — Garrison pref: Random Rotation (default **on** in the mod, which is the tell that it is the right default) | T-645 |
| 3DEN-PLACE-026 | — | no | c | §7.2 — Garrison pref: Disable Pathfinding. Every TBD body spawns with AI disabled (`TBD_SpawnManager.c:963,1166`) — no pathfinder to disable | — |
| 3DEN-PLACE-027 | — | want | a | §7.2 — Garrison pref: Auto Select — units that did not fit **stay selected**, so the maker can immediately re-place them. A small idea that removes a whole failure mode | T-645 |
| 3DEN-PLACE-028 | — | want | a | §7.2 — Garrison pref: Only garrison empty positions | T-645 |
| 3DEN-PLACE-029 | — | want | a | §7.2 — Garrison pref: Stance, incl. Random and **"Random except prone"**. TBD already carries `stance` per slot (`attributes.rs:297-312`, `store.rs:1246`), so this writes an existing field | T-645 |
| 3DEN-PLACE-030 | — | want | a | §3.1/§2.5 — **Name Objects** (`ALT+N`): batch-name a selection `PREFIX_INDEX` with a configurable start index. TBD's analogue is batch-setting `callsign` across a squad (`store.rs:1293`), authored one row at a time in the ORBAT inspector today (`orbat_manager.rs:1314-1333`) | T-649 |
| 3DEN-PLACE-031 | — | have | a | §2.5/§4.1/§11 — **Set as default Layer**: new entities file into the marked layer automatically. TBD: `active_layer` is the drop target — `editor_ops.rs:1025` `set_active_layer`, minted lazily on first place (`outliner.rs:17`) | — |
| 3DEN-PLACE-032 | — | have | a | §4.1 — Reset default Layer. Same mechanism, `set_active_layer(None)` (`editor_ops.rs:1025`) | — |
| 3DEN-PLACE-033 | — | maybe | a | §2.5/§5.4/§7.1 — Show Building Positions: numbered `drawIcon3D` at every building position within 100 m. Only worth building **if -022 garrison ships** — on its own it is 3D-scene clutter with no 2D equivalent purpose | T-645 |

### 2.2 `3DEN-MEAS` — measurement (§2.1)

The mod has **exactly one** measurement tool, and it is weaker than the ruler TBD has already
specified. See §6.1 for the full priority answer.

| 3den_id | tbd_id | want | build_class | notes | ticket |
|---|---|---|---|---|---|
| 3DEN-MEAS-001 | — | want | a | §2.1 — Measure Distance: two-step stateful click, reports 2D **and** 3D distance. T-642 is a strict superset (persistent polyline, per-leg bearing, running total). Take the 2D/3D pair — the delta is the only elevation readout the mod has | T-642 |
| 3DEN-MEAS-002 | — | want | a | §2.1 — **On-foot travel time** at a hard-coded 14.15 km/h. The one genuinely novel idea in the mod's measurement surface, and the one thing a milsim planner actually wants from a ruler. Make the speed a setting rather than a constant | T-642 |
| 3DEN-MEAS-003 | — | no | a | §2.1 — Readout is a 3DEN notification shown for 20 s. T-642's summary already rejects this by name ("a 20-second notification as the readout"); a measurement you cannot re-read is not a measurement | — |
| 3DEN-MEAS-004 | — | maybe | a | §2.1 — When the 2D map is open it lays a `BIS_fnc_markerPath` at **50 m spacing** between the points. Tick marks along a leg are a decent 2D affordance; turns on whether they read as clutter next to contours (T-640) and the scale bar (T-667) | T-642 |
| 3DEN-MEAS-005 | — | no | d | §2.1 — `drawLine3D` in solid red via a `Draw3D` mission EH. 3D-scene only | — |
| 3DEN-MEAS-006 | — | no | a | §2.1 — 5-second auto-expiry, **not configurable**. T-642 is persistent by design; this is the defect the ticket exists to avoid | — |
| 3DEN-MEAS-007 | — | no | d | §2.1 — `waitUntil` re-entry guard against `ENH_EH_DrawDist`. An SQF global-state artefact, not a feature | — |
| 3DEN-MEAS-008 | — | maybe | a | §2.1 — **Log Object Info** reports the selected object's bounding-box W×L×H — the only other dimension the mod reports anywhere. TBD holds measured prefab OBBs already (T-090.3.3); surfacing them turns on whether anyone asks "will this fit" | new — E5 |

### 2.3 `3DEN-TOOL` — Tools ▸ Utilities, the standalone dialogs (§3.1)

Six of these are BI's own utilities the mod merely links; they are `no`/`d` without argument. The
interesting rows are the Command Palette and the Briefing Editor.

| 3den_id | tbd_id | want | build_class | notes | ticket |
|---|---|---|---|---|---|
| 3DEN-TOOL-001 | — | no | d | §3.1 — CfgDisabled Commands Template Generator. BI utility, SQF config tooling | — |
| 3DEN-TOOL-002 | — | no | d | §3.1 — Jukebox (BI music preview). No CfgMusic analogue | — |
| 3DEN-TOOL-003 | — | no | d | §3.1 — Moon Phases (BI reference utility) | — |
| 3DEN-TOOL-004 | — | no | d | §3.1 — Print Config. Config dumping has no Enfusion analogue in TBD's editor | — |
| 3DEN-TOOL-005 | — | no | d | §3.1 — Script Commands browser | — |
| 3DEN-TOOL-006 | — | no | d | §3.1 — 3DEN Radio: CfgMusic browser/player with playlist (`ALT+M`). A3-specific | — |
| 3DEN-TOOL-007 | — | maybe | a | §3.1 — **Scenario Attributes Manager**: save/load complete scenario attribute sets as named reusable templates across missions. TBD's Mission Settings is currently ~8 fields (`eden_chrome.rs:3771-3906`, `:3916-3960`) — too thin to template. Revisit if the settings surface grows; the idea is right, the subject is not there yet | — |
| 3DEN-TOOL-008 | — | no | d | §3.1 — CfgSentences Browser | — |
| 3DEN-TOOL-009 | — | no | d | §3.1 — Texture Finder (`ALT+T`), scans configs for texture paths | — |
| 3DEN-TOOL-010 | — | want | a | §3.1 — **Briefing Editor** (`ALT+B`): composer with tag wrapping, **reusable persistent templates**, export as SQF or raw text. The template half is the borrow — makers write the same briefing skeleton every time (framework_synthesis B4 makes the same finding from WOG's 80+ hand-written sections) | T-671 |
| 3DEN-TOOL-011 | — | want | a | §3.1 — **Search Attributes**: search the *text* attributes of **all entities in the scenario** for a string. TBD has search over the asset catalogue (`asset_catalog.rs:396-414`) and **none over the document** — you cannot find a slot by role, callsign or tag. Reframed as document search, this is real | new — E4 |
| 3DEN-TOOL-012 | — | no | d | §3.1 — Manage Zeus Addons. No curator concept | — |
| 3DEN-TOOL-013 | — | want | a | §3.1 — **Command Palette** (`ALT+SPACE`): fuzzy search over every menu-strip command. TBD's editor chrome is a menu strip whose commands are already enumerable, and a palette is the standard web-app answer to "the UI is inconsistent and disjointed" (the exact complaint T-668 exists for). Highest value-per-effort row in §3 | new — E2 |
| 3DEN-TOOL-014 | — | maybe | a | §3.1/§11 — Palette **learns**: entries ranked by usage frequency then alphabetically, with a Reset Command Priority command. Good, but it is polish on -013 and should not gate it | new — E2 |
| 3DEN-TOOL-015 | — | no | a | §3.1/§13.2 — Custom palette commands registered from config / `description.ext` / a JSON file (the JSON path **requires Pythia**). An extensibility surface for SQF modders; TBD has no third-party authoring story for its editor | — |

### 2.4 `3DEN-LOAD` — Loadout Tools + Equipment Storage Editor (§3.3)

TBD's Arsenal is the most mature part of its editor (14 pick rows, `arsenal_rules.rs:49-162`; cargo
panel `arsenal.rs:1131`; export gate `arsenal.rs:616`). What the mod has that TBD does not is
**movement of loadouts between slots** and **import**.

| 3den_id | tbd_id | want | build_class | notes | ticket |
|---|---|---|---|---|---|
| 3DEN-LOAD-001 | — | want | a | §3.3 — **Copy Loadout/s** (`CTRL+SHIFT+C`): buffer the loadout of every selected entity. TBD has no way to move a loadout between slots at all — Apply Template populates from the faction library, then each slot owns its copy (`arsenal.rs:489-596`) | new — E7 |
| 3DEN-LOAD-002 | — | want | a | §3.3/§2.5 — **Apply Loadout/s** (`CTRL+SHIFT+A`): with several buffered, **picks one at random per entity**. The randomisation is the borrow — it turns a copy verb into a variety verb for free | new — E7 |
| 3DEN-LOAD-003 | — | no | d | §3.3 — Export Loadout (CfgRespawnInventory). A3 config format | — |
| 3DEN-LOAD-004 | — | have | a | §3.3 — Export Loadout (Config). TBD exports `loadout-export.schema.json` v2 with a refusal gate — `arsenal.rs:616` `try_export`, `:520` `picks_to_export`, download at `:1049-1066` | — |
| 3DEN-LOAD-005 | — | maybe | a | §3.3 — Remove NVGs (`CTRL+SHIFT+N`). Per-category strip verbs are cheap once -001 exists; the argument against is nine verbs for a 14-row form where clearing one `<select>` is one click | new — E7 |
| 3DEN-LOAD-006 | — | maybe | a | §3.3 — Remove Vests. Same argument as -005 | new — E7 |
| 3DEN-LOAD-007 | — | maybe | a | §3.3 — Remove Goggles (`CTRL+SHIFT+G`) | new — E7 |
| 3DEN-LOAD-008 | — | maybe | a | §3.3 — Remove Headgear (`CTRL+SHIFT+H`) | new — E7 |
| 3DEN-LOAD-009 | — | maybe | a | §3.3 — Remove Weapons (`CTRL+SHIFT+W`) | new — E7 |
| 3DEN-LOAD-010 | — | want | a | §3.3 — **Remove Everything** (`CTRL+SHIFT+D`): clear the whole inventory. The one strip verb that is unambiguously worth it — "start this slot from nothing" has no equivalent today | new — E7 |
| 3DEN-LOAD-011 | — | have | a | §3.3 — Equipment Storage Editor: vehicle/container inventory management. TBD: per-slot cargo panel over `["vest","pants","jacket","backpack"]` (`arsenal.rs:1131`, `arsenal_rules.rs:606`) plus vehicle cargo rows (`eden_chrome.rs:2126-2186`), capacity-checked at `wire_safety.rs:360` `scan_cargo_capacity` | — |
| 3DEN-LOAD-012 | — | want | a | §3.3 — ESE **templates**: save / load / preview / delete named inventory sets. This is framework_synthesis **U3** (named loadout templates with inheritance) arriving from a second, independent direction — worth recording as convergence | — (U3, unfiled) |
| 3DEN-LOAD-013 | — | want | a | §3.3/§2.8 — ESE **import from clipboard**. framework_synthesis **U2** ranks loadout import #6 of 16; TBD emits the document already and ingesting the same shape closes the round-trip with no new format. Caveat from §14: the ESE's `ImportFromClipboard` menu class appears in **no `items[]` array**, so the mod's own import is shortcut-only at best — take the idea, not the wiring | — (U2, unfiled) |
| 3DEN-LOAD-014 | — | no | d | §3.3 — ESE export to SQF / ACE Arsenal / BI Arsenal formats. Three A3-specific serialisations | — |
| 3DEN-LOAD-015 | — | maybe | a | §3.3/§6.5 — ESE quantity keys `1`–`6` as presets. A good in-dialog idiom for TBD's cargo rows, but it collides with any future direct-select keys — decide inside T-648's keymap pass | — |
| 3DEN-LOAD-016 | — | no | d | §3.3/§13.4 — ACE Arsenal shortcut (`CTRL+SHIFT+L`, separate companion addon). No ACE | — |

### 2.5 `3DEN-VEH` — vehicle customization and pylons (§3.4)

| 3den_id | tbd_id | want | build_class | notes | ticket |
|---|---|---|---|---|---|
| 3DEN-VEH-001 | — | maybe | c | §3.4 — Copy Vehicle Customization (textures/animations). Enfusion has variant prefabs rather than A3's `setObjectTexture` slots; a "copy appearance" verb needs a variant concept in `apps/mod/tbd-framework` first. Not obviously worthless, clearly not next | — |
| 3DEN-VEH-002 | — | maybe | c | §3.4 — Apply Vehicle Customization, random pick per entity if several were copied. Blocked behind -001 | — |
| 3DEN-VEH-003 | — | maybe | c | §3.4 — Randomize Vehicle Customization across a selection. Same gate | — |
| 3DEN-VEH-004 | — | no | d | §3.4 — Copy Pylon Settings. Pylons are an A3 aircraft-loadout concept with no Enfusion analogue | — |
| 3DEN-VEH-005 | — | no | d | §3.4 — Apply Pylon Settings (random from the copied set) | — |
| 3DEN-VEH-006 | — | no | d | §3.4 — Export Pylons to SQF | — |

### 2.6 `3DEN-DBG` — Debug Tools (§3.5) and the 28 Debug Options (§7.3)

§7.3 is preview-time cheats for a game TBD's editor does not launch. The four rows that survive are
the ones that turn out to be **things TBD already has**, which is worth recording precisely because
the catalogue does not say so.

| 3den_id | tbd_id | want | build_class | notes | ticket |
|---|---|---|---|---|---|
| 3DEN-DBG-001 | — | no | d | §3.5 — Variable Viewer (mission/UI/profile namespace browser). No namespace concept | — |
| 3DEN-DBG-002 | — | no | d | §3.5 — RPT Viewer (`CTRL+ALT+V`), reads the game log inside Eden; **requires Pythia** | — |
| 3DEN-DBG-003 | — | no | d | §3.5 — Log Game Info | — |
| 3DEN-DBG-004 | — | no | d | §3.5 — Clear Chat (`clearRadio`) | — |
| 3DEN-DBG-005 | — | no | d | §3.5 — Export GUI Base Classes | — |
| 3DEN-DBG-006 | — | no | d | §3.5 — Open GUI Test Grids (`RscTestGrids`) | — |
| 3DEN-DBG-007 | — | no | d | §7.3 General (9 settings, collapsed): Enable Arsenal · Virtual Garage · Kill Units ×4 · Kill Cursor Target · Delete Corpses · Teleport · Variable Viewer · Log active Scripts. All preview-time; TBD's editor has no preview | — |
| 3DEN-DBG-008 | — | no | d | §7.3 Player (8 settings, collapsed): Invulnerability · Captive · Stamina · Zeus · Recoil · Sway · Unlimited Ammo · Reload Time. Preview-time cheats | — |
| 3DEN-DBG-009 | — | have | a | §7.3 Visualization ▸ **Show FPS**. TBD: the editor debug HUD renders `z −2.00 · c0 · glyph 0 · 57 FPS · rf 0.92ms` — `mission_editor.rs:664` (signal), `:2077-2088` (render). T-635 exists to stop it overlapping the toolbelt | — |
| 3DEN-DBG-010 | — | have | a | §7.3 Visualization ▸ **Show Groups** (3D + map). TBD draws squad leader→member hairlines on their own render lane — `crates/map-engine-render/src/draw_order.rs:57` `LaneRole::SquadLinks` (T-180.4), ordered above `Grid` and below `Slots` (`:491`, `:501`) | — |
| 3DEN-DBG-011 | — | have | a | §7.3 Visualization ▸ **Draw Trigger Areas**. TBD has no triggers, but its zones are always drawn and always editable — `editor_ops.rs:2579` `zone_rows`, draw lane `MissionZones` (`draw_order.rs:582`). The capability the option provides is on by default | — |
| 3DEN-DBG-012 | — | no | c | §7.3 Visualization ▸ Debug Path (Disabled / 2D / 2D+3D). AI pathfinding visualisation; every TBD body spawns AI-disabled | — |
| 3DEN-DBG-013 | — | no | d | §7.3 Visualization ▸ Bullet Tracking · Draw View Direction · Dynamic Simulation debug. Runtime 3D overlays | — |
| 3DEN-DBG-014 | — | no | d | §7.3 Environment: Skip Time · Time Multiplier. Preview-time only | — |

### 2.7 `3DEN-MISC` — Miscellaneous Tools (§3.6)

| 3den_id | tbd_id | want | build_class | notes | ticket |
|---|---|---|---|---|---|
| 3DEN-MISC-001 | — | want | a | §3.6 — Create Trigger (**Whole Map Coverage**): one trigger sized and positioned to cover the terrain exactly. TBD's analogue is a play-area zone covering the whole terrain, which every mission needs and which today means drawing a 12.8 km polygon by hand (`editor_ops.rs:2350` `begin_zone_draw`). One button | new — E11 |
| 3DEN-MISC-002 | — | want | a | §3.6/§2.9 — **Switch Time** (`ALT+UP`): jump the *editor* to 12:00 with fog/overcast/rain zeroed for maximum visibility, **without touching the scenario**, reversible. TBD's time scrubber writes `meta` (`eden_chrome.rs:1084`), so "make the map readable" and "set the mission's time" are the same control — they should not be | new — E9 |
| 3DEN-MISC-003 | — | have | a | §3.6/§2.9 — Toggle Grass (`ALT+DOWN`), editor-only clutter hiding. TBD has **12 editor-only world layer toggles** including trees, forest mass and props — `world_layer_prefs.rs:63-74` | — |
| 3DEN-MISC-004 | — | no | c | §3.6 — Toggle Simple Object (`objectIsSimple`). No simple-object concept in `apps/mod/tbd-framework` | — |
| 3DEN-MISC-005 | — | no | c | §3.6 — Toggle Simulation (`enableSimulation`) | — |
| 3DEN-MISC-006 | — | no | c | §3.6 — Toggle Dynamic Simulation | — |
| 3DEN-MISC-007 | — | no | d | §3.6 — Toggle Local Object (`isLocalOnly`). An A3 MP locality concept | — |
| 3DEN-MISC-008 | — | no | c | §3.6 — Toggle AI Features (inverts all 21 at once). Nothing to invert — see `3DEN-ATTR-004` | — |
| 3DEN-MISC-009 | — | no | b | §3.6 — Toggle Playable State. Every TBD slot **is** a playable slot by construction (`$defs/slot`); a non-playable slot would be a new entity kind, not a toggle | — |
| 3DEN-MISC-010 | — | want | a | §3.6 — `ENH_fnc_toggleAttributes`: **one function inverts any named boolean across a whole selection**, and all nine Toggle commands route through it. The pattern is the borrow, not the nine A3 attributes — TBD's multi-edit ticket should build the batch-invert primitive rather than nine handlers | T-649 |

### 2.8 `3DEN-LYR` — layer commands and the default layer (§3.7, §4.1, §8.5)

TBD's layer mutators **all exist and none are called** — `rename_editor_layer` (`store.rs:1886`),
`reparent_editor_layer` (`:1895`), `remove_editor_layer` (`:1527`), `move_slot_to_layer` (`:1915`)
have zero callers; `add_editor_layer` (`:1872`) has exactly one. T-666 is the ticket; these rows say
which commands to put in front of it.

| 3den_id | tbd_id | want | build_class | notes | ticket |
|---|---|---|---|---|---|
| 3DEN-LYR-001 | — | maybe | a | §3.7/§5.3 — Select all Layers. Useful only once layers carry flags worth acting on in bulk; turns on T-665 landing first | T-666 |
| 3DEN-LYR-002 | — | want | a | §3.7/§5.3 — **Delete Empty Layers**: one-click housekeeping. TBD mints a layer lazily on first place (`outliner.rs:17`), so empty layers accumulate by construction — this is the cleanup verb for a problem TBD already has | T-666 |
| 3DEN-LYR-003 | ATTR-FIELD-LYR-ENABLE-VIS | want | a | §3.7 — Show Layer (visibility). Exactly T-665's first half; the mod proxies vanilla's action, which confirms makers use it enough to want it in two places | T-665 |
| 3DEN-LYR-004 | ATTR-FIELD-LYR-ENABLE-XFORM | want | a | §3.7 — Enable Layer (transform lock). T-665's second half | T-665 |
| 3DEN-LYR-005 | — | maybe | a | §5.3 — The same two commands **also** as buttons in the left panel's edit toolbar. Two entry points for one verb is right in Eden's 3D chrome; in TBD's dock it may just be duplication — decide in T-666 | T-666 |
| 3DEN-LYR-006 | — | maybe | a | §8.5 — `ENH_DefaultLayer`: a **deliberately invisible** attribute (`conditionScript = "false"`) persisted to `mission.sqm` so the default-layer choice survives a reload. TBD's `active_layer` is editor-session state (`editor_ops.rs:1025`) and does not persist. Cheap to add; nobody has asked | T-666 |
| 3DEN-LYR-007 | — | want | a | §4.1/§9 — **Move to layer…**: a dialog that bulk-reassigns the whole selection to a chosen layer, **with an auto-focused search box**. TBD's refile is per-slot drag (`editor_ops.rs:2142` `begin_refile`, `:2161` `refile_slot`) with no bulk path and no picker | T-666 |

### 2.9 `3DEN-CTX` — right-click context-menu additions (§4)

**TBD has no context menu at all** — `oncontextmenu` is `prevent_default()` and nothing else
(`mission_editor.rs:1844-1847`), because RMB is a pan button. T-662 frees it and **T-664 builds the
menu**; every row here is menu content that lands after T-664, and the `ticket` column names where the
*behaviour* lives.

| 3den_id | tbd_id | want | build_class | notes | ticket |
|---|---|---|---|---|---|
| 3DEN-CTX-001 | — | want | a | §4.1/§2.6 — **Add to Favorites** (context root): add the selection to a persistent Favorites tree. TBD has no favourites of any kind — a word-boundary grep for `favorite` / `favourite` over the SPA returns one hit, prose in `deployments.rs:8` | new — E3 |
| 3DEN-CTX-002 | — | no | d | §4.1 — **Module Information**: describes a selected system/module entity that has no Eden description. TBD has no module/system family — `gap_analysis.md` scores `RIGHT-MODE-005` un-enumerable | — |
| 3DEN-CTX-003 | — | maybe | a | §4.2/§2.8 — Log **Faction Names** to clipboard. TBD's factions are typed and already visible in the ORBAT dock (`orbat_manager.rs:296-315`); the clipboard path adds little | new — E5 |
| 3DEN-CTX-004 | — | maybe | a | §4.2/§2.1 — Log **Object Info**: class, kind-of, selection names, config parents, model, materials, textures, animations, **and the bounding box**. Only the bbox has a TBD subject (see `3DEN-MEAS-008`) — the rest is A3 config introspection | new — E5 |
| 3DEN-CTX-005 | — | want | a | §4.2 — Log **Classes as String** to clipboard (unique class names of the selection). TBD's analogue is `resource_name`, and "give me the asset ids I just placed" is a real modpack-debugging workflow | new — E5 |
| 3DEN-CTX-006 | — | want | a | §4.2 — Log **Positions (3D)** to clipboard. §2.3 calls this "the closest thing to a height readout" the mod has — for TBD it is simply the export path for a selection's coordinates | new — E5 |
| 3DEN-CTX-007 | — | want | a | §4.2 — Log **Positions (2D)** to clipboard | new — E5 |
| 3DEN-CTX-008 | — | want | a | §4.2 — Log **Grid Position** (`mapGridPosition`) to clipboard. The highest-value member of the Log folder for a milsim: grid refs are how orders are written, and T-667 is already adding edge grid labels | new — E5 |
| 3DEN-CTX-009 | — | maybe | a | §4.2 — Log **3DEN Entity IDs**. TBD's analogue is the stable slot `uid`; useful for support tickets, invisible to makers | new — E5 |
| 3DEN-CTX-010 | — | no | b | §4.2 — Log **Variable Names**. TBD slots have no variable name — `ATTR-FIELD-OBJ-UNIT-NAME` is contract-blocked and routes to the workbench program (T-674) | — |
| 3DEN-CTX-011 | — | maybe | a | §4.3/§9 — **Delete Crew**: strip the crew from selected vehicles. TBD vehicles carry no crew yet (`RIGHT-CREW-001`, `\bcrew\b` → 0 in the SPA); this becomes buildable the moment T-076 lands, and is meaningless before | T-076 |
| 3DEN-CTX-012 | — | no | c | §4.4 — **Set Player as Trigger Owner**. Triggers are an unbuilt Enfusion runtime (T-676) | — |
| 3DEN-CTX-013 | — | want | a | §4.5/§9 — **Selection Filter**: narrow the *current selection* by type / side / faction / class. TBD's side and faction are typed fields already resident in the doc, and there is no way to say "of these 200, keep the OPFOR riflemen". Pairs with `SEL-ALL-001` (T-649) — filter is what makes select-all safe | new — E4 |
| 3DEN-CTX-014 | — | have | a | §4.6/§2.9 — **Move camera here**, overridden to also centre the 2D map. TBD is the 2D map, and `Space` centres it on the selection — `mission_editor.rs:1026` → `editor_ops.rs:354` `center_on_selection` | — |

### 2.10 `3DEN-CHROME` — status bar, panels, scene overlays (§5)

| 3den_id | tbd_id | want | build_class | notes | ticket |
|---|---|---|---|---|---|
| 3DEN-CHROME-001 | — | no | d | §5.1/§10 — **Camera direction** readout with rotation icon (`IDC_STATUSBAR_CAMDIR`). TBD's camera is a fixed top-down `OrthoCamera` (`select_tool.rs:86-96`) — north is always up, so a heading readout would be a constant | — |
| 3DEN-CHROME-002 | — | maybe | a | §5.1 — **Session timer**: elapsed editing time this session. Charming, and there is a real use (billing a contract mission, pacing a build session). Not load-bearing | new — E6 |
| 3DEN-CHROME-003 | — | want | a | §5.1 — **Per-type entity counters with icons**: Objects, Groups, Triggers, Waypoints, Systems, Markers. TBD shows **one** total plus selection — `eden_chrome.rs:3741-3766` (`OBJ`/`SEL`/`SZ`). T-659 already specifies per-**side** counts; per-**type** is the same widget and the mod is evidence both are wanted | T-659 |
| 3DEN-CHROME-004 | — | maybe | a | §5.1/§7.1 — Show Entity Counter as a preference (default on). Only meaningful once a preferences store exists | new — E6 |
| 3DEN-CHROME-005 | — | no | a | §5.1 — Product version on status-bar hover. TBD ships from one build; there is no version skew to diagnose | — |
| 3DEN-CHROME-006 | — | want | a | §5.2/§2.6 — **Favorites tab**: a third right-panel tab beside Assets and History, with its own search box, collapse-all and delete. TBD's right dock already has Factions / Vehicles / Objects / Markers tabs (`eden_chrome.rs:2961-3007`) — this is a fourth, over a catalogue of thousands of prefabs | new — E3 |
| 3DEN-CHROME-007 | — | maybe | a | §5.2 — Favorites **hover preview picture** overlay. Depends on the registry carrying preview images; `INFERRED:` TBD's `RegistryItem` has no image field today | new — E3 |
| 3DEN-CHROME-008 | — | no | d | §5.4/§2.9/§10 — **Minimap**: camera-following map inset in the 3D scene, altitude-scaled zoom, rotating camera icon, auto-hide. **TBD is the map.** The entire feature exists to give a 3D editor the view TBD starts from | — |
| 3DEN-CHROME-009 | — | no | d | §5.4/§7.1/§10 — **Show DLC Icons**: draws each nearby object's source-mod logo in the 3D scene. TBD runs one modpack; the ownership question does not arise | — |
| 3DEN-CHROME-010 | — | maybe | b | §5.4/§7.1/§11 — **Custom marker shape preview** on hover. Blocked behind markers existing at all (T-069) and marker style being schema-widened (T-673) | T-673 |

### 2.11 `3DEN-KEY` — keybinding policy (§6)

Bindings for features owned elsewhere are **not** re-minted here — §6 is almost entirely a shortcut
index for §3. What is left is policy, and one genuinely transferable piece of engineering.

| 3den_id | tbd_id | want | build_class | notes | ticket |
|---|---|---|---|---|---|
| 3DEN-KEY-001 | — | no | a | §6.2 — **`G` is a bare unmodified key** for Garrison. The mod's own §6 conflict note flags it. TBD's editor is full of text inputs guarded by `in_editable_field()` (`mission_editor.rs:1010`); a bare letter binding is a trap, and T-648's keymap pass should say so | — |
| 3DEN-KEY-002 | — | want | a | §6 — The repo ships `ENH_fnc_checkShortCutsDuplicates`, an **internal collision detector for its own shortcut table**. `interactions_sweep.md` found five collisions in TBD's keymap. A unit test asserting no two editor bindings collide is hours of work and permanently closes the class | new — E10 |
| 3DEN-KEY-003 | — | no | a | §6.2 — Align / orient bound to `CTRL+ALT+NUM…` and `CTRL+NUM…`. Numpad-only bindings are unreachable on the laptops mission makers use; T-645 should not copy them | — |
| 3DEN-KEY-004 | — | maybe | a | §6.5 — Rich **per-dialog keymaps** (Functions Viewer `1`–`4` view modes, ESE `1`–`6` quantities, Briefing Editor `CTRL+1..6` tag insert). The right instinct — modal keymaps inside modal dialogs — but each one is a fresh collision surface. Adopt per-dialog, not as policy | — |
| 3DEN-KEY-005 | — | want | a | §14 — `ENH_fnc_initSearchControls`: **one shared search-box helper, `CTRL+F`, used by most dialogs**. TBD has search in three places already (asset catalogue per-tab, arsenal filter `arsenal.rs:699`, and whatever E4 adds) with no shared component or shared key | new — E8 |

### 2.12 `3DEN-PREF` — the Preferences block (§7.1, §7.2, §7.4)

**TBD has no editor preferences at all** — `grep -rniw Preferences apps/website/frontend/src` returns
zero. Everything user-scoped in the mod is `profileNamespace`-backed and never written to the mission,
which is exactly the seam TBD is missing: today the 12 world-layer toggles and the time scrubber are
per-*mission* (`world_layer_prefs.rs:63-74`, `eden_chrome.rs:1084`), so a viewing preference is stored
as mission data.

| 3den_id | tbd_id | want | build_class | notes | ticket |
|---|---|---|---|---|---|
| 3DEN-PREF-001 | — | want | a | §7 — **A per-user, profile-backed preferences store that is never written to the mission.** ~55 settings live in it. This is the enabling row for -002…-005, `3DEN-CHROME-002/004`, `3DEN-MISC-002` and `3DEN-CTX-001` — none of which can be built cleanly without it | new — E6 |
| 3DEN-PREF-002 | — | maybe | a | §7.1/§2.7 — Collapse Asset Browser on Start. Startup state for a dock TBD cannot yet collapse at all; blocked behind T-638 | T-638 |
| 3DEN-PREF-003 | — | maybe | a | §7.1/§2.7 — Collapse Entity List on Start. Same gate | T-638 |
| 3DEN-PREF-004 | — | maybe | a | §7.1/§2.7 — Show Left Panel on Start | T-638 |
| 3DEN-PREF-005 | — | maybe | a | §7.1/§2.7 — Show Right Panel on Start. §2.7 records that the mod adds **no runtime hotkey** to toggle panels — startup state only. T-638 is already the better design (keys `E`/`R`/`Backspace`), so take the persistence and not the shape | T-638 |
| 3DEN-PREF-006 | — | no | d | §7.1 — Minimap Size (Disabled / Small / Medium / Large). See `3DEN-CHROME-008` | — |
| 3DEN-PREF-007 | — | no | d | §7.1 — Minimap Scale multiplier | — |
| 3DEN-PREF-008 | — | no | a | §7.1/§11 — **Adjust Title Width**: progressively truncates attribute labels with `...` and re-fits them so they never overflow. A layout engine solving a problem CSS solves; TBD's nine-field modal does not have it | — |
| 3DEN-PREF-009 | — | no | d | §7.1/§13.2 — Command Palette Path (custom commands JSON; **requires Pythia**) | — |
| 3DEN-PREF-010 | — | no | d | §7.4/§10 — **Enable Dynamic View Distance**: camera Z 0–2000 m maps to view distance 200–12000 m. A 3D render-budget control with no orthographic analogue | — |
| 3DEN-PREF-011 | — | have | a | §7.4/§2.8/§11 — **Backup mission.sqm on every save and autosave**. TBD banks pre-adopt / pre-restore snapshots per document in IndexedDB — `yrs_persist.rs:180`, `:490` (T-191), plus immutable server-side versions. Stronger than the mod's, which needs Pythia | — |
| 3DEN-PREF-012 | — | no | d | §7.4 — Path for mission backups. TBD's store is IndexedDB + the versions API; there is no filesystem to point at | — |
| 3DEN-PREF-013 | — | no | c | §7.4 — Hold Action Icons: an editable list of extra icon paths for the Hold Action attribute. Blocked behind `3DEN-ATTR-006` | — |

### 2.13 `3DEN-CAM` — camera, view and map tools not defined elsewhere (§10)

| 3den_id | tbd_id | want | build_class | notes | ticket |
|---|---|---|---|---|---|
| 3DEN-CAM-001 | — | want | a | §2.6/§5.3/§10 — **Custom Locations**: ➕ saves the current camera position and vector under a typed name, **per terrain**, persisted in the profile; double-click flies there; 🗑 deletes. TBD has no editor bookmarks — `grep -rniw bookmark` over the SPA hits only the mission library (`missions.rs:73`). On a 12.8 km map where the camera is the only navigation, this is the cheapest large quality-of-life win in the catalogue | new — E1 |
| 3DEN-CAM-002 | — | want | a | §2.6 — **Enhanced Locations panel**: auto-generated categories scanned from the terrain — Chapels, Churches, Fuel Stations, Power Production, Shipwrecks, Transmitters, Airports. TBD already classifies 391 prefabs with a data-driven taxonomy (T-090.3.3) and renders town labels + an airfield layer (`world_layer_prefs.rs:71-73`), so the index is derivable from data it already ships | new — E1 |

### 2.14 `3DEN-ATTR` — attribute-window additions (§8)

**~310 attributes, and this is where the sweep says `no` most often.** The reason is structural and
was verified rather than assumed: TBD's per-slot attribute surface is **nine fields** (`SlotAttrs`,
`editor_ops.rs:122-132`; `read_attrs` `:631-663`), the States tab is a stub that *cannot* read a slot
(`fn states_tab() -> impl IntoView` takes no arguments — `attributes.rs:351`), and 25
`additionalProperties: false` in the compiled schema close the door on new keys. Most of §8 has
nothing to attach to, and building the attribute would mean building the runtime that reads it first.

#### §8.1 — object / entity attributes, new categories

| 3den_id | tbd_id | want | build_class | notes | ticket |
|---|---|---|---|---|---|
| 3DEN-ATTR-001 | — | no | c | §8.1 — **Advanced Damage**: enumerates every hitpoint of the selection and applies per-hitpoint `setHitPointDamage` at start. Needs a damage model in `apps/mod/tbd-framework` first | — |
| 3DEN-ATTR-002 | — | no | c | §8.1 — **Ambient Animations**: animation-set picker looping via an `AnimDone` EH, with break-out on damage/death/COMBAT. No animation-authoring concept | — |
| 3DEN-ATTR-003 | ATTR-FIELD-OBJ-SKILL | no | c | §8.1 — **AI Skill**: 10 `setSkill` sliders. `gap_analysis.md` already scores the Eden id `na` for the same reason — every TBD body spawns AI-disabled (`TBD_SpawnManager.c:963,1166`), so `skill` has no subject | — |
| 3DEN-ATTR-004 | — | no | c | §8.1 — **AI Features**: 21 `disableAI` checkboxes (Move, Target, Cover, Autotarget, **Raycasts**, Path, …). Same gate. Note §2.2: the "Raycasts" entry is `disableAI "CHECKVISIBLE"` — a behaviour toggle, **not** a line-of-sight tool | — |
| 3DEN-ATTR-005 | — | want | b | §8.1 — **Unit Traits**: Is Medic · Is Engineer · Is Explosive Specialist · Is UAV Hacker · Camouflage / Audible / Load / Stamina coefficients. TBD's States tab **advertises exactly these two** and delivers nothing: `"Medic (soon)"` / `"Engineer (soon)"` (`attributes.rs:358-363`). Schema-blocked (`$defs/slot` closed), and framework_synthesis **U5** wants the *derived badge* rather than a stored string — build the typed traits, derive the badge | T-674 (wb) |
| 3DEN-ATTR-006 | — | no | c | §8.1 — **Hold Action** builder: 13 fields wrapping `BIS_fnc_holdActionAdd` (condition-show, condition-progress, four code slots, duration, radius…). A scripting surface with no scripting layer | — |
| 3DEN-ATTR-007 | — | no | d | §8.1 — **Object Events**: 73 per-object event-handler **code** slots (AmmoExplodedNear … WeaponRested), each gated to relevant entity types. [Collapse row 1 of 5 — 145 EH slots total] | — |

#### §8.1 — object attributes injected into vanilla sections

| 3den_id | tbd_id | want | build_class | notes | ticket |
|---|---|---|---|---|---|
| 3DEN-ATTR-008 | — | no | c | §8.1 Special States — Enable Captive Mode (`setCaptive`) | — |
| 3DEN-ATTR-009 | — | no | c | §8.1 Special States — Allow Sprinting (`allowSprint false`) | — |
| 3DEN-ATTR-010 | — | no | c | §8.1 Special States — Force Walking (`forceWalk`) | — |
| 3DEN-ATTR-011 | — | no | c | §8.1 Special States — Make Hostage: surrender anim + MOVE disabled + captive + a "Free Hostage" hold action. Three unbuilt runtimes in one attribute | — |
| 3DEN-ATTR-012 | — | no | c | §8.1 Special States — Start in Parachute (spawns a steerable chute at 150 m) | — |
| 3DEN-ATTR-013 | — | no | c | §8.1 Special States — Enable Headlights (`setPilotLight`) | — |
| 3DEN-ATTR-014 | — | no | c | §8.1 Special States — Forbid Disembarking (`allowCrewInImmobile` + crew FSM off + CARELESS) | — |
| 3DEN-ATTR-015 | — | no | c | §8.1 Special States — Engine on/off at start | — |
| 3DEN-ATTR-016 | — | no | c | §8.1 Special States — Disable NVG Equipment | — |
| 3DEN-ATTR-017 | — | no | c | §8.1 Special States — Disable Thermal Optics. **Prior rejection on record:** T-193 removed View Distance and Thermals from TBD, and T-663 exists to delete the dead DTO fields — do not re-add them from this catalogue | — |
| 3DEN-ATTR-018 | — | no | c | §8.1 Special States — Stay on position (`doStop`) | — |
| 3DEN-ATTR-019 | — | no | c | §8.1 Special States — Disable Deletion on Death (`removeFromRemainsCollector`) | — |
| 3DEN-ATTR-020 | — | no | d | §8.1 Special States — Single Player Respawn Tickets. framework_synthesis **C.4** rejects tickets outright: 4/4 corpus frameworks have none and TBD reached one-life independently (`eden_chrome.rs:3916-3960`) | — |
| 3DEN-ATTR-021 | — | no | d | §8.1 Transformation — **Scale** (`setObjectScale`) with live editor preview. Two blockers: Enfusion has no per-instance scale in TBD's pipeline, and a top-down 2D map cannot show the result | — |
| 3DEN-ATTR-022 | — | no | c | §8.1 Inventory — Add Gun Light (forces the gun light on, adding `acc_flashlight` if needed) | — |
| 3DEN-ATTR-023 | — | no | d | §8.1 Inventory — **Arsenal**: attaches a full Virtual Arsenal to the object at runtime. framework_synthesis **C.4** rejects a play-time arsenal 4/4; TBD's arsenal is an authoring tool by design | — |
| 3DEN-ATTR-024 | — | no | d | §8.1 State — **Visibility** (`setFeatureType`). §2.2 flags this as the mod's most convincing LoS false positive: it is a *draw-distance* setting | — |
| 3DEN-ATTR-025 | — | no | c | §8.1 State — Turret Stabilization (`enableGunStabilization`) | — |
| 3DEN-ATTR-026 | — | no | c | §8.1 State — Add Flag (`forceFlagTexture` without a flag holder) | — |
| 3DEN-ATTR-027 | — | no | c | §8.1 State — Unladen Weight (`setMass` as 0–100 % of config mass) | — |
| 3DEN-ATTR-028 | — | no | c | §8.1 State — Leakage (`setWaterLeakiness`) | — |
| 3DEN-ATTR-029 | — | no | c | §8.1 State — Speed Limit (`limitSpeed`, AI-driven vehicles only) | — |
| 3DEN-ATTR-030 | — | no | c | §8.1 State — Fuel Consumption Coefficient. Note T-680 (workbench) already owns vehicle lock/fuel/ammo — that is the slice this would join, not a new one | — |

#### §8.2 — mission attributes

| 3den_id | tbd_id | want | build_class | notes | ticket |
|---|---|---|---|---|---|
| 3DEN-ATTR-031 | ENV-SETTINGS-002 | no | b | §8.2 Intel — View Distance (`setViewDistance`). Removed from TBD by T-193 on purpose; `eden_chrome.rs:4624` refuses to author it | — |
| 3DEN-ATTR-032 | — | no | b | §8.2 Intel — Object View Distance. Same rejection | — |
| 3DEN-ATTR-033 | — | no | d | §8.2 Intel — Terrain Detail (`setTerrainGrid`, Grass disabled → Ultra). An A3 clutter-density control | — |
| 3DEN-ATTR-034 | — | no | c | §8.2 Date — Time Multiplier (`setTimeMultiplier` 0.1–120) | — |
| 3DEN-ATTR-035 | — | no | c | §8.2 Scenario — **Airdrop**: classes + centre + condition + altitude + radius + side, spawned under parachutes when the condition passes. A mission-runtime feature, not an editor one | — |
| 3DEN-ATTR-036 | — | no | c | §8.2 Scenario — **Ambient Flyby**: looping captive aircraft between two points. Same class | — |
| 3DEN-ATTR-037 | — | maybe | c | §8.2 Scenario — **Mission Ending (Casualties)**: end the mission for all clients when a side passes an N% casualty threshold. A real, well-specified end condition and TBD has objectives work coming (framework_synthesis **B5**) — but it is Enfusion runtime, and B5 says that belongs in its own program | — |
| 3DEN-ATTR-038 | — | no | d | §8.2 Scenario — Establishing Shot (`BIS_fnc_establishingShot`) | — |
| 3DEN-ATTR-039 | — | maybe | c | §8.2 Scenario — **Intro Text**: delay + three lines + type (text tiles / infoText / SITREP). Cheap presentation polish; needs a mod-side reader and competes with real briefing work (T-671) | — |
| 3DEN-ATTR-040 | — | no | d | §8.2 Scenario — Single Player Respawn ruleset. Same rejection as -020 | — |
| 3DEN-ATTR-041 | — | no | d | §8.2 Scenario — Music / Sound / Radio / Environment volume sliders (`fadeSound`) | — |
| 3DEN-ATTR-042 | — | no | d | §8.2 Scenario — Random Music from a chosen CfgMusic set | — |
| 3DEN-ATTR-043 | — | want | a | §8.2 Scenario — **Briefing**: per-side diary sections. TBD's schema already carries `briefings[faction] = {situation, mission, execution, markers[]}` and **nothing writes it** (`mission_hydrate.rs:525`). The mod's 4×4 grid is 3/4 of that shape, independently arrived at. [Collapses 12 of the 16 fields] | T-671 |
| 3DEN-ATTR-044 | — | want | b | §8.2 Scenario — Briefing **Signal** section — the comms plan, the one section TBD's schema does **not** have. Two communities in framework_synthesis §A.6 independently invented a uniform-recognition section; this is the same finding for comms. A one-key widening of `$defs/briefing`. [Collapses 4 of the 16 fields] | new — E12 (wb) |
| 3DEN-ATTR-045 | — | no | d | §8.2 Scenario — Briefing fields accept stringtable keys (`BIS_fnc_localize`). TBD has no stringtable | — |
| 3DEN-ATTR-046 | — | no | d | §8.2 Scenario — **Mission Events – Global**: 42 `addMissionEventHandler` code slots. [Collapse row 2 of 5] | — |
| 3DEN-ATTR-047 | — | no | d | §8.2 Scenario — **Mission Events – Server**: 9 server-only code slots. [Collapse row 3 of 5] | — |
| 3DEN-ATTR-048 | — | no | d | §8.2 Scenario — **Music Events – Global**: MusicStart / MusicStop code slots. [Collapse row 4 of 5] | — |
| 3DEN-ATTR-049 | — | no | a | §8.2 Misc — Disable mission.sqm Backup, per-scenario opt-out. TBD's persistence is not opt-outable and should not be | — |
| 3DEN-ATTR-050 | — | no | d | §8.2 Misc — Editable Objects (Zeus): Disabled / Editor-placed only / All. No curator | — |
| 3DEN-ATTR-051 | — | no | c | §8.2 Misc — Map Indicators (`disableMapIndicators [friendly, enemy, mines, ping]`). Affects the **in-game** map, not the editor — an easy misread | — |
| 3DEN-ATTR-052 | — | no | c | §8.2 MP — Dynamic Groups | — |
| 3DEN-ATTR-053 | — | no | c | §8.2 MP — Dynamic AI Skill Settings, min/max per side scaled by player count | — |
| 3DEN-ATTR-054 | — | no | d | §8.2 MP — Respawn Tickets ×4 sides. framework_synthesis **C.4** also directs removing TBD's own dead `settings.respawn: "tickets"` enum value | — |
| 3DEN-ATTR-055 | — | no | c | §8.2 MP — Save Loadout (Disabled / Original / Death / Arsenal) | — |

#### §8.3 — group attributes

| 3den_id | tbd_id | want | build_class | notes | ticket |
|---|---|---|---|---|---|
| 3DEN-ATTR-056 | — | maybe | b | §8.3 — **Group Marker**: a local marker that follows the group leader, updated every second, optionally appending unit count and vehicle name, deleted when the group empties. TBD already draws squad leader→member links (`draw_order.rs:57`); a *labelled* group marker is the increment. Blocked behind markers (T-069) | T-069 |
| 3DEN-ATTR-057 | — | no | c | §8.3 — **Patrol** (`BIS_fnc_taskPatrol` radius around the leader). Waypoints are already blocked on AI existing at all (T-677) | — |
| 3DEN-ATTR-058 | — | no | d | §8.3 — **Group Events**: 19 group event-handler code slots. [Collapse row 5 of 5 — 73+42+9+2+19 = 145] | — |

#### §8.4 — marker attributes

| 3den_id | tbd_id | want | build_class | notes | ticket |
|---|---|---|---|---|---|
| 3DEN-ATTR-059 | — | no | d | §8.4 Transformation — **Position gains Z**: vanilla control replaced with `EditXYZ` so markers can be placed in 3D. TBD markers are `{x, z, icon, label}` on a top-down map; a third dimension has nothing to render into | — |
| 3DEN-ATTR-060 | — | want | b | §8.4 Transformation — **Priority** (`setMarkerDrawPriority`): controls which marker draws on top when markers overlap. The single most useful map-legibility feature in the mod, and it is **not among T-673's six ids** — add it there. TBD already has a formal lane-ordering system (`draw_order.rs`), so the render half is nearly free; the schema key is the blocker | T-673 |
| 3DEN-ATTR-061 | ATTR-FIELD-MRK-SHAPE | maybe | b | §8.4 Style — **Special Shape**: 8 polygons (Triangle → Decagon) beyond vanilla's set, not applicable to Icon markers. Already one of T-673's six ids. Whether 8 polygons beat TBD's closed 64-icon vocabulary is a design question, not a build one | T-673 |
| 3DEN-ATTR-062 | ATTR-FIELD-MRK-COLOR | maybe | b | §8.4 Style — **Marker Color (RGBA)**: replaces vanilla's colour picker with a hex field + four R/G/B/A sliders + a live swatch. Already in T-673. `maybe` because free colour fights framework_synthesis **K10** — closed vocabularies are what stopped WOG's marker types rotting | T-673 |
| 3DEN-ATTR-063 | — | want | a | §8.4 Style — **Saveable colour preset history** on the picker. This half is editor-local, needs no schema, and is the part that actually produces consistent-looking maps across a community — take it even if -062 stays closed-vocabulary | new — E6 |
| 3DEN-ATTR-064 | — | no | c | §8.4 — **Hide on Start**: stores the original alpha and sets alpha 0 at mission start | — |
| 3DEN-ATTR-065 | — | no | c | §8.4 — Hide on Start ▸ **Condition**, re-evaluated every 0.5 s, restores the original alpha when true. A scripted condition with no scripting layer | — |

#### §8.5–§8.6 — layer attribute and attribute removal

| 3den_id | tbd_id | want | build_class | notes | ticket |
|---|---|---|---|---|---|
| 3DEN-ATTR-066 | — | no | a | §8.6 — **53 `remove_*` optional PBOs**: move one into `addons/` and that attribute disappears from the UI. A per-user mitigation for attribute-window overload — a problem TBD's nine-field modal does not have. [Collapses all 53] | — |
| 3DEN-ATTR-067 | — | no | a | §8.6 — **Five of the 53 are stale and silently do nothing** because they name classes that no longer exist (incl. a case-duplicate pair, `remove_marker_markercolor` / `…markerColor`). Recorded as an **anti-pattern**, not a feature: this is the same class of defect as framework_synthesis's "a checks-passed state that was never watched fail" | — |

### 2.15 `3DEN-QOL` — behaviours with no menu entry (§11)

Small, cheap, and disproportionately what makes the mod feel finished. Rows already defined elsewhere
(default layer, palette learning, backup, title width, marker preview, minimap behaviours) are cited
there and not re-minted.

| 3den_id | tbd_id | want | build_class | notes | ticket |
|---|---|---|---|---|---|
| 3DEN-QOL-001 | — | want | a | §11/§2.6 — **Entity list tooltips**: on mouse-enter, every tree row's tooltip is set to its own text, so a clipped name or variable name is still readable. TBD's outliner and ORBAT dock are both narrow trees full of clippable names (`outliner.rs`, `orbat_manager.rs:296-315`). Hours of work | new — E8 |
| 3DEN-QOL-002 | — | maybe | a | §11/§2.6 — **Asset-browser mod filter cleanup**: rebuilds the mod dropdown omitting addons that contain no units or weapons. TBD runs one modpack, so the subject is thin — but the *principle* (never offer a filter value that selects nothing) generalises to the side chips and E4's filters | — |
| 3DEN-QOL-003 | — | want | a | §11/§2.6 — **Favorites persist across sessions** in `profileNamespace`. The half of `3DEN-CTX-001` that makes it worth building; needs `3DEN-PREF-001` | new — E3 |
| 3DEN-QOL-004 | — | no | d | §11/§2.7/§2.9 — **Panel-aware minimap**: shifts 54 grid units when the left panel is hidden. See `3DEN-CHROME-008` | — |
| 3DEN-QOL-005 | — | no | d | §11/§2.9 — **Arsenal-aware minimap**: auto-hides during preview, when the full map is open, and when a BI or ACE Arsenal is up | — |
| 3DEN-QOL-006 | — | have | a | §11 — **No player dependency**: attribute effects are baked into `mission.sqm` on save, so published scenarios need neither the mod nor CBA. TBD's equivalent is stronger by construction — the editor compiles to a versioned JSON contract validated server-side (`apps/website/api/src/contract/validate.rs`) and the mod reads that; there is no client-side authoring plugin to be missing | — |
| 3DEN-QOL-007 | — | no | a | §11 — The **caveat** attached to the above: reopening such a mission *without* the mod loaded **strips the previously set attributes**. Recorded as an anti-pattern TBD has already avoided — paste re-merges unknown keys (`store.rs:2571-2582` pins the known set; `:1496` is the extras branch) and hydrate loads `editor.slots` opaquely (`:1832-1838`), so an unknown key survives a round trip | — |
| 3DEN-QOL-008 | — | no | d | §11 — CBA versioning when CBA is present | — |

### 2.16 `3DEN-EXT` — extensibility surfaces and companion addons (§13, §14)

| 3den_id | tbd_id | want | build_class | notes | ticket |
|---|---|---|---|---|---|
| 3DEN-EXT-001 | — | no | d | §13.1 — **58 event scripts**: drop `<EVENT>.sqf` into `missionRoot/.enh_eventScripts/` and it auto-runs on that Eden event. A per-mission scripting extension point; TBD's editor has no scripting host and adding one is a security surface, not a feature | — |
| 3DEN-EXT-002 | — | no | d | §13.1 — The **grid/snap hooks specifically** — `onGridChange`, `onMoveGridIncrease/Decrease/Toggle`, `onRotateGrid*`, `onScaleGridToggle`, `onSurfaceSnapToggle`. Worth naming separately because §2.4 uses their existence to argue the mod is strong on grids: **it is not**. It adds no grid system; it exposes vanilla's. TBD has no grid at all (T-648) | — |
| 3DEN-EXT-003 | — | maybe | a | §13.3 — **Menu-strip self-documentation**: every menu entry in the source carries a `wikiDescription`, and the wiki is generated from config by `ENH_fnc_exportMenuStripToGitHub`. The right instinct — UI definition as documentation source of truth, which is what TBD's `@contract`/`@route` tags already do for the API. **But §Sourcing is the counter-evidence in the same document**: the generated wiki drifted and now documents ten removed features. Generate from source *and* delete on removal, or do not generate | — |
| 3DEN-EXT-004 | — | no | d | §13.4 — `ace_arsenal_shortcut` companion addon | — |
| 3DEN-EXT-005 | — | no | d | §13.4 — `captureframeui`: improves BI's Capture Frame window with a search box, collapse/expand-all, an "Open Perfetto…" link and a solid background. Config-only, and about profiling A3's UI | — |
| 3DEN-EXT-006 | — | maybe | a | §14 — **19 custom dialogs as standalone windows** rather than inline panels. TBD is already dialog-heavy (Mission Settings, ORBAT Manager, Attributes, Faction Manager) and T-668 is about making them feel like one product; the mod is prior art for both the shape and its cost | T-668 |
| 3DEN-EXT-007 | — | no | a | §14 — **Published dead-code register**: `IDD_VAM`, `IDC_CUSTOMIZE_MENU_STRIP`, `IDC_SHORTCUTS_*`, `IDC_GARRISON_*`, `IDC_PLACEMENTTOOLS_FINECONTROL/CENTERX/CENTERY`, `IDC_DEBUGOPTIONS_FPS` — defined, referenced nowhere. A process note, not a feature; TBD's equivalent is T-663 | — |

### 2.17 `3DEN-GONE` — documented but removed (§12)

**All `no`, all `d`, all for the same reason.** The mod's auto-generated wiki still lists several of
these; the catalogue was rebuilt from v8.8.0 config source precisely so this sweep would not inherit
them. Nothing here is available to borrow.

| 3den_id | tbd_id | want | build_class | notes | ticket |
|---|---|---|---|---|---|
| 3DEN-GONE-001 | — | no | d | §12 — **Toggle Marker Alpha** (was `V`): forced all markers to alpha 1 so zero-alpha markers could be selected. Not in shipping source. *The idea* — a "reveal everything hidden" override so an invisible object stays selectable — is a good one and TBD will meet it at T-665 (layer visibility); it is recorded here as an idea, **not** as something the mod ships | — |
| 3DEN-GONE-002 | — | no | d | §12 — **Eden Shortcuts viewer** (was `ALT+F1`), a GUI listing every Eden shortcut. Not in shipping source | — |
| 3DEN-GONE-003 | — | no | d | §12 — **Toggle Minimap / Adjust Minimap Size** menu entries. Not in shipping source; the minimap is preferences-only now | — |
| 3DEN-GONE-004 | — | no | d | §12 — **Garrison Cover Map module** (`ENH_Garrison_AreaHelper`), an area-scaling helper for garrisoning. Not in shipping source — relevant because `3DEN-PLACE-022` might otherwise be scoped to include it | — |
| 3DEN-GONE-005 | — | no | d | §12 — **Vehicle hitpoint display tool**, superseded by the Advanced Damage attribute. Not in shipping source | — |
| 3DEN-GONE-006 | — | no | d | §12 — **Set entities to ATL/ASL 0**. Not in shipping source | — |
| 3DEN-GONE-007 | — | no | d | §12 — **Quick extraction setup**. Not in shipping source | — |
| 3DEN-GONE-008 | — | no | d | §12 — **Action Creator GUI**. Not in shipping source | — |
| 3DEN-GONE-009 | — | no | d | §12 — **Insignia attribute**. Not in shipping source | — |
| 3DEN-GONE-010 | — | no | d | §12 — **Show area markers in 3D via triggers**. Not in shipping source | — |

---

## 3. Counts

**245 rows.** Regenerate any number below with:

```bash
cd .ai/artifacts/parity
grep -cE '^\| 3DEN-' 3den_sweep.md                                                    # 245
grep -E '^\| 3DEN-' 3den_sweep.md | awk -F'|' '{gsub(/ /,"",$4); print $4}' | sort | uniq -c
grep -E '^\| 3DEN-' 3den_sweep.md | awk -F'|' '{gsub(/ /,"",$5); print $5}' | sort | uniq -c
```

### 3.1 By `want`

| want | n | share |
|---|---:|---:|
| **no** | 134 | 55 % |
| **want** | 60 | 24 % |
| **maybe** | 40 | 16 % |
| **have** | 11 | 4 % |

**The filter did its job.** 55 % of the mod is not wanted, and more than half of *that* is not even a
judgement call — it is 3D-editor or A3-engine surface with no subject in a 2D browser editor. Of the
60 `want`, **21 are already inside T-645's scope**, so the genuinely new decision surface is 39 rows.

### 3.2 By `build_class`

| class | n | share |
|---|---:|---:|
| **a** — SPA-buildable today | 111 | 45 % |
| **d** — n/a for a 2D browser editor | 76 | 31 % |
| **c** — mod-blocked | 47 | 19 % |
| **b** — schema-blocked | 11 | 4 % |

### 3.3 Cross-tab — the shape that matters

| | a | b | c | d | total |
|---|---:|---:|---:|---:|---:|
| **have** | 11 | 0 | 0 | 0 | **11** |
| **want** | 57 | 3 | 0 | 0 | **60** |
| **maybe** | 31 | 4 | 5 | 0 | **40** |
| **no** | 12 | 4 | 42 | 76 | **134** |
| **total** | **111** | **11** | **47** | **76** | **245** |

Three readings:

1. **`want` is almost entirely class `a`** — 57 of 60. Nothing worth taking from this mod is blocked
   on the schema or the mod except three rows (unit traits, marker draw priority, briefing Signal).
   This catalogue is a *factory* wishlist, not a workbench one, which is the opposite of what the
   attributes sweep produced.
2. **`no`/`d` (76) and `no`/`c` (42) are 88 % of the `no` pile**, and the `c` pile has essentially one
   cause: **AI**. Every TBD body spawns AI-disabled (`TBD_SpawnManager.c:963,1166`), which kills AI
   Skill, the 21 AI Features, Patrol, Debug Path, garrison pathfinding, dynamic AI skill and the
   Special-States behaviour block in one stroke.
3. **`no`/`a` is only 12** — the true "we could and we won't" boundary is small and is enumerated
   individually in §5.2.

---

## 4. Proposed ticket groupings, ranked by value per effort

Rule applied throughout: **map to an existing ticket or propose a new slice, never both.** The
`want`/`maybe` rows land in 11 new factory slices (`E1`–`E11`), one new workbench slice (`E12`), nine
existing tickets to extend, and two rows that belong to framework_synthesis items that were ranked but
never filed.

### 4.1 Existing tickets to extend — do these first, they are already queued

| Rank | Ticket | Rows added | What the sweep contributes | Effort |
|---:|---|---:|---|---|
| **1** | **T-645** — placement helpers | 21 `want` + 3 `maybe` | **The scope list.** T-645 was already scoped from this catalogue's executive summary; this sweep supplies the itemised contents (5 pattern modes, 4 align commands, 2 space commands, 6 orient commands, 7 garrison preferences) plus **three corrections**: drop Align Z± / Space Z (`3DEN-PLACE-017`/`-020` — they fight T-092.1's ground-everything contract), drop the numpad bindings (`3DEN-KEY-003`), and **scope garrison against the data first** — TBD's world export has building OBBs but `INFERRED:` no interior position list, so positions must be synthesised | Large (already sized) |
| **2** | **T-642** — ruler | 2 `want` + 1 `maybe` | Two cells, not a redesign: **on-foot travel time** (make the 14.15 km/h configurable) and the **2D/3D distance pair**. Tick marks at fixed spacing along a leg are the `maybe`. See §6.1 | Hours on a queued ticket |
| **3** | **T-666** — outliner layer authoring | 2 `want` + 3 `maybe` | **Delete Empty Layers** (TBD mints layers lazily, so empties accumulate by construction) and a **bulk Move-to-layer picker with an auto-focused search box** — TBD's refile is per-slot drag with no bulk path | Small |
| **4** | **T-649** — multi-edit | 2 `want` | Build the **batch-invert primitive** (`ENH_fnc_toggleAttributes` inverts any named boolean across a selection; nine commands route through one function) rather than per-field handlers; plus **batch naming** `PREFIX_INDEX` over `callsign` | Small |
| **5** | **T-671** — briefing | 2 `want` | The three schema-carried briefing sections, and the Briefing Editor's **reusable persistent templates** — makers write the same skeleton every time | Small–medium |
| **6** | **T-665** — layer flags | 2 `want` | Independent confirmation only: the mod re-exposes vanilla's Show Layer / Enable Layer in a second place, which is evidence makers reach for them constantly. No scope change | — |
| **7** | **T-659** — census badge | 1 `want` | Per-**type** counters (Objects / Groups / Triggers / Waypoints / Systems / Markers) alongside the per-**side** counts T-659 already specifies. Same widget | Small |
| **8** | **T-673** — marker style (workbench) | 1 `want` + 2 `maybe` | **Add `setMarkerDrawPriority` to T-673's six ids** — it is the single best map-legibility feature in the mod and it is currently in neither T-069 nor T-673. See §6.4 | Small widening |
| **9** | **T-674** — slot identity (workbench) | 1 `want` | **Unit Traits.** TBD's States tab literally advertises `"Medic (soon)"` / `"Engineer (soon)"` (`attributes.rs:358-363`) and cannot read a slot at all (`:351`). Typed traits belong with the identity widening; the *badge* is derived (framework U5) | Medium |

### 4.2 New slices — 11 factory + 1 workbench

Ranked by value per effort. `E8`, `E1`, `E5` and `E11` are the cheap ones and should go first.

| Rank | Slice | Rows | What it is | Class | Effort |
|---:|---|---:|---|---|---|
| **1** | **E8 — small-UI idioms** | 3 `want` | Three unrelated hours-each fixes that share a reviewer: **numeric-field nudge** (PageUp/PageDown at ±1/0.1/0.01/0.001 by modifier — TBD's `number_field` has none), **one shared search box on `CTRL+F`** across every dialog (TBD has three ad-hoc search inputs and no shared component), and **tree-row tooltips** so clipped outliner/ORBAT names stay readable | a | XS |
| **2** | **E1 — map bookmarks + locations index** | 2 `want` | Save/name/delete camera positions per terrain, double-click to fly; plus an auto-generated named-places index derived from TBD's existing 391-prefab taxonomy (chapels, fuel stations, transmitters, airfields). **The cheapest large QoL win in the catalogue** — on a 12.8 km map the camera is the only navigation and TBD has no bookmarks at all | a | S |
| **3** | **E5 — clipboard exporters** | 4 `want` + 4 `maybe` | One function over the selection, five outputs: **grid position** (the milsim-relevant one — grid refs are how orders are written, and T-667 is already adding edge grid labels), 2D positions, 3D positions, resource names, slot uids | a | S |
| **4** | **E11 — whole-terrain zone** | 1 `want` | One button that lays a play-area zone over the whole terrain. Every mission needs one; today it means drawing a 12.8 km polygon by hand | a | XS |
| **5** | **E7 — loadout buffer** | 3 `want` + 5 `maybe` | **Copy/apply loadouts between slots** — TBD has no path for this at all — with **random pick per entity** when several are buffered (a copy verb that becomes a variety verb for free), plus Clear All. The five per-category strip verbs are the `maybe` | a | S–M |
| **6** | **E4 — selection filter + document search** | 2 `want` | **Selection Filter** (narrow the current selection by type/side/faction/class — TBD's side and faction are typed and resident) and **document search** (find a slot by role/callsign/tag; TBD can only search the *catalogue* today). Filter is what makes T-649's Select All safe | a | M |
| **7** | **E9 — editor-only visibility override** | 1 `want` | "Switch Time": force the editor to max-visibility conditions **without touching the scenario**, reversible. Today TBD's time scrubber writes `meta`, so "make the map readable" and "set the mission's time" are the same control | a | S |
| **8** | **E6 — editor preferences store** | 2 `want` + 2 `maybe` | A per-user, never-compiled preferences store, plus the marker-colour **preset history**. Ranked here rather than higher because it is infrastructure — but **E1, E3, E9, `3DEN-CHROME-002/004` and `3DEN-PREF-002..005` all want it**, so build it as part of the first consumer rather than as its own ticket | a | M |
| **9** | **E3 — asset browser favourites** | 3 `want` + 1 `maybe` | A Favorites tab beside Factions/Vehicles/Objects/Markers, with its own search, and an Add-to-Favorites verb. Depends on E6 to persist | a | M |
| **10** | **E2 — command palette** | 1 `want` + 1 `maybe` | Fuzzy search over every editor command. The largest new build here and the highest ceiling: it is the standard web answer to *"the current UI is very inconsistent and very disjointed"*, which is the complaint **T-668** exists to fix. Usage-frequency ranking is the follow-on | a | M–L |
| **11** | **E10 — keybinding collision test** | 1 `want` | The mod ships `ENH_fnc_checkShortCutsDuplicates` to police its own shortcut table. `interactions_sweep.md` found **five collisions** in TBD's keymap. A unit test asserting no two editor bindings collide closes the class permanently | a | XS |
| **12** | **E12 — briefing Signal section** *(workbench)* | 1 `want` | A one-key widening of `$defs/briefing` for the comms plan — the only briefing section TBD's schema lacks. Ships after T-671 | b | S |

### 4.3 Two `want` rows with nowhere to go

`3DEN-LOAD-012` (ESE loadout **templates**) and `3DEN-LOAD-013` (ESE **import from clipboard**) are
framework_synthesis **U3** (rank 12) and **U2** (rank 6). Both are ranked in Part C and **neither has a
registry ticket**. This sweep reaches them from a second, independent source, which strengthens the
case. They are listed as `— (U2, unfiled)` / `— (U3, unfiled)` rather than assigned, because inventing
an `E` slice for work Part C already owns would duplicate it. **Recommendation: file U2 and U3.**

One caveat on U2, from §14: the ESE's `ImportFromClipboard` / `ImportToFilter` menu classes exist but
appear in **no `items[]` array**, so the mod's own clipboard-import path is shortcut-only at best.
Take the idea; do not take it as proof the shape works.

### 4.4 Where this triage disagrees with Part C

Part C's build list is authoritative and is not re-litigated. Four notes, in descending importance:

1. **Part C's C.5 ranking contains no placement row at all**, and that silence should not be read as a
   low ranking. Its evidence base was four *mission frameworks*, none of which is an editor mod, so
   placement helpers could never have entered it. This sweep produces **21 `want` rows for T-645** —
   more than for any other destination, and more than the next four destinations combined. T-645's own
   summary calls placement *"the single highest-yield borrow… and what mission makers actually use all
   day"*. Both statements can be true; the gap is coverage, not judgement.
2. **Part C ranks B4 (briefing authoring) #11 and calls it "blocked on markers (T-069) for links".**
   The mod's Briefing Editor shows the **template half is independent of markers** and shippable now
   (`3DEN-TOOL-010`). That is a refinement of the dependency, not a disagreement with the rank.
3. **Part C's U4 (aggregated settings view, rank 4) and the mod's Scenario Attributes Manager
   (`3DEN-TOOL-007`) are the same idea**, with the mod adding save/load templates on top. Scored
   `maybe` here rather than `want` **because TBD's settings surface is ~8 fields** — too thin to
   aggregate, let alone template. U4 is correctly ranked; the mod's version is what U4 becomes later.
4. **Full agreement on C.4.** This catalogue supplies four rows that would have re-opened decisions
   Part C closed — `3DEN-ATTR-023` (a play-time Virtual Arsenal attached to any object) and
   `3DEN-ATTR-020` / `-040` / `-054` (three separate respawn-ticket surfaces). All four are `no`,
   citing C.4. Recording this because "the wishlist told us to" is exactly how a closed decision
   re-opens.

---

## 5. What I scored `no`, and why — the scope boundary

134 rows. The boundary matters as much as the yes list, so here it is by cause rather than by row.

### 5.1 The four causes

| Cause | n | The shape of it |
|---|---:|---|
| **3D-editor or A3-engine surface** (`d`) | 76 | Split three ways: **affordances that exist to give a 3D editor what TBD starts with** (the minimap and its four satellites, building-position icons, DLC icons, `drawLine3D`, camera-direction readout, dynamic view distance, marker Z); **A3 engine concepts with no Enfusion analogue** (pylons ×3, `setFeatureType`, `setTerrainGrid`, `setObjectScale`, CfgMusic ×3, MP locality, stringtables); and **scripting handles with no scripting layer** (145 EH slots → 5 rows, Hold Action, 58 event scripts, the six BI utilities) |
| **Mod-blocked** (`c`) | 42 | **~30 of these have one cause: AI.** Every TBD body spawns AI-disabled (`TBD_SpawnManager.c:963,1166`), which takes out AI Skill's 10 sliders, the 21 AI Features, Patrol, Debug Path, garrison pathfinding, dynamic AI skill and most of the Special States block at once. The rest need a damage model, an animation system or a trigger runtime that does not exist |
| **Schema-blocked and deliberately so** (`b`) | 4 | View Distance and Object View Distance (**removed on purpose** by T-193; T-663 exists to delete the dead DTO fields), Playable State (every TBD slot *is* playable by construction), Variable Names (contract-blocked, routes to T-674) |
| **Buildable and still refused** (`a`) | 12 | Enumerated below — this is the only genuinely discretionary part of the boundary |

### 5.2 The 12 `no`/`a` rows — "we could, and we won't"

| Row | Refused because |
|---|---|
| `3DEN-MEAS-003` — 20-second notification as the measurement readout | A measurement you cannot re-read is not a measurement. T-642 rejects it by name |
| `3DEN-MEAS-006` — 5-second auto-expiry, not configurable | The defect T-642's persistent polyline exists to avoid |
| `3DEN-PLACE-014` — dialog hides the left panel and repositions on panel change | A workaround for a 3D editor whose dialogs float over the scene TBD's dialogs are modal over a map that need not stay visible |
| `3DEN-KEY-001` — bare unmodified `G` for Garrison | The mod's own §6 flags it as a conflict. TBD's editor is full of text inputs behind `in_editable_field()` (`mission_editor.rs:1010`); bare letter bindings are traps |
| `3DEN-KEY-003` — align/orient on `CTRL+ALT+NUM…` | Numpad-only bindings are unreachable on the laptops mission makers use |
| `3DEN-PREF-008` — Adjust Title Width (progressive `...` truncation + refit) | A layout engine solving a problem CSS solves |
| `3DEN-CHROME-005` — product version on status-bar hover | TBD ships from one build; there is no version skew to diagnose |
| `3DEN-TOOL-015` — custom palette commands from config / `description.ext` / JSON | An extensibility surface for SQF modders. TBD has no third-party editor-authoring story, and adding one is a security surface, not a feature |
| `3DEN-ATTR-049` — per-scenario opt-out from backup | TBD's persistence is not opt-outable and should not be |
| `3DEN-ATTR-066` — 53 `remove_*` PBOs to delete attributes from the UI | A per-user mitigation for attribute-window overload. TBD's nine-field modal does not have that problem, and buying the mitigation before the problem is how you get the problem |
| `3DEN-ATTR-067` — five of those 53 are stale and silently do nothing | Recorded as an **anti-pattern**: same class as framework_synthesis's *"a checks-passed state that was never watched fail"*. Includes a case-duplicate pair (`remove_marker_markercolor` / `…markerColor`) |
| `3DEN-QOL-007` — attributes are stripped when a mission is reopened without the mod | An anti-pattern TBD has **already avoided**: paste re-merges unknown keys (`store.rs:2571-2582` pins the known set, `:1496` is the extras branch) and hydrate loads `editor.slots` opaquely (`:1832-1838`) |

### 5.3 One `no` that is really a warning

**`3DEN-EXT-002` — the grid/snap event hooks.** §2.4 is titled *"Snapping, alignment, grid — PRESENT
AND STRONG"* and lists nine features, then closes with: *"No custom grid/guide system is added —
vanilla Eden's move/rotate/scale grids are untouched. However every vanilla grid event is exposed as a
scriptable hook."*

Read quickly, §2.4 says the mod solves snapping. **It does not.** It ships align, space and orient —
which are *batch transforms*, not snapping — and re-exports vanilla's grid events for scripts. TBD has
**no grid at all** (verified: every `snap` hit in the SPA is `read_snapshot`). So:

- **T-645 must not be scoped as "this gives us snapping."** It gives us batch transforms.
- **T-648 gets no help from this catalogue.** Translation / rotation / scaling grids and the
  transformation widget are Eden's, not the mod's, and remain entirely TBD's build.

This is the one place in the catalogue where a section heading and its own body disagree, and it is
exactly the shape of error this program has recorded seven times.

### 5.4 The §12 boundary, restated

All ten `3DEN-GONE-*` rows are `no`. They are documented by a wiki that **drifted from shipping
source**, and four of them are still presented as current there. They are in the table so that a later
reader who finds them on the wiki has a row saying they were checked and are not available — not so
they can be picked up. `3DEN-GONE-001`'s underlying *idea* (a "reveal everything hidden" override) is
worth having and TBD meets it at T-665; that is an idea with a home, not a borrowed feature.

---

## 6. The four priority answers

### 6.1 Ruler / measuring — one weak tool; take two things from it

**Confirmed against the body.** §2.1 is the whole surface: one context-menu entry at root level
(ruler icon `ruler_ca.paa`), **no keybind**, `ENH_fnc_measureDistance` — cross-checked against §4.1
(context menu root, shown `1 - hoverLayer`) and §14 (`ENH_fnc_measureDistance` + `ENH_fnc_floatToTime`
in the non-dialog function list). It is two-step and stateful: click one stores `ENH_Pos_Start` and
raises *"Select second point"*; click two computes `distance` and `distance2D` and derives on-foot
travel time from a hard-coded **14.15 km/h**.

**What it cannot do**, verbatim from §2.1: no chained/polyline measurement, no persistent measurement,
no area or perimeter, **no bearing/azimuth readout**, no slope or grade, and the 5-second auto-expiry
is not configurable. Its readout is a notification that vanishes after 20 s.

**T-642 is already a strict superset** — persistent polyline, per-leg distance **and bearing**, running
total. So the question is only what to add, and the answer is two cells:

| Take | Why |
|---|---|
| **On-foot travel time** (`3DEN-MEAS-002`) | The one genuinely novel idea in the mod's measurement surface and the one a milsim planner actually wants: not "how far" but "how long". Make the speed a setting rather than a 14.15 km/h constant |
| **The 2D **and** 3D distance pair** (`3DEN-MEAS-001`) | §2.3 notes the delta between them is the mod's only inferable vertical-separation readout. TBD has a real DEM, so it can report slope directly — but shipping both distances is free and familiar |

One `maybe`: tick marks at fixed spacing along a leg (`3DEN-MEAS-004`, the mod uses 50 m), which turns
on whether they read as clutter beside contours (T-640) and the scale bar (T-667).

**Reject** the notification readout and the 5-second expiry — T-642's summary already does.

### 6.2 Line of sight / viewshed — zero prior art, confirmed

**Confirmed, and the confirmation method is itself checkable.** §2.2 states a grep of the entire
1,700-file source tree and the 592 KB stringtable for `lineIntersect`, `lineIntersects`,
`lineIntersectsWith`, `lineIntersectsSurfaces`, `terrainIntersect`, `terrainIntersectASL`,
`terrainIntersectAtASL`, `viewshed`, `lineOfSight` and `checkVisibility` — **zero hits**.

I cross-checked both near-misses against the body rather than trusting §2.2's summary:

| Looks like LoS | Actually is | Confirmed at |
|---|---|---|
| Object attribute **"Visibility"** | `setFeatureType` — a draw-distance / render setting (Unchanged / limit by object VD / limit by terrain VD) | §8.1 State table — `3DEN-ATTR-024` |
| AI feature **"Raycasts"** | `disableAI "CHECKVISIBLE"` — one of 21 `disableAI` checkboxes; it *disables* the AI's own raycasts | §8.1 AI Features — `3DEN-ATTR-004` |

Both hold. **There is nothing to borrow and no precedent to copy.** The only 3D-annotation primitives
the mod demonstrates are `Draw3D` + `drawLine3D` (Measure Distance) and `drawIcon3D` (Show Building
Positions), neither of which transfers to a 2D wgpu editor.

**T-643 and T-644 are ours from scratch** — which both summaries already state ("NO PRIOR ART").
Worth noting that TBD is far better equipped than the mod ever was: a 6400² uint16 DEM verified to
**±0.204 m** against 11 survey anchors (T-091.0), with `sample_elevation_meters` at
`crates/map-engine-core/src/dem/sample.rs:108` and the bulk path at `:130` — which is exactly the
sampling lane T-643 is specified to prove and T-644 to reuse.

### 6.3 Contours / height readout — confirmed absent in the mod, and **TBD is already ahead**

**Confirmed.** §2.3: no contour rendering, no topographic display mode, no elevation profile, no
height-above-terrain readout, no slope indicator; the mod **never calls `getTerrainHeightASL`,
`getTerrainInfo` or `selectBestPlaces`**. I checked all nine of §2.3's "touches elevation at all" rows
against their definition sites in the body, and all nine hold — and every one is either a camera
readout, a batch transform, or a clipboard export:

status-bar Z and camera direction (§5.1 — the latter is *heading*, not elevation) · Align Z± and
Space Z (§3.2) · Log Positions 3D (§4.2) · Snap to Surface (§3.2) · Dynamic View Distance (§7.4) ·
Terrain Detail (§8.2, an in-game clutter density, not an editor aid) · the Measure 2D-vs-3D delta
(§2.1).

§2.3's conclusion — *"a contour/elevation overlay would be built from scratch"* — is correct for the
mod and **already obsolete for TBD**:

| Capability | TBD status |
|---|---|
| Contour isolines over the DEM | **Ships.** `crates/map-engine-core/src/geometry/contours.rs`; hairline compose at `geometry/vector_compose.rs:108`; `CONTOUR_RGBA` `:129`; user toggle `world_layer_prefs.rs:68` |
| Spot heights / peak labels | **Ships.** `crates/map-engine-core/src/dem/peaks.rs` — `HeightLabel` `:32`, `HeightLabelKind` `:25`, screen-space separation `:13`/`:45`, zoom window `:19-21`/`:51`; user toggle `world_layer_prefs.rs:72` |
| Point elevation sampling | **Ships.** `dem/sample.rs:108`, `:130`; DEM verified ±0.204 m (T-091.0) |
| Hypsometric sea band | **Ships.** `geometry/mod.rs:1-2` |

The open contour work — **T-639** (zoom-adaptive interval), **T-640** (tint, not colour), **T-641**
(Eden-parity screen-space label density) — is **refinement of a shipped capability**, not
construction. Nothing in this catalogue advances it. This is the one operator priority where TBD is
strictly ahead of the mod the community runs.

### 6.4 Map legibility — partial; the four named items split two and two

§2.9's own verdict is **PARTIAL**, and it is honest: *"No terrain render modes, no contour/topographic
toggle, no map colour schemes."* The mod's entire map-legibility surface is markers plus a minimap.

| The operator's item | Verdict | Detail |
|---|---|---|
| **Marker draw-priority** | **TAKE IT** — `want` / `b` (`3DEN-ATTR-060`) | `setMarkerDrawPriority`, Marker attributes ▸ Transformation (§8.4, §2.9). Higher draws on top when markers overlap — the single most useful legibility feature in the mod, because overlapping markers is the failure mode every real briefing map hits. **It is in neither T-069's four schema-carried fields nor T-673's six style ids** — add it to T-673. TBD already has a formal lane-ordering system (`crates/map-engine-render/src/draw_order.rs`), so the render half is nearly free; the closed `$defs/marker` (`{x, z, icon, label}`) is the blocker |
| **Extra shapes** | `maybe` / `b` (`3DEN-ATTR-061`) | Eight polygons, Triangle → Decagon, beyond vanilla's set; not applicable to Icon markers (§8.4). **Already T-673's `-MRK-SHAPE`** — no new scope. Whether eight free polygons beat TBD's closed 64-icon vocabulary is a design argument, and framework_synthesis **K10** leans against free vocabularies (WOG's open marker-type string produced `loc_Fuelstation` 1,803 times as an accidental default) |
| **RGBA picker** | **Splits.** `maybe` / `b` + `want` / `a` | The picker itself — hex RGBA field, four R/G/B/A sliders, live swatch (§8.4) — is **already T-673's `-MRK-COLOR`/`-MRK-ALPHA`**, and carries the same K10 objection. But the **saveable preset history** (`3DEN-ATTR-063`) is a *separate, editor-local* feature needing no schema, and it is the half that actually produces consistent-looking maps across a community. Take it even if the free picker stays closed |
| **Camera minimap** | **`no` / `d`** (`3DEN-CHROME-008`) | The clearest single example in this sweep of a feature that exists **only because Eden is 3D**. It is a camera-following map inset with altitude-scaled zoom and a rotating camera icon — i.e. it reconstructs, badly and in a corner, the view TBD opens with. Its own sophistication proves the point: it repositions when the left panel hides (§2.7), hides during preview and when a BI or ACE Arsenal opens (§11), and hides when the *full map* is opened — because when the real map is up, the minimap is redundant. **In TBD the real map is always up.** Five rows die with it: `3DEN-CHROME-008`, `3DEN-PREF-006`, `-007`, `3DEN-QOL-004`, `-005` |

**One thing under legibility the operator did not name, worth flagging:** `3DEN-CTX-014`
("Move camera here" overridden to also centre the 2D map) is one of only **two vanilla entries the mod
overrides at all** — a strong signal that keeping the camera and the map in sync is a real pain point.
TBD has it already (`Space` → `editor_ops.rs:354` `center_on_selection`), because in a 2D editor the
camera *is* the map. Recorded as `have`, and as evidence that TBD's architecture deletes a class of
problem rather than solving it.

