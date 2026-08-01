# Screenshot-corpus sweep — the Eden surfaces the authority docs only sampled

**Written 2026-08-01.** Triage of `.ai/artifacts/eden_screenshots/` (75 operator screenshots of Arma 3
Eden, build `2.20.153973`, already transcribed into eight batch documents) into parity rows in the
same vocabulary `docs/specs/Mission_Creator_Architecture/eden/gap_analysis.md` uses.

**Why this exists.** `gap_analysis.md` was rewritten this week into a 191-row census — but a census
**of the authority docs, not of Eden**. The authority docs are themselves a sample:

| Surface | Captured in the screenshots | Ids in the authority docs |
|---|---|---|
| Menu bar + dropdowns | **101 entries**, verbatim, with shortcuts and enabled/greyed state | `interactions.md` declares **2** ids, both `TOOLBAR-*` — **zero** `MENU-*` ids |
| Scenario dialogs | **5 modals**, ~106 fields transcribed with values | `attributes.md` declares **11** `SCN-` ids |
| Viewport context menu | **39 rows** across two selection states + 6 submenus | 3 ids touch it (`PLACE-COMMENT-001`, `CTX-FORMATION-001`, `ACTION-*`) |
| Toolbar | **26 controls**, 12 with verbatim hover tooltips | 2 ids (`TOOLBAR-NEW-001`, `TOOLBAR-TUTORIAL-001`) |

Verified, not inherited:

```bash
cd docs/specs/Mission_Creator_Architecture/eden
grep -oE '\bSCN-[A-Z0-9-]+\b' attributes.md | sort -u | wc -l          # 11
grep -oE '\bTOOLBAR-[A-Z0-9-]+\b' interactions.md | sort -u | wc -l    # 2
grep -oE '\bMENU-[A-Z0-9-]+\b' interactions.md | sort -u | wc -l       # 0
```

The corpus was mined once, for **design decisions** (contour ladder → T-639, contour tint → T-640,
spot heights → T-641, panel chevrons → T-638). It was never triaged as a **feature inventory**. That
is the gap this file closes.

---

## 1. Method

### 1.1 Sources, in precedence order

1. [`eden_screenshots/README.md`](../eden_screenshots/README.md) — the cross-batch reconciliation.
   Applied: **batch 06 supersedes batch 05** on the F-tab names (F1 = Objects, F2 = Compositions);
   the 23 px right-edge crop on batches 01–03 is a capture artefact, not a layout fact; "orange is
   hover, not toggled-on"; contour labels are **spot heights**, not contour labels.
2. `batch02_menus.md` (menu bar) · `batch03_dialogs.md` (the five modals) — the two richest.
3. `batch01_context_menu.md` · `batch04`/`batch05`/`batch06` (toolbar tooltips, F1–F6 strip,
   left-panel footer) · `batch07`/`batch08` (2D map view, panel chevrons).

The images were **not** re-read. Every Eden-side claim traces to a batch document line.

### 1.2 Ground truth for `match` / `partial`

Read from `apps/website/frontend/src/` this session, not inherited:

| Live surface | Where | What is actually there |
|---|---|---|
| Menu bar | `eden_chrome.rs:112` `MENUS` | **5 menus, 8 items**: File (Save Version…, Export JSON, Export Compiled Mission…), Edit (Undo, Redo), View (one inert label, `action: None` `:145-147`), Mission (Mission Settings…), Environment (Time & Weather…) |
| Top strip | `eden_chrome.rs:831` `TopCommandStrip` | ORBAT Manager button `:1005`; title input `:1047`; dirty dot `:1069`; time scrubber `:1084`; weather select `:1105`; History button **disabled** `:1131-1139`; Undo `:1141`; Redo `:1156`; Save Version `:1173` |
| Toolbelt | `eden_chrome.rs:3706-3767` | Select (active) · Ruler **disabled** "(soon)" · LoS **disabled** "(soon)" · `CUR`/`SEL` X/Y/Z · `OBJ`/`SEL`/`SZ`. **No scale, no version, no play target** |
| Left dock | `eden_chrome.rs:2806-2841` `DockLeft` | **Editor Layers tree only** — no tabs, no search box, no collapse-all, no per-row visibility checkbox, no footer button row |
| Right dock | `eden_chrome.rs:2960-3340` `DockRight` | 4 tabs (Factions LIVE / Vehicles LIVE / Zones LIVE / **Markers stub** `:3337`) + `EDEN_SIDE_CHIPS` `:2871` + per-tab search `:3115-3127` |
| Mission Settings | `eden_chrome.rs:3788` | **10 editable controls** behind the `author_env` gate |
| Attributes modal | `attributes.rs:16` | 4 tabs, **9 per-slot fields** + loadout; States tab is a stub that takes no arguments (`:351`) |
| Keyboard | `mission_editor.rs:1008-1043`, `mission_history.rs:481-509` | **10 bindings total** |

Three prior inventories were used and spot-checked rather than re-derived:
[`editor_inventory_mission_settings.md`](editor_inventory_mission_settings.md) ·
[`editor_inventory_attributes_modal.md`](editor_inventory_attributes_modal.md) ·
[`editor_inventory_absent_entities.md`](editor_inventory_absent_entities.md). The keyboard column
defers to [`interactions_sweep.md`](interactions_sweep.md) §4, which already built the full Eden↔TBD
key map from this same corpus — **this file does not re-litigate a single binding.**

### 1.3 The gate that decides `na` vs `missing`

Verified live this session by reading the source, not by trusting the inventory:

| Refusal | Citation | Keys |
|---|---|---|
| No control may write a key nothing reads | `eden_chrome.rs:4623-4630` test `keys_nothing_reads_are_not_authored` | `viewDistance` · `thermals` · `windDirDeg` · `fog` · `wind` |
| Four fields T-224 asked for that must **not** get a control | `eden_chrome.rs:4686-4694` test `fields_with_no_mod_reader_get_no_control` | `respawn` · `spectatorPolicy` · `nightVision` · `tickets` |
| The prose that pins it | `eden_chrome.rs:370` `SETTINGS_UNREAD_NOTE` — *"TBD events are one life."* | — |
| The environment allow-list | `eden_chrome.rs:231` `CARRIED_ENV_KEYS`, pinned at 5 by `:4648-4652` | `time` · `weather` · `showHillshade` · `hillshadeOpacity` · `showGrid` |
| The flow allow-list | `eden_chrome.rs:332` `AUTHORED_FLOW_KEYS`, pinned at 4 by `:4700-4711` | `briefingSeconds` · `safeStartSeconds` · `timeLimitSeconds` · `jip` |

`eden_chrome.rs:4619-4621`, verbatim: *"the schema HAS a slot for it … A schema field is not a
reader."* Anything on those two lists is a **closed question**, scored `na` with the citation.

### 1.4 Numbering scheme

Ids are minted per Eden **surface**, three digits, in the order the operator walked the surface.

**Collision check, run rather than assumed:**

```bash
cd docs/specs/Mission_Creator_Architecture/eden
for p in MENU CTX DLG PANEL ENT-SEC MAP2D VPORT SBAR TOOLBAR STATUS; do
  echo "$p: $(grep -ohE "\b${p}-[A-Z0-9-]+\b" attributes.md interactions.md | sort -u | tr '\n' ' ')"
done
```

Result — five prefixes are already in use in `interactions.md`:

| Prefix | Existing ids | How this file avoids the collision |
|---|---|---|
| `TOOLBAR-` | `TOOLBAR-NEW-001`, `TOOLBAR-TUTORIAL-001` | numeric-only `TOOLBAR-001…026`; both authority ids are cited in `tbd_id` on the rows they overlap (`TOOLBAR-001`, `TOOLBAR-026`) |
| `STATUS-` | 7 ids (`STATUS-X/Y/Z/ZOOM/VER/SRV/MOD-001`) | status bar uses **`SBAR-`** here; each overlapping authority id is cited in `tbd_id` |
| `CTX-` | `CTX-FORMATION-001` | numeric-only `CTX-001…039`; no literal collision, and `CTX-FORMATION-001` is cited on `CTX-031` |
| `PANEL-` | `PANEL-001` | this file uses `PANEL-L-` / `PANEL-R-`; no literal collision |
| `VIEW-` | `SCN-VIEW-DIST` (matched as a suffix) | viewport overlays use **`VPORT-`** here |

`MENU-*`, `DLG-*`, `ENT-SEC-*`, `ENT-UX-*` and `MAP2D-*` return **zero** hits in both files.

| Prefix | Surface | Range | Source |
|---|---|---|---|
| `MENU-BAR-` | the eight menu-bar buttons | 001–008 | batch02, batch03, batch04 |
| `MENU-SCEN-` | Scenario menu + Export submenu | 001–016 | batch02 `163448` |
| `MENU-EDIT-` | Edit menu + 4 submenus | 001–032 | batch02 `163508`–`163533` |
| `MENU-VIEW-` | View menu + Search/Interface submenus | 001–018 | batch02 `163546`–`163553`, batch08 |
| `MENU-ATTR-` | Attributes menu | 001–004 | batch02 `163608`, batch03 `163658` |
| `MENU-TOOLS-` | Tools menu | 001–006 | batch03 `163901` |
| `MENU-SET-` | Settings menu | 001–005 | batch03 `163909` |
| `MENU-PLAY-` | Play menu | 001–005 | batch04 `163940` |
| `MENU-HELP-` | Help menu | 001–007 | batch04 `163950` |
| `MENU-UX-` | menu-system mechanics | 001–006 | batch02 §Layout rules |
| `TOOLBAR-` | the icon row + phase combo + tutorials | 001–026 | batch04/05/06 tooltips |
| `CTX-` | viewport context menu, both takes, 6 submenus | 001–039 | batch01 |
| `CTX-UX-` | context-menu mechanics | 001–004 | batch01 §Interaction rules |
| `DLG-SHELL-` | the shared modal shell + control vocabulary | 001–014 | batch03 §shell |
| `DLG-GEN-` | `Edit: General` | 001–027 | batch03 `163629`/`163642` |
| `DLG-ENV-` | `Edit: Environment` | 001–027 | batch03 `163720`/`163735` |
| `DLG-MP-` | `Edit: Multiplayer` | 001–023 | batch03 `163758`/`163811` |
| `DLG-PERF-` | `Edit: Performance` | 001–016 | batch03 `163830` |
| `DLG-PREF-` | `Edit: Preferences` (editor prefs) | 001–013 | batch03 `163916` |
| `ENT-SEC-` | entity attribute dialog — the 9 sections | 001–009 | batch02 `163121`–`163151` |
| `ENT-UX-` | entity dialog widget vocabulary | 001–005 | batch02 `163138` |
| `PANEL-L-` | left panel (Entity List) | 001–018 | batch01/02/03/06 |
| `PANEL-R-` | right panel (Asset Browser) | 001–021 | batch04/05/06 |
| `SBAR-` | status bar | 001–009 | batch01–08 |
| `VPORT-` | 3D viewport overlays | 001–006 | batch01/03/06 |
| `MAP2D-` | 2D map view + panel collapse | 001–010 | batch07/08 |

### 1.5 Vocabulary — identical to `gap_analysis.md`

`match` equivalent (2D acceptable) · `partial` exists but incomplete · `missing` not built ·
`deferred` intentional later phase · `na` out of scope, **reason mandatory** · `tbd_only` TBD
addition with no Eden id.

`build_class`: **a** SPA-buildable today · **b** `mission.schema.json` must widen first ·
**c** Enfusion support must exist first (`executor: workbench`) · **d** `na`.

**Ticket column convention.** `T-xxx` = maps to an existing ticket as scoped · `T-xxx (extend)` =
existing ticket, scope needs one more item · `NEW-Fn` / `NEW-Wn` = proposed new factory / workbench
slice, defined in §4 · `—` = no owner proposed, and the row says why. **Never both** a mapping and a
proposal on one row.

### 1.6 Evidence discipline

`INFERRED:` prefixes anything derived rather than read. `UNKNOWN` is used where the corpus
**contradicts itself or is silent** — there are exactly **three** such rows (`TOOLBAR-022`,
`PANEL-L-017`, `MAP2D-008`), each dissected in §5.3, and none is guessed. Batch documents already
carry their own `*(inferred)*` marks for un-hovered icons; those are carried forward verbatim rather
than promoted to fact. Where two batches disagree, the batch that captured a **tooltip** wins over
the batch that read a **glyph** — the rule the README itself uses on the F-tabs, applied twice more
here (§2.10, §2.20).

---

## 2. The table

### 2.1 Menu bar — the eight top-level menus (`MENU-BAR-001…008`)

TBD's bar is `MENUS` at `eden_chrome.rs:112`: **File · Edit · View · Mission · Environment** — 5
menus, 8 items. Eden's is 8 menus, 101 items.

| eden_id | tbd_id | parity | build_class | gap_notes | ticket |
|---|---|---|---|---|---|
| MENU-BAR-001 | `File` (`eden_chrome.rs:113-129`) | match | a | Eden `Scenario`. Menu exists; contents differ wholesale (16 Eden entries vs 3). Naming divergence is deliberate — a web editor has no "scenario file" | T-634 |
| MENU-BAR-002 | `Edit` (`:131-142`) | match | a | Eden `Edit`. TBD's holds Undo/Redo only; the other 30 Eden entries are scored individually below | T-634 |
| MENU-BAR-003 | `View` (`:143-149`) | partial | a | **TBD's View menu is a dead menu.** Its single item is `"Map layers — render host (T-159.28)"` with `action: None` (`:145-147`) — it renders, opens, and does nothing. Eden's View has 18 live entries | T-634 (extend) |
| MENU-BAR-004 | `Mission` + `Environment` (`:150-163`) | partial | a | Eden `Attributes` (4 items → 4 modals). TBD splits one dialog across **two** menus that both fire `MenuAction::Settings` — two doors, one room | T-634 |
| MENU-BAR-005 | — | na | d | Eden `Tools` — Debug Console, Config Viewer, Functions/Animations Viewer, Camera, Field Manual. All are A3 **script/config runtime** inspectors. TBD has no scripting layer; §5 | — |
| MENU-BAR-006 | — | partial | a | Eden `Settings`. TBD has no editor-preferences menu; the only editor prefs (basemap view, 12 world layers) are buried in Mission Settings and stored in `localStorage` (`world_layer_prefs.rs:61-76`) | NEW-F2 |
| MENU-BAR-007 | — | na | d | Eden `Play` — five in-client preview modes. A browser editor cannot host an Arma client; TBD's preview is a real server consuming `GET /missions/:id/compiled`; §5 | — |
| MENU-BAR-008 | — | missing | a | Eden `Help`. No help entry point anywhere in the editor chrome. Six of Eden's seven items are Bohemia web links (`na`); one generic docs link is buildable | NEW-F3 |

### 2.2 `Scenario` menu (`MENU-SCEN-001…016`) — batch02 `163448`

Sixteen entries, two of them greyed in the capture. **Nine are `na`** — this menu is where a desktop
editor's filesystem, Steam and binarise concerns live, and none of them survive the port.

