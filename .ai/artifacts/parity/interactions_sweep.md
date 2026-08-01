# Interactions sweep — all 83 `eden/interactions.md` ids vs the live Leptos editor

**Written 2026-08-01.** Companion to [`README.md`](README.md) (§"What must be re-run" — this is the
`interactions_sweep.md` row). Peer of the not-yet-written `attributes_sweep.md` and
`owns_and_waves.md`.

**Sources.** `docs/specs/Mission_Creator_Architecture/eden/interactions.md` (the id set),
`eden/gap_analysis.md` (format + parity vocabulary, **not modified**),
[`../eden_screenshots/`](../eden_screenshots/) (8 batches + README, 75 real Eden frames),
and the live editor under `apps/website/frontend/src/`.

**Evidence rule.** Every `match` / `partial` carries a `file:line`. Every claim of absence carries
the command that produced the zero. Inference is prefixed `INFERRED:`. Anything not established is
`UNKNOWN`, not guessed.

---

## 1. Method — enumeration and count

### 1.1 The id count is 83

```bash
cd docs/specs/Mission_Creator_Architecture/eden
grep -oE '\b[A-Z][A-Z0-9]*(-[A-Z0-9]+)*-[0-9]{3}\b' interactions.md | sort -u | wc -l
# → 83
```

Those 83 are **not** all defined the same way, which is why a heading-only sweep undercounts:

```bash
grep -cE '^#{2,4} [A-Z][A-Z0-9-]*-[0-9]{3} ' interactions.md
# → 50      (ids with a `#### ID — Title` heading + a field table)
```

The remaining **33** are defined in summary tables or index prose, not headings:

| Where | Line(s) | Ids | Count |
|---|---|---|---|
| `## TOOLBAR — Index` (prose range, endpoints only) | 371 | `TOOLBAR-NEW-001`, `TOOLBAR-TUTORIAL-001` | 2 |
| `## SEL / LAYER / ATTR` summary table | 483–495 | `SEL-*` ×6, `LAYER-*` ×2, `ATTR-OPEN-001`, `ATTR-MULTI-CHK-001` | 10 |
| `## KEY shortcuts` table | 518–523 | `KEY-WP-001`, `KEY-WIDGET-001`, `KEY-GRID-001`, `KEY-HIDE-UI-001` | 4 |
| `## ACTION appendix` table | 531–542 | `ACTION-*` ×10 | 10 |
| `## STATUS bar` table | 550–558 | `STATUS-*` ×7 | 7 |

`ATTR-MULTI-001` appears in **both** the summary table (494) and as a `####` detail block (497), so
it is one id counted once. 50 + 33 = 83. ✅

### 1.2 The parity table is a sample, not a census

```bash
awk '/^\| eden_id \| tbd_id/{t=1;next} /^\|---/{next} /^$/{t=0} t&&/^\|/{n++} END{print n}' gap_analysis.md
# → 59      (data rows across the six parity tables)
```

Those 59 rows carry **58 distinct** `eden_id` values (two `tbd_only` rows use `—`). Only **41** of
the 58 are ids from `interactions.md`; the other 17 come from `feature_inventory.md` /
`attributes.md` (`TOOLBAR-INTEL-001`, `ATTR-FIELD-OBJ-*`, `SEL-MAP-003`, `XFORM-DEL-001`, `TOP-*`,
`MAP-*`, `ENV-*`, `DATA-*`, the two `—`).

```bash
comm -23 <(grep -oE '\b[A-Z][A-Z0-9]*(-[A-Z0-9]+)*-[0-9]{3}\b' interactions.md | sort -u) \
         <(awk -F'|' '/^\|/{gsub(/^ +| +$/,"",$2);print $2}' gap_analysis.md | grep -E '^[A-Za-z]' | sort -u) | wc -l
# → 42      (interaction ids with NO row anywhere in gap_analysis.md)
```

**41 covered / 42 uncovered.** Planning read 59 rows as coverage of 83 ids; it is coverage of 41.
This document produces all 83.

### 1.3 What "TOOLBAR" costs us

`interactions.md:371` writes the whole toolbar as a range — *"IDs: `TOOLBAR-NEW-001` …
`TOOLBAR-TUTORIAL-001` (New, Open, Save, Workshop, Undo, Redo, widgets, snap, grids, intel, map,
flashlight, vision, phase, tutorials)"*. Only the two endpoints are literal ids, so only two are
triaged below. The ~13 intermediate buttons are named in prose but **never assigned ids** — they are
`UNKNOWN` as ids and cannot be triaged. The screenshot corpus has the real toolbar inventory with
pixel bounds and verbatim tooltips (`batch01_context_menu.md:343-365`,
`batch05_asset_browser_2.md:275-295`); minting `TOOLBAR-*` ids from it is a separate docs task, not
this sweep's to invent.

### 1.4 Live-editor read set

`select_tool.rs` (729) · `mission_editor.rs` (3830) · `eden_chrome.rs` (5119) · `editor_ops.rs`
(2730) · `outliner.rs` (760) · `mission_history.rs` · `attributes.rs` · `asset_catalog.rs` ·
`missions.rs`, plus `crates/map-engine-core/src/doc/store.rs` for doc mutators.

The two facts the brief supplied were re-verified, not assumed:

* **Multi-select suppresses Attributes** — `editor_ops.rs:583-585`
  (`if ctx.selection.borrow().len() > 1 { return; }`), same guard in `open_arsenal`
  `editor_ops.rs:605-607`.
* **Dbl-click pick is slot-only** — `mission_editor.rs:1963` calls `select_tool::pick`
  (`select_tool.rs:128-130`), which delegates to `MissionDocCore::pick_slot` over the **slot SoA**.
  The click/drag path uses `pick_slot_or_vehicle` (`mission_editor.rs:1537`, `:1733`), so vehicles
  are selectable but **never** open Attributes; placed objects (`entities[]`) and zones are outside
  both.

---

## 2. The full table — all 83 ids

Parity vocabulary exactly as `gap_analysis.md:13-20`: `match` · `partial` · `missing` · `deferred` ·
`na` · `tbd_only`.

### 2.1 RIGHT — Asset Browser (13)

| eden_id | tbd_id | parity | gap_notes | maps_to_ticket |
|---|---|---|---|---|
| RIGHT-MODE-001 | RIGHT-CAT-001 | partial | Live registry tree exists (`asset_catalog.rs:146` `build_catalog_tree`; DockRight tab 0 "Factions" `eden_chrome.rs:3040-3055`). But Eden's F1 Object mode is **one** tree over units+vehicles+props; TBD splits it across a Factions tab, a Vehicles tab (`eden_chrome.rs:2961-3007`) and an Objects side-chip. No `F1` binding — F-keys are **deliberately banned** (`eden_chrome.rs:4998-5011`, T-180.5). `gap_analysis` calls this `match`; downgraded — see §6. | T-146 |
| RIGHT-MODE-002 | — | missing | No composition mode. `grep -rniwE 'composition' --include=*.rs apps/website/frontend/src` → **3 hits, all `asset_catalog.rs:345,362,782`**, the `comp:` *alias slug* for a Reforger prefab. That is not Eden's author-saved composition — see §6. | T-078 |
| RIGHT-MODE-003 | — | missing | No trigger entity. `grep -rnwE 'trigger' --include=*.rs apps/website/frontend/src` → 6 hits, **all prose/comments** (`auth.rs:514`, `yrs_persist.rs:94`, `attributes.rs:19`, `mission_commands.rs:258`, `editor_ops.rs:68`, `mission_editor.rs:1170`). | T-079 |
| RIGHT-MODE-004 | — | missing | `grep -rliwE 'waypoint' --include=*.rs apps/website/frontend/src` → **0 files**. | T-079 |
| RIGHT-MODE-005 | — | missing | No systems/modules family. | T-079 |
| RIGHT-MODE-006 | RIGHT-STUB-002 | missing | Markers tab is a stub (`eden_chrome.rs:3000-3005`, tab index 2). | T-069 / T-213 |
| RIGHT-SUBMODE-001 | EDEN-SIDE-CHIPS | partial | Eden **side** chips BLUFOR/OPFOR/INDFOR/Objects filter the tree (`eden_chrome.rs:2871` `EDEN_SIDE_CHIPS`, `:2919` `apply_eden_chip`, rebuild via `build_catalog_tree(_, side)`). Eden's submode is a per-mode faction/side sub-tab row cycled by `Tab`; no `Tab` binding in TBD (`grep -rnoE '"Tab"' --include=*.rs` → 0). `gap_analysis` says `missing | T-074`; T-074 is **cancelled** and the chips shipped in T-180.5 — see §6. | — (shipped T-180.5) |
| RIGHT-SEARCH-001 | — | match | `asset_catalog.rs:396-414` `filter_catalog` — case-insensitive label substring, folder self-match keeps subtree, descendant match keeps matching children. Wired at `eden_chrome.rs:3113-3140`. Per-tab search boxes (Factions / Vehicles / Objects) each own their query (`eden_chrome.rs:2989-3011`). | T-055 (shipped) |
| RIGHT-SEARCH-002 | — | missing | `filter_catalog` parses **no** prefixes — one `to_lowercase().contains(q)` (`asset_catalog.rs:402`). | T-084 |
| RIGHT-SEARCH-003 | — | missing | Same; no `mod:` handling. | T-084 |
| RIGHT-SEARCH-004 | — | missing | Same; no glob. | T-084 |
| RIGHT-SEARCH-005 | — | deferred | Same; no regex. Lowest value of the four search modes for a milsim authoring flow — deliberately behind `class:`/`mod:`. | T-084 |
| RIGHT-CREW-001 | — | missing | No crew concept anywhere: `grep -rowE '\bcrew\b' --include=*.rs apps/website/frontend/src \| wc -l` → **0**; `\bseat\b` → **0**. Vehicles carry **cargo** rows only (`eden_chrome.rs:1985` `placed_vehicles_panel`, `editor_ops.rs:1811` `set_vehicle_cargo`). | T-076 |