| eden_id | tbd_id | parity | build_class | gap_notes | ticket |
|---|---|---|---|---|---|
| MENU-SCEN-001 | Library `New Mission` + `Cmd/Ctrl+N` (`missions.rs:212-231`) | match | a | `New… Ctrl+N`. TBD's lives on the **Library route**, not the editor menu — a deliberate T-048 move (the `/missions/create` wizard was replaced by a dialog). Same key, different surface | — (shipped T-048) |
| MENU-SCEN-002 | Mission Library route | match | a | `Open… Ctrl+O`. TBD opens a mission by navigating `/missions/:id/edit`; there is no in-editor open. Equivalent, not identical. `Ctrl+O` is free (`interactions_sweep.md` §4.2) | — |
| MENU-SCEN-003 | `Save Version` (`eden_chrome.rs:1173`) + `File ▸ Save Version…` (`:116-119`) | partial | a | `Save Ctrl+S`. Button and menu item both exist; **`Ctrl+S` is unbound** — the 10-binding table (`mission_editor.rs:1008-1043`, `mission_history.rs:481-509`) has no `KeyS` arm. Highest-frequency missing shortcut after `Ctrl+F` | NEW-F1 |
| MENU-SCEN-004 | — | na | d | `Save As… Ctrl+Shift+S`. TBD versions are **immutable and semver-keyed** (unique semver, 409 on duplicate — T-038); every save *is* a Save As. The Eden concept has no target | — |
| MENU-SCEN-005 | — | na | d | `Publish to Steam Workshop`. Steam Workshop distribution; TBD ships missions through its own API + modpacks; §5 | — |
| MENU-SCEN-006 | `File ▸ Export JSON` / `Export Compiled Mission…` (`:120-127`) | match | a | `Export ▶` submenu parent. TBD has two export targets to Eden's four, and both are live | — (shipped T-243) |
| MENU-SCEN-007 | — | na | d | `Export to Singleplayer` — binarise into the A3 SP scenarios folder. OS filesystem + A3 packaging | — |
| MENU-SCEN-008 | `Export Compiled Mission…` (`:123-127`) | match | a | `Export to Multiplayer`. TBD's closest analogue: the compiled mod document a game server loads. Downloads rather than writes to a folder | — (shipped T-243) |
| MENU-SCEN-009 | — | na | d | `Export to Terrain Builder` — dumps an object list for BI's terrain tool. No analogue | — |
| MENU-SCEN-010 | — | na | d | `Export to SQF` — emits the scenario as A3 spawn script. **Scripting handle with no scripting layer** (class-d definition, `gap_analysis.md:111`) | — |
| MENU-SCEN-011 | — | missing | a | `Merge Ctrl+M` — merge another scenario into this one. Zero hits for a merge feature in the SPA (`grep -rnE '\bMerge\b' --include=*.rs` → 15 hits, all `tailwind-merge` prose or doc-CRDT merge). Genuinely useful for composing ORBATs from templates | NEW-F4 |
| MENU-SCEN-012 | `OBJ` / `SEL` / `SZ` toolbelt readout (`eden_chrome.rs:3745-3766`) | partial | a | `Statistics` — entity/asset counts dialog. TBD shows three live counters in the toolbelt but no breakdown dialog. T-659 already owns "slot census badge + generated mission summary line" | T-659 |
| MENU-SCEN-013 | — | na | d | `Show Required Addons` (greyed in capture). A3 addon dependency list. TBD's analogue is the **modpacks** platform feature, not editor chrome | — |
| MENU-SCEN-014 | — | na | d | `Open Scenario Folder` (greyed). OS file manager | — |
| MENU-SCEN-015 | — | na | d | `Open Log Folder`. OS file manager / RPT logs | — |
| MENU-SCEN-016 | browser tab / route exit | na | d | `Exit`. Leaving a web editor is navigation, and the unload guard already exists (`mission_history::register_unload_guard`, `mission_editor.rs:1000`) | — |

### 2.3 `Edit` menu + 4 submenus (`MENU-EDIT-001…032`) — batch02 `163508`–`163533`

The single richest menu in the corpus: 32 entries, 4 submenus, 14 keyboard shortcuts. **Nothing
snap-related or gizmo-related exists in TBD** — `owns_parity.md` records the check that killed the
false positive: all 37 word-boundary `snap` hits are `let snap = read_snapshot()`.

| eden_id | tbd_id | parity | build_class | gap_notes | ticket |
|---|---|---|---|---|---|
| MENU-EDIT-001 | `Edit ▸ Undo` (`:134-137`), strip button `:1141`, `Ctrl/Cmd+Z` (`mission_history.rs:497`) | match | a | `Undo Ctrl+Z`. Three entry points to Eden's two | — (shipped T-052) |
| MENU-EDIT-002 | `Edit ▸ Redo` (`:138-141`), strip `:1156`, `Ctrl+Y` (`mission_history.rs:500`) + `Ctrl+Shift+Z` (`:494`) | match | a | `Redo Ctrl+Y`. TBD is a **superset** — Eden has no `Ctrl+Shift+Z` | — (shipped T-052) |
| MENU-EDIT-003 | — | missing | a | `Select All on Screen Ctrl+A`. Note the wording — Eden's is **viewport-scoped**, not document-scoped. The rect primitive the marquee uses already exists — `select_tool.rs:300-312` (`marquee_vehicle_ids` / `marquee_ids_with_vehicles`) | T-649 |
| MENU-EDIT-004 | — | missing | a | `Transformation Widget ▶` submenu parent. TBD's toolbelt has one non-switchable Select tool (`eden_chrome.rs:3708`); there is no widget concept | T-648 |
| MENU-EDIT-005 | — | missing | a | `Toggle Widget  Space`. **Collision 1** — `Space` is TBD's centre-on-selection (`mission_editor.rs:1026`). `interactions_sweep.md` §4.4 resolves it: use Eden's own direct `1`–`5` and drop the cycle | T-648 |
| MENU-EDIT-006 | Select tool (`eden_chrome.rs:3707-3711`) | partial | a | `No Widget  1`. TBD's only mode, permanently active, no key | T-648 |
| MENU-EDIT-007 | drag-move (`select_tool.rs`, `editor_ops::move_entities`) | partial | a | `Translation Widget  2`. TBD moves by direct drag, no gizmo and no axis constraint. `2` is free | T-648 |
| MENU-EDIT-008 | `Rotation` number field (`attributes.rs:292`) | partial | a | `Rotation Widget  3`. Rotation round-trips **numerically** only; no map gesture. `3` is free | T-648 |
| MENU-EDIT-009 | `ops::begin_zone_draw(kind, shape)` (`eden_chrome.rs:2242-2244`) | partial | a | `Area Scaling Widget  4`. TBD's nearest is the T-582 zone draw tool, which **creates** an area but has no resize handles. `4` is free | T-079 (extend) |
| MENU-EDIT-010 | zone `shape` (`mission.schema.json` `$defs/zone`) | partial | a | `Area Widget  5`. Zone shape exists in the document and the panel arms a draw; editing an existing area's extents was not found | T-079 (extend) |
| MENU-EDIT-011 | — | missing | a | `Grid ▶` submenu parent — the snapping-grid family | T-648 |
| MENU-EDIT-012 | — | missing | a | `Toggle Translation Grid  ö`. **Do not copy the key** — `odiaeresis` is an X11 keysym artefact of the operator's Nordic layout (`README.md`, `batch02:355-357`), not an Eden string | T-648 |
| MENU-EDIT-013 | — | missing | a | `Toggle Rotation Grid` (unbound in Eden too) | T-648 |
| MENU-EDIT-014 | — | missing | a | `Toggle Area Scaling Grid` (unbound in Eden too) | T-648 |
| MENU-EDIT-015 | — | missing | a | `Decrease Grid Size  å` (keysym artefact) | T-648 |
| MENU-EDIT-016 | — | missing | a | `Increase Grid Size  ¨` (keysym artefact) | T-648 |
| MENU-EDIT-017 | — | na | d | `Vertical Mode ▶` submenu parent. TBD is a top-down 2D editor; the substitute (numeric Z with terrain-follow, `attributes.rs:255-319`, `store.rs:1356-1358`) already ships. `na` ≠ nothing to do — the substitute is named | — |
| MENU-EDIT-018 | — | na | d | `Toggle Vertical Mode  ä`. Same substitute; a vertical drag axis needs a 3D camera | — |
| MENU-EDIT-019 | terrain-follow Z (`store.rs:1356-1358`) | match | a | `Above Terrain Level (ATL)`. **TBD is ATL by construction** — editing X or Y resets Z to 0.0 and the DEM z is sampled JS-side. The comment on that line is the citation | — |
| MENU-EDIT-020 | — | missing | b | `Above Sea Level (ASL)`. Would need a per-slot reference-frame flag; `$defs/slot` is `additionalProperties: false`. **No mod reader either** — not worth a ticket before a consumer exists | — |
| MENU-EDIT-021 | terrain-follow Z (`store.rs:1356-1358`) | match | a | `Toggle Surface Snapping  '` — Eden's is a **toggle**; TBD's is permanently on and enforced mod-side (`interactions_sweep.md` `XFORM-SNAP-001`). Equivalent capability, no control | — |
| MENU-EDIT-022 | — | missing | c | `Toggle Waypoint Snapping  -`. No waypoint entity exists anywhere — 2 repo-wide hits, both marker **icon aliases** (`editor_inventory_absent_entities.md` §3). Gated on AI units existing | T-677 (wb) |
| MENU-EDIT-023 | — | na | d | `Phase ▶` — A3 scenario phases (Scenario / Intro / Outro; `INFERRED:` never expanded in the corpus, matches the toolbar `Scenario` combo). No Enfusion analogue | — |
| MENU-EDIT-024 | `DockRight` tab strip (`eden_chrome.rs:3041-3046`) | partial | a | `Asset Type ▶` submenu parent. TBD has 4 tabs + an Objects **chip** to Eden's 7 modes; **F-keys are deliberately banned** by a unit test (`eden_chrome.rs:4998-5011`, T-180.5) | T-646 |
| MENU-EDIT-025 | Factions tab + `EdenChip::Objects` (`:2875-2880`) | partial | a | `Objects  F1`. Eden's F1 is **one** tree over units + vehicles + props; TBD splits it across Factions / Vehicles / an Objects chip | T-646 |
| MENU-EDIT-026 | — | missing | a | `Compositions  F2`. Eden's F2 holds pre-made **group templates** (`Fire Team`, `Rifle Squad`…) — batch06 corrects batch05's "Groups" reading. TBD's `comp:` alias is a path classifier, not authoring (`asset_catalog.rs:362`) | T-650 |
| MENU-EDIT-027 | — | missing | a | `Triggers  F3`. No trigger entity; the nearest is `$defs/zone` and it has no activation model | T-079 |
| MENU-EDIT-028 | — | missing | c | `Waypoints  F4` | T-677 (wb) |
| MENU-EDIT-029 | — | missing | a | `Systems  F5` — 23 module categories (`Ambient`…`Zeus`, batch06 `170020`). **Unsized**: the `SYS` family declares zero ids in `attributes.md:223-225`, and T-079c was deliberately not filed for exactly this reason (`registry_write_log.md` §7.2). Needs a catalogue pass first | — |
| MENU-EDIT-030 | Markers tab stub (`eden_chrome.rs:3046`, body `:3337`) | missing | a | `Markers  F6`. Tab exists, body is one sentence, pinned as a stub by a test (`:4499-4502`) | T-069 |
| MENU-EDIT-031 | — | missing | a | `Favorites  F7` (greyed in capture — none saved). A starred-asset list over the live catalogue is pure SPA work | NEW-F5 |
| MENU-EDIT-032 | `EDEN_SIDE_CHIPS` (`eden_chrome.rs:2871`) | partial | a | `Toggle Asset Sub-type  Tab`. TBD's side chips are click-only; no `Tab` cycle. `Tab` is free but is also the browser's focus key — needs care | T-646 |

### 2.4 `View` menu + Search/Interface submenus (`MENU-VIEW-001…018`) — batch02 `163546`/`163553`

Eden's View menu is 18 live entries. **TBD's View menu is one dead label** (`MENU-BAR-003`). The
interesting result: five of the eighteen already have working TBD equivalents — they are just not in
a menu, they are checkboxes inside Mission Settings.

| eden_id | tbd_id | parity | build_class | gap_notes | ticket |
|---|---|---|---|---|---|
| MENU-VIEW-001 | — | missing | a | `Center on Random Position  Ctrl+R`. Cheap, but **`Ctrl+R` is the browser reload** — do not copy the key (`interactions_sweep.md` §4.4). Low value; no owner proposed | — |
| MENU-VIEW-002 | `Space` → `editor_ops::center_on_selection` (`mission_editor.rs:1026`, `editor_ops.rs:354`) | match | a | `Center on Selected Entity  F`. Capability ships on a different key. **Semantic clash 2** — binding `F` as an alias is a one-line addition inside the widget slice | T-648 |
| MENU-VIEW-003 | — | na | d | `Center on Player  Home`. TBD's editor has **no player unit** — slots are ORBAT slots filled at event time, not a placed player. The precondition does not exist | — |
| MENU-VIEW-004 | — | na | d | `Toggle Map  M` — swaps the 3D viewport for the 2D map. **TBD is always the 2D map.** The toggle's precondition (a 3D viewport) does not exist; §5 | — |
| MENU-VIEW-005 | Mission Settings basemap radio — Satellite / Map (`eden_chrome.rs:4057` `render_prefs_section`, `localStorage` `tbd-mc-basemap-view`) | match | a | `Toggle Map Textures  Ctrl+T`. Live since T-090.1.1. Different surface (settings radio, not a menu), and **`Ctrl+T` is the browser new-tab** — do not copy the key | — (shipped T-090.1.1) |
| MENU-VIEW-006 | — | na | d | `Vision Mode ▶` — NV / thermal camera modes (never expanded in the corpus). **Refused with a test**: `nightVision` (`eden_chrome.rs:4686-4694`) and `thermals` (`:4623-4630`); §5 | — |
| MENU-VIEW-007 | — | na | d | `Toggle Flashlight  L` — a camera-mounted light for night editing in a 3D scene. No 3D lighting model | — |
| MENU-VIEW-008 | `Town labels` + `Road names` world layers (`world_layer_prefs.rs:61-76`) | match | a | `Toggle Location Labels (3D)`. TBD's are 2D map labels and there are two toggles to Eden's one. Equivalent capability | — (shipped T-173) |
| MENU-VIEW-009 | `Forest mass` + `Trees` world layers (`world_layer_prefs.rs:61-76`) | match | a | `Toggle Foliage  Ctrl+G`. Two layer toggles cover it; `Ctrl+G` unbound | — (shipped T-173) |
| MENU-VIEW-010 | per-tab search boxes (`eden_chrome.rs:3115-3127`, `:3254-3256`) | partial | a | `Search ▶` submenu parent — Eden's is a **focus-the-box** command family, not a search feature | T-646 |
| MENU-VIEW-011 | asset search (`asset_catalog.rs:396-414` `filter_catalog`) | partial | a | `Search in Asset Browser  Ctrl+F`. The box ships (T-055); **the shortcut does not**, and `Ctrl+F` is the browser find — needs `preventDefault` scoped to the editor route. `interactions_sweep.md` P-12 calls this the highest-frequency Eden shortcut TBD has no answer for | T-646 (extend) |
| MENU-VIEW-012 | — | missing | a | `Search in Entity List  Ctrl+Shift+F`. **`DockLeft` has no search box at all** (`eden_chrome.rs:2806-2841`) — the whole control is absent, not just the key. `Ctrl+Shift+F` is free | T-666 (extend) |
| MENU-VIEW-013 | — | missing | a | `Interface ▸` submenu parent — panel-visibility family | T-638 |
| MENU-VIEW-014 | — | missing | a | `Toggle Interface  Backspace` — hides **all** editor chrome. **Collision 3, and it is dangerous**: `Backspace` currently deletes the selection (`mission_editor.rs:1027`), so an Eden author reaching for a clean screenshot destroys work. T-662 frees the key; the hide-chrome behaviour is the superset of T-638's dock collapse | T-638 (extend) |
| MENU-VIEW-015 | — | missing | a | `Entity List  E` — show/hide the left dock. `E` is free | T-638 |
| MENU-VIEW-016 | — | missing | a | `Asset Browser  R` — show/hide the right dock. `R` is free | T-638 |
| MENU-VIEW-017 | — | missing | a | `Controls Hint` — on-screen key hints. TBD's 10 bindings are documented nowhere in the UI | NEW-F3 |
| MENU-VIEW-018 | — | missing | a | `Navigation Widget` — show/hide the camera/compass gizmo. TBD's 2D analogue is a north/axis indicator; T-667 already owns map furniture (scale bar + edge grid labels) and this is the same slot | T-667 (extend) |

### 2.5 `Attributes` menu (`MENU-ATTR-001…004`) — batch02 `163608`, batch03 `163658`

Four items, four modals, no submenus. This is the menu whose *contents* §2.9–§2.12 triage.

| eden_id | tbd_id | parity | build_class | gap_notes | ticket |
|---|---|---|---|---|---|
| MENU-ATTR-001 | `Mission ▸ Mission Settings…` (`eden_chrome.rs:151-157`) | partial | a | `General…`. Opens a dialog; TBD's holds 10 editable controls to Eden's 27, and the overlap is **Title only** | T-671 |
| MENU-ATTR-002 | `Environment ▸ Time & Weather…` (`:158-163`) → same dialog | match | a | `Environment…  Ctrl+I`. TBD's Environment section (`eden_chrome.rs:3841-3895`) is time + weather. `Ctrl+I` is free | — (shipped T-159.26) |
| MENU-ATTR-003 | — | na | d | `Multiplayer…`. The whole respawn/lobby/revive surface is **refused by design** — `SETTINGS_UNREAD_NOTE` (`eden_chrome.rs:370`): *"TBD events are one life."* Pinned by `fields_with_no_mod_reader_get_no_control` (`:4686-4694`); §5 | — |
| MENU-ATTR-004 | — | na | d | `Performance…` — A3 garbage collection + dynamic simulation. Engine-runtime knobs with no Enfusion analogue exposed to a mission; §5 | — |

### 2.6 `Tools` menu (`MENU-TOOLS-001…006`) — batch03 `163901`

**All six `na`.** This menu is the A3 scripting/config runtime, and the brief names it as correctly
out of scope. Recorded rather than skipped so the boundary is inspectable.

| eden_id | tbd_id | parity | build_class | gap_notes | ticket |
|---|---|---|---|---|---|
| MENU-TOOLS-001 | — | na | d | `Debug Console…  §` (Eden prints the key name as the literal word `section`). Live SQF execution; TBD has no scripting layer | — |
| MENU-TOOLS-002 | — | na | d | `Functions Viewer…` — browse compiled SQF functions | — |
| MENU-TOOLS-003 | — | na | d | `Config Viewer…` — browse the A3 config tree. TBD's analogue is the asset **registry** (`GET /api/v1/registry`), already a product feature, not editor chrome | — |
| MENU-TOOLS-004 | — | na | d | `Animations Viewer…` — 3D animation browser | — |
| MENU-TOOLS-005 | — | na | d | `Camera…` — A3 camera scripting tool | — |
| MENU-TOOLS-006 | — | na | d | `Field Manual…` — in-game manual | — |

### 2.7 `Settings` menu (`MENU-SET-001…005`) — batch03 `163909`

| eden_id | tbd_id | parity | build_class | gap_notes | ticket |
|---|---|---|---|---|---|
| MENU-SET-001 | render-prefs block inside Mission Settings (`eden_chrome.rs:4057`) | partial | a | `Preferences…  Ctrl+K` — Eden's **editor** preferences, cleanly separated from game options by a menu separator (worth copying). TBD has editor prefs (basemap view, 12 world layers) but files them under *mission* settings, where they do not belong: they are `localStorage`, not document state | NEW-F2 |
| MENU-SET-002 | — | na | d | `Video Options…` — game client video settings; §5 | — |
| MENU-SET-003 | — | na | d | `Audio Options…`; §5 | — |
| MENU-SET-004 | — | na | d | `Game Options…` — A3 gameplay difficulty settings | — |
| MENU-SET-005 | — | na | d | `Controls…` — A3 key bindings UI. A TBD keybinding editor is conceivable but nothing is rebindable today (10 hard-coded bindings) | — |

### 2.8 `Play` (`MENU-PLAY-001…005`) and `Help` (`MENU-HELP-001…007`) — batch04 `163940`/`163950`

| eden_id | tbd_id | parity | build_class | gap_notes | ticket |
|---|---|---|---|---|---|
| MENU-PLAY-001 | — | na | d | `Play in Singleplayer (SP)  Enter`. A browser tab cannot host an Arma client. TBD's preview path is: Save Version → a game server pulls `GET /missions/:id/compiled` (T-092.2); §5 | — |
| MENU-PLAY-002 | — | na | d | `Play in SP with Briefing  Shift+Enter` | — |
| MENU-PLAY-003 | — | na | d | `Play in SP at Camera Position  Ctrl+Shift+Enter` | — |
| MENU-PLAY-004 | — | na | d | `Spectate in SP` — and TBD refuses `spectatorPolicy` outright (`eden_chrome.rs:4686-4694`) | — |
| MENU-PLAY-005 | — | na | d | `Play in Multiplayer (MP)` — hosts locally. TBD's MP path is the real dedicated server | — |
| MENU-HELP-001 | — | missing | a | `Documentation…` — the only Help entry with a real TBD target (the doctrine wiki route `/wiki`). One link in the editor chrome | NEW-F3 |
| MENU-HELP-002 | — | na | d | `Scripting…` — SQF reference | — |
| MENU-HELP-003 | — | na | d | `Community Wiki…` — Bohemia web property | — |
| MENU-HELP-004 | — | na | d | `Forums…` | — |
| MENU-HELP-005 | — | na | d | `Feedback Tracker…` | — |
| MENU-HELP-006 | — | na | d | `Dev Hub…` | — |
| MENU-HELP-007 | — | na | d | `Tutorials…` — in-game tutorials (the one Help item with a graduation-cap glyph instead of `↗`, because it stays in-client) | — |

### 2.9 Menu-system mechanics (`MENU-UX-001…006`) — batch02 §Layout rules

These are not commands; they are the behaviours a reimplementation either copies or gets wrong.
Three of the six are things the corpus says **not** to copy.

| eden_id | tbd_id | parity | build_class | gap_notes | ticket |
|---|---|---|---|---|---|
| MENU-UX-001 | dropdown (`eden_chrome.rs:983-994`, `w-64`, `animate-menu-in`) | partial | a | Four-column row grid: **checkmark gutter → icon gutter → label → right-aligned shortcut**. TBD renders label-only rows. Shortcuts appear nowhere in a TBD menu | T-634 (extend) |
| MENU-UX-002 | — | na | d | **The check gutter is allocated only when a menu contains a checked item**, so label indent shifts between menus. batch02:650-652 flags this as a defect to fix, not copy. Recorded as a design rule, not a feature | — |
| MENU-UX-003 | click-away scrim `:1017-1030`, `Escape` `:872-880` | partial | a | Submenus open right, flip left when they would cross the viewport edge (`batch01:464`), parent stays highlighted, only one open at a time. **TBD has no submenus at all** | T-634 (extend) |
| MENU-UX-004 | — | missing | a | **Disabled items keep their layout and grey the shortcut too.** TBD's only disabled menu-adjacent controls are the History/Ruler/LoS buttons; menu items are never disabled. Folds into the state-vocabulary work | T-668 |
| MENU-UX-005 | — | missing | a | `…` suffix = "opens a dialog". TBD is accidentally consistent (`Save Version…`, `Mission Settings…`, `Export Compiled Mission…`) but `Export JSON` opens no dialog and correctly has none. Worth pinning as a rule | T-668 |
| MENU-UX-006 | — | na | d | **Open menu dims the background; a modal dims *and* diagonally hatches it** — two distinct "not now" treatments (batch02:659-660, batch03:76). The hatch is the single most-praised affordance in the corpus, but it is a scrim style, not a feature; the row exists so the distinction is not lost. Style input to T-668 | T-668 |

### 2.10 Toolbar (`TOOLBAR-001…026`) — batch04/05/06 tooltip sweep

**A reconciliation the README does not carry.** batch02 (`:71-74`) *inferred* the three buttons at
x 280–340 as waypoint-snap / surface-snap / vertical-mode. batch05 (`:204-232`) **hovered all three
and read the tooltips verbatim**: `Toggle Widget Coordinate Space`, `Toggle Vertical Mode
(adiaeresis)`, `Toggle Surface Snapping (')`. The tooltip wins, by the same rule the README uses to
prefer batch06 over batch05 on the F-tabs. **batch02's toolbar inference for those three is wrong and
is not used here.** There is no waypoint-snapping toolbar button.

TBD has no Eden-style toolbar. Its 3 tool buttons live in the **bottom** toolbelt
(`eden_chrome.rs:3707-3717`) and 2 of the 3 are `disabled=true` with a `(soon)` title.

| eden_id | tbd_id | parity | build_class | gap_notes | ticket |
|---|---|---|---|---|---|
| TOOLBAR-001 | `TOOLBAR-NEW-001`; Library `New Mission` (`missions.rs:212-231`) | match | a | Tooltip verbatim `New (Ctrl+N)`. Mirrors `MENU-SCEN-001` | — (shipped T-048) |
| TOOLBAR-002 | route navigation | match | a | `Open (Ctrl+O)`. Mirrors `MENU-SCEN-002` | — |
| TOOLBAR-003 | `Save Version` button (`eden_chrome.rs:1173`) | partial | a | `Save (Ctrl+S)`. Button exists, key does not | NEW-F1 |
| TOOLBAR-004 | — | na | d | Steam logo, never hovered. `INFERRED:` Publish to Steam Workshop; §5 | — |
| TOOLBAR-005 | strip Undo (`eden_chrome.rs:1141-1155`) | match | a | `Undo (Ctrl+Z)`. Icon at full white = enabled — Eden encodes enablement in **ink value**, TBD in `disabled` + opacity | — (shipped T-052) |
| TOOLBAR-006 | strip Redo (`:1156-1171`) | match | a | `Redo (Ctrl+Y)`. **The single best interaction finding in the toolbar sweep**: a disabled Eden button takes *no hover state at all* yet **still shows its tooltip** (batch04 `164031`, pixel-diff against the no-hover frame = zero). Explanation without a false affordance. TBD's disabled buttons keep `title` — same idea, unpinned | T-668 |
| TOOLBAR-007 | Select tool (`:3707-3711`) | partial | a | `No Widget (1)` — drawn with a persistent flat grey frame = **active**, distinct from the orange **hover** fill. Both states can be visible at once and stay unambiguous (batch05 §1) | T-648 · T-668 |
| TOOLBAR-008 | — | missing | a | `Translation Widget (2)` | T-648 |
| TOOLBAR-009 | — | missing | a | `Rotation Widget (3)` | T-648 |
| TOOLBAR-010 | — | missing | a | `Area Scaling Widget (4)` | T-079 (extend) |
| TOOLBAR-011 | — | missing | a | `Area Widget (5)` | T-079 (extend) |
| TOOLBAR-012 | — | na | d | `Toggle Widget Coordinate Space` — flips gizmo axes world ↔ local. Meaningless without a gizmo **and** without per-object local axes in a top-down 2D view. Unbound in Eden too | — |
| TOOLBAR-013 | — | na | d | `Toggle Vertical Mode (adiaeresis)` — duplicate of `MENU-EDIT-018`; same 2D substitute | — |
| TOOLBAR-014 | terrain-follow Z (`store.rs:1356-1358`) | match | a | `Toggle Surface Snapping (')` — duplicate of `MENU-EDIT-021`; TBD's is always-on | — |
| TOOLBAR-015 | — | missing | a | `Toggle Translation Grid (odiaeresis)` — duplicate of `MENU-EDIT-012` | T-648 |
| TOOLBAR-016 | — | missing | a | Translation-grid **step caret**. The caret is a *separate hit target* from the button body — batch05 `164132` proves it (hover highlight covers 360–379 only, caret 380–389 stays idle). Copy the split-button, not just the toggle | T-648 |
| TOOLBAR-017 | — | missing | a | `Toggle Rotation Grid` | T-648 |
| TOOLBAR-018 | — | missing | a | Rotation-grid step caret (angle step) | T-648 |
| TOOLBAR-019 | — | missing | a | `Toggle Area Scaling Grid` | T-648 |
| TOOLBAR-020 | — | missing | a | Area-scaling step caret | T-648 |
| TOOLBAR-021 | `Environment ▸ Time & Weather…` (`eden_chrome.rs:158-163`) + strip scrubber/select (`:1084`, `:1105`) | match | a | Cloud+sun icon = `Attributes ▸ Environment… Ctrl+I`. **The one place Eden puts the same glyph in a menu gutter and a toolbar button for the same command** (batch03 §H.9) — a cheap consistency cue TBD has no equivalent of | — |
| TOOLBAR-022 | `Forest mass` / `Trees` layers **or** basemap radio | UNKNOWN | a | **The corpus contradicts itself and this is not guessed.** The button at x ≈ 490–508 is read as *four vertical blades/curtains* by batch01 (`:362`), batch02 (`:80`) and batch03 (`:55`) — all three infer **Toggle Foliage**; batch06 (`:330`) reads it as a *folded map* and calls it **Toggle Map**, "confirmed from the frame 8 s later" (`170020` → the 2D-map batch 07 at `170028`). Never hovered, so no tooltip settles it. **If Toggle Foliage → `match`** (world-layer toggles, T-173). **If Toggle Map → `na`** (TBD is always 2D, cf. `MENU-VIEW-004`). Either way no new work; recorded so the count is honest | — |
| TOOLBAR-023 | — | na | d | Lightbulb = `View ▸ Toggle Flashlight (L)` per batch02 `:81`; batch03/06 read it as a lighting/time-of-day preview. Both readings are 3D-scene lighting; `na` under either | — |
| TOOLBAR-024 | — | na | d | Binoculars = `View ▸ Vision Mode` per batch02 `:612`; batch03/06 read it as a view-distance preview. `nightVision`/`thermals`/`viewDistance` are all refused with tests (`eden_chrome.rs:4623-4630`, `:4686-4694`); `na` under either reading | — |
| TOOLBAR-025 | — | na | d | Combo box reading `Scenario`. `INFERRED:` the **phase** selector, matching `Edit ▸ Phase` (batch02 `:613`) — batch01 `:56` flags it *uncertain*, batch06 `:333` guesses "active layer / edit context". No Enfusion analogue for A3 phases; if the alternate reading (active layer) is right, TBD already has an active layer (`activeLayerId`, T-033) | — |
| TOOLBAR-026 | `TOOLBAR-TUTORIAL-001` | na | d | Mortarboard glyph with a red `!` unread badge at the far top-right. In-game tutorials; §5. The **badge pattern** (unread marker on a chrome button) is the copyable part and is input to T-668 | T-668 |

### 2.11 Viewport context menu (`CTX-001…039`) — batch01, both selection states

Eden's headline lesson here: **menu contents are keyed on selection state, not just greyed** — 6 rows
with nothing selected, 14 with one unit. And two *different* unavailability strategies are used
deliberately in the same menu system: the `Edit` submenu keeps a stable 5-row shape and greys
Cut/Copy/Delete (muscle memory), while `Select` and `Log` **drop** inapplicable rows entirely.

**TBD has no context menu at any surface.** `contextmenu` is unconditionally suppressed
(`mission_editor.rs:1844-1847`) and RMB is a pan button (`:1402`). **T-662 frees the button, T-664
builds the menu** — those two are the gate for most of this section, and every row below maps to
T-664 rather than proposing anything new.