### 2.2 PLACE — Entity placement (7)

| eden_id | tbd_id | parity | gap_notes | maps_to_ticket |
|---|---|---|---|---|
| PLACE-001 | — | missing | Click-then-click is **structurally impossible** today. A place is armed on a palette leaf `pointerdown` (`editor_ops.rs:1037` `begin_place` / `:1044` vehicle / `:1049` object) and committed on the map `pointerup` (`mission_editor.rs:1662-1699`). A release that is **not** inside the chrome-free rect cancels (`mission_editor.rs:1668-1671, 1693` → `cancel_pending`), and a plain click on the leaf releases over the leaf. So the arm dies on the click that made it. `gap_analysis` says `partial`; downgraded — see §6. | new (P-1) |
| PLACE-002 | PLACE-DROP-001 | match | Press-drag-release from palette to map places at the cursor world point (`mission_editor.rs:1662-1699` → `editor_ops.rs:2191` `place_at`). Live translucent ghost follows the cursor (`mission_editor.rs:1493-1500` `set_place_preview`, T-175 B2). | — |
| PLACE-003 | — | missing | The dblclick handler picks and returns on a miss — `mission_editor.rs:1936-1969`: `let hit = …pick(…); if let Some(id) = hit { open_attributes(id) }`. No empty-space branch, so no type picker. | new (P-2) |
| PLACE-004 | — | missing | `place_at` unconditionally `take()`s the pending arm (`editor_ops.rs:2204`) — one-shot by construction. No modifier is read in the place path (`mission_editor.rs:1655-1700` contains no `ctrl_key`). | **T-072** |
| PLACE-005 | ZONE-DRAW-001 | partial | An area **does** get drawn, by a different gesture and for a different family. Circle = click centre then rim; polygon = click each vertex then Close (`editor_ops.rs:2436-2497` `advance_zone_draw`, `:2499` `close_zone_polygon`; UI `eden_chrome.rs:2228-2300`). Eden is LMB **hold-drag**. TBD's areas are schema `zone.type` play areas/objectives (`eden_chrome.rs:3518` `zone_types`), **not** trigger or marker areas. | T-582 shipped; drag modality + marker/trigger areas → T-069 / T-079 |
| PLACE-COMMENT-001 | — | missing | Blocked twice. No annotation entity, and **TBD has no context menu at all**: `contextmenu` is `prevent_default()` and nothing else (`mission_editor.rs:1844-1847`), because RMB is a pan button (`mission_editor.rs:1402`). | new (P-3, needs the menu first) |
| PLACE-CREW-001 | — | missing | `grep -rn 'alt_key()' --include=*.rs apps/website/frontend/src` → **3 hits, all disqualifiers** (`mission_editor.rs:1020`, `:1023`, `mission_history.rs:490`). Alt is read nowhere in the pointer path. | **T-077** |

### 2.3 XFORM — Basic transform (5)

| eden_id | tbd_id | parity | gap_notes | maps_to_ticket |
|---|---|---|---|---|
| XFORM-MOVE-001 | XFORM-MOVE-001 | match | Pending→Move promotion past a 4 px threshold (`select_tool.rs:31`, `mission_editor.rs:1526-1534`), GPU preview via `push_drag_preview` (`select_tool.rs:232-247`), one-txn commit `move_entities_and_vehicles` (`mission_editor.rs:1779-1785`). Dragging a selected entity moves the whole selection (`select_tool.rs:189-195`). | — |
| XFORM-ALT-001 | — | **na** | Altitude-by-drag needs a screen axis that is not the ground plane. TBD's camera is a fixed top-down `OrthoCamera` (`select_tool.rs:86-96`) where every screen pixel maps to exactly one (x,y) — there is no Z direction to drag along. Z is read-only DEM sampling for the CUR readout (`mission_editor.rs:1480-1484`) and is written `0.0` on paste (`editor_ops.rs:491`). Numeric Z is the correct 2D substitute and already exists (`attributes.rs` Transform tab, `editor_ops.rs:667` `attrs_update_position`). | — |
| XFORM-SHIFT-001 | XFORM-ROT-001 | missing | The Pending→Move/Marquee promotion reads no `shift_key` (`mission_editor.rs:1525-1582`). Rotation is authored **numerically only** (`attributes.rs:292` `number_field("Rotation", a.rotation, Some("°"), …)`; vehicles via `editor_ops.rs:1841` `set_vehicle_heading`). | **T-073** |
| XFORM-VERT-001 | — | **na** | Eden's Vertical Mode swaps the drag axis to Z and picks an ATL/ASL datum (`batch02_menus.md:556-559`). Both halves are meaningless without a Z drag axis (see XFORM-ALT-001) and without a sea-vs-terrain height model in the doc. | — |
| XFORM-SNAP-001 | — | **na** | There is no un-snapped state to toggle. TBD's compiled slot carries no authored altitude and the mod grounds every spawn — `TBD_SpawnManager` resolves `jsonY` → `GetSurfaceY` with `CAPSULE_GROUND_OFFSET_M = 0.0` (T-092.1). Surface snapping is therefore always-on and non-optional by contract, not a missing control. | — |

### 2.4 CREW — Vehicle crew (4)

| eden_id | tbd_id | parity | gap_notes | maps_to_ticket |
|---|---|---|---|---|
| CREW-PANEL-001 | — | missing | `\bcrew\b` → 0 hits in the SPA (command in RIGHT-CREW-001). No hover panel; hover picking was removed for perf (T-057) and never returned. | T-076 |
| CREW-BOARD-001 | — | missing | Dragging a character onto a vehicle runs the ordinary move commit (`mission_editor.rs:1762-1795`) — the two entities overlap and nothing else happens. | T-076 |
| CREW-UNBOARD-001 | — | missing | No crew relation exists to detach. | T-076 |
| CREW-SEAT-001 | — | missing | Needs both a seat model and a context menu; TBD has neither (`mission_editor.rs:1844-1847`). | T-076 |

> **Compile-side caveat.** Even a built crew model would not reach the game today: the T-216 drop
> ledger (`crates/map-engine-core/src/mission/flatten.rs:2584-2649`) records that the **entire
> vehicle roster** is silently dropped by the compile. Any T-076 scoping must include that.