| eden_id | tbd_id | parity | build_class | gap_notes | ticket |
|---|---|---|---|---|---|
| CTX-001 | — | missing | a | `Go Here` (camera icon) — teleport the editor camera to the clicked point. Present in **both** takes. TBD's camera already has `center_on_selection`; this is centre-on-click | T-664 |
| CTX-002 | — | na | d | `Play from Here` (no-selection take) — preview with the player spawned at this point. In-client preview; §5 | — |
| CTX-003 | — | na | d | `Play as the Character` (selection take) — replaces the row above when a unit is selected. Same reason | — |
| CTX-004 | — | missing | a | `Select ▶` submenu parent | T-664 |
| CTX-005 | — | missing | a | `Edit ▶` submenu parent (clipboard) | T-664 |
| CTX-006 | — | na | d | `Log ▶` submenu parent — debug logging to the OS clipboard for script work | — |
| CTX-007 | — | missing | a | `Place Comment` (speech-bubble icon, no-selection take only) — drop an annotation at the clicked point. T-651 is exactly this ticket, and its entry point is this menu row | T-651 |
| CTX-008 | — | missing | a | `Connect ▶` submenu parent (selection take only) — the entity link/attachment family | T-672 |
| CTX-009 | — | missing | a | `Transform ▶` submenu parent (selection take only) | T-664 |
| CTX-010 | — | missing | a | `Grid ▶` submenu parent (selection take only) — set grid step from the object's own bounding box | T-648 |
| CTX-011 | — | missing | a | `Save Custom Composition…` (3-node icon) — the save entry point compositions need | T-650 |
| CTX-012 | asset search (`asset_catalog.rs:396-414`) | missing | a | `Find in Asset Browser…` (magnifier) — reveal the selected entity's class in the right panel. The tree and the filter both exist; the reveal-and-scroll does not | T-646 (extend) |
| CTX-013 | — | na | d | `Find in Config Viewer…` (`{ }`) — opens the A3 config tree; cf. `MENU-TOOLS-003` | — |
| CTX-014 | `open_arsenal` (`editor_ops.rs:605-614`, forces tab 3) | partial | a | `Edit Loadout…` (pistol icon). TBD has a full Arsenal (14 pick rows, `arsenal_rules.rs:49-162`) but reaches it only through the Attributes modal, not a context menu | T-664 |
| CTX-015 | — | missing | a | `Reset Loadout` — revert to the class default. TBD writes one `loadout` key (`arsenal.rs:682` → `store.rs:1319`); clearing it is trivial and has no UI | T-664 |
| CTX-016 | dbl-click → Attributes (`mission_editor.rs:1963`) | partial | a | `Attributes…`. TBD's only entry point is double-click, and it is **suppressed for multi-selection** (`editor_ops.rs:583-585`) and works only for slots — not vehicles, objects or zones | T-664 · T-647 |
| CTX-017 | — | missing | c | `Connect ▸ Sync to` — begin a synchronisation drag to another entity. No sync/link graph exists | T-672 |
| CTX-018 | ORBAT Manager squad refile (`orbat_manager.rs`), `store.rs:1527` | partial | a | `Connect ▸ Group to` — add this unit to another group. TBD does this in the **ORBAT Manager tree**, never on the map. `interactions_sweep.md` collision 5 governs the map gesture | T-647 |
| CTX-019 | — | missing | a | `Connect ▸ Set Trigger Owner` — no trigger entity to own | T-079 |
| CTX-020 | — | missing | a | `Select ▸ Select All in View  Ctrl+A` — the **only** row present in both takes of this submenu | T-649 |
| CTX-021 | — | missing | a | `Select ▸ Select Matching Classes (Selected)`. Omitted, not greyed, when nothing is selected | T-649 |
| CTX-022 | — | missing | a | `Select ▸ Select Matching Classes (View)` | T-649 |
| CTX-023 | — | missing | a | `Select ▸ Select Matching Types (Selected)`. `(Selected)`/`(View)` is Eden's scope-qualifier convention — cheap disambiguation worth copying verbatim | T-649 |
| CTX-024 | — | missing | a | `Select ▸ Select Matching Types (View)` | T-649 |
| CTX-025 | — | missing | a | `Edit ▸ Cut  Ctrl+X`. Both primitives already exist (`copy_selection` `editor_ops.rs:394` + `delete_selection` `:327`) — one match arm | T-669 |
| CTX-026 | `Ctrl/Cmd+C` (`mission_editor.rs:1020`) | match | a | `Edit ▸ Copy  Ctrl+C` | — (shipped T-056) |
| CTX-027 | `Ctrl/Cmd+V` (`mission_editor.rs:1023`) | match | a | `Edit ▸ Paste  Ctrl+V` — TBD pastes at the map cursor, exactly as Eden does | — (shipped T-056) |
| CTX-028 | — | missing | a | `Edit ▸ Paste on Original Position  Ctrl+Shift+V`. The primitive exists — `paste_at_cursor(None, None)`; TBD's `KeyV` arm requires `!shift`, so the key is free | T-669 |
| CTX-029 | `Delete`/`Backspace` (`mission_editor.rs:1027`) | match | a | `Edit ▸ Delete  Delete`. See `MENU-VIEW-014` — the `Backspace` alias is the dangerous half | — (shipped T-036) |
| CTX-030 | leader fields (`orbat_manager.rs`, `leaderSlotId`) | partial | a | `Transform ▸ Set as Group Leader`. TBD sets a leader in the ORBAT Manager; not from the map | T-664 |
| CTX-031 | `CTX-FORMATION-001` | missing | c | `Transform ▸ Move to Formation` — snap a unit back into its group's formation slot. Needs a formation model; `T-678` (wb) owns group AI formation | T-678 (wb) |
| CTX-032 | terrain-follow Z (`store.rs:1356-1358`) | match | a | `Transform ▸ Snap to Surface` — TBD does this on every move | — |
| CTX-033 | — | na | d | `Transform ▸ Orient to Terrain Normal` — align the up-vector to the slope. A top-down 2D editor authors one rotation about Z; pitch/roll have no control and no compiled field | — |
| CTX-034 | — | na | d | `Transform ▸ Orient to Sea Normal` — same reason | — |
| CTX-035 | — | missing | a | `Grid ▸ Use X (Width) as Grid` — set the snap step from the selection's own bounding-box width. Depends on a grid existing at all | T-648 |
| CTX-036 | — | missing | a | `Grid ▸ Use Y (Length) as Grid` | T-648 |
| CTX-037 | — | na | d | `Grid ▸ Use Z (Height) as Grid` — no vertical grid in a 2D editor | — |
| CTX-038 | — | na | d | `Log ▸ Log Position to Clipboard` — writes world coords as text for script work. TBD shows live X/Y/Z in the toolbelt (`eden_chrome.rs:3722-3744`); no script consumer for a clipboard dump | — |
| CTX-039 | — | na | d | `Log ▸ Log Classes to Clipboard` — same | — |

### 2.12 Context-menu mechanics (`CTX-UX-001…004`)

| eden_id | tbd_id | parity | build_class | gap_notes | ticket |
|---|---|---|---|---|---|
| CTX-UX-001 | — | missing | a | **Contents keyed on selection state** — 6 rows empty vs 14 rows with a unit; whole blocks appear rather than grey. This is the architecture of the menu, not a nicety | T-664 |
| CTX-UX-002 | — | missing | a | **Two unavailability strategies, deliberately mixed**: clipboard verbs keep their slots and grey; scope/query verbs are omitted. batch01 `:459-460` states the rule of thumb explicitly | T-664 · T-668 |
| CTX-UX-003 | — | missing | a | **Submenu flip and screen fit** — submenus open right (parent right + 6 px), flip left when they would cross the viewport bound, and the parent menu repositions rather than scrolling when it would overflow the bottom (take B is 447 px and ends flush at y 1072) | T-664 |
| CTX-UX-004 | — | missing | a | **Submenu width is fitted to the longest label** (169–266 px observed), not fixed. Row pitch 23 px, panel 232–234 px, separators ~12 px | T-664 |

### 2.13 The shared modal shell + control vocabulary (`DLG-SHELL-001…014`) — batch03 §shell

One shell, five dialogs, **zero layout variance**. This is the section with the highest
copy-value-per-row in the whole sweep: build these fourteen once and Environment, Multiplayer,
Performance and Preferences are all layout.

| eden_id | tbd_id | parity | build_class | gap_notes | ticket |
|---|---|---|---|---|---|
| DLG-SHELL-001 | Mission Settings dialog (`eden_chrome.rs:3788`), `ui.rs` Dialog | partial | a | `Edit: <category>` orange title bar, black text, **no window buttons** — OK/CANCEL only. TBD's dialogs have a title but no category-prefix convention | T-668 |
| DLG-SHELL-002 | — | missing | a | **Fixed 560 × 679 body, not content-driven** — Preferences leaves 230 px of empty body rather than shrinking. Deliberate: the footer never moves between dialogs | T-668 |
| DLG-SHELL-003 | Mission Settings has no commit/cancel pair | missing | a | `OK` / `CANCEL` pinned bottom-right, 100 × 19, no Apply, no Help. **TBD's Mission Settings commits live on every `on:change`** (`eden_chrome.rs:3919-3924`) — a different and defensible model, but there is no cancel path and no batch commit anywhere in the editor | NEW-F2 |
| DLG-SHELL-004 | — | missing | a | **Scrollbar drawn only when content overflows** (Performance and Preferences have none). Group headers scroll with the content — no sticky headers | T-668 |
| DLG-SHELL-005 | scrim (`ui.rs` Dialog backdrop) | partial | a | **Modal scrim is a diagonal hatch that *lightens*, not a dim** — so the dark dialog is the highest-contrast thing on screen. The corpus's single most-praised affordance (batch03 §H.1). TBD uses a conventional dark scrim | T-668 |
| DLG-SHELL-006 | Mission Settings sections (`render_flow_section` `:3927`, `render_prefs_section` `:4057`) | partial | a | **Collapsible group header** — solid triangle + grey caption + full-width hairline. TBD's sections are headings without a disclosure control | T-668 |
| DLG-SHELL-007 | — | missing | a | **Non-collapsible sub-header** — indented, no triangle, short vertical tick, lighter band. The two-level hierarchy is what lets Performance and Environment have structure with **no tab strip at all** | T-668 |
| DLG-SHELL-008 | — | missing | a | **Dependency gating never hides a control**: disabled rows keep position, label and value (*including a checked tick*) and drop to grey. Two mechanisms shown — a per-group `Manual Override` checkbox, and a mode combo whose value implies its dependents. TBD has no gated control anywhere | T-668 |
| DLG-SHELL-009 | hillshade slider (`eden_chrome.rs:4143`), time scrubber (`:1084`) | partial | a | **Slider composite `[◀][track][▶][value box]`, value box carries the unit** (`30%`, `0 m`, `500m`, `2x`, `00:00:06`). One widget covers every numeric range in three dialogs. TBD's two `type="range"` inputs are bare, no arrows, no editable value box | T-633 |
| DLG-SHELL-010 | `<input type="time">` (`eden_chrome.rs:3850-3868`) | partial | a | **Time field = three independently hoverable, independently editable `HH : MM : SS` segments**, each with its own tooltip (`Hours` captured at `163811`). TBD uses the native browser time input — a defensible substitute, and the `:SS` tail is already tolerated (`hhmm_to_minutes`) | — |
| DLG-SHELL-011 | — | missing | a | **Tooltips on dialog fields** (`Forecasted rain strength.`) — dark unbordered box, overlays content. TBD's dialog fields carry no help text at all | T-668 |
| DLG-SHELL-012 | — | missing | a | **Two-column grid with right-aligned labels** ending at a fixed x, controls starting at a fixed x, narrow numerics ending early. Five dialogs, identical grid | T-668 |
| DLG-SHELL-013 | — | missing | a | **Checkbox-list control** (`Rulesets`) — a bordered list box with checkbox-**left** rows, against the dialog's otherwise universal label-right/checkbox-right layout. Recorded as the one internal inconsistency | — |
| DLG-SHELL-014 | — | missing | a | **Axis-tagged numeric field** — a small blue `Z` chip immediately left of the number box (`Direction Start`). Same idea as Eden's red/green/blue X/Y/Z chips in the entity dialog; TBD's Transform tab labels axes with bare letters | T-082 |

### 2.14 `Edit: General` (`DLG-GEN-001…027`) — batch03 `163629`/`163642`