### 2.5 WIDGET — Transformation widget (6)

| eden_id | tbd_id | parity | gap_notes | maps_to_ticket |
|---|---|---|---|---|
| WIDGET-CYCLE-001 | — | missing | No widget to cycle, and **`Space` is already bound** to centre-on-selection (`mission_editor.rs:1026` → `editor_ops.rs:354` `center_on_selection`). This is the known collision. | **T-075** |
| WIDGET-TRANS-001 | — | missing | Buildable in 2D (X/Y axis handles); direct drag currently substitutes. `grep -rniwE 'widget' --include=*.rs apps/website/frontend/src` → 1 hit, an unrelated comment at `server_control.rs:1138`. | T-075 |
| WIDGET-ROT-001 | — | missing | A 2D yaw ring is buildable and is the natural home for Shift-drag rotate. | T-073 / T-075 |
| WIDGET-AREA-SCALE-001 | ZONE-RESHAPE-001 | partial | Radius **is** re-authorable — `editor_ops.rs:2378` `begin_zone_reshape` re-arms the circle draw and `set_zone_circle` replaces the shape (`editor_ops.rs:2483-2486`), preserving label/faction/rules. But it is a re-draw, not an on-map scaling handle, and it never touches trigger areas (none exist). | T-582 follow-on |
| WIDGET-AREA-001 | ZONE-RESHAPE-001 | partial | Same mechanism for polygons (`eden_chrome.rs:2576` "Redraw this zone as a polygon"). No vertex handles. | T-582 follow-on |
| WIDGET-COORD-001 | — | **na** | A global/local reference toggle presupposes a widget with axes and an entity with a full orientation. TBD's 2D doc stores **yaw only** (`attributes.rs:292`; compiled as `headingDeg`, T-092.1) in a single axis-aligned world frame, so "local" and "global" describe the same two axes. | — |

### 2.6 TOOLBAR (2 literal ids of a ~15-button strip)

| eden_id | tbd_id | parity | gap_notes | maps_to_ticket |
|---|---|---|---|---|
| TOOLBAR-NEW-001 | LIB-NEWMISSION-001 | partial | The capability exists on a **different surface**: `CreateMissionDialog` from the Mission Library, incl. `Cmd/Ctrl+N` (`missions.rs:212-231`), mission_maker+ only. Missions are DB rows minted before the editor opens (T-048), so the editor's own strip has File ▸ Save Version / Export / Export Compiled only (`eden_chrome.rs:112-128`). Eden's `Ctrl+N` inside the editor has no TBD equivalent. | — |
| TOOLBAR-TUTORIAL-001 | — | deferred | No in-editor tutorial or onboarding surface. Low value before the editor's feature set settles. | — |
| *(13 unnamed toolbar buttons)* | — | **UNKNOWN** | `interactions.md:371` names them in prose (Open, Save, Workshop, Undo, Redo, widgets, snap, grids, intel, map, flashlight, vision, phase) but assigns **no ids**, so they are not triageable here. Verbatim tooltips + pixel bounds exist in `batch01_context_menu.md:343-365` and `batch05_asset_browser_2.md:275-295`. | docs task |

### 2.7 COMP — Custom compositions (5)

| eden_id | tbd_id | parity | gap_notes | maps_to_ticket |
|---|---|---|---|---|
| COMP-SAVE-001 | — | missing | No composition family (command in RIGHT-MODE-002). Eden's entry point is RMB ▸ `Save Custom Composition...` (`batch01_context_menu.md:212`) — also blocked by the missing context menu. | T-078 |
| COMP-EDIT-001 | — | missing | No metadata (title/author/category) record to edit. | T-078 |
| COMP-PLACE-001 | — | missing | Nothing multi-entity is placeable. Note `place_at` places exactly one entity per arm (`editor_ops.rs:2197-2266`). | T-078 |
| COMP-WORKSHOP-001 | — | deferred | Steam Workshop publish. The **transport** is genuinely unavailable to a browser (no Steam client), which argues `na`; the **capability** — share an authored composition with the community — maps onto TBD's mission library/versions API, so `deferred` (matching `gap_analysis.md:72`) is the honest call. | T-078 (stretch) |
| COMP-SUBSCRIBE-001 | — | deferred | Same reasoning, consuming side. | T-078 (stretch) |

### 2.8 CONN — Connections (8)

| eden_id | tbd_id | parity | gap_notes | maps_to_ticket |
|---|---|---|---|---|
| CONN-START-001 | — | missing | Eden's whole connect flow starts at RMB ▸ `Connect ▸` (`batch01_context_menu.md:201`). TBD suppresses the context menu (`mission_editor.rs:1844-1847`). | T-079 (+ P-3 menu) |
| CONN-GROUP-001 | ORBAT-* | partial | Grouping **exists**, off the map. ORBAT Manager modal (T-071, `orbat_manager.rs`) plus `editor_ops.rs:1293` `orbat_add_squad`, `:1335` `orbat_add_slot`, `:1446` `orbat_set_leader`, `:1526` `orbat_rename_squad`, and outliner refile-drag `:2142` `begin_refile` → `:2152` `complete_refile_onto_squad` (drop target `eden_chrome.rs:1583-1594`). Eden's **map-surface Ctrl+drag character→character** does not exist. | T-071 shipped; map gesture → new (P-4) |
| CONN-SYNC-001 | — | missing | `\bsync\b` in the SPA is 30 hits of `onSynced`/persist plumbing, none an entity relation. No sync edge in the doc. | T-079 |
| CONN-TRG-OWNER-001 | — | missing | Requires triggers. | T-079 |
| CONN-RAND-START-001 | — | missing | Requires waypoints. | T-079 |
| CONN-WP-ACT-001 | — | missing | Requires waypoints. | T-079 |
| CONN-WP-ATTACH-001 | — | missing | Requires waypoints. | T-079 |
| CONN-DEL-001 | — | missing | `Delete` is bound (`mission_editor.rs:1027` → `editor_ops.rs:327` `delete_selection`) but only over the slot set — there is no connection line to be the selection. | T-079 |

### 2.9 SEL / LAYER / ATTR / CTX (12)

| eden_id | tbd_id | parity | gap_notes | maps_to_ticket |
|---|---|---|---|---|
| SEL-001 | SEL-MAP-001 | match | Sub-threshold release = click; picks against the **frozen** press camera and replaces/clears (`mission_editor.rs:1727-1757`, `select_tool.rs:165-181` `apply_click`). Covers slots **and** placed vehicles (`pick_slot_or_vehicle`, `select_tool.rs:150-158`). | — |
| SEL-MOD-001 | SEL-MOD-001 | match | `additive = ev.ctrl_key() \|\| ev.meta_key()` (`mission_editor.rs:1731`) → toggle in/out, empty+additive preserves (`select_tool.rs:166-180`). Eden's `AddUnitToSel` is add-only; TBD toggles — a superset, T-053 decision. Shift stays unbound. | T-053 (shipped) |
| SEL-ALL-001 | — | missing | `grep -rnoE '"KeyA"' --include=*.rs apps/website/frontend/src \| wc -l` → **0**. Eden's real label is `Select All on Screen` / `Select All in View`, `Ctrl+A` (`batch02_menus.md:541`, `batch01_context_menu.md:245`) — **viewport-scoped**, not whole-document. Cheap: the marquee already has a viewport-AABB primitive (`select_tool.rs:308` `marquee_ids_with_vehicles`). | new (P-5) |
| SEL-GROUP-ICON-001 | LEFT-ORBAT-001 | missing | Squad rows render as **non-interactive `<div>`s** — `eden_chrome.rs:1578-1611`: the `orbat_refile` branch has only `on:pointerup` (refile drop), the other branch has no handler at all. No group glyph on the map either (T-180 draws leader lines, not a selectable group icon). `gap_analysis` says `partial \| T-071`; T-071 shipped and this did not change — see §6. | new (P-6) |
| SEL-LAYER-CHILDREN-001 | — | missing | Folder click sets the **drop target**, not a selection: `eden_chrome.rs:1624-1627` → `editor_ops.rs:1025` `set_active_layer`. `title="Make this the drop target"` (`eden_chrome.rs:1622`). | new (P-6) |
| SEL-LAYER-DESC-001 | — | missing | Same handler; no descendant walk. | new (P-6) |
| LAYER-CREATE-001 | LEFT-LAYER-005 | missing | The doc mutator exists and is **uncalled from any UI**: `crates/map-engine-core/src/doc/store.rs:1872` `add_editor_layer`, whose only SPA caller is the auto-seed `editor_ops.rs:1136` inside `ensure_layer`. `grep -rnwE 'add_editor_layer\|rename_editor_layer\|reparent_editor_layer' --include=*.rs apps/website/frontend/src` → 3 hits, **all comments plus that one seed**. DockLeft has no `+`/rename control (`eden_chrome.rs:2838-2860`; its five footer buttons are `disabled=true`, "visual only"). `gap_analysis` says `tbd_only`, which is a claim about *semantics* not *existence* — see §6. | new (P-6) |
| LAYER-DEL-001 | LEFT-LAYER-007 | missing | `store.rs:1527` `remove_editor_layer` has **zero** SPA callers (same grep). `Delete` removes the selected slots only (`editor_ops.rs:342` `core.remove_slots(ids)`). | new (P-6) |
| ATTR-OPEN-001 | ATTR-OPEN-001 | partial | Opens from map dblclick (`mission_editor.rs:1936-1969`) and outliner row dblclick (`eden_chrome.rs:1655-1660`). Two real limits: the pick is **slot-only** (`select_tool.rs:128-130`), so vehicles / placed objects / zones never open it; and any multi-selection **suppresses** it (`editor_ops.rs:583-585`). | new (P-7) |
| ATTR-MULTI-001 | — | missing | Not merely unbuilt — actively refused by the same guard (`editor_ops.rs:583-585`). Eden reaches it via RMB ▸ `Attributes...` (`batch01_context_menu.md:219`); TBD has no RMB menu. | T-082 |
| ATTR-MULTI-CHK-001 | — | missing | Per-field "values differ" opt-in checkbox. Presupposes ATTR-MULTI-001. | T-082 |
| CTX-FORMATION-001 | — | missing | No context menu, no formation action. `grep -rnwE 'formation' --include=*.rs apps/website/frontend/src` → **1 hit, a comment** (`editor_ops.rs:1324`) describing the `APPLY_ANCHOR_X + 15.0 * i` **line-up** used when applying a faction library — a placement spacing rule, not a formation. | T-079 / new (P-3) |

### 2.10 KEY shortcuts (4)

| eden_id | tbd_id | parity | gap_notes | maps_to_ticket |
|---|---|---|---|---|
| KEY-WP-001 | — | missing | Shift+RMB. RMB is the pan button (`mission_editor.rs:1402`) and no `shift_key` is read in `onpointerdown`. Needs waypoints first. | T-079 |
| KEY-WIDGET-001 | KEY-SPACE-CENTER-001 | missing | **Collision** — `Space` = centre on selection (`mission_editor.rs:1026`). | **T-075** |
| KEY-GRID-001 | — | missing | No snapping grid at all (`\bsnap\b` in the SPA is 33 hits, **every one** a `snapshot`/`snap` local — `mission_commands.rs:187-304`, `orbat_manager.rs:283-306`). Also: `interactions.md:522` records the key as `` ` ; ` `` but the screenshots show `odiaeresis` — see §4.3 and §6. | new (P-8) |
| KEY-HIDE-UI-001 | — | missing | **Collision** — `Backspace` = delete selection (`mission_editor.rs:1027`). | new (P-9) |

### 2.11 ACTION appendix (10)

| eden_id | tbd_id | parity | gap_notes | maps_to_ticket |
|---|---|---|---|---|
| ACTION-COPY-001 | ACTION-COPY-001 | match | `Ctrl/Cmd+C`, Alt+Shift disqualify (`mission_editor.rs:1020`) → `editor_ops.rs:394` `copy_selection` (snapshots the selected slot dicts). | T-056 (shipped) |
| ACTION-CUT-001 | — | missing | `grep -rnoE '"KeyX"' --include=*.rs apps/website/frontend/src \| wc -l` → **0**. Trivially `copy_selection() && delete_selection()`. | new (P-10) |
| ACTION-PASTE-001 | ACTION-PASTE-001 | match | `Ctrl/Cmd+V` (`mission_editor.rs:1023`) → `editor_ops.rs:436` `paste_at_cursor(cx, cy)`; centroid → cursor, terrain-clamped, one undo step, paste becomes the selection (`:542-562`). | T-056 (shipped) |
| ACTION-PASTE-ORIG-001 | — | missing | The V arm requires `!ev.shift_key()` (`mission_editor.rs:1023`), so `Ctrl+Shift+V` falls through. The primitive already exists — `paste_at_cursor(None, None)` is the paste-at-original path (`editor_ops.rs:436`, `cx/cy` optional). Near-free. | new (P-10) |
| ACTION-LEVEL-001 | — | **na** | `LevelWithSurface` aligns an object's up-vector to the terrain normal, which needs pitch **and** roll. The 2D doc stores yaw only (`attributes.rs:292`; compiled `headingDeg`, T-092.1) and the mod re-grounds every spawn. There is no orientation to level. | — |
| ACTION-SNAP-001 | — | **na** | Same as XFORM-SNAP-001: snapping is unconditional mod-side, so there is no action to invoke. | — |
| ACTION-SEAT-001 | — | missing | Needs the crew model. | T-076 |
| ACTION-FORM-001 | — | missing | See CTX-FORMATION-001 — the one `formation` hit is a comment. | T-079 / new |
| ACTION-TOGGLE-SEL-001 | SEL-MOD-001 | match | `ToggleUnitSel` **is** TBD's Ctrl+LMB semantics — `select_tool.rs:166-173` removes if present else pushes. Duplicate of SEL-MOD-001 in Eden's own id space. | T-053 (shipped) |
| ACTION-WP-QUICK-001 | — | missing | Duplicate of KEY-WP-001 (same Shift+RMB, same MOVE waypoint). Counted once for tickets. | T-079 |

### 2.12 STATUS bar (7)

| eden_id | tbd_id | parity | gap_notes | maps_to_ticket |
|---|---|---|---|---|
| STATUS-X-001 | BOTTOM-CUR-X | match | `eden_chrome.rs:3727-3730`, fed by `mission_editor.rs:1461-1487` (frozen-cam unproject, un-throttled). Swaps to `SEL` X when exactly one entity is selected (`eden_chrome.rs:3686-3703`). | T-049/T-050 (shipped) |
| STATUS-Y-001 | BOTTOM-CUR-Y | match | `eden_chrome.rs:3731-3734`. | T-049/T-050 |
| STATUS-Z-001 | BOTTOM-CUR-Z | match | `eden_chrome.rs:3735-3738`; DEM-sampled (`mission_editor.rs:1480-1484` `sample_grid_meters`), em-dash outside coverage. | T-091.2 (shipped) |
| STATUS-ZOOM-001 | — | missing | The toolbelt shows CUR/SEL X/Y/Z + OBJ/SEL/SZ (`eden_chrome.rs:3706-3767`) and **no scale readout**. Worth more than it looks: Eden's printed `m/pix` is what let `batch08` derive the contour ladder (`eden_screenshots/README.md:51-59`), and the engine already owns `zoom()` (`select_tool.rs:518`). | new (P-11) |
| STATUS-VER-001 | — | **na** | "Game version" is the running Arma build. A browser editor has no attached game build. *(A schema/mod-version chip would be a `tbd_only` addition, not this id.)* | — |
| STATUS-MOD-001 | — | **na** | "Mods loaded" is the local addon set of a running game client. TBD's equivalent lives at platform level (`modpacks.rs`), not in the editor. | — |
| STATUS-SRV-001 | — | **na** | Eden reports the MP editing server it is attached to. TBD's editor is single-author with IndexedDB persistence + immutable server versions; server state lives on the Server Intel / Server Control pages. | — |

---

## 3. Summary counts

### By parity