Eden's scenario-presentation dialog. **Two rows are the real gap** (`Overview Text`, `Overview
Picture`) and T-671 already owns both. Fifteen are `na`: A3 campaign keys, DLC gating, binarisation,
debug console and the player-HUD flags.

| eden_id | tbd_id | parity | build_class | gap_notes | ticket |
|---|---|---|---|---|---|
| DLG-GEN-001 | `meta.title` — strip input (`eden_chrome.rs:1047-1063`) | match | a | `Title`. TBD authors it in the **top strip**, not the settings dialog. Note the editor title does **not** PATCH the `missions` row (`RowMirror` sends `time_of_day` + `weather` only, `:501-508`) | — |
| DLG-GEN-002 | server `author_id` (`dto.rs:828`) | na | d | `Author` (value `Darkforce` — the only non-default in the whole batch). TBD assigns authorship from the Discord session; an author-editable field would be a **security regression**, not a feature | — |
| DLG-GEN-003 | `missions.thumbnail` API-side only (`dto.rs:850`, `handlers/missions.rs:658`) | missing | a | `Picture` — mission-select overview image. The DTO and the API accept it; **no frontend caller sends it** | T-671 |
| DLG-GEN-004 | `meta.briefing` — hydrate-only (`store.rs:1658-1663`) | missing | a | `Text` — overview description. **The single clearest hole in the scenario surface**: `PATCH /missions/:id` accepts `briefing` (`handlers/missions.rs:657`) and reaches the doc on hydrate, but nothing in the SPA can author it | T-671 |
| DLG-GEN-005 | — | na | d | `DLC` combo. A3 DLC gating; TBD's analogue is the **modpacks** platform feature, outside the editor | — |
| DLG-GEN-006 | — | na | d | `Require DLC` checkbox — same | — |
| DLG-GEN-007 | — | na | d | `Overview (Locked) ▸ Picture` — shown while the mission is campaign-locked. No campaign progression model | — |
| DLG-GEN-008 | — | na | d | `Overview (Locked) ▸ Text` — same | — |
| DLG-GEN-009 | — | na | d | `Loading Screen ▸ Picture`. `INFERRED:` the Reforger loading screen is not mission-authored; no `apps/mod/tbd-framework` consumer found | — |
| DLG-GEN-010 | — | na | d | `Loading Screen ▸ Text` — same | — |
| DLG-GEN-011 | `briefingSeconds` (`AUTHORED_FLOW_KEYS` `eden_chrome.rs:332`, default 600 `:377`) | partial | a | `Show Briefing` — Eden authors **availability**, TBD authors **duration** (and the mod reads it: `TBD_FrameworkManager.OnEnterBriefing`). Different axis of the same feature; no gap worth a slice | — |
| DLG-GEN-012 | — | na | d | `Show Debriefing`. No debriefing screen exists in `tbd-framework`; a boolean with no reader is exactly what `eden_chrome.rs:4619-4621` forbids | — |
| DLG-GEN-013 | — | na | d | `Enable Saving` — mid-mission saves. TBD events are one life, multiplayer, server-hosted; §5 | — |
| DLG-GEN-014 | — | missing | c | `Show Map` — gate the player's map gadget. Enfusion has per-gadget availability; **nothing in the mission document carries it** and `flatten` emits no such block | NEW-W1 |
| DLG-GEN-015 | — | missing | c | `Show Compass` | NEW-W1 |
| DLG-GEN-016 | — | missing | c | `Show Watch` | NEW-W1 |
| DLG-GEN-017 | — | missing | c | `Show GPS` | NEW-W1 |
| DLG-GEN-018 | — | missing | c | `Show HUD` | NEW-W1 |
| DLG-GEN-019 | — | na | d | `Show UAV Feed`. `INFERRED:` Reforger has no UAV/drone feed to gate | — |
| DLG-GEN-020 | — | na | d | `Advanced Flight Model` — A3 helicopter AFM. No AFM concept in Reforger | — |
| DLG-GEN-021 | — | na | d | `Debug Console` combo (`Available only in editor`). Script console access; the brief names it out of scope; §5 | — |
| DLG-GEN-022 | — | na | d | `Unlocked Keys` — A3 campaign progression keys | — |
| DLG-GEN-023 | — | na | d | `Required Keys` — same | — |
| DLG-GEN-024 | — | na | d | `Required Keys Limit` — same | — |
| DLG-GEN-025 | — | na | d | `Init` multiline code box — scenario init SQF. **Scripting handle with no scripting layer** | — |
| DLG-GEN-026 | `EDEN_SIDE_CHIPS` are placement-side, not relations (`eden_chrome.rs:2871`) | missing | c | `Independents Allegiance` — a 2-option icon toolbox choosing which side INDFOR is friendly to. TBD has an INDFOR chip but **no faction-relations model** anywhere in schema or mod. Not worth a ticket before a consumer exists | — |
| DLG-GEN-027 | — | na | d | `Binarize the Scenario File` — `mission.sqm` file format. TBD's document is JSON over HTTP | — |

### 2.15 `Edit: Environment` (`DLG-ENV-001…027`) — batch03 `163720`/`163735`

**The most `na`-dense dialog in the sweep, and it is the correct answer.** `fog` and `wind` are named
in the refusal test verbatim (`eden_chrome.rs:4623-4630`), and Eden's whole start/forecast weather
model has no counterpart: TBD carries a **four-value weather enum** (`clear` / `overcast` /
`heavy_rain` / `dense_fog`, `eden_chrome.rs:3877-3895`) plus `time`, and `CARRIED_ENV_KEYS` is pinned
at five keys by a test.

`T-682 (wb)` in the ticket column means: **not factory work, and it reopens only if the workbench
row lands a mod reader.** It is a mapping, not a proposal.

| eden_id | tbd_id | parity | build_class | gap_notes | ticket |
|---|---|---|---|---|---|
| DLG-ENV-001 | — | missing | c | `Date` — three combos (year `2035` / month `June` / day `24`, weekday shown as a greyed secondary label). TBD's environment carries no date; a season/date would need a schema key **and** a mod reader | T-682 (extend, wb) |
| DLG-ENV-002 | `environment.time` — scrubber (`:1084`) + `<input type="time">` (`:3850-3868`) | match | a | `Time`. TBD authors it on **two surfaces** that mirror to the `missions` row (T-192). Eden's is skinned to its domain — day/night gradient track, sun-glyph thumb, secondary progress bar — which TBD's bare range input is not | T-633 |
| DLG-ENV-003 | — | na | d | `Time of Changes` — how long weather takes to reach the forecast. **No forecast model exists**: TBD's weather is a single enum value for the whole round | — |
| DLG-ENV-004 | `overcast` weather preset (`:3877-3895`) | partial | c | `Overcast Start` (`30%`) — a continuous 0–100 % channel. TBD has it only as one enum value | T-682 (extend, wb) |
| DLG-ENV-005 | — | na | d | `Overcast Forecast` — no forecast model | — |
| DLG-ENV-006 | `dense_fog` weather preset | na | d | `Fog Start`. **`fog` is named verbatim in the refusal test** `keys_nothing_reads_are_not_authored` (`eden_chrome.rs:4623-4630`) — nothing in the mod reads it; §5 | T-682 (wb) |
| DLG-ENV-007 | — | na | d | `Fog ▸ Decay (start)` — fog thinning with altitude. Same refusal | T-682 (wb) |
| DLG-ENV-008 | — | na | d | `Fog ▸ Base (start)` — fog base altitude in metres. Same refusal | T-682 (wb) |
| DLG-ENV-009 | — | na | d | `Fog Forecast`. Same refusal + no forecast model | T-682 (wb) |
| DLG-ENV-010 | — | na | d | `Fog ▸ Decay (forecast)` — Eden **duplicates the label** inside the group; the only place in the corpus it does | T-682 (wb) |
| DLG-ENV-011 | — | na | d | `Fog ▸ Base (forecast)` — duplicate label | T-682 (wb) |
| DLG-ENV-012 | — | na | d | `Rain ▸ Manual Override` — the per-channel gate checkbox. The **gating pattern** is captured as `DLG-SHELL-008`; the channel itself is refused | — |
| DLG-ENV-013 | `heavy_rain` weather preset | partial | c | `Rain Start` (`0%`, disabled). TBD has rain as one enum value, not a channel | T-682 (extend, wb) |
| DLG-ENV-014 | — | na | d | `Rain Forecast` — no forecast model. Carried the one field tooltip in the batch: *"Forecasted rain strength."* | — |
| DLG-ENV-015 | — | na | d | `Lightnings ▸ Manual Override` | — |
| DLG-ENV-016 | — | na | d | `Lightnings Start`. No lightning channel in schema or mod | — |
| DLG-ENV-017 | — | na | d | `Lightnings Forecast` | — |
| DLG-ENV-018 | — | na | d | `Waves ▸ Manual Override` | — |
| DLG-ENV-019 | — | na | d | `Waves Start` — sea state. No sea-state model; TBD's sea is a static render layer | — |
| DLG-ENV-020 | — | na | d | `Waves Forecast` | — |
| DLG-ENV-021 | — | na | d | `Wind ▸ Manual Override` | — |
| DLG-ENV-022 | — | na | d | `Wind Start`. **`wind` is named verbatim in the refusal test** (`eden_chrome.rs:4623-4630`); §5 | T-682 (wb) |
| DLG-ENV-023 | — | na | d | `Wind Forecast`. Same refusal | T-682 (wb) |
| DLG-ENV-024 | — | na | d | `Gusts Start` | T-682 (wb) |
| DLG-ENV-025 | — | na | d | `Gusts Forecast` | T-682 (wb) |
| DLG-ENV-026 | `windDirDeg` — **declared in schema, authored by nothing** | na | d | `Direction Start` — a compass bearing on a blue `Z` chip. This is the exact example `eden_chrome.rs:4619-4621` uses: *"the schema HAS a slot for it … A schema field is not a reader."* The purest `na` in the sweep | T-682 (wb) |
| DLG-ENV-027 | — | na | d | `Direction Forecast` — same | T-682 (wb) |

### 2.16 `Edit: Multiplayer` (`DLG-MP-001…023`) — batch03 `163758`/`163811`

**Refused wholesale by design.** `SETTINGS_UNREAD_NOTE` (`eden_chrome.rs:370`): *"Respawn, spectator
policy, night vision and per-faction tickets are not authored here — the mission document declares
them and no mod script reads them. **TBD events are one life.**"* Pinned by
`fields_with_no_mod_reader_get_no_control` (`:4686-4694`). The three rows that are **not** `na` are
the lobby-shape fields, and they are real gaps: min/max players and game mode are **create-time
only** and cannot be changed after a mission exists.

| eden_id | tbd_id | parity | build_class | gap_notes | ticket |
|---|---|---|---|---|---|
| DLG-MP-001 | create-dialog `Game Mode` select (`create_mission_dialog.rs:156-160`, posted `:73`) | partial | a | `Game Type` combo. Chosen at create time, **not editable in the editor** — a mission's shape cannot be changed after creation | NEW-F6 |
| DLG-MP-002 | — | missing | a | `Min Players`. TBD has no minimum-players concept at all | NEW-F6 |
| DLG-MP-003 | create-dialog `Max Players` select (`create_mission_dialog.rs:197-213`); compiled from the row (`dto.rs:895`) | partial | a | `Max Players`. Create-time only, same as game mode. Eden's is a free numeric; TBD's is a fixed ladder (16/32/48/64/96/128) | NEW-F6 |
| DLG-MP-004 | `meta.briefing` (hydrate-only) | missing | a | `Summary` — the lobby description. Same field family as `DLG-GEN-004`; one authoring surface should serve both | T-671 |
| DLG-MP-005 | — | na | d | `Enable AI` — AI fills empty slots. TBD slots are claimed by humans through ORBAT registration; **no AI-fill runtime exists** | — |
| DLG-MP-006 | — | na | d | `Auto Assign Slots`. TBD's slot assignment is the ORBAT/event system, a platform feature outside the editor | — |
| DLG-MP-007 | `respawn` — **declared, refused** | na | d | `Respawn` combo. Named in `fields_with_no_mod_reader_get_no_control`; `TBD_MissionDocumentStruct` has no `settings` member; §5 | — |
| DLG-MP-008 | — | na | d | `Rulesets` checkbox list (the container) | — |
| DLG-MP-009 | — | na | d | `Rulesets ▸ Mission fail when everyone is dead`. `INFERRED:` TBD's win/lose lives in `$defs/winConditions`, mission-global, and is not a respawn ruleset | — |
| DLG-MP-010 | — | na | d | `Rulesets ▸ Singleplayer death screen` | — |
| DLG-MP-011 | — | na | d | `Respawn Delay` (disabled — the clearest cascade in the corpus) | — |
| DLG-MP-012 | — | na | d | `Vehicle Respawn Delay` — stays **enabled** while player respawn is off, because it does not depend on it. Good gating design; no TBD vehicle-respawn runtime | — |
| DLG-MP-013 | — | na | d | `Show Scoreboard` (checked but greyed — Eden preserves the value, does not clear it) | — |
| DLG-MP-014 | — | na | d | `Allow Manual Respawn` | — |
| DLG-MP-015 | — | na | d | `Enable Team Switch` | — |
| DLG-MP-016 | — | na | d | `Allow AI Score` | — |
| DLG-MP-017 | `$defs/zone` objective types, `$defs/flow` | partial | a | `Shared Objectives` combo — task-sharing scope. TBD has objective zones (`objective_capture` / `_destroy` / `_hold_until`) and a radio/objectives runtime (T-181) but no sharing-scope setting | — |
| DLG-MP-018 | — | na | d | `Revive Mode`. One life by design | — |
| DLG-MP-019 | — | na | d | `Revive ▸ Required Trait` | — |
| DLG-MP-020 | — | na | d | `Revive ▸ Required Items` | — |
| DLG-MP-021 | — | na | d | `Revive Duration` | — |
| DLG-MP-022 | — | na | d | `Medic Speed Multiplier` | — |
| DLG-MP-023 | — | na | d | `Force Respawn Duration` | — |

### 2.17 `Edit: Performance` (`DLG-PERF-001…016`) — batch03 `163830`

**All sixteen `na`.** Garbage collection and dynamic simulation are Arma-engine runtime knobs; there
is no `apps/mod/tbd-framework` consumer for any of them and no Enfusion API surfaced for a mission to
set them. Recorded in full because "we skipped the Performance dialog" and "the Performance dialog is
out of scope" are different claims and only the second one is defensible.

| eden_id | tbd_id | parity | build_class | gap_notes | ticket |
|---|---|---|---|---|---|
| DLG-PERF-001 | — | na | d | `Garbage Collection ▸ Minimum distance` — don't clean up within this range of a player | — |
| DLG-PERF-002 | — | na | d | `Character Corpses ▸ Mode` | — |
| DLG-PERF-003 | — | na | d | `Character Corpses ▸ Limit` (`15`) | — |
| DLG-PERF-004 | — | na | d | `Character Corpses ▸ Min Delay` | — |
| DLG-PERF-005 | — | na | d | `Character Corpses ▸ Max Delay` | — |
| DLG-PERF-006 | — | na | d | `Vehicle Wrecks ▸ Mode` | — |
| DLG-PERF-007 | — | na | d | `Vehicle Wrecks ▸ Limit` | — |
| DLG-PERF-008 | — | na | d | `Vehicle Wrecks ▸ Min Delay` | — |
| DLG-PERF-009 | — | na | d | `Vehicle Wrecks ▸ Max Delay` | — |
| DLG-PERF-010 | — | na | d | `Enable Dynamic Simulation` — the master switch. A3-specific entity-sleep system | — |
| DLG-PERF-011 | — | na | d | `Activation Distance ▸ Characters` (`500m`) | — |
| DLG-PERF-012 | — | na | d | `Activation Distance ▸ Manned Vehicles` (`350m`) | — |
| DLG-PERF-013 | — | na | d | `Activation Distance ▸ Props` (`50m`) | — |
| DLG-PERF-014 | — | na | d | `Activation Distance ▸ Empty Vehicles` (`250m`) | — |
| DLG-PERF-015 | — | na | d | `Activation Distance Modifiers ▸ Is Moving` (`2x`) — the only `x`-unit value in the batch | — |
| DLG-PERF-016 | — | na | d | `Limit by View Distance`. `viewDistance` is also refused outright (`eden_chrome.rs:4623-4630`, T-193 removed the controls) | — |

### 2.18 `Edit: Preferences` — editor settings, not mission data (`DLG-PREF-001…013`) — batch03 `163916`

The distinction Eden draws with a **menu separator** — editor preferences above, game options below —
is one TBD has not drawn at all: its only editor preferences (basemap view, 12 world layers) sit
inside *Mission* Settings and are `localStorage`, not document state.

| eden_id | tbd_id | parity | build_class | gap_notes | ticket |
|---|---|---|---|---|---|
| DLG-PREF-001 | IDB autosave (`persistence/`, T-062.1) | partial | a | `Auto-save` combo (`15 min`). TBD autosaves continuously to IndexedDB with a debounce; the **interval is not configurable and not visible** | NEW-F2 |
| DLG-PREF-002 | — | na | d | `Binarize New Scenario Files` — file format default | — |
| DLG-PREF-003 | — | missing | a | `Camera ▸ Default Speed` (`1x`) — pan speed | NEW-F2 |
| DLG-PREF-004 | — | missing | a | `Camera ▸ Fast Speed` — shift-boost speed | NEW-F2 |
| DLG-PREF-005 | wheel zoom (`mission_editor.rs`, T-151.11.6) | missing | a | `Camera ▸ Mouse wheel Sensitivity`. Wheel zoom exists and is unconfigurable. (Label casing verbatim: lower-case "wheel", capital "Sensitivity") | NEW-F2 |
| DLG-PREF-006 | — | na | d | `Camera ▸ Copy Terrain` — `INFERRED:` camera follows terrain height. A 3D free-camera concern | — |
| DLG-PREF-007 | — | na | d | `Camera ▸ Adaptive Speed` — `INFERRED:` scale camera speed with altitude. Same | — |
| DLG-PREF-008 | — | na | d | `Camera ▸ Start in Map` — open the editor in 2D map mode. **TBD always starts in map mode**; the preference has no other state to choose | — |
| DLG-PREF-009 | — | missing | a | `Camera ▸ Start on Random Position`. Cheap; low value; grouped with the other camera prefs | NEW-F2 |
| DLG-PREF-010 | `ensureDefaultSquad` on place (T-033/T-180) | partial | a | `Misc ▸ Automatic Grouping` — auto-group units placed together. **TBD does this unconditionally** (every placed slot attaches to a squad); Eden makes it optional | — |
| DLG-PREF-011 | — | na | d | `Misc ▸ Recompile Functions` — recompile SQF on preview. No scripting layer | — |
| DLG-PREF-012 | — | na | d | `Misc ▸ Environmental Sounds` — ambient audio while editing. No audio in the SPA editor | — |
| DLG-PREF-013 | — | missing | a | `Misc ▸ Automatic Composition Layering` — put each placed composition on its own layer. Depends on compositions existing; the **layer** half already works (`activeLayerId`, T-033) | T-650 (extend) |

### 2.19 Entity attribute dialog — sections and widget vocabulary (`ENT-SEC-001…009`, `ENT-UX-001…005`)

batch02 `163121`–`163151` scrolls the `Edit: <entity>` modal top to bottom for one infantry unit:
**nine collapsible sections in one continuously scrolling column, no tabs.**

**Scope note — deliberately not duplicated.** The individual *fields* in these sections are exactly
what `attributes.md`'s **93 `ATTR-FIELD-*` ids** enumerate, and `attributes_sweep.md` already walked
all 93 against live source. This sweep adds only what a field-level census cannot carry: the
**section taxonomy** (which Eden groups belong together) and the **widget vocabulary**. TBD's answer
to the whole surface is 4 tabs and **nine per-slot fields** (`editor_ops.rs:631-663`).

| eden_id | tbd_id | parity | build_class | gap_notes | ticket |
|---|---|---|---|---|---|
| ENT-SEC-001 | `Type` is not editable post-place (`attributes.rs:16` TABS) | missing | a | `Object: Type` — a **class tree with its own search box** inside the dialog, letting you re-type a placed entity in place. TBD's `assetId` is fixed at drop; there is no re-type path | T-082 |
| ENT-SEC-002 | — | na | d | `Object: Init` — `Variable Name` + a multiline SQF `Init` box. Scripting handle with no scripting layer | — |
| ENT-SEC-003 | Transform tab (`attributes.rs:255-319`) | partial | a | `Object: Transformation` — Position X/Y/Z + Rotation X/Y/Z + `Placement Radius`. TBD has X/Y/Z + one rotation; **no pitch/roll, no placement radius** (radius is `T-679` (wb)) | T-082 |
| ENT-SEC-004 | — | missing | c | `Object: Control` — `Player`, `Playable`, `Role Description`. TBD's slots are all "playable" by construction (ORBAT slots); `Role Description` is the one field T-082 already names | T-082 |
| ENT-SEC-005 | States tab — **a stub that takes no arguments** (`attributes.rs:351`) | missing | c | `Object: States` — Skill / Health / Ammunition sliders + `Rank` (7-cell icon radio) + `Stance` (4-cell). TBD has **Stance** (Transform tab) and **Rank** (ORBAT Manager only, `orbat_manager.rs:1336-1355`); skill/health/ammo need a mod reader | T-681 (wb) |
| ENT-SEC-006 | — | missing | c | `Object: Special States` — Wake-Up Dynamic Simulation, Enable Simulation, Show Model, Enable Damage, Enable Stamina, Revive Enabled. All six are entity-runtime flags | T-681 (wb) |
| ENT-SEC-007 | Identity tab — Role / Tag / Squad (read-only) (`attributes.rs:321-348`) | partial | b | `Object: Identity` — Name, Face, Call Sign, Voice, Voice Pitch, Insignia. TBD has Role + Tag, and `callsign` lives only in the ORBAT Manager. `$defs/slot` is `additionalProperties: false` | T-674 (wb) |
| ENT-SEC-008 | — | na | d | `Object: Presence` — `Probability of Presence` slider + a `Condition of Presence` **SQF expression** (`true`). Scripting handle; the probability half would need a mod randomiser | — |
| ENT-SEC-009 | — | na | d | `Object: Electronics & Sensors` — Data Link Send / Receive / Position. A3 sensor/datalink model; no Enfusion analogue | — |
| ENT-UX-001 | hillshade + time sliders only (`eden_chrome.rs:1084`, `:4143`) | missing | a | **No per-entity slider exists anywhere in TBD** — the only two `type="range"` inputs are mission-level. Eden uses `◀ track ▶ [value]` for Skill, Health, Ammunition, Voice Pitch and Probability of Presence | T-082 · T-633 |
| ENT-UX-002 | `<select>` stance (`attributes.rs:297-312`) | partial | a | **Icon radio strip** for small enums — `Rank` 7 cells, `Stance` 4 cells including an explicit `⊘` "no preference" cell. TBD uses a text `<select>` for stance and has no "no preference" value | T-668 |
| ENT-UX-003 | — | missing | a | **Combo with an embedded asset thumbnail** — the Face combo renders the face, the Voice combo renders a flag. TBD's asset pickers are text-only | T-646 (extend) |
| ENT-UX-004 | — | na | d | **Inline audition button** — a `▶` beside the Voice combo that plays the sample. No audio in the SPA editor | — |
| ENT-UX-005 | toolbelt `Z` (`eden_chrome.rs:3736-3739`) | partial | a | **ATL vs ASL shown in two frames at once**: the dialog reads `Z 0` (above terrain) while the status bar reads `22.3452 m` (above sea) for the same point, and the status-bar glyph carries a terrain squiggle saying which frame it is. TBD shows one Z and never says which frame | T-670 (extend) |

### 2.20 Left panel — Entity List (`PANEL-L-001…018`)

TBD's `DockLeft` is **Editor Layers only** (`eden_chrome.rs:2806-2841`) — the ORBAT tree moved out to
a modal at T-177 B1. Nine of Eden's eighteen left-panel controls have no TBD counterpart at all.

| eden_id | tbd_id | parity | build_class | gap_notes | ticket |
|---|---|---|---|---|---|
| PANEL-L-001 | — | missing | a | `«` collapse chevron, 24 × 24 in the panel's **outer top corner**, inside the tab-strip row. Collapses to exactly that 24 px stub, which **overlays** the map; the viewport genuinely reflows 1440 → 1920 px (batch08 `:254-264`) | T-638 |
| PANEL-L-002 | `"Editor Layers"` heading (`eden_chrome.rs:2841`) | partial | a | `Entities` tab (also bound to `E`). TBD's dock has a heading, not a tab strip | T-638 |
| PANEL-L-003 | — | missing | a | `Locations` tab — the terrain's named places. TBD **has the data** (`world_assets/labels.rs`, `Town labels` layer) but no browsable list, and no way to fly to a named place | NEW-F7 |
| PANEL-L-004 | — | missing | a | Entity-tree **search field**. Absent entirely from `DockLeft` — cf. `MENU-VIEW-012` | T-666 (extend) |
| PANEL-L-005 | — | missing | a | Search **magnifier / run** button (both panels share the identical search-row kit — learn one, know both) | T-666 (extend) |
| PANEL-L-006 | outliner collapsed-set (`outliner.rs:312-369`) | partial | a | **Collapse-all** `−` button. Per-node collapse works (T-172 B6); there is no collapse-all control | T-666 (extend) |
| PANEL-L-007 | — | missing | a | **Expand-all** stacked-pages `+` button | T-666 (extend) |
| PANEL-L-008 | — | missing | a | **Per-row visibility checkbox** on every tree row, including the ten category roots. T-665 owns per-layer visibility; Eden's is per **row**, at every depth | T-665 |
| PANEL-L-009 | 4 layer roots at most (`activeLayerId`, T-033) | partial | a | **Ten fixed category roots** — BLUFOR / OPFOR / Independent / Civilian / Empty / Ambient life / Triggers / Systems / Markers / Comments — always shown, **empty ones greyed rather than hidden**, so the taxonomy is always legible. TBD's tree shows only what exists | T-666 (extend) |
| PANEL-L-010 | Editor Layers tree (`eden_chrome.rs:2806`) | partial | a | Side → group → unit hierarchy with a side-coloured icon per row. TBD's is layer → slot; the ORBAT hierarchy lives in a separate modal — **two trees for one document** | — |
| PANEL-L-011 | `Delete`/`Backspace` key (`mission_editor.rs:1027`) | partial | a | Footer **`Delete`** button (tooltip verbatim, batch06 `165920`), pushed hard left and alone — a deliberate mis-click guard against the four constructive buttons on the right. TBD has the key, no button | T-666 (extend) |
| PANEL-L-012 | `store.rs:1872` `add_editor_layer` — **one non-UI caller** | missing | a | Footer **`New Layer`** (tooltip verbatim, `165926`). The mutator's only caller is the default-layer seed (`editor_ops.rs:1136`); no UI path reaches it | T-666 |
| PANEL-L-013 | `store.rs:1895` `reparent_editor_layer` / `:1915` `move_slot_to_layer` — **zero call sites** | missing | a | Footer **`Move to Root`** (tooltip verbatim, `165932`) — moves the selection out of its layer to top level. batch01/02/03 all *inferred* this button as "disable layer"; **batch06's tooltip supersedes them**. Both mutators appear only in doc comments (`outliner.rs:131`, `:19`) | T-666 |
| PANEL-L-014 | — | missing | a | Footer **`Toggle Layer Transformation`** (tooltip verbatim, `165938`) — makes the layer transform as one rigid unit. Batches 01–03 inferred "lock layer"; the tooltip says otherwise. This is exactly T-665's "transform lock" | T-665 |
| PANEL-L-015 | — | missing | a | Footer **`Toggle Layer Visibility`** (tooltip verbatim, `165945`) | T-665 |
| PANEL-L-016 | selection tint (T-151.6) | partial | a | **Triple-redundant selection feedback**: cyan wireframe box in the viewport + a floating editor icon on a leader line + an **amber row highlight** in the outliner. TBD tints the map icon and highlights the tree row; no leader line, no floating icon | T-668 |
| PANEL-L-017 | — | UNKNOWN | a | The selected unit's tree label renders in **red**, distinct from the amber row highlight. Persists across all eight batches and **was never explained** (README §Caveats). `INFERRED:` the player unit. Not scored — the meaning is unknown, so the parity of a TBD equivalent is undefined | — |
| PANEL-L-018 | opaque dock (`overlayDocked`) | partial | a | Both panels are **translucent over the 3D view**, and the footer bar is anchored to the bottom of the **screen**, not the tree. TBD's docks are glass-translucent already; the footer bar does not exist | T-637 |

### 2.21 Right panel — Asset Browser (`PANEL-R-001…021`)

TBD's `DockRight` (`eden_chrome.rs:2960-3340`): 4 tabs — Factions (LIVE), Vehicles (LIVE, T-215),
Zones (LIVE, T-582), **Markers (stub, `:3337`, pinned by a test at `:4499-4502`)** — plus an
`EdenChip::Objects` mode that re-skins tab 0, and a per-tab search box.

| eden_id | tbd_id | parity | build_class | gap_notes | ticket |
|---|---|---|---|---|---|
| PANEL-R-001 | tab strip (`eden_chrome.rs:3041-3046`) | partial | a | `Assets` tab. Eden's window-tab pair sits above the mode strip; TBD collapses both levels into one 4-tab row, which is why the strip overflows (T-632, absorbed into T-637) | T-637 |
| PANEL-R-002 | strip History button — **`disabled=true`** (`eden_chrome.rs:1131-1139`, title `"Version history (soon)"`) | missing | a | `History` tab — recently placed / used assets. TBD's History button is in the top strip, disabled, and is about **version** history, not asset history. Two different features sharing a word | — |
| PANEL-R-003 | — | missing | a | `»` collapse chevron, mirror of the left panel's. Same 24 × 24 stub behaviour | T-638 |
| PANEL-R-004 | Factions tab + Objects chip | partial | a | **`F1` Objects** — one tree over units + vehicles + props. Corrected by README: batch05 mis-read F1/F2; **batch06's tooltips are authoritative** | T-646 |
| PANEL-R-005 | — | missing | a | **`F2` Compositions** — pre-made group templates. batch06 `165959` confirms via tooltip **and** via the `Place vehicles with crew` checkbox being absent here | T-650 |
| PANEL-R-006 | — | missing | a | **`F3` Triggers** — a flat 4-row list: `Trigger`, `Trigger (Ø 100 m)`, `Trigger (Ø 500 m)`, `Trigger (10x10x10 m)`. Note it is *presets*, not a builder | T-079 |
| PANEL-R-007 | — | missing | c | **`F4` Waypoints** — 29 types in `Advanced` + `Default` groups, every leaf in ALL CAPS (`CYCLE`, `SEEK AND DESTROY`, `TRANSPORT UNLOAD`…). The full list is the only enumeration of Eden waypoint types in the corpus | T-677 (wb) |
| PANEL-R-008 | — | missing | a | **`F5` Systems** — 23 module categories (`Ambient` … `Zeus`). Unsized; cf. `MENU-EDIT-029` | — |
| PANEL-R-009 | Markers tab stub (`:3046`, `:3337`) | missing | a | **`F6` Markers** | T-069 |
| PANEL-R-010 | `EDEN_SIDE_CHIPS` = 4 (`:2871`) | partial | a | **Side/faction chip row.** Eden's is **per-tab and variable-count** — 5 chips on Objects, 6 on Compositions (adds a Logic side), **0** on Triggers and Waypoints, 2 on Systems. TBD has one fixed 4-chip row for every tab | T-646 |
| PANEL-R-011 | — | missing | a | **Fixed-height optional row**: the chip strip keeps its 36 px even when a tab has zero chips, so the search box and tree never jump between tabs. Layout stability over density — a rule, not a control | T-668 |
| PANEL-R-012 | — | missing | a | Search **scope `▼` dropdown**, leading the search row. **Present on the Assets panel only** — the Entities panel has no equivalent. Never opened, so its options are unknown | T-084 |
| PANEL-R-013 | per-tab search box (`eden_chrome.rs:3115-3127`, `:3254-3256`) | match | a | Asset search field. TBD keeps per-tab query state and force-expands matches | — (shipped T-055) |
| PANEL-R-014 | — | missing | a | Search **magnifier / run** button | T-646 (extend) |
| PANEL-R-015 | — | missing | a | **Collapse-all** `−` | T-646 (extend) |
| PANEL-R-016 | — | missing | a | **Expand-all** stacked-`+` | T-646 (extend) |
| PANEL-R-017 | tree (`asset_catalog.rs:146` `build_catalog_tree`) | partial | a | 3-level faction → category → asset tree, 16 px indent step, `►`/`▼` expanders. TBD's is equivalent; Eden adds a **side symbol per leaf** (APP-6: rect+X infantry, rect+slash recon) that TBD does not render | T-646 |
| PANEL-R-018 | — | missing | a | **Row badges** in a right-hand gutter — DLC / content-provenance markers on *some* rows. **One gutter, two jobs**: badges shift left when the scrollbar appears. TBD renders no provenance on catalogue rows | T-658 (extend) |
| PANEL-R-019 | — | missing | a | **`Place vehicles with crew`** checkbox, pinned at the panel foot, **Objects tab only** — the cleanest confirmation that F1 is Objects. TBD's `\bcrew\b` count in the SPA is **0** | T-646 |
| PANEL-R-020 | — | na | d | **`PLAY SCENARIO` / `IN SINGLEPLAYER ▶`** — a full-width, pure-black block at the panel foot, the only pure-black surface and the highest-contrast control in the whole editor. In-client preview; §5. **The pattern is the copyable part**: T-636 is explicitly looking for what fills the bottom-right primary-action slot | T-636 |
| PANEL-R-021 | — | missing | a | **Both panels share one control vocabulary** — identical search row, identical `«`/`»`, identical 240 px width, identical tab styling. TBD's two docks are asymmetric today (left has a heading, right has tabs; only the right has search) | T-637 |

### 2.22 Status bar (`SBAR-001…009`)

TBD has no status bar. Its readouts are in the **bottom toolbelt** — which is exactly T-636's
complaint: *"stop the toolbelt conflating tools with telemetry."*

| eden_id | tbd_id | parity | build_class | gap_notes | ticket |
|---|---|---|---|---|---|
| SBAR-001 | `STATUS-X-001`; toolbelt `X` (`eden_chrome.rs:3722-3726`) | match | a | X readout in metres, its own inset field with an `X→` glyph | — (shipped T-159.21) |
| SBAR-002 | `STATUS-Y-001`; toolbelt `Y` (`:3727-3731`) | match | a | Y readout, `Y↑` glyph | — (shipped) |
| SBAR-003 | `STATUS-Z-001`; toolbelt `Z` (`:3732-3739`) | partial | a | Z readout — and **the glyph carries a terrain squiggle naming the reference frame** (ASL here, vs the entity dialog's ATL). TBD shows a bare `Z`; cf. `ENT-UX-005` | T-670 (extend) |
| SBAR-004 | `STATUS-ZOOM-001` — **absent** | missing | a | Camera-to-cursor **distance** readout (eye glyph). batch08 records the same field printing `m/pix` in 2D map mode — i.e. **it becomes a scale readout**, which is exactly T-670 | T-670 |
| SBAR-005 | `STATUS-VER-001` — **absent** | missing | a | Build/version string (`2.20.153973`), boxed, bottom-right of the bar. TBD shows no build id in the editor | T-636 (extend) |
| SBAR-006 | `STATUS-SRV-001` / `STATUS-MOD-001` — **absent** | na | d | MP / SP **play-target** toggle pair (network glyph dim, monitor glyph lit). Chooses what the PLAY button launches; §5 | — |
| SBAR-007 | — | missing | a | `OBJ` / `SEL` / `SZ` have **no Eden counterpart** — this is a `tbd_only` addition (T-058), recorded here so the surface comparison is symmetric | — (`tbd_only`) |
| SBAR-008 | `on:pointerleave → None` (`mission_editor.rs:1848-1852`) | match | a | **Readouts freeze while a menu is open** — byte-identical across all nine batch01 frames. TBD blanks to `—` when the pointer leaves the map, which is the better behaviour | — |
| SBAR-009 | — | na | d | **A defect worth *not* copying**: when the pick ray misses terrain, Eden prints wildly out-of-bounds values (`X -4515.67 m`, `Z -185.97 m`) instead of blanking or clamping. TBD already handles this correctly. Recorded so nobody "restores parity" by breaking it | — |

### 2.23 Viewport overlays (`VPORT-001…006`)

| eden_id | tbd_id | parity | build_class | gap_notes | ticket |
|---|---|---|---|---|---|
| VPORT-001 | — | missing | a | **World-axis triad**, bottom-left, arrows rotating live with the camera. In 2D map mode it degrades to **two axes** (red `X` right, green `Y` up) and is the *only* orientation aid Eden gives. TBD has no orientation indicator | T-667 |
| VPORT-002 | — | na | d | **Three-axis translate gizmo** on the selection, each arm labelled in its axis colour. Requires a widget mode; cf. `MENU-EDIT-007` | — |
| VPORT-003 | selection tint (T-151.6 / T-175) | partial | a | **Cyan wireframe bounding box** around the selection | T-668 |
| VPORT-004 | — | missing | a | **Floating editor icon above the entity on a leader line** — the map-symbol representation of a 3D object. TBD's 2D icons *are* the symbols, so the leader line has no job; the **selected-state** half does | T-668 |
| VPORT-005 | `Town labels` layer, `world_assets/labels.rs` | match | a | **In-world location pins** with names (`Neri`, `Panochori`) | — (shipped T-173) |
| VPORT-006 | `title=` attributes across the chrome | partial | a | **Tooltips are cursor-anchored, not control-anchored** — ~25 px right, ~6 px below the pointer, sized to the text, pure black, no border, no tail, and shown even on disabled controls. TBD uses native browser `title` tooltips, which are control-anchored and delayed | T-668 |

### 2.24 2D map view and panel collapse (`MAP2D-001…010`) — batch07/08

Checked against the existing tickets before minting, per the brief. **Six of the ten are already
ticketed and are listed only so the corpus is fully accounted for**; four carry new information.

| eden_id | tbd_id | parity | build_class | gap_notes | ticket |
|---|---|---|---|---|---|
| MAP2D-001 | contour layer + `Contours` toggle (`world_layer_prefs.rs:61-76`) | partial | a | **Zoom-adaptive contour interval** — a doubling ladder pinned to a constant 14–19 px screen band (5 m → 10 m → 20 m across three measured `m/pix` scales). Already the driving finding for T-639 | T-639 |
| MAP2D-002 | contour render | partial | a | **Contours are a fixed-alpha tint over hillshade** (`r − b = +28`), not a saturated fixed colour; the **innermost closed ring of each peak** is drawn darker (`r − b ≈ 51`) — a per-peak rule, not every-Nth | T-640 |
| MAP2D-003 | `Height labels` layer + `dem/peaks.rs` | partial | a | **Spot heights, screen-space culled at ~1 per 150 × 150 px**, always horizontal, never rotated to the line. README refutes the operator's recollection that Eden labels contours — **it does not** | T-641 |
| MAP2D-004 | — | missing | a | **Northing grid labels down both viewport edges**, with a tick dash; **no easting labels anywhere** (top, bottom and menu bar all checked). Asymmetric on purpose | T-667 |
| MAP2D-005 | — | missing | a | **No scale bar, no north rose, no minimap, no zoom slider, no legend** in Eden either — a saturated-colour sweep of the whole viewport found only the axis gizmo. **Eden is not the model for T-667's scale bar**; TBD would be adding something Eden lacks, which is a defensible product call but should be made knowingly | T-667 |
| MAP2D-006 | dock chevrons — absent | missing | a | **Panel collapse affordance**: a 24 × 24 chevron in each dock's outer top corner; the glyph bbox is byte-identical expanded vs collapsed and **only the direction flips**. No toolbar toggle exists — the edge tabs and `E`/`R`/`Backspace` are the only paths | T-638 |
| MAP2D-007 | — | missing | a | **Collapsed = a 24 px stub overlaying the map**, not a rail, not a gutter, not a splitter; the viewport runs full-bleed underneath and genuinely reflows | T-638 |
| MAP2D-008 | — | UNKNOWN | a | **No hover or pressed state for the chevron is captured anywhere in 75 screenshots**, nor a single-panel-collapsed state, nor the full `Backspace` hidden-interface state (README §Caveats). T-638 will have to invent these; flagged so they are not "restored" from a source that does not exist | T-638 |
| MAP2D-009 | — | na | d | **No 3D topographic overlay exists** — `View ▸ Toggle Map Textures Ctrl+T` is in the menu but every 3D frame in the corpus is plain satellite terrain. Recorded because a "3D contour overlay" is the kind of feature that gets inferred from a menu label | — |
| MAP2D-010 | waypoint polyline / marker ring render | missing | a | In 2D map mode Eden draws a **closed waypoint polyline** back to the unit and a **segmented marker ring** with a selection rectangle. Both are render targets for `T-069` (markers) and `T-677` (waypoints); listed here because the *2D symbology* is a separate job from the entity | T-069 |

---

## 3. Counts

**374 rows.** Every count below is derived from the table body above, not carried forward. No
duplicate ids. Every `na` row has `build_class: d` and every `d` row has `parity: na` (148 = 148) —
the two columns are consistent by construction.

### 3.1 By parity

| parity | n | share |
|---|---:|---:|
| **na** | 148 | 39.6 % |
| **missing** | 132 | 35.3 % |
| **partial** | 59 | 15.8 % |
| **match** | 32 | 8.6 % |
| **UNKNOWN** | 3 | 0.8 % |
| `deferred` | 0 | — |
| `tbd_only` | 0 (one noted in-row: `SBAR-007`) | — |

### 3.2 By build class

| class | n | meaning |
|---|---:|---|
| **a** — SPA-buildable today | 207 | factory work, `executor: claude-code` |
| **b** — schema-blocked | 2 | `MENU-EDIT-020` (ASL reference frame), `ENT-SEC-007` (slot identity) |
| **c** — mod-blocked | 17 | `executor: workbench` |
| **d** — na | 148 | closed; §5 |

Cross-tab: `missing`/a **116** · `partial`/a **56** · `match`/a **32** · `missing`/c **15** ·
`partial`/c **2** · `missing`/b **1** · `partial`/b **1** · `UNKNOWN`/a **3** · `na`/d **148**.

### 3.3 By surface

| Surface | rows | match | partial | missing | na | UNKNOWN | a | b | c | d |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `MENU-BAR-` | 8 | 2 | 3 | 1 | 2 | 0 | 6 | 0 | 0 | 2 |
| `MENU-SCEN-` | 16 | 4 | 2 | 1 | 9 | 0 | 7 | 0 | 0 | 9 |
| `MENU-EDIT-` | 32 | 4 | 8 | 17 | 3 | 0 | 26 | 1 | 2 | 3 |
| `MENU-VIEW-` | 18 | 4 | 2 | 8 | 4 | 0 | 14 | 0 | 0 | 4 |
| `MENU-ATTR-` | 4 | 1 | 1 | 0 | 2 | 0 | 2 | 0 | 0 | 2 |
| `MENU-TOOLS-` | 6 | 0 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 6 |
| `MENU-SET-` | 5 | 0 | 1 | 0 | 4 | 0 | 1 | 0 | 0 | 4 |
| `MENU-PLAY-` | 5 | 0 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 5 |
| `MENU-HELP-` | 7 | 0 | 0 | 1 | 6 | 0 | 1 | 0 | 0 | 6 |
| `MENU-UX-` | 6 | 0 | 2 | 2 | 2 | 0 | 4 | 0 | 0 | 2 |
| `TOOLBAR-` | 26 | 6 | 2 | 10 | 7 | 1 | 19 | 0 | 0 | 7 |
| `CTX-` | 39 | 4 | 4 | 22 | 9 | 0 | 28 | 0 | 2 | 9 |
| `CTX-UX-` | 4 | 0 | 0 | 4 | 0 | 0 | 4 | 0 | 0 | 0 |
| `DLG-SHELL-` | 14 | 0 | 5 | 9 | 0 | 0 | 14 | 0 | 0 | 0 |
| `DLG-GEN-` | 27 | 1 | 1 | 8 | 17 | 0 | 4 | 0 | 6 | 17 |
| `DLG-ENV-` | 27 | 1 | 2 | 1 | 23 | 0 | 1 | 0 | 3 | 23 |
| `DLG-MP-` | 23 | 0 | 3 | 2 | 18 | 0 | 5 | 0 | 0 | 18 |
| `DLG-PERF-` | 16 | 0 | 0 | 0 | 16 | 0 | 0 | 0 | 0 | 16 |
| `DLG-PREF-` | 13 | 0 | 2 | 5 | 6 | 0 | 7 | 0 | 0 | 6 |
| `ENT-SEC-` | 9 | 0 | 2 | 4 | 3 | 0 | 2 | 1 | 3 | 3 |
| `ENT-UX-` | 5 | 0 | 2 | 2 | 1 | 0 | 4 | 0 | 0 | 1 |
| `PANEL-L-` | 18 | 0 | 7 | 10 | 0 | 1 | 18 | 0 | 0 | 0 |
| `PANEL-R-` | 21 | 1 | 4 | 15 | 1 | 0 | 19 | 0 | 1 | 1 |
| `SBAR-` | 9 | 3 | 1 | 3 | 2 | 0 | 7 | 0 | 0 | 2 |
| `VPORT-` | 6 | 1 | 2 | 2 | 1 | 0 | 5 | 0 | 0 | 1 |
| `MAP2D-` | 10 | 0 | 3 | 5 | 1 | 1 | 9 | 0 | 0 | 1 |
| **Total** | **374** | **32** | **59** | **132** | **148** | **3** | **207** | **2** | **17** | **148** |

The shape of that table is the finding. The **dialogs** carry the `na` mass — `DLG-*` is 120 rows and
80 of them are `na`; Eden's scenario modals are three-quarters A3-engine concerns. The **chrome**
carries the buildable mass — `CTX-*` + `DLG-SHELL-*` + `PANEL-L-*` + `PANEL-R-*` + `MENU-EDIT-*` is
128 rows, **109 of them class a**.

### 3.4 Ownership — where the 374 rows land

| Disposition | rows |
|---|---:|
| Maps to an **existing** ticket (as scoped, or `(extend)`) | **176** |
| Already **shipped** — cited with the ticket that shipped it | 18 |
| Maps to a **proposed new slice** (`NEW-F*` / `NEW-W1`) | **24** |
| **No owner proposed**, and the row says why | 156 |
| | **374** |

Of the 156 no-owner rows: **133 are `na`**, 9 are `match` needing nothing, 2 are `UNKNOWN`, and
**12 are open rows deliberately left unowned**, each naming its reason in-row — `MENU-EDIT-020`
(no consumer for an ASL frame), `MENU-EDIT-029` and `PANEL-R-008` (Systems is *unsized*, per
`registry_write_log.md` §7.2), `MENU-VIEW-001` (browser owns `Ctrl+R`; low value), `DLG-SHELL-010`,
`DLG-SHELL-013`, `DLG-GEN-011`, `DLG-PREF-010`, `PANEL-L-010` (TBD's substitute is defensible),
`DLG-GEN-026` (no faction-relations model), `DLG-MP-017` (no sharing-scope concept), `PANEL-R-002`
(asset history — genuinely low value).

### 3.5 The headline — how much is genuinely new factory work

| | rows |
|---|---:|
| Open (`missing` or `partial`) **and** class `a` | **172** |
| …mapping onto **existing** tickets | **143** |
| …needing a **new factory slice** | **19** |
| …deliberately unowned | 10 |
| Open, class `c`, needing a new workbench slice | **5** (all `NEW-W1`) |

**19 rows of genuinely new factory work, in 7 proposed slices.** That is the number that matters:
the screenshot corpus does **not** blow up the program. It **thickens 34 already-filed tickets** and
adds seven small ones.

The five existing tickets the corpus thickens most:

| Ticket | rows | what the corpus adds to its scope |
|---|---:|---|
| **T-648** transform / snap / widget | 24 | Eden's *entire* grid family — three grids, each a split button with its own step caret, plus increase/decrease, plus grid-step-from-object-bbox. The draft scoped "snap grid" as one thing; it is six controls across two menus and a toolbar group |
| **T-668** state vocabulary | 22 | Now has a concrete source: hover ≠ active ≠ disabled ≠ checked; disabled-still-shows-tooltip; hatch-vs-dim scrims; the `…` convention; the unread-badge pattern |
| **T-682** environment readers (wb) | 15 | Every refused weather channel enumerated with its Eden control shape, so the reader ticket knows exactly what it would have to feed |
| **T-646** asset browser | 14 | Per-tab **variable-count** chip rows, collapse-all/expand-all, `Ctrl+F` focus, reveal-in-browser, thumbnail combos, the crew checkbox |
| **T-664** context menu | 12 | The verbatim two-state item lists and the omission-vs-greying policy — the ticket previously said only "render a menu" |

---

## 4. Proposed ticket groupings

### 4.1 Existing tickets — what the corpus adds, and what it corrects

Nothing below proposes a new ticket. Each row is a **scope note** for a ticket that already exists.

| Ticket | Status | Rows | What this sweep adds |
|---|---|---:|---|
| **T-648** transform | queued | 24 | The grid family is six controls, not one; the split-button caret is a **separate hit target** (proved by pixel diff, batch05 `164132`). Widget modes 1–5 are direct keys, all free. `MENU-EDIT-005` dissolves the `Space` collision |
| **T-668** state vocabulary | queued | 22 | Six named states with measured treatments; the **disabled-shows-tooltip** rule; the two-scrim distinction. This ticket was the thinnest-specified in the program and is now the best-sourced |
| **T-646** asset browser | queued | 14 | Chip row is **per-tab and variable-count** (6/5/0/0/2) with a fixed 36 px height so the tree never jumps; adds collapse-all, expand-all, magnifier, `Ctrl+F`, reveal-in-browser, thumbnail combos |
| **T-664** context menu | queued | 12 | Verbatim 6-row and 14-row item lists; submenu flip and screen-fit geometry; the **clipboard-verbs-grey / query-verbs-omit** policy |
| **T-638** dock collapse | queued | 10 | Exact chevron geometry; collapsed = a 24 px **overlay stub**, not a rail; `E` / `R` / `Backspace` bindings; and the honest warning that **no hover/pressed state exists anywhere in the corpus** |
| **T-666** outliner authoring | queued | 9 | Adds the **footer button row** (Delete / New Layer / Move to Root) with verbatim tooltips, the search box, collapse-all/expand-all, and the ten always-visible category roots. **Corroborated**: four of five layer mutators have zero call sites; the fifth (`add_editor_layer`) has one non-UI caller at `editor_ops.rs:1136` |
| **T-682** environment readers | queued (wb) | 15 | The full channel list with Eden's control shapes: overcast/rain/fog/lightning/waves/wind each have Start + Forecast, fog adds Decay + Base, wind adds Gusts + Direction |
| **T-079** triggers | queued | 7 | F3 is a **4-preset flat list**, not a builder; `Set Trigger Owner` is a context-menu verb; Area/Area-Scaling widgets are the geometry-edit half |
| **T-634** top strip | queued | 6 | **`View` is a dead menu** (`action: None`, `eden_chrome.rs:145-147`); Mission + Environment are two doors to one room; menus need a shortcut column |
| **T-649** select-all | queued | 6 | Eden's `Ctrl+A` is **view-scoped**, and the context menu adds four `Select Matching Classes/Types (Selected\|View)` verbs on the same seam |
| **T-082** attributes modal | queued | 5 | The 9-section taxonomy; a **class tree with its own search** for re-typing a placed entity; icon-radio strips; the axis-chip numeric field |
| **T-650** compositions | queued | 4 | `Save Custom Composition…` is a **context-menu** entry point; F2's tree is the group-template catalogue; `Automatic Composition Layering` is the preference that pairs with it |
| **T-667** map furniture | queued | 4 | **Eden has no scale bar** — it has northing-only edge labels and a 2-axis gizmo. Adding a scale bar is a defensible divergence, not parity |
| **T-671** presentation | queued | 4 | Three separate Picture+Text blocks in Eden (overview / locked / loading); TBD needs only the first, and the MP `Summary` is the same field |
| **T-665** layer flags | queued | 3 | Verbatim tooltips settle it: the padlock button is **`Toggle Layer Transformation`**, not "lock" — three batches inferred it wrong |
| **T-633** native controls | queued | 3 | Eden's slider is `◀ track ▶ [editable value box with unit]`; the time-of-day slider is skinned to its domain (gradient track, sun thumb) |
| **T-637** dock density | queued | 3 | Both Eden panels are 240 px and share one control vocabulary; TBD's two docks are asymmetric |
| **T-670** scale readout | queued | 3 | The eye-glyph field **becomes** the `m/pix` readout in 2D map mode — the same slot, two modes. Also: name the Z reference frame |
| **T-636** status bar | queued | 2 | `PLAY SCENARIO` is the model for the bottom-right primary-action slot the ticket is looking for; add the build-id field |
| **T-647** placement | queued | 2 | `Attributes…` and `Group to` are context-menu entry points onto the same seam |
| **T-669** clipboard | queued | 2 | Verbatim confirmation of `Ctrl+X` / `Ctrl+Shift+V` and their menu labels |
| **T-672** connection graph | queued | 2 | `Connect ▸ Sync to` / `Group to` / `Set Trigger Owner` is the whole submenu |
| **T-681** entity states (wb) | queued | 2 | Two whole Eden sections (`States`, `Special States`) = 11 fields |
| **T-677** waypoints (wb) | queued | 3 | The **complete 29-type waypoint catalogue** — the only enumeration of it in the corpus |
| T-069 · T-639 · T-640 · T-641 · T-651 · T-658 · T-659 · T-674 · T-678 · T-084 | various | 1 each | Single-row confirmations; details in-table |

### 4.2 New slices — seven factory, one workbench

Smallest first. Ids are placeholders (`NEW-F*`); the registry's `next_id` is **683**.

| # | Proposed slice | Rows | Why it is one slice, and why it is not an existing ticket |
|---|---|---:|---|
| **NEW-F1** | **`Ctrl+S` saves a version** | 2 (`MENU-SCEN-003`, `TOOLBAR-003`) | One match arm in `mission_editor.rs:1019-1031` firing the existing `save_open` signal. The single highest-frequency Eden shortcut TBD has a button for and no key. Does not belong in T-669 (clipboard) or T-648 (transform) — different file region, different owner. **Hours.** |
| **NEW-F3** | **Editor help surface** — a Help menu entry + a Controls Hint overlay | 3 (`MENU-BAR-008`, `MENU-VIEW-017`, `MENU-HELP-001`) | TBD's 10 keyboard bindings are documented **nowhere in the UI**. A `?` overlay listing them plus one docs link. Pairs naturally with T-668's vocabulary work but is content, not styling |
| **NEW-F6** | **Mission shape is editable after creation** — game mode, min/max players | 3 (`DLG-MP-001…003`) | The one genuine gap in `Edit: Multiplayer`. All three are create-dialog-only today (`create_mission_dialog.rs:156-160`, `:197-213`), so a mission's shape is frozen at birth. Needs a `PATCH /missions/:id` caller, not new schema. **Distinct from T-671** (presentation) — that is text and images, this is lobby shape |
| **NEW-F4** | **Merge another mission into this one** | 1 (`MENU-SCEN-011`) | `Ctrl+M`. High value for composing an ORBAT from a template mission, and the paste primitives already exist (`paste_at_cursor`, `PASTE_KNOWN_SLOT_KEYS` `store.rs:2571-2582`). Sized as its own slice because it needs an import path and an id-remap, not just a key |
| **NEW-F5** | **Favorites — a starred-asset list** | 1 (`MENU-EDIT-031`) | Eden's F7. Pure SPA + `localStorage` over the live catalogue. Deliberately **not** folded into T-646: that ticket already carries 14 rows and owns `eden_dock_right.rs`; a favourites store is additive and can land in any wave |
| **NEW-F7** | **Locations list — browse and fly to a named place** | 1 (`PANEL-L-003`) | Eden's `Locations` tab. **TBD already has the data** (`world_assets/labels.rs`, the `Town labels` layer) and no way to browse or jump to it. On a 12.8 km terrain that is a real navigation gap |
| **NEW-F2** | **Editor preferences, separated from mission settings** | 8 (`MENU-BAR-006`, `MENU-SET-001`, `DLG-SHELL-003`, `DLG-PREF-001`, `-003`, `-004`, `-005`, `-009`) | The largest new slice, and the one with a **structural** argument rather than a feature list: TBD's editor preferences (basemap view, 12 world layers, autosave interval, camera speed) are `localStorage` state filed under *Mission* Settings, where a reader reasonably expects document state. Eden draws the line with a menu separator. Also the natural home for `DLG-SHELL-003` — TBD has **no cancel path on any settings dialog** |
| **NEW-W1** | **Player gadget availability flags** (`executor: workbench`) | 5 (`DLG-GEN-014…018`) | Show Map / Compass / Watch / GPS / HUD. Mod-blocked: the mission document carries no such block and `flatten` emits none, so this is the same shape as T-681/T-682 — a schema slot **plus** a reader, filed together. **Not** folded into T-681 (entity states) because the gate is different: T-681 needs per-entity runtime state, this needs per-mission gadget gating |

### 4.3 Wave-packing note — checked against `wave_plan.tsv`, not guessed

The 43-row editor-UI block occupies plan labels **100–118** (`docs/platform/wave_plan.tsv:438-480`;
program wave *N* = plan label *N* + 100). Column 4 is the `owns` list. Free waves below are the ones
where **no already-planned ticket claims the same file**.

Claim counts, computed from that block:

| File | waves claiming it | free |
|---|---:|---:|
| `mission_editor.rs` | **18 of 19** (101–118) | 1 (100 only) |
| `editor_ops.rs` | **16 of 19** | 3 (100, 115, 118) |
| `eden_dock_right.rs` | 10 | 9 |
| `eden_top_strip.rs` · `eden_toolbelt.rs` | 7 each | 12 each |
| `select_tool.rs` | 6 | 13 |
| `eden_dock_left.rs` | 5 | 14 |
| `eden_settings.rs` · `context_menu.rs` | 3 each | 16 each |
| `asset_catalog.rs` · `ui.rs` | 2 each | 17 each |
| `create_mission_dialog.rs` | 1 (T-671 @ 117) | 18 |
| `world_layer_prefs.rs` · `mission_commands.rs` | **0** | 19 |

| Slice | Proposed `owns` | Free plan labels |
|---|---|---|
| **NEW-F1** `Ctrl+S` | `mission_editor.rs` | **100 only — i.e. none usable.** Recommendation: **do not file it.** Fold it into **T-669** (plan 113, owns `mission_editor.rs` alone, and is already the keyboard-arm ticket). One extra match arm in the same block |
| **NEW-F2** editor preferences | `eden_settings.rs` · `world_layer_prefs.rs` | 101–109, 111–116, 118 (16 slots) |
| **NEW-F3** help surface | new `eden_help.rs` · `eden_top_strip.rs` | 101–106, 108, 109, 112, 113, 116, 118 (12) |
| **NEW-F4** merge mission | `mission_commands.rs` · `editor_ops.rs` | **115, 118 only.** Scoping it to `mission_commands.rs` + `store.rs` instead opens every wave — worth doing |
| **NEW-F5** favorites | `eden_dock_right.rs` | 101, 102, 104, 106, 111–115 (9) |
| **NEW-F6** mission shape | `create_mission_dialog.rs` · `eden_settings.rs` | every label except 100, 110, 117 (16) |
| **NEW-F7** locations list | `eden_dock_left.rs` | 101–103, 105, 106, 108, 109, 111–117 (14) |
| **NEW-W1** gadget flags | `apps/mod/tbd-framework/…` + `mission.schema.json` | `executor: workbench` — **takes no wave row**, exactly like T-673…T-682 |

Three consequences worth stating:

1. **Six of the seven pack into existing waves without adding one.** Only NEW-F1 has no usable
   slot, and its cheapest resolution is to not exist as a ticket — fold it into T-669.
2. **All seven assume T-661 has landed.** Before the ten-module split every one of them owns the
   same ~5,000-line `eden_chrome.rs` and none can be packed against anything.
3. `mission_editor.rs` at **18/19** and `editor_ops.rs` at **16/19** are the measured version of
   what `owns_correction_chrome3.md` already recorded: the **second** split (`editor_ops.rs`, then
   `mission_editor.rs`) is what would buy the program headroom. The operator has declined it on risk
   grounds. This sweep adds two more tickets' worth of pressure, **not a new argument** — the trade
   is unchanged and is recorded so it stays visible.

---

## 5. What was scored `na`, and why — the scope boundary

**148 rows, 39.6 % of the corpus.** This is the section that matters most, because a wrong `na` is
invisible: it silently shrinks the program and nobody ever audits it. So every `na` is bucketed
here, the buckets are **mutually exclusive and exhaustive** (verified — no row is in two buckets and
no `na` row is unbucketed), and each bucket states the *precondition that does not exist* rather
than a preference.

| # | Bucket | rows | The precondition TBD does not have |
|---|---|---:|---|
| **F** | **A3 engine concept with no Enfusion analogue** | **48** | Garbage collection, dynamic simulation, DLC gating, campaign keys, advanced flight model, datalink sensors, weather *forecasts*, sea state, lightning, scenario phases, AI-fill, UAV feed |
| **E** | **Refused by design, with a test pinning the refusal** | **32** | `eden_chrome.rs:4623-4630` and `:4686-4694`. Fog, wind, wind direction, view distance, thermals, night vision, respawn, spectator policy, tickets, revive |
| **B** | **A3 scripting / config runtime** | **17** | No scripting layer at all: SQF init boxes, presence conditions, Debug Console, Functions/Config/Animations Viewers, clipboard class dumps, `Export to SQF` |
| **D** | **3D-viewport precondition** | **15** | TBD is a top-down 2D editor: vertical drag mode, widget coordinate space, terrain/sea-normal orientation, flashlight, camera-follows-terrain, 3D map toggle |
| **C** | **OS filesystem, Steam, packaging, file format** | **11** | Publish to Workshop, Open Scenario/Log Folder, Export to SP/Terrain Builder, Save As, binarise, Exit |
| **A** | **In-client preview** | **10** | A browser tab cannot host an Arma client. TBD's preview is a real server pulling `GET /missions/:id/compiled` (T-092.2) |
| **H** | **Bohemia web properties / in-game tutorials** | **7** | Community wiki, forums, feedback tracker, dev hub, scripting reference, tutorials |
| **G** | **Game-client settings** | **4** | Video / Audio / Game Options / Controls belong to the Arma client, not a mission editor |
| **I** | **Recorded as a design rule, not a feature** | **3** | The conditional check gutter and the hatch-vs-dim scrim are *styling inputs to T-668*; `SBAR-009` is an Eden **defect** recorded so nobody "restores parity" by breaking TBD's better behaviour |
| **J** | **Security / architecture** | **1** | `Author` is server-assigned from the Discord session (`dto.rs:828`); an author-editable field is a regression, not a feature |
| | | **148** | |

### 5.1 The three `na` buckets that are *not* permanently closed

Being honest about this is the point of the section.

- **Bucket E is `na` at the *factory*, not `na` forever.** Fifteen of its 32 rows carry
  `T-682 (wb)` in the ticket column — the queued workbench row that would build the environment
  readers. They are class **d** rather than class **c** because `gap_analysis.md:111` defines class
  d to include *"something TBD refused by design with a test pinning the refusal"*, and these are
  exactly that. **If a reader ever lands, re-score them; do not build a control first.**
  `eden_chrome.rs:4619-4621`, verbatim: *"the schema HAS a slot for it … A schema field is not a
  reader."*
- **Bucket A's *pattern* is wanted even though its *feature* is not.** `PANEL-R-020`
  (`PLAY SCENARIO`) is scored `na` — TBD will never launch an Arma client from a browser — but it
  is mapped to **T-636**, because that ticket is explicitly asking what fills the bottom-right
  primary-action slot, and Eden's answer (full-width, pure black, the only pure-black surface in the
  chrome, always visible) is the best available reference.
- **Bucket D shrinks if the editor ever gains a 3D view.** Nothing suggests it will, and every row
  in it names its 2D substitute where one exists (`MENU-EDIT-017` → numeric Z with terrain-follow;
  `CTX-033`/`-034` → TBD authors one rotation about Z).

### 5.2 Where `na` was *not* used, and could have been

Three surfaces were tempting to write off wholesale and were triaged instead:

1. **`Edit: Performance`** — 16 rows, all `na`, and the honest reason is uniform across all 16.
   Enumerated anyway, because *"we skipped the Performance dialog"* and *"the Performance dialog is
   out of scope"* are different claims and only the second is defensible.
2. **`Edit: Multiplayer`** — 18 of 23 `na`, but the other **5** are real, and `DLG-MP-001…003` are
   the finding that produced `NEW-F6`: a TBD mission's **game mode and player count are frozen at
   creation**. A blanket `na` on "the MP dialog" would have hidden that.
3. **The toolbar** — 7 of 26 `na`. `TOOLBAR-022` was the one place where marking `na` would have
   been *convenient* (batch06 reads it as `Toggle Map`, which is cleanly `na`) and three other
   batches read the same glyph as `Toggle Foliage`, which is a `match`. It is scored **UNKNOWN**.

### 5.3 The three UNKNOWN rows

`UNKNOWN` is used only where the corpus contradicts itself or is silent, never as a soft `na`:

| id | What is unknown | Why it was not guessed |
|---|---|---|
| `TOOLBAR-022` | Whether the button at x ≈ 490–508 is `Toggle Foliage` or `Toggle Map` | batch01/02/03 read the glyph as vertical blades → Foliage; batch06 reads a folded map → Toggle Map, "confirmed" only circumstantially. Never hovered, so no tooltip exists. Parity would be `match` under one reading and `na` under the other |
| `PANEL-L-017` | Why the selected unit's outliner label renders **red** | Persists across all eight batches and the README explicitly logs it as never explained. `INFERRED:` the player unit — but a TBD equivalent's parity is undefined until the meaning is |
| `MAP2D-008` | The chevron's hover and pressed states, the single-panel-collapsed state, the full `Backspace` hidden state | **Not captured in any of the 75 screenshots.** T-638 will have to invent them; flagged so they are not later "restored" from a source that does not exist |

---

## 6. Corrections this sweep contributes

Five claims that were true elsewhere, or inferred once and repeated, and would otherwise have been
inherited:

| # | Correction | Evidence |
|---|---|---|
| 1 | **batch02's toolbar inference at x 280–340 is wrong.** It reads the three buttons as waypoint-snap / surface-snap / vertical-mode. batch05 hovered all three: `Toggle Widget Coordinate Space`, `Toggle Vertical Mode`, `Toggle Surface Snapping`. **There is no waypoint-snapping toolbar button.** The README reconciles batch05↔06 on the F-tabs but not this | `batch02:71-74` vs `batch05:204-232` |
| 2 | **The left-panel padlock button is `Toggle Layer Transformation`, not "lock layer".** batches 01, 02 and 03 all inferred "lock"; batch06 hovered it. Same for the `⊘` button — it is **`Move to Root`**, not "disable layer". This changes what T-665 and T-666 are actually building | `batch06:165932`, `165938` vs `batch01:68`, `batch02:622`, `batch03:405` |
| 3 | **`interactions.md` declares zero `MENU-*` ids.** The two `TOOLBAR-*` ids are the entire menu/toolbar catalogue in the authority docs, against 101 menu entries and 26 toolbar controls in the corpus | grep in §1.4 |
| 4 | **"Four of five layer mutators have zero callers" is confirmed, with a refinement.** `rename_editor_layer`, `remove_editor_layer`, `reparent_editor_layer` and `move_slot_to_layer` have **zero call sites** (the last two appear only in doc comments). `add_editor_layer` has **one** — the default-layer seed at `editor_ops.rs:1136` — which is not a UI path | `registry_write_log.md` §6 (T-666), verified this session |
| 5 | **Eden has no scale bar, north rose, minimap, zoom slider or legend.** T-667 proposes a scale bar; the corpus shows Eden shipping *without* one, with northing-only edge labels and a two-axis gizmo instead. Adding one is a defensible divergence — it should be made knowingly, not as "parity" | `batch08:97-109`, `batch07:130-134` |

---

## 7. What this file does **not** do

- It does **not** edit `gap_analysis.md`, `registry.json` or `wave_plan.tsv`. Folding these 374 rows
  into `gap_analysis.md` is a separate, deliberate act — that file's own provenance section warns
  what happens when a sample is read as a census, and appending a second sample under the same
  heading would repeat the error. If it is folded in, the coverage table at its head must be
  rewritten, not extended.
- It does **not** re-litigate the keyboard map. [`interactions_sweep.md`](interactions_sweep.md) §4
  built it from this same corpus; every shortcut cited here defers to it.
- It does **not** re-derive the 93 `ATTR-FIELD-*` per-entity ids. `attributes_sweep.md` walked all
  93; §2.19 adds only the section taxonomy and widget vocabulary a field census cannot carry.
- It does **not** re-read the images. Every Eden claim traces to a batch-document line.