| parity | count | share |
|---|---:|---:|
| `match` | 12 | 14.5 % |
| `partial` | 9 | 10.8 % |
| `missing` | 46 | 55.4 % |
| `deferred` | 4 | 4.8 % |
| `na` | 12 | 14.5 % |
| `tbd_only` | 0 | 0 % |
| **total** | **83** | |

`tbd_only` is 0 by construction: this sweep walks Eden's id space, so a TBD addition has no Eden id
to sit under. The TBD-only surfaces (zones, ORBAT Manager, workflow layers, semver versions, the
compiled-mission export, the SZ payload estimate) are named in the notes above and in
`gap_analysis.md:120-121`.

### By domain

| domain | ids | match | partial | missing | deferred | na |
|---|---:|---:|---:|---:|---:|---:|
| RIGHT | 13 | 1 | 2 | 9 | 1 | 0 |
| PLACE | 7 | 1 | 1 | 5 | 0 | 0 |
| XFORM | 5 | 1 | 0 | 1 | 0 | 3 |
| CREW | 4 | 0 | 0 | 4 | 0 | 0 |
| WIDGET | 6 | 0 | 2 | 3 | 0 | 1 |
| TOOLBAR | 2 | 0 | 1 | 0 | 1 | 0 |
| COMP | 5 | 0 | 0 | 3 | 2 | 0 |
| CONN | 8 | 0 | 1 | 7 | 0 | 0 |
| SEL/LAYER/ATTR/CTX | 12 | 2 | 1 | 9 | 0 | 0 |
| KEY | 4 | 0 | 0 | 4 | 0 | 0 |
| ACTION | 10 | 3 | 0 | 5 | 0 | 2 |
| STATUS | 7 | 3 | 0 | 1 | 0 | 3 |
| **total** | **83** | **12** | **9** | **46** | **4** | **12** |

### The 12 `na` and why each is genuinely `na`

Not one of these is "hard, skip it" — each is a capability whose **precondition** a 2D top-down web
editor does not have:

| id | missing precondition |
|---|---|
| XFORM-ALT-001 | a screen axis that is not the ground plane |
| XFORM-VERT-001 | ditto, plus an ATL/ASL height datum in the doc |
| XFORM-SNAP-001 | an un-snapped state — the mod grounds every spawn (T-092.1) |
| WIDGET-COORD-001 | an orientation with more than yaw; one axis-aligned world frame |
| ACTION-LEVEL-001 | pitch/roll in the slot record |
| ACTION-SNAP-001 | as XFORM-SNAP-001 |
| STATUS-VER-001 | an attached game build |
| STATUS-MOD-001 | a local addon set |
| STATUS-SRV-001 | an in-editor MP session |
| *(3 counted above under XFORM/WIDGET/ACTION)* | | |

**`na` ≠ nothing to do.** XFORM-ALT-001's 2D substitute (numeric Z) already ships; XFORM-SNAP-001's
substitute is enforced mod-side. Where a substitute is missing it is named in the row, not hidden by
the `na`.

---

## 4. Keyboard map — Eden's bindings, TBD's bindings, every collision

### 4.1 What TBD binds today — the complete set

Derived by reading the handlers, not by assuming. Three listeners own every editor key:

| # | Listener | File:line | Keys |
|---|---|---|---|
| 1 | editor actions | `mission_editor.rs:1008-1043` | `Ctrl/Cmd+C`, `Ctrl/Cmd+V`, `Space`, `Delete`, `Backspace` |
| 2 | undo/redo | `mission_history.rs:481-509` | `Ctrl/Cmd+Z`, `Ctrl/Cmd+Shift+Z`, `Ctrl+Y` |
| 3 | dismissal | `eden_chrome.rs:872-880`, `attributes.rs:35-40`, `ui.rs:207/282`, `orbat_manager.rs:249`, `faction_manager.rs:70` | `Escape` |

Route-scoped, **outside** the editor: `Cmd/Ctrl+N` on the Mission Library (`missions.rs:212-231`).

Both editor listeners guard on `in_editable_field()` (`mission_editor.rs:1010`,
`mission_history.rs:487`; impl `mission_history.rs:~460-471`) and match on **`ev.code()`**, not
`ev.key()` — layout-independent, so a modifier cannot remap the binding (`mission_history.rs:476`).

**The complete TBD editor binding table:**

| Keys | Action | Evidence |
|---|---|---|
| `Ctrl/Cmd+C` | copy selection | `mission_editor.rs:1020` → `editor_ops.rs:394` |
| `Ctrl/Cmd+V` | paste at cursor | `mission_editor.rs:1023` → `editor_ops.rs:436` |
| `Space` | centre camera on selection centroid | `mission_editor.rs:1026` → `editor_ops.rs:354` |
| `Delete` | delete selection | `mission_editor.rs:1027` → `editor_ops.rs:327` |
| `Backspace` | delete selection (alias) | `mission_editor.rs:1027` |
| `Ctrl/Cmd+Z` | undo | `mission_history.rs:497` |
| `Ctrl/Cmd+Shift+Z` | redo | `mission_history.rs:494` |
| `Ctrl+Y` | redo | `mission_history.rs:500` |
| `Escape` | close topmost modal / menu | `ui.rs:208`, `eden_chrome.rs:873` |
| `Ctrl/Cmd+N` | new mission *(Library route only)* | `missions.rs:219` |

That is **10 bindings**. Everything else is unbound — verified by literal search:

```bash
cd apps/website/frontend/src
for k in KeyA KeyX KeyF KeyM KeyE KeyR KeyL KeyG KeyT KeyI KeyS KeyO KeyN \
         Digit1 Digit2 Digit3 Digit4 Digit5 Tab Home Quote Minus Semicolon; do
  printf '%-10s %s\n' "$k" "$(grep -rnoE "\"$k\"" --include=*.rs . | wc -l)"; done
# every line → 0
```

`F1`–`F6` appear exactly twice, both in a **negative** unit test that bans an F-key mode row
(`eden_chrome.rs:4998-5011`, T-180.5).

### 4.2 Eden's bindings — the full sweep

From `interactions.md` **plus** the screenshot corpus, which carries ~20 shortcuts
`interactions.md` never records. Cross-check column flags disagreements.

| Eden keys | Eden action | Source | TBD binds | Parity |
|---|---|---|---|---|
| `Ctrl+N` | Scenario ▸ New | `batch02:518` | `Ctrl+N` **on Library route** | different surface |
| `Ctrl+O` | Scenario ▸ Open | `batch02:519` | — | free |
| `Ctrl+S` | Scenario ▸ Save | `batch02:520` | — | free |
| `Ctrl+Shift+S` | Save As | `batch02:521` | — | free |
| `Ctrl+M` | Merge scenario | `batch02:531` | — | free |
| `Ctrl+Z` | Undo | `batch02:537` | `Ctrl/Cmd+Z` | ✅ **match** |
| `Ctrl+Y` | Redo | `batch02:538` | `Ctrl+Y` | ✅ **match** |
| — | *(Eden has no `Ctrl+Shift+Z`)* | — | `Ctrl/Cmd+Shift+Z` redo | TBD superset |
| `Ctrl+A` | Select All **on Screen** | `batch02:541`, `batch01:245` | — | free → P-5 |
| `Ctrl+X` | Cut | `batch01:154,268` | — | free → P-10 |
| `Ctrl+C` | Copy | `batch01:155,269` | `Ctrl/Cmd+C` | ✅ **match** |
| `Ctrl+V` | Paste | `batch01:156,270` | `Ctrl/Cmd+V` | ✅ **match** |
| `Ctrl+Shift+V` | Paste on Original Position | `batch01:157,271` | — (V arm requires `!shift`) | free → P-10 |
| `Delete` | Delete | `batch01:158,272` | `Delete` | ✅ **match** |
| `Space` | Toggle/cycle Transformation Widget | `batch02:544`, `interactions:521` | **`Space` = centre on selection** | 🔴 **COLLISION 1** |
| `1` | No Widget | `batch02:545` | — | free |
| `2` | Translation Widget | `batch02:546` | — | free |
| `3` | Rotation Widget | `batch02:547` | — | free |
| `4` | Area Scaling Widget | `batch02:548` | — | free |
| `5` | Area Widget | `batch02:549` | — | free |
| `ö` (`odiaeresis`) | Toggle Translation Grid | `batch02:550`, `batch05:239` | — | free (see §4.3) |
| `å` (`aring`) | Decrease Grid Size | `batch02:552` | — | free (see §4.3) |
| `¨` (`dead_diaeresis`) | Increase Grid Size | `batch02:553` | — | free (see §4.3) |
| `ä` (`adiaeresis`) | Toggle Vertical Mode | `batch02:556`, `batch05:293` | — | free (see §4.3) |
| `'` | Toggle Surface Snapping | `batch02:560` | — | free |
| `-` | Toggle Waypoint Snapping | `batch02:561` | — | free |
| `F1` | Asset Type ▸ Objects | `batch02:565`, `batch06` | — (**F-keys banned**, `eden_chrome.rs:4998-5011`) | deliberate divergence |
| `F2` | Asset Type ▸ Compositions | `batch02:566` | — | deliberate divergence |
| `F3` | Triggers | `batch02:567` | — | deliberate divergence |
| `F4` | Waypoints | `batch02:568` | — | deliberate divergence |
| `F5` | Systems | `batch02:569` | — | deliberate divergence |
| `F6` | Markers | `batch02:570` | — | deliberate divergence |
| `F7` | Favorites *(greyed)* | `batch02:571` | — | deliberate divergence |
| `Tab` | Toggle Asset Sub-type | `batch02:572` | — | free |
| `Ctrl+R` | Center on Random Position | `batch02:573` | — | ⚠️ **browser reload** — avoid |
| `F` | Center on Selected Entity | `batch02:574` | — (TBD uses `Space`) | 🟡 **semantic clash 2** |
| `Home` | Center on Player | `batch02:575` | — | free |
| `M` | Toggle Map (3D ⇄ 2D) | `batch02:577` | — | 🟡 `na`-adjacent — TBD is always 2D |
| `Ctrl+T` | Toggle Map Textures | `batch02:578` | — | ⚠️ **browser new-tab** — avoid |
| `L` | Toggle Flashlight | `batch02:581` | — | free (`na` — no 3D lighting) |
| `Ctrl+G` | Toggle Foliage | `batch02:583` | — | free; TBD's analogue is the forest toggle in Mission Settings |
| `Ctrl+F` | Search in Asset Browser | `batch02:586` | — | ⚠️ **browser find** — but high value, P-12 |
| `Ctrl+Shift+F` | Search in Entity List | `batch02:587` | — | free |
| `Backspace` | Toggle Interface (hide chrome) | `batch02:589`, `interactions:523` | **`Backspace` = delete selection** | 🔴 **COLLISION 3** |
| `E` | Show/hide Entity List (left panel) | `batch02:590` | — | free |
| `R` | Show/hide Asset Browser (right panel) | `batch02:591` | — | free |
| `Ctrl+I` | Attributes ▸ Environment… | `batch02:596` | — | free |
| `Shift+RMB` | Quick MOVE waypoint | `interactions:520,542` | **RMB = pan** (`mission_editor.rs:1402`) | 🔴 **COLLISION 4** |
| `Ctrl` (held) | Multi-place | `interactions:202` | **Ctrl+LMB = additive select** (`mission_editor.rs:1731`) | 🔴 **COLLISION 5** |
| `Ctrl` + drag char→char | Group | `interactions:435` | same Ctrl+LMB | 🔴 **COLLISION 5** (same modifier) |
| `Shift` + drag | Rotate to cursor | `interactions:262` | unbound in the drag path | free → T-073 |
| `Alt` + drag | Altitude | `interactions:254` | unbound (`na` in 2D) | free |
| `Alt` (held) | Invert crew toggle when placing | `interactions:233` | unbound | free → T-077 |
| `RMB` (empty) | Context menu ▸ Place Comment | `interactions:222`, `batch01:128` | **RMB = pan**, `contextmenu` killed | 🔴 **COLLISION 4** (same cause) |

### 4.3 The X11 keysym artefacts — map to glyphs, do not ship the strings

The screenshot corpus caught Eden printing **raw X11 keysym identifiers** in its menus and tooltips
instead of glyphs, on a Nordic/German layout under Proton:

| Printed string | Real key | Eden action | Verbatim source |
|---|---|---|---|
| `odiaeresis` | **ö** | Toggle Translation Grid | `batch02_menus.md:349,549`; tooltip `Toggle Translation Grid (odiaeresis)` — `batch05_asset_browser_2.md:235,239` |
| `aring` | **å** | Decrease Grid Size | `batch02_menus.md:352,552` |
| `dead_diaeresis` | **¨** | Increase Grid Size | `batch02_menus.md:353,553` |
| `adiaeresis` | **ä** | Toggle Vertical Mode | `batch02_menus.md:556`; tooltip `Toggle Vertical Mode (adiaeresis)` — `batch05_asset_browser_2.md:220,293` |

**These are Proton/X11 key-name-lookup artefacts of the operator's Linux build, not Eden strings.**
`batch02_menus.md:355-357` states it directly: *"the shortcut column prints the raw X11/engine keysym
name … rather than a glyph. This operator is on a Nordic/German layout and Eden does not localise the
key name."* On a US layout the same three grid bindings sit on `;` `[` `]`-adjacent keys — which is
almost certainly where `interactions.md:522`'s `` ` ; ` `` for `KEY-GRID-001` came from.

**Consequences for TBD:**

1. Never render a keysym identifier in a shortcut hint. If TBD ever prints shortcuts, print glyphs.
2. `interactions.md`'s `` ` ; ` `` and the corpus's `odiaeresis` are **the same binding on two
   layouts** — not a contradiction, and neither is portable. Any TBD grid toggle should pick its own
   key rather than copy either.
3. TBD already avoids the whole class of bug by matching on `ev.code()` (physical position), not
   `ev.key()` (layout-dependent glyph) — `mission_history.rs:476`, `mission_editor.rs:1019`.

### 4.4 The collisions — full list

| # | Eden | TBD today | Severity | Resolution |
|---|---|---|---|---|
| **1** | `Space` cycles the transformation widget | `Space` centres the camera on the selection (`mission_editor.rs:1026` → `editor_ops.rs:354`) | **hard** — same key, both wanted | **T-075**. Eden itself gives the widget four *direct* keys (`1`–`5`, `batch02:545-549`), all unbound in TBD. Cleanest fix: widget on `1`–`5`, keep `Space` = centre, drop the cycle. Eden's own `F` (Center on Selected, `batch02:574`) is also free if `Space` is ever wanted back. |
| **2** | `F` centres on the selected entity | unbound; TBD does this on `Space` | **soft** — semantic clash, no key clash | Bind `F` as an alias in the T-075 slice. Zero cost, closes the muscle-memory gap. |
| **3** | `Backspace` hides all editor chrome | `Backspace` deletes the selection (`mission_editor.rs:1027`) | **hard** — and dangerous: an Eden author reaching for a screenshot **deletes their selection** | P-9. Either drop the `Backspace` delete alias (`Delete` already covers it) or bind hide-UI elsewhere. Deleting the alias is one line and strictly safer. |
| **4** | `RMB` opens the context menu; `Shift+RMB` drops a quick waypoint | RMB is a **pan** button and `contextmenu` is unconditionally suppressed (`mission_editor.rs:1402`, `:1844-1847`) | **hard, structural** — blocks 7 ids at once: PLACE-COMMENT-001, CREW-SEAT-001, CONN-START-001, CTX-FORMATION-001, ATTR-MULTI-001, COMP-SAVE-001, KEY-WP-001/ACTION-WP-QUICK-001 | P-3. MMB already pans (`mission_editor.rs:1402` accepts button 1 **or** 2), so RMB can be freed for the menu with the pan intact. This is the single highest-leverage keyboard/pointer change in the sweep. |
| **5** | `Ctrl` held = multi-place; `Ctrl` + drag character→character = group | `Ctrl/Cmd+LMB` = additive select toggle (`mission_editor.rs:1731`) | **hard, three-way** — Eden overloads `Ctrl` on *press context* (empty vs entity vs armed palette) | **T-072** must disambiguate by state, not by key: armed-palette + `Ctrl` → repeat place; entity-drag + `Ctrl` → group (T-071 map gesture); otherwise → additive select (unchanged). Land T-072 and the CONN-GROUP-001 map gesture **together** or the second one re-opens the first. |
| ⚠ | `Ctrl+R` random position, `Ctrl+T` map textures, `Ctrl+F` asset search | unbound, but the **browser** owns all three (reload / new tab / find) | **environmental** | Do not copy `Ctrl+R` or `Ctrl+T`. `Ctrl+F` is worth `preventDefault`-ing when the editor route is focused (P-12) — it is the highest-frequency Eden shortcut TBD has no answer for. |

---

## 5. Proposed ticket groupings

### 5.1 Existing tickets — what each absorbs

| Ticket | Status | Ids it absorbs | Notes |
|---|---|---|---|
| **T-072** Ctrl multi-place | queued | `PLACE-004` | Must resolve **collision 5**. Scope: make `place_at` re-arm instead of `take()`ing while `Ctrl` is held (`editor_ops.rs:2204`). Ship with the CONN-GROUP-001 map gesture. |
| **T-073** Shift + map rotation | queued | `XFORM-SHIFT-001`, `WIDGET-ROT-001` | `Shift` is entirely free in the drag path. Rotation already round-trips numerically (`attributes.rs:292`, `editor_ops.rs:1841`), so this is a gesture on an existing field. |
| **T-075** Spacebar flyTo vs widget | queued | `WIDGET-CYCLE-001`, `KEY-WIDGET-001`, `WIDGET-TRANS-001`, + **collision 2** (`F` alias) | Promote from "resolve a key clash" to "the widget slice". Eden's `1`–`5` direct keys are free; use them and the clash dissolves. |
| **T-076** Vehicle crew UI | idea | `RIGHT-CREW-001`, `CREW-PANEL-001`, `CREW-BOARD-001`, `CREW-UNBOARD-001`, `CREW-SEAT-001`, `ACTION-SEAT-001` | **6 ids.** Must include the T-216 compile drop of the vehicle roster (`flatten.rs:2584-2649`) or the UI authors state the game never sees. |
| **T-077** Alt empty vehicle | idea | `PLACE-CREW-001` | Depends on T-076 (there must be a crew to suppress). `Alt` is free. |
| **T-078** Custom compositions | deferred | `RIGHT-MODE-002`, `COMP-SAVE-001`, `COMP-EDIT-001`, `COMP-PLACE-001`, `COMP-WORKSHOP-001`, `COMP-SUBSCRIBE-001` | **6 ids.** Save entry point needs P-3 (context menu) or a palette button. |
| **T-079** Triggers + waypoints + systems | idea | `RIGHT-MODE-003/004/005`, `CONN-START-001`, `CONN-SYNC-001`, `CONN-TRG-OWNER-001`, `CONN-RAND-START-001`, `CONN-WP-ACT-001`, `CONN-WP-ATTACH-001`, `CONN-DEL-001`, `KEY-WP-001`, `ACTION-WP-QUICK-001`, `ACTION-FORM-001`, `CTX-FORMATION-001` | **14 ids — the largest single absorber.** Far too big for one ticket; split (see §5.3). |
| **T-069 / T-213** Markers | deferred / idea | `RIGHT-MODE-006` | Marker area draw also wants PLACE-005's drag modality. |
| **T-084** Classname / mod prefix search | deferred | `RIGHT-SEARCH-002`, `RIGHT-SEARCH-003`, `RIGHT-SEARCH-004`, `RIGHT-SEARCH-005` | **4 ids, one function.** All four are prefix/pattern parsing in front of `filter_catalog` (`asset_catalog.rs:396`). Cheapest ratio in the sweep — **promote**. |
| **T-082** Full attribute fields | deferred | `ATTR-MULTI-001`, `ATTR-MULTI-CHK-001` | Multi-edit needs the `editor_ops.rs:583-585` suppression removed first. |
| **T-146** Asset Browser Data Wiring | queued | `RIGHT-MODE-001` | Unifying the split Factions/Vehicles/Objects trees toward one Eden-style Object mode. |

Existing tickets absorb **40 of the 83**. `na` accounts for **12**. The remaining **31** need the
new slices below.

### 5.2 New slices — proposed, coherent, smallest-first

| # | Proposed slice | Ids | Why it is one slice |
|---|---|---|---|
| **P-10** | **Clipboard completion** — `Ctrl+X` cut, `Ctrl+Shift+V` paste-at-original | `ACTION-CUT-001`, `ACTION-PASTE-ORIG-001` | Both are one match arm each in `mission_editor.rs:1019-1031`; both primitives already exist (`copy_selection`+`delete_selection`; `paste_at_cursor(None, None)`). Hours, not days. |
| **P-5** | **Select All in View** (`Ctrl+A`) | `SEL-ALL-001` | Eden's is viewport-scoped; the marquee already has the exact primitive (`select_tool.rs:308`). One arm + one screen-AABB. |
| **P-11** | **Scale readout in the toolbelt** | `STATUS-ZOOM-001` | One `<span>` beside CUR/SEL (`eden_chrome.rs:3706-3767`); `zoom()` is already exposed. Also unblocks any future contour-ladder work (`eden_screenshots/README.md:51-59`). |
| **P-9** | **Backspace collision + hide-UI** | `KEY-HIDE-UI-001` | Drop the `Backspace` delete alias (`mission_editor.rs:1027`), bind hide-chrome. Pairs naturally with the T-638 panel show/hide chevron work. |
| **P-12** | **`Ctrl+F` focuses asset search** *(not an `interactions.md` id — from the corpus)* | — | `batch02:586`; the highest-frequency Eden shortcut with no TBD answer. Needs `preventDefault` on the editor route. Fold into P-5 or T-146. |
| **P-1/P-2/P-7** | **Placement + Attributes entry points** — click-then-click place; dbl-click-empty type picker; dbl-click opens for vehicles/objects/zones | `PLACE-001`, `PLACE-003`, `ATTR-OPEN-001` | All three are the same seam: what a click or dbl-click on the map means. P-7 in particular is one call-site swap — `mission_editor.rs:1963` `pick` → `pick_slot_or_vehicle`, which the click path already uses. |
| **P-6** | **Outliner layer authoring** — create / rename / delete / reparent, folder-click selects children + descendants | `LAYER-CREATE-001`, `LAYER-DEL-001`, `SEL-LAYER-CHILDREN-001`, `SEL-LAYER-DESC-001`, `SEL-GROUP-ICON-001` | **5 ids, and the doc mutators already exist and are uncalled** — `store.rs:1872/1886/1895/1527`. This is pure UI wiring onto shipped, tested core functions. Best value-per-line in the sweep. |
| **P-3** | **Right-click context menu** (the unblocking slice) | `PLACE-COMMENT-001` directly; **unblocks** `CREW-SEAT-001`, `CONN-START-001`, `CTX-FORMATION-001`, `ATTR-MULTI-001`, `COMP-SAVE-001`, `KEY-WP-001` | Free RMB from pan (MMB already pans — `mission_editor.rs:1402`), stop the blanket `prevent_default` (`:1844-1847`), render a menu. Verbatim item lists for both takes are in `batch01_context_menu.md:119-292`. Six other tickets are gated on this. |
| **P-4** | **Map-surface grouping** (`Ctrl` + drag character→character) | `CONN-GROUP-001` (map half) | Ship **with T-072** — they overload the same modifier (collision 5). |
| **P-8** | **Snapping grids** — translation grid toggle + step | `KEY-GRID-001` | Nothing snap-related exists (`\bsnap\b` = 33 hits, all `snapshot`). Pick a TBD key; do **not** copy `odiaeresis` or `;` (§4.3). |

### 5.3 Split T-079 before dispatching it

T-079 as written absorbs 14 ids across three unrelated entity families. Proposed split:

| Slice | Ids | Depends on |
|---|---|---|
| **T-079a** triggers (entity + area + owner) | `RIGHT-MODE-003`, `CONN-TRG-OWNER-001` | PLACE-005 drag modality |
| **T-079b** waypoints (entity + attach + activation + quick-drop) | `RIGHT-MODE-004`, `CONN-WP-ACT-001`, `CONN-WP-ATTACH-001`, `CONN-RAND-START-001`, `KEY-WP-001`, `ACTION-WP-QUICK-001` | **P-3** (Shift+RMB needs RMB back) |
| **T-079c** systems/modules | `RIGHT-MODE-005` | — |
| **T-079d** connection graph (draw / sync / delete / formation) | `CONN-START-001`, `CONN-SYNC-001`, `CONN-DEL-001`, `ACTION-FORM-001`, `CTX-FORMATION-001` | **P-3** |

### 5.4 Suggested order

1. **P-3 (context menu)** — gates six tickets; nothing else unlocks as much.
2. **P-6 (layer authoring)** — 5 ids onto already-shipped doc mutators.
3. **T-084 (search prefixes)** — 4 ids, one function.
4. **P-10 + P-5 + P-11 + P-9** — four small, independent, same-file slices; one bundle.
5. **T-075 + T-073 + T-072/P-4** — the modifier/widget bundle. Land together (collisions 1, 2, 5).
6. **T-076 → T-077** — crew, with the T-216 compile fix in scope.
7. **T-079a–d, T-078, T-069/T-213** — the absent entity families.

---

## 6. Cross-check notes — where the three sources disagree

Ordered by how much a plan built on the wrong one would cost.

### 6.1 `gap_analysis.md` vs live source — 5 rows are stale or over-generous

| Row | `gap_analysis` says | Live source says | Evidence |
|---|---|---|---|
| `RIGHT-MODE-001` | `match` (T-068.3) | **partial** — the tree is live, but Eden's single Object mode is three separate TBD surfaces (Factions tab / Vehicles tab / Objects chip) and there is no `F1` | `eden_chrome.rs:2990-3007`, `:4998-5011` |
| `RIGHT-SUBMODE-001` | `missing \| T-074` | **partial**, and **T-074 is `cancelled`** — Eden side chips shipped in T-180.5 and do filter the tree by side | `eden_chrome.rs:2871`, `:2919`; registry `T-074 = cancelled` |
| `PLACE-001` | `partial` ("TBD drag-only; no click-then-click") | **missing** — the note is right, the value is not. Click-then-click is not partially present; the arm dies on the click that creates it | `editor_ops.rs:1037`, `mission_editor.rs:1668-1671, 1693` |
| `SEL-GROUP-ICON-001` | `partial \| T-071` ("read-only until T-071") | **missing** — T-071 shipped; squad rows are still non-interactive `<div>`s with no click-to-select-group | `eden_chrome.rs:1578-1611` |
| `LAYER-CREATE-001` | `tbd_only` ("Editor Layers ≠ Eden layers") | **missing** — the *semantic* claim is fair, but there is no create control at all; the mutator is uncalled | `store.rs:1872` vs `editor_ops.rs:1136` (sole caller) |

Also confirmed stale, already flagged by [`README.md:69-71`](README.md): `ENV-SETTINGS-002`'s
`partial` predates T-193, which **removed** View Distance and Thermals. Not an `interactions.md` id,
so it has no row here — but it sits in the same table and should be corrected in the same pass.

`gap_analysis.md` also uses `working` as a parity value for `RIGHT-SEARCH-001` (line 35), which is
**not in its own legend** (`:13-20`). Read as `match`. **Not edited** — the brief forbids modifying
that file.

### 6.2 `interactions.md` vs the screenshot corpus — the doc is thin and one binding is wrong

| Item | `interactions.md` | Screenshot corpus | Verdict |
|---|---|---|---|
| Grid toggle key | `` ` ; ` `` (`:522`) | `odiaeresis` = **ö** (`batch02:349,549`; `batch05:239`) | **Same binding, two layouts.** Neither is portable — §4.3. The corpus is the *observed* value; `interactions.md` is presumably the US-layout wiki value. Do not copy either. |
| Widget keys | `Space` only (`:521`) | `Space` **plus** direct `1`/`2`/`3`/`4`/`5` (`batch02:544-549`) | Corpus is richer and **dissolves collision 1**. `interactions.md` is incomplete. |
| F-tab names | `F1 Object / F2 Composition` (`:22-100`) | batch 06 confirms **Objects / Compositions** by tooltip; batch 05 wrongly read them as Units/Groups | `interactions.md` **agrees with batch 06**, the correct one. Corpus README already resolves this (`README.md:31-43`). |
| Select-all scope | "Select all on screen" (`:487`) | `Select All on Screen` (menu, `batch02:541`) / `Select All in View` (context menu, `batch01:245`) | **Two different labels for one action.** Both agree it is **viewport-scoped**. Do not build a whole-document select-all. |
| Menu-bar shortcuts | ~4 shortcuts total | ~35, verbatim, with enabled/greyed state (`batch02:508-600`) | The corpus is the far better keyboard source. §4.2 is built from it. |
| Context menu contents | one line, "RMB → Connect → type" (`:427`) | full item lists for both takes — 6 items empty / 14 items with a unit selected, with icons, y-bounds and states (`batch01:119-292`) | The corpus is the spec for P-3. |
| Toolbar ids | prose range, endpoints only (`:371`) | full button inventory with bounds + verbatim tooltips (`batch01:343-365`, `batch05:275-295`) | 13 buttons have no id and cannot be triaged. |

### 6.3 The recovered `parity/` inventories — spot-checks

Per [`README.md:17-22`](README.md), these are chat-report transcriptions and were treated as claims
to verify. Two load-bearing lines were checked directly:

* ✅ **`README.md:73-79` — the T-216 drop ledger.** Verified present at
  `crates/map-engine-core/src/mission/flatten.rs:2584-2649`. Load-bearing for T-076 (§5.1) — the
  entire vehicle roster is dropped by the compile, so a crew UI would author state the game never
  receives.
* ✅ **`README.md:83-88` — the `stance` word-boundary trap.** The methodology reproduced here and
  it caught a live false positive: `grep -rn 'snap' apps/website/frontend/src` looks like ~33 hits
  of snapping support; `grep -rnwE 'snap'` shows **every one** is a `snapshot`/`snap` local
  (`mission_commands.rs:187-304`, `orbat_manager.rs:283-306`). TBD has **no** snapping. Every
  absence claim in §2 is word-boundary.

### 6.4 Two TBD surfaces that look like Eden parity and are not

* **`comp:` is not a composition.** `asset_catalog.rs:345,362` derives a `comp:<slug>` alias when a
  Reforger `ResourceName` contains `Composition` — that is an Enfusion *prefab path*, a single
  placeable entity. Eden's COMP-* family is an author-saved, re-placeable multi-entity group with
  metadata. A `grep -i composition` returns hits and means nothing for COMP parity.
* **Zones are not trigger areas.** `PLACE-005`/`WIDGET-AREA-*` are `partial` because zones give the
  *geometry* authoring (`editor_ops.rs:2436-2528`), but `zone_types()` is schema-driven
  (`eden_chrome.rs:3518`) and covers play areas / objectives. No trigger entity, no activation
  condition, no marker area. Do not close the trigger ids on zone evidence.

### 6.5 One live inconsistency found in passing (not an Eden parity gap)

`Delete` routes to `editor_ops.rs:327` `delete_selection`, which calls `core.remove_slots(ids)`
(`:342` → `store.rs:1379`) — the **slot** path only. But the selection can legitimately contain
**vehicle** ids (`select_tool.rs:150-158` `pick_slot_or_vehicle`; `mission_editor.rs:1733`), and
vehicles have a separate `editor_ops.rs:1908` `remove_vehicle` reached only from the dock's
"Remove vehicle" button (`eden_chrome.rs:2067-2073`).

**INFERRED:** pressing `Delete` with a vehicle selected removes nothing for that vehicle while
clearing the selection (`editor_ops.rs:344`). Not verified in-browser; recorded here as a lead, not
a finding. Compare the drag commit, which *does* handle both lanes in one txn
(`mission_editor.rs:1769-1785` `move_entities_and_vehicles`).
