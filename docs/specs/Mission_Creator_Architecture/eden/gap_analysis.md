# Eden Gap Analysis — TBD vs Arma 3 Eden (Phase 2)

**Document:** `eden/gap_analysis.md`  
**Inputs:** [feature_inventory.md](../feature_inventory.md) + [interactions](./interactions.md) + [ui_anatomy](./ui_anatomy.md) + [attributes](./attributes.md)  
**Schema:** [reference/feds_schema.md](../reference/feds_schema.md)

**Coverage: 191 ids — a census, not a sample.** 191 = 93 `attributes.md` ids + 83 `interactions.md` ids + 15 legacy/TBD-only rows. Verified by set-diff in both directions (empty), zero duplicate ids. Stated as an *id* count deliberately: a table-*row* count is parser-dependent here (legend, corrections and summary tables also carry parity words) and three careful parsers read it three ways.

| Source catalogue | Ids defined | Rows here |
|---|---:|---:|
| [`interactions.md`](./interactions.md) | 83 | **83** |
| [`attributes.md`](./attributes.md) | 93 | **93** |
| [`feature_inventory.md`](../feature_inventory.md) + TBD-only | — | **15** |
| | | **191** |

Both catalogues are covered **exactly** — no id in either file is missing a row, and no row cites an
id that is not in one of them. Verify:

```bash
cd docs/specs/Mission_Creator_Architecture/eden
grep -oE '\bATTR-FIELD-[A-Z0-9-]+\b' attributes.md | sort -u | wc -l                       # 93
grep -oE '\b[A-Z][A-Z0-9]*(-[A-Z0-9]+)*-[0-9]{3}\b' interactions.md | sort -u | wc -l      # 83
```

**Execution order (recorded 2026-06, historical):** … → **T-061..T-067** → **T-090 → T-091 → T-092**
(map hard gate) → **T-071** → **T-068 Phase 2** → Eden **T-069+** → **T-110** terrain base
([`t110_terrain_base_mission_layers.md`](../t110_terrain_base_mission_layers.md)). T-071, T-091,
T-092 and T-180 have since shipped; T-090 is active. This line is kept for provenance and is not the
current queue — see [`docs/TICKET_LEAD.md`](../../../TICKET_LEAD.md).

---

## Provenance — 2026-08-01 rewrite

**What this file was.** A **59-row sample** carrying a parity value, written incrementally as
individual tickets shipped. It covered **41 of 83** interaction ids and **3 of 93** attribute ids.

**What it had been read as.** A census of Eden parity. Every planning document written against it
inherited that reading. [`docs/platform/EDITOR_UI_HANDOFF.md`](../../../platform/EDITOR_UI_HANDOFF.md)
originally described it as *"87 rows, 32 missing"* — both numbers wrong, and the error propagated
into the program plan and the ticket drafts before anyone opened the file it described. That
document now carries a correction; this one is the fix it points at. Keep the two consistent.

**What changed here.** Every id from both catalogues now has a row, the ticket column is
reconciled against `.ai/tickets/registry.json`, attribute rows carry a **`build_class`**, and seven
known-stale or known-wrong values are corrected (listed below). The summary counts at the end are
recomputed from this body, not carried forward.

**Sources — the triage, not re-derived here.** This table is a transcription of two sweeps that
walked every id against live source with `file:line` evidence:

| Sweep | Scope |
|---|---|
| [`attributes_sweep.md`](../../../../.ai/artifacts/parity/attributes_sweep.md) | all 93 `ATTR-FIELD-*` ids — parity, build class, ticket mapping |
| [`interactions_sweep.md`](../../../../.ai/artifacts/parity/interactions_sweep.md) | all 83 `interactions.md` ids + the full Eden/TBD keyboard map and five collisions |
| [`owns_parity.md`](../../../../.ai/artifacts/parity/owns_parity.md) · [`owns_and_waves.md`](../../../../.ai/artifacts/parity/owns_and_waves.md) | per-ticket file ownership and wave packing |
| [`README.md`](../../../../.ai/artifacts/parity/README.md) | what in that directory is verified and what is a recovered chat report |

**Corrections applied in this rewrite**

| # | Row | Was | Now | Why |
|---|---|---|---|---|
| 1 | `ATTR-FIELD-OBJ-SKILL` | `missing` | **`na`** | Every body spawns AI-disabled (`TBD_SpawnManager.c:963,1166`); `skill` word-boundary in `apps/mod` = 0. `missing` implies buildable work with no subject |
| 2 | `ENV-SETTINGS-002` | `partial` | **`missing`** | T-193 (`b30f5490`) *removed* View Distance and Thermals; `eden_chrome.rs:4624` now refuses to author them |
| 3 | `ATTR-FIELD-LYR-NAME` | `match` | **`missing`** | `rename_editor_layer` has exactly one mention repo-wide — its own definition (`store.rs:1886`). Same for `reparent_editor_layer` / `remove_editor_layer` / `move_slot_to_layer`. T-037 shipped the core; nothing reaches it |
| 4 | `OBJ-CALLSIGN` · `OBJ-RANK` · `OBJ-STANCE` | (no rows) | **`partial`**, not `match` | Authored by live controls, then **silently dropped at compile** — T-216 ledger `flatten.rs:2584-2649`, cross-checked against `TBD_MissionSlotStruct.c:59-69` |
| 5 | `RIGHT-MODE-001` · `RIGHT-SUBMODE-001` · `PLACE-001` · `SEL-GROUP-ICON-001` · `LAYER-CREATE-001` | stale / over-generous | see rows | `interactions_sweep.md` §6.1. **T-074 is `cancelled`** — nothing cites it as live |
| 6 | `CONN-GROUP-001` | `missing` | **`partial`** | Shipped via T-071 / T-180; the table was never updated. Only Eden's *map-surface* Ctrl+drag is absent |
| 7 | `working` (parity value) | not in the legend | **retired → `match`** | See below |

**The `working` reconciliation.** The old table scored `RIGHT-SEARCH-001` as `working`, a value
absent from its own legend. It leaked in from `feature_inventory.md`, where `working` is a **Status**
field value, not a parity value. **Retired rather than legalised**, and that one row is mapped onto
**`match`** — which is what `interactions_sweep.md` independently scores it, with evidence
(`asset_catalog.rs:396-414`, T-055 shipped). Adding a sixth parity value for a single row would have
grown the vocabulary instead of reconciling it.

**One disagreement between the sweeps, recorded not laundered.** `attributes_sweep.md` scores
`ATTR-FIELD-LYR-NAME` **`match`** on the *existence* of `rename_editor_layer`;
`interactions_sweep.md` scores its peers `LAYER-CREATE-001` / `LAYER-DEL-001` **`missing`** on the
*reachability* of the same family of mutators. Both greps are correct; they answer different
questions. The main thread verified reachability directly (one mention, its own definition) and this
table follows the reachability reading — see correction 3 and
[`gap_analysis_rewrite_log.md`](../../../../.ai/artifacts/parity/gap_analysis_rewrite_log.md).

---

## Parity legend

| Status | Meaning |
|--------|---------|
| **match** | Equivalent (2D acceptable) |
| **partial** | Exists but incomplete |
| **missing** | Not built |
| **deferred** | Intentional later phase |
| **na** | 3D-only / out of scope — a capability whose **precondition** a 2D web editor does not have |
| **tbd_only** | TBD addition, no Eden id |

`working` is **not** a parity value — see the provenance note.

## Build-class legend — attribute rows only

The single most useful thing this table carries: it separates factory-dispatchable work from
`executor: workbench` work.

| Class | Test | Program |
|---|---|---|
| **a** | No mission-contract blocker — the compiled schema already carries the key, or the value is editor-only and never compiled. SPA (± website API) work | factory, `executor: claude-code` |
| **b** | `mission.schema.json` must be widened first (25 `additionalProperties: false`, incl. `$defs/slot`, `group`, `meta`, `environment`, `marker`) | `executor: workbench` |
| **c** | Enfusion-side support must be built — no concept exists in `apps/mod/tbd-framework`, or a runtime (AI, triggers, damage model) would have to exist first. Usually needs (b) too; the deeper blocker wins | `executor: workbench` |
| **d** | Out of scope: an A3-engine concept with no Enfusion analogue, a scripting handle with no scripting layer, or something TBD refused **by design with a test pinning the refusal** | none — closed |

**(d) ⇔ parity `na` by construction.** All 22 `d` rows are `na` and no other attribute row is.
Interaction rows are **not** build-classed — the interactions sweep did not triage that axis, and
inventing it here would launder an unmeasured claim.

## Ticket column legend

`T-0xx` values are **registry-verified** against `.ai/tickets/registry.json`. `✅` = `shipped`.
`new — Px` / `new — Nx` name a proposed slice defined in the sweeps
(`interactions_sweep.md` §5.2, `attributes_sweep.md` §5) that has **no registry ticket yet**; the
editor-UI draft set that adopts several of them (T-631…T-660) was not in the registry when this
table was written. `wb` marks a row whose ticket is `executor: workbench` — a second program.

**`T-074` is `cancelled`** (absorbed by T-180.5). No row cites it as live.

---

# Part 1 — Interaction parity (83 ids from `interactions.md`)

## Asset browser & placement — RIGHT (13)

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| RIGHT-MODE-001 | RIGHT-CAT-001 | partial | T-146 | **Was `match`.** Live registry tree exists (`asset_catalog.rs:146` `build_catalog_tree`, DockRight tab 0), but Eden's F1 Object mode is **one** tree over units+vehicles+props; TBD splits it across a Factions tab, a Vehicles tab (`eden_chrome.rs:2961-3007`) and an Objects side-chip. F-keys are deliberately banned (`eden_chrome.rs:4998-5011`, T-180.5) |
| RIGHT-MODE-002 | — | missing | T-078 | No composition mode. The 3 `composition` hits (`asset_catalog.rs:345,362,782`) are the `comp:` **alias slug** for a Bohemia prefab, not an author-saved composition |
| RIGHT-MODE-003 | RIGHT-STUB-003 | missing | T-079 (a) | No trigger entity. `grep -rnwE 'trigger' --include=*.rs` → 6 hits, all prose/comments |
| RIGHT-MODE-004 | — | missing | T-079 (b) · wb | `grep -rliwE 'waypoint' --include=*.rs apps/website/frontend/src` → **0 files**. Gated on AI units existing |
| RIGHT-MODE-005 | — | missing | T-079 (c) | No systems/modules family. **Un-enumerable today** — the `SYS` family declares no ids (`attributes.md:223-225`) |
| RIGHT-MODE-006 | RIGHT-STUB-002 | missing | T-069 / T-213 | Markers tab is a stub (`eden_chrome.rs:3000-3005`, body at `:3337`, pinned by a test at `:4499-4502`) |
| RIGHT-SUBMODE-001 | EDEN-SIDE-CHIPS | partial | — (shipped T-180.5) | **Was `missing \| T-074`; T-074 is `cancelled`.** BLUFOR/OPFOR/INDFOR/Objects chips filter the tree (`eden_chrome.rs:2871` `EDEN_SIDE_CHIPS`, `:2919` `apply_eden_chip`). Eden's `Tab`-cycled per-mode sub-tab row does not exist |
| RIGHT-SEARCH-001 | — | match | T-055 ✅ | **Was `working`** (not a legend value). `asset_catalog.rs:396-414` `filter_catalog`; per-tab query state (`eden_chrome.rs:2989-3011`) |
| RIGHT-SEARCH-002 | — | missing | T-084 | `class:` prefix. `filter_catalog` parses **no** prefixes — one `to_lowercase().contains(q)` (`asset_catalog.rs:402`) |
| RIGHT-SEARCH-003 | — | missing | T-084 | `mod:` prefix — same function |
| RIGHT-SEARCH-004 | — | missing | T-084 | Glob — same function |
| RIGHT-SEARCH-005 | — | deferred | T-084 | Regex. Lowest value of the four search modes; deliberately behind `class:` / `mod:` |
| RIGHT-CREW-001 | — | missing | T-076 | Vehicle crew toggle. `\bcrew\b` → **0**, `\bseat\b` → **0** in the SPA. Vehicles carry **cargo** rows only (`eden_chrome.rs:1985`, `editor_ops.rs:1811`) |

## Asset browser & placement — PLACE (7)

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| PLACE-001 | — | missing | new — P-1 | **Was `partial`.** Click-then-click is **structurally impossible**: a place is armed on palette `pointerdown` (`editor_ops.rs:1037`) and committed on map `pointerup` (`mission_editor.rs:1662-1699`); a release outside the chrome-free rect cancels (`:1668-1671`), so a plain click on the leaf kills its own arm. The note was right, the value was not |
| PLACE-002 | PLACE-DROP-001 | match | — | Press-drag-release places at the cursor world point (`mission_editor.rs:1662-1699` → `editor_ops.rs:2191` `place_at`), live ghost (`:1493-1500`, T-175 B2) |
| PLACE-003 | — | missing | new — P-2 | Dbl-click empty picker. The handler picks and returns on a miss (`mission_editor.rs:1936-1969`); no empty-space branch |
| PLACE-004 | — | missing | **T-072** | Ctrl multi-place. `place_at` unconditionally `take()`s the arm (`editor_ops.rs:2204`) — one-shot by construction; no modifier read in the place path |
| PLACE-005 | ZONE-DRAW-001 | partial | T-582 ✅ (follow-on) | An area **is** drawable, by a different gesture for a different family: circle = click centre then rim, polygon = click vertices then Close (`editor_ops.rs:2436-2497`). Eden is LMB **hold-drag**. TBD's areas are schema `zone.type` play areas/objectives — **not** trigger or marker areas |
| PLACE-COMMENT-001 | — | missing | new — P-3 | Blocked twice: no annotation entity, and **TBD has no context menu at all** — `contextmenu` is `prevent_default()` and nothing else (`mission_editor.rs:1844-1847`), because RMB is a pan button (`:1402`) |
| PLACE-CREW-001 | — | missing | **T-077** | Alt + empty vehicle. `alt_key()` → 3 hits, **all disqualifiers** (`mission_editor.rs:1020`, `:1023`, `mission_history.rs:490`). Alt is free |

## Transform, widget, toolbar — XFORM (5)

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| XFORM-MOVE-001 | XFORM-MOVE-001 | match | — | 4 px promotion (`select_tool.rs:31`), GPU preview (`:232-247`), one-txn commit `move_entities_and_vehicles` (`mission_editor.rs:1779-1785`) |
| XFORM-ALT-001 | — | na | — | Altitude-by-drag needs a screen axis that is not the ground plane; the camera is a fixed top-down `OrthoCamera` (`select_tool.rs:86-96`). **2D substitute already ships** — numeric Z (`editor_ops.rs:667`) |
| XFORM-SHIFT-001 | XFORM-ROT-001 | missing | **T-073** | No Shift rotate — the drag promotion reads no `shift_key` (`mission_editor.rs:1525-1582`). Rotation is numeric-only (`attributes.rs:292`; vehicles `editor_ops.rs:1841`) |
| XFORM-VERT-001 | — | na | — | Vertical Mode swaps the drag axis to Z and picks an ATL/ASL datum. Both halves are meaningless without a Z axis and a sea-vs-terrain height model |
| XFORM-SNAP-001 | XFORM-SNAP-001 | na | — | There is no un-snapped state to toggle: the mod grounds every spawn (`jsonY` → `GetSurfaceY`, `CAPSULE_GROUND_OFFSET_M = 0.0`, T-092.1). Always-on **by contract**, not a missing control |

## Transform, widget, toolbar — WIDGET (6)

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| WIDGET-CYCLE-001 | — | missing | **T-075** | No widget to cycle, and **`Space` is already bound** to centre-on-selection (`mission_editor.rs:1026` → `editor_ops.rs:354`). **Collision 1** |
| WIDGET-TRANS-001 | — | missing | T-075 | Axis widget. Buildable in 2D (X/Y handles); direct drag substitutes today |
| WIDGET-ROT-001 | — | missing | T-073 / T-075 | A 2D yaw ring is buildable and is the natural home for Shift-drag rotate |
| WIDGET-AREA-SCALE-001 | ZONE-RESHAPE-001 | partial | T-582 ✅ (follow-on) | Radius **is** re-authorable (`editor_ops.rs:2378` `begin_zone_reshape`, `:2483-2486`), preserving label/faction/rules — but it is a re-draw, not an on-map handle, and never touches trigger areas |
| WIDGET-AREA-001 | ZONE-RESHAPE-001 | partial | T-582 ✅ (follow-on) | Same mechanism for polygons (`eden_chrome.rs:2576`). No vertex handles |
| WIDGET-COORD-001 | — | na | — | Global/local reference presupposes axes and a full orientation. The doc stores **yaw only** in one axis-aligned world frame, so "local" and "global" name the same two axes |

## Transform, widget, toolbar — TOOLBAR (2 literal ids of a ~15-button strip)

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| TOOLBAR-NEW-001 | LIB-NEWMISSION-001 | partial | — (T-048 ✅) | The capability exists on a **different surface**: `CreateMissionDialog` from the Mission Library incl. `Cmd/Ctrl+N` (`missions.rs:212-231`). Missions are DB rows minted before the editor opens, so the editor strip has Save Version / Export only. Eden's in-editor `Ctrl+N` has no equivalent |
| TOOLBAR-TUTORIAL-001 | — | deferred | — | No in-editor tutorial surface. Low value before the feature set settles |

> **The ~13 other toolbar buttons have no ids and cannot be triaged.** `interactions.md:371` writes
> the strip as a prose range and literalises only the two endpoints. Verbatim tooltips and pixel
> bounds exist in [`.ai/artifacts/eden_screenshots/`](../../../../.ai/artifacts/eden_screenshots/) (`batch01_context_menu.md:343-365`,
> `batch05_asset_browser_2.md:275-295`); **minting ids from them is a docs task, not this table's to
> invent.** Four were minted ad hoc by the old sample and are preserved in Part 3.

## Compositions — COMP (5)

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| COMP-SAVE-001 | — | missing | T-078 | Eden's entry point is RMB ▸ `Save Custom Composition…` — also blocked by the missing context menu (P-3) |
| COMP-EDIT-001 | — | missing | T-078 | No metadata (title/author/category) record to edit |
| COMP-PLACE-001 | — | missing | T-078 | Nothing multi-entity is placeable; `place_at` places exactly one entity per arm (`editor_ops.rs:2197-2266`) |
| COMP-WORKSHOP-001 | — | deferred | T-078 (stretch) | Steam Workshop publish. The **transport** is unavailable to a browser, which argues `na`; the **capability** maps onto TBD's mission library/versions API, so `deferred` is the honest call |
| COMP-SUBSCRIBE-001 | — | deferred | T-078 (stretch) | Same reasoning, consuming side |

## Connections, crew & groups — CONN (8)

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| CONN-START-001 | — | missing | T-079 (d) + P-3 | Eden's connect flow starts at RMB ▸ `Connect ▸`; TBD suppresses the context menu (`mission_editor.rs:1844-1847`) |
| CONN-GROUP-001 | ORBAT-* | partial | T-071 ✅ · map gesture new — P-4 | **Was `missing`.** Grouping exists, off the map: ORBAT Manager modal plus `editor_ops.rs:1293` `orbat_add_squad`, `:1335` `orbat_add_slot`, `:1446` `orbat_set_leader`, outliner refile-drag `:2142`. Only Eden's **map-surface Ctrl+drag character→character** is absent |
| CONN-SYNC-001 | XFORM-SYNC-001 | missing | T-079 (d) | `\bsync\b` is 30 hits of `onSynced`/persist plumbing — no entity relation, no sync edge in the doc |
| CONN-TRG-OWNER-001 | — | missing | T-079 (a) | Requires triggers |
| CONN-RAND-START-001 | — | missing | T-079 (b) · wb | Requires waypoints — AI-gated |
| CONN-WP-ACT-001 | — | missing | T-079 (b) · wb | Requires waypoints — AI-gated |
| CONN-WP-ATTACH-001 | — | missing | T-079 (b) · wb | Requires waypoints — AI-gated |
| CONN-DEL-001 | — | missing | T-079 (d) | `Delete` is bound (`mission_editor.rs:1027` → `editor_ops.rs:327`) but only over the slot set — there is no connection line to be the selection |

## Connections, crew & groups — CREW (4)

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| CREW-PANEL-001 | — | missing | T-076 | Hover crew list. `\bcrew\b` → 0 in the SPA; hover picking was removed for perf at T-057 and never returned |
| CREW-BOARD-001 | — | missing | T-076 | Dragging a character onto a vehicle runs the ordinary move commit — the two entities overlap and nothing else happens |
| CREW-UNBOARD-001 | — | missing | T-076 | No crew relation exists to detach |
| CREW-SEAT-001 | — | missing | T-076 | Change seat via RMB. Needs both a seat model and a context menu; TBD has neither |

> **Compile-side caveat for any T-076 scoping.** The T-216 drop ledger
> (`crates/map-engine-core/src/mission/flatten.rs:2584-2649`) records that the **entire vehicle
> roster** is silently dropped by the compile. A crew UI built today would author state the game
> never receives — that half of T-076 is `executor: workbench`.

## Layers, selection & attributes entry — SEL / LAYER / ATTR / CTX (12)

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| SEL-001 | SEL-MAP-001 | match | — | Sub-threshold release = click; picks against the **frozen** press camera and replaces/clears (`mission_editor.rs:1727-1757`, `select_tool.rs:165-181`). Covers slots **and** placed vehicles |
| SEL-MOD-001 | SEL-MOD-001 | match | T-053 ✅ | `additive = ctrl \|\| meta` (`mission_editor.rs:1731`) → toggle in/out; empty+additive preserves. Eden's `AddUnitToSel` is add-only, TBD toggles — a superset. Shift stays unbound |
| SEL-ALL-001 | KEY-SELALL-001 | missing | new — P-5 | `"KeyA"` → **0** hits. Eden's is **viewport-scoped** (`Select All on Screen` / `in View`), not whole-document. Cheap — the marquee already has the primitive (`select_tool.rs:308`) |
| SEL-GROUP-ICON-001 | LEFT-ORBAT-001 | missing | new — P-6 | **Was `partial \| T-071`.** T-071 shipped and this did not change: squad rows are non-interactive `<div>`s (`eden_chrome.rs:1578-1611` — one branch has only a refile `on:pointerup`, the other no handler). No selectable group glyph on the map either |
| SEL-LAYER-CHILDREN-001 | — | missing | new — P-6 | Folder click sets the **drop target**, not a selection (`eden_chrome.rs:1624-1627` → `editor_ops.rs:1025`; `title="Make this the drop target"`) |
| SEL-LAYER-DESC-001 | — | missing | new — P-6 | Same handler; no descendant walk |
| LAYER-CREATE-001 | LEFT-LAYER-005 | missing | new — P-6 | **Was `tbd_only`** — a claim about *semantics* standing in for a claim about *existence*. Editor Layers ≠ Eden layers is still true, and there is **no create control at all**: `store.rs:1872` `add_editor_layer`'s only SPA caller is the auto-seed `editor_ops.rs:1136`; DockLeft's five footer buttons are `disabled=true` |
| LAYER-DEL-001 | LEFT-LAYER-007 | missing | new — P-6 | **Was `partial`.** `store.rs:1527` `remove_editor_layer` has **zero** SPA callers. `Delete` removes selected slots only (`editor_ops.rs:342`) |
| ATTR-OPEN-001 | ATTR-OPEN-001 | partial | T-647 ✅ · P-7 residual | Opens from map dblclick (`mission_editor.rs:4777-4842` → `pick_slot_or_vehicle` → `open_attributes`) and outliner slot dblclick (`eden_tree.rs:858-862`). **Was:** multi-selection suppressed the modal — **T-649 inverted that** (`open_attrs_modal` `editor_ops.rs:1057-1098` / `:1075`): a multi-selection now OPENS multi-edit. Residual: objects/zones/markers still do not open Attributes; outliner activate is slot-row only |
| ATTR-MULTI-001 | ATTR-MULTI-001 | match | T-649 ✅ | **Was `missing` / actively refused.** T-649 inverted the suppress-on-multi guard: `open_attrs_modal` (`editor_ops.rs:1057-1098`) opens multi-edit over the whole selection; Identity/Transform fan-out via `attrs_update_slot_multi` / `attrs_update_position_multi` (`:2304+`, `:1447`) |
| ATTR-MULTI-CHK-001 | ATTR-MULTI-CHK-001 | match | T-649 ✅ | **Was `missing`.** Per-field "Multiple values" opt-in checkboxes in `attributes.rs` (banner `:359`, checkbox `:178`); disagreement via `AttrDiff` / `attrs_multi_ids` (`editor_ops.rs:2368`, `:2408`) |
| CTX-FORMATION-001 | — | missing | T-079 (d) + P-3 | No context menu, no formation action. The single `formation` hit is a comment (`editor_ops.rs:1324`) describing an `APPLY_ANCHOR_X + 15.0 * i` line-up — placement spacing, not a formation |

## Keyboard, actions & status — KEY (4)

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| KEY-WP-001 | — | missing | T-079 (b) · wb + P-3 | Shift+RMB quick waypoint. RMB is the pan button (`mission_editor.rs:1402`) and no `shift_key` is read in `onpointerdown`. **Collision 4** |
| KEY-WIDGET-001 | KEY-SPACE-CENTER-001 | missing | **T-075** | **Collision 1** — `Space` = centre on selection (`mission_editor.rs:1026`). Eden's own `1`–`5` direct widget keys are all free in TBD, which dissolves the clash |
| KEY-GRID-001 | — | missing | new — P-8 | No snapping grid at all — `\bsnap\b` is 33 hits, **every one** a `snapshot`/`snap` local. `interactions.md:522` records the key as `` ` ; ` `` while the screenshots show `odiaeresis`: the **same binding on two layouts**, neither portable. Pick a TBD key, copy neither |
| KEY-HIDE-UI-001 | — | missing | new — P-9 | **Collision 3, and dangerous** — `Backspace` = delete selection (`mission_editor.rs:1027`), so an Eden author reaching to hide chrome for a screenshot deletes their selection. Dropping the alias is one line (`Delete` already covers it) |

## Keyboard, actions & status — ACTION (10)

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| ACTION-COPY-001 | ACTION-COPY-001 | match | T-056 ✅ | `Ctrl/Cmd+C`, Alt+Shift disqualify (`mission_editor.rs:1020`) → `editor_ops.rs:394` |
| ACTION-CUT-001 | — | missing | new — P-10 | `"KeyX"` → **0** hits. Trivially `copy_selection() && delete_selection()` |
| ACTION-PASTE-001 | ACTION-PASTE-001 | match | T-056 ✅ · T-743 ✅ | `Ctrl/Cmd+V` → `editor_ops.rs` `paste_at_cursor`; centroid → cursor, terrain-clamped, one undo step. T-743: with the pointer off the map (over any chrome panel) the arm anchors on the **centre of the visible map** — it resolves the fallback itself rather than passing no anchor, which now means something else |
| ACTION-PASTE-ORIG-001 | ACTION-PASTE-ORIG-001 | match | T-669 ✅ · T-743 ✅ | `Ctrl/Cmd+Shift+V` → `paste_at_cursor(None, None)`. **Corrected by T-743 — the previous note here was false as written.** It claimed the no-anchor primitive *is* paste-at-original; in fact `paste_slots`' no-anchor arm added a fixed 20 m (`PASTE_NUDGE`, byte-parity with `ydoc.pasteSlots`) to both axes, so the command landed every slot 20 m off its source. Operator decision 2026-08-08: the JS parity was a migration safety net, not a contract. The nudge is deleted and the paste lands **exactly** on the source coordinates |
| ACTION-LEVEL-001 | — | na | — | `LevelWithSurface` needs pitch **and** roll; the 2D doc stores yaw only and the mod re-grounds every spawn. No orientation to level |
| ACTION-SNAP-001 | — | na | — | As XFORM-SNAP-001 — snapping is unconditional mod-side, so there is no action to invoke |
| ACTION-SEAT-001 | — | missing | T-076 | Needs the crew model |
| ACTION-FORM-001 | — | missing | T-079 (d) | See CTX-FORMATION-001 |
| ACTION-TOGGLE-SEL-001 | SEL-MOD-001 | match | T-053 ✅ | `ToggleUnitSel` **is** TBD's Ctrl+LMB semantics (`select_tool.rs:166-173`). Duplicate of SEL-MOD-001 in Eden's own id space |
| ACTION-WP-QUICK-001 | — | missing | T-079 (b) · wb | Duplicate of KEY-WP-001 (same Shift+RMB, same MOVE waypoint). Counted once for tickets |

## Keyboard, actions & status — STATUS (7)

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| STATUS-X-001 | BOTTOM-CUR-X | match | T-049 / T-050 ✅ | `eden_chrome.rs:3727-3730`, fed by frozen-cam unproject (`mission_editor.rs:1461-1487`). Swaps to `SEL` X on a single selection |
| STATUS-Y-001 | BOTTOM-CUR-Y | match | T-049 / T-050 ✅ | `eden_chrome.rs:3731-3734` |
| STATUS-Z-001 | BOTTOM-CUR-Z | match | T-091.2 ✅ | `eden_chrome.rs:3735-3738`; DEM-sampled (`mission_editor.rs:1480-1484`), em-dash outside coverage |
| STATUS-ZOOM-001 | — | missing | new — P-11 | No scale readout (`eden_chrome.rs:3706-3767` is CUR/SEL X/Y/Z + OBJ/SEL/SZ). Worth more than it looks — Eden's printed `m/pix` is what let `batch08` derive the contour ladder, and `zoom()` is already exposed (`select_tool.rs:518`) |
| STATUS-VER-001 | — | na | — | "Game version" is the running Arma build; a browser editor has no attached game build. *(A schema/mod-version chip would be a `tbd_only` addition, not this id.)* |
| STATUS-MOD-001 | — | na | — | "Mods loaded" is a running client's local addon set. TBD's equivalent is the platform Modpacks page |
| STATUS-SRV-001 | — | na | — | Eden reports the MP editing server. TBD's editor is single-author (IndexedDB + immutable server versions); server state lives on Server Intel / Server Control |

---

# Part 2 — Attribute parity (93 ids from `attributes.md`)

Attribute rows carry **`build_class`**. `wb` in the ticket column = `executor: workbench`.

## Object — OBJ (31)

| eden_id | tbd_id | parity | build_class | ticket | gap_notes |
|---------|--------|--------|:---:|----------|-----------|
| ATTR-FIELD-OBJ-TYPE | ATTR-TAB-002 | partial | a | T-082 | `assetId` is authored on palette drop (`store.rs:551`) and mutable (`:1276`), but `read_attrs` never reads it (`editor_ops.rs:122-132`) so the type cannot be changed in the modal. Compiles to `kit:` — 342 of 354 characters take a faction default (`flatten.rs:411,479`) |
| ATTR-FIELD-OBJ-VARNAME | — | na | d | — | Scripting handle; TBD has no author-facing script layer. `slot.uid` already gives durable identity |
| ATTR-FIELD-OBJ-INIT | — | na | d | — | SQF string eval'd at spawn. TBD compiles a declarative JSON document; author-supplied code has no evaluator and would be a new attack surface |
| ATTR-FIELD-OBJ-POSITION | ATTR-TAB-001 | match | a | T-049 ✅ | X/Y/Z editable (`attributes.rs:255-319`) → `store.rs:1345-1355`; compiled `x`/`z`/`y`, all three read by `TBD_MissionSlotStruct.c:65-67`. Editing X or Y resets Z to 0.0 (`store.rs:1354-1358`) — a **deliberate terrain-follow**, not a defect |
| ATTR-FIELD-OBJ-ROTATION | ATTR-TAB-001 | match | a | T-049 ✅ | Number field normalised `[0,360)` → compiled `headingDeg`, read `TBD_MissionSlotStruct.c:68`. Vehicles have their own `Heading°` input (`eden_chrome.rs:2088-2104`) |
| ATTR-FIELD-OBJ-SIZE | — | missing | c | new — N7 · wb | No size/scale key on `$defs/slot` or `$defs/entity` (both closed); `entities[]` has no consumer on any shipped build |
| ATTR-FIELD-OBJ-SHAPE | — | missing | b | new — N5 · wb | Eden pairs `IsRectangle` with `placementRadius` to shape a random-spawn area. `$defs/shape` is zone-only; `$defs/slot` closed |
| ATTR-FIELD-OBJ-PLACEMENT-RADIUS | — | missing | b | new — N5 · wb | Same pair. A 2D top-down editor is the *natural* surface — draw the scatter circle |
| ATTR-FIELD-OBJ-PLAYER-SP | — | na | d | — | Single-player playable flag; TBD is MP-only milsim |
| ATTR-FIELD-OBJ-PLAYABLE-MP | — | na | d | — | Every TBD slot is a roster seat by construction; `TBD_SpawnManager.c:963,1166` spawns each body **AI-disabled**. No playable/AI distinction to author |
| ATTR-FIELD-OBJ-ROLE-DESC | ATTR-TAB-002 | partial | a | T-082 | `role` (`attributes.rs:336`) doubles as label and description and does reach the game (`TBD_MissionSlotStruct.c:63`). No separate free-text description |
| ATTR-FIELD-OBJ-LOCK | — | missing | c | new — N6 · wb | Vehicle lock state. `lock` word-boundary in `apps/mod` = 4, none an authored vehicle lock. T-215 shipped placement + cargo, not lock |
| ATTR-FIELD-OBJ-SKILL | ATTR-TAB-003 | **na** | d | — | **Was `missing` — correction 1.** AI skill with no AI to skill: `TBD_SpawnManager.c:963` spawns AI-disabled and `skill` word-boundary in `apps/mod` = **0**. A closed question unless TBD adds AI units, which is a product decision far larger than an attribute |
| ATTR-FIELD-OBJ-HEALTH | — | missing | c | new — N7 · wb | No authored initial health. `health` word-boundary in `apps/mod` = **0**; the 95 `damage` hits are the runtime damage system |
| ATTR-FIELD-OBJ-FUEL | — | missing | c | new — N6 · wb | `fuel` word-boundary in `apps/mod` = 1. No key on `$defs/entity` (closed) and no reader |
| ATTR-FIELD-OBJ-AMMO | — | missing | c | new — N6 · wb | Turret ammo as an *attribute*. `slot.loadout.cargo` / `$defs/entityInventory` cover carried items, not a turret count |
| ATTR-FIELD-OBJ-RANK | ORBAT Manager | **partial** | b | new — N4 · wb | **Correction 4 — authored and dropped.** Input `orbat_manager.rs:1336-1355` → `store.rs:1301` → dropped at compile (T-216 ledger `flatten.rs:2584-2649`). `$defs/slot` closed; `TBD_MissionSlotStruct.c:59-69` has no field. Related: T-242 |
| ATTR-FIELD-OBJ-STANCE | ATTR-TAB-001 | **partial** | c | new — N4 · wb | **Correction 4 — authored and dropped.** `<select>` stand/crouch/prone `attributes.rs:297-312` → `store.rs:1246` → dropped. `stance` word-boundary in `apps/mod` = **0** — needs a schema key *and* an Enfusion spawn-pose call |
| ATTR-FIELD-OBJ-DYN-SIM | — | na | d | — | A3 dynamic simulation. `simulation` word-boundary in `apps/mod` = **0**; no Enfusion equivalent |
| ATTR-FIELD-OBJ-WAKE-DYN-SIM | — | na | d | — | Same system |
| ATTR-FIELD-OBJ-ENABLE-SIM | — | na | d | — | Same system |
| ATTR-FIELD-OBJ-SIMPLE-OBJ | — | na | d | — | A3 render-optimisation concept (`objectIsSimple`); no Enfusion analogue |
| ATTR-FIELD-OBJ-SHOW-MODEL | — | missing | c | new — N7 · wb | `hideObject`. No per-entity hide; `entities[]` unconsumed. Distinct from the editor's own layer visibility (`LYR-ENABLE-VIS`) |
| ATTR-FIELD-OBJ-ALLOW-DAMAGE | — | missing | c | new — N7 · wb | Per-entity invulnerability. No key, no reader |
| ATTR-FIELD-OBJ-STAMINA | — | missing | c | new — N7 · wb | No per-slot stamina key or reader. **`UNKNOWN:`** whether Reforger exposes a per-character stamina toggle at all — a Workbench API check, not a code search |
| ATTR-FIELD-OBJ-REVIVE | — | na | d | — | **Refused by design.** `SETTINGS_UNREAD_NOTE` (`eden_chrome.rs:370`), rationale at `:4672-4685` — *"TBD events are one life"*. `$defs/settings` is mission-global |
| ATTR-FIELD-OBJ-DOORS | — | na | d | — | Per-model door states, driven in Eden by 3D gestures on the door. No 2D representation and no Enfusion authoring path |
| ATTR-FIELD-OBJ-LOCAL-ONLY | — | na | d | — | A3 MP locality flag; Enfusion replication is engine-managed |
| ATTR-FIELD-OBJ-UNIT-NAME | — | missing | b | new — N4 · wb | No per-slot display name. `slot.id` is **derived** each compile (`{faction}:{groupCallsign}:{role}:{index}`) and shifts under renames — not a name field. `$defs/slot` closed. *(The sweep's family table files this under T-082; its own §6 and `owns_parity.md` §3 route it to the T-216 follow-on, which is the build-class-correct home.)* |
| ATTR-FIELD-OBJ-FACE | — | na | d | — | Character face/identity — a 3D appearance value with no 2D surface and no mod reader |
| ATTR-FIELD-OBJ-CALLSIGN | ORBAT Manager | **partial** | b | new — N4 · wb | **Correction 4 — authored and dropped.** Input `orbat_manager.rs:1314-1333` → `store.rs:1293` → dropped (T-216). `PASTE_KNOWN_SLOT_KEYS` (`store.rs:2571-2582`) also omits it, so it survives paste only via the extras branch |

> **TBD-only, no Eden id:** slot `tag` (`MED · ENG · SL…`, `attributes.rs:337`) and a squad's
> `leaderSlotId`. Both are authored and **dropped by the same T-216 ledger**; they belong in the N4
> follow-on with `callsign` / `rank` / `stance` even though this catalogue has no id for them.

## Comment — CMT (3)

| eden_id | tbd_id | parity | build_class | ticket | gap_notes |
|---------|--------|--------|:---:|----------|-----------|
| ATTR-FIELD-CMT-TITLE | — | missing | a | new — N2 | Editor-only annotation, **never compiled** (`attributes.md:79`) → the editor payload carries it with no contract change. No entity, no tool, no registry ticket. The mission-detail "Comments" Sheet (`missions.rs:2501-2513`) is a social thread, not a canvas annotation |
| ATTR-FIELD-CMT-TOOLTIP | — | missing | a | new — N2 | Same |
| ATTR-FIELD-CMT-POSITION | — | missing | a | new — N2 | Same. `grep -in comment mission.schema.json` → 0 hits, as expected for editor-only state |

## Group — GRP (10)

| eden_id | tbd_id | parity | build_class | ticket | gap_notes |
|---------|--------|--------|:---:|----------|-----------|
| ATTR-FIELD-GRP-VARNAME | — | na | d | — | Scripting handle; no script layer |
| ATTR-FIELD-GRP-INIT | — | na | d | — | SQF string; no evaluator |
| ATTR-FIELD-GRP-CALLSIGN | LEFT-ORBAT-001 | match | a | T-180 ✅ | Squad callsign → compiled `$defs/group.callsign`, copied verbatim into `slot.groupCallsign`, read by `TBD_MissionSlotStruct.c:62`. One of only two ORBAT identity values that survives the compile |
| ATTR-FIELD-GRP-PLACEMENT-RADIUS | — | missing | b | new — N5 · wb | `$defs/group` is `callsign`/`type`/`roles`, closed. Natural 2D control |
| ATTR-FIELD-GRP-COMBAT-MODE | — | missing | c | T-079 · wb | `combatMode` word-boundary = **0** in both trees. Presupposes AI groups, which do not exist |
| ATTR-FIELD-GRP-BEHAVIOUR | — | missing | c | T-079 · wb | `behaviour`/`behavior` as a field = **0** in both trees (the 44 + 13 hits are doc prose). No AI subject |
| ATTR-FIELD-GRP-FORMATION | — | missing | c | T-079 · wb | `formation` = 1 frontend hit (prose, `editor_ops.rs:1324`), **0** in `apps/mod` |
| ATTR-FIELD-GRP-SPEED-MODE | — | missing | c | T-079 · wb | `speedMode` = **0** both trees |
| ATTR-FIELD-GRP-DYN-SIM | — | na | d | — | Same A3 system as `OBJ-DYN-SIM` |
| ATTR-FIELD-GRP-DELETE-EMPTY | — | na | d | — | Garbage-collects an emptied AI group; no AI groups to collect |

## Marker — MRK (10)

Every row `missing` because **the producer does not exist**: tab button `eden_chrome.rs:3046`, no
match arm, body one sentence at `:3337`, pinned by a test at `:4499-4502`. `PaletteKind` (`:1833`)
has no marker variant. The two doc mutators `set_faction_briefing_marker` / `remove_…`
(`store.rs:2030`, `:2070`) have **zero product callers**.

| eden_id | tbd_id | parity | build_class | ticket | gap_notes |
|---------|--------|--------|:---:|----------|-----------|
| ATTR-FIELD-MRK-TYPE | RIGHT-STUB-002 | missing | a | T-069 / T-213 | `$defs/marker.icon` already exists — a **closed 64-alias enum**. Schema is ready; the authoring UI is not |
| ATTR-FIELD-MRK-VARNAME | — | na | d | — | Scripting handle |
| ATTR-FIELD-MRK-TEXT | RIGHT-STUB-002 | missing | a | T-069 / T-213 | `$defs/marker.label` exists and is `required` |
| ATTR-FIELD-MRK-POSITION | RIGHT-STUB-002 | missing | a | T-069 / T-213 | `$defs/marker.x`/`.z` exist and are `required`. The 2D map is the ideal surface |
| ATTR-FIELD-MRK-SIZE | — | missing | b | new — G2 · wb | `$defs/marker` is exactly `{x,z,icon,label}` and closed. Eden's Area marker has no representation |
| ATTR-FIELD-MRK-ROTATION | — | missing | b | new — G2 · wb | Same closed def |
| ATTR-FIELD-MRK-SHAPE | — | missing | b | new — G2 · wb | Icon-vs-Area is the whole second half of Eden's marker model; absent |
| ATTR-FIELD-MRK-BRUSH | — | missing | b | new — G2 · wb | Area fill pattern; presupposes Area markers |
| ATTR-FIELD-MRK-COLOR | — | missing | b | new — G2 · wb | Closed def. Colour is partly implied already — `TBD_MarkerService.BuildForPlayer` (`TBD_MarkerData.c:82`) is **side-scoped** |
| ATTR-FIELD-MRK-ALPHA | — | missing | b | new — G2 · wb | Closed def |

> **Do not fold G2 into T-069/T-213.** The marker *core* (`{x,z,icon,label}`) needs **no schema
> edit** and is the one absent-entity family that is factory-dispatchable today; the six style
> fields are a `$defs/marker` widening. Merging them converts a factory ticket into a workbench one.

## Layer — LYR (3)

Eden layers and TBD "Editor Layers" are different concepts, but all three Eden fields have a direct
TBD meaning.

| eden_id | tbd_id | parity | build_class | ticket | gap_notes |
|---------|--------|--------|:---:|----------|-----------|
| ATTR-FIELD-LYR-NAME | LEFT-LAYER-005 | **missing** | a | new — P-6 | **Was `match` — correction 3.** T-037 shipped the core mutator and **nothing reaches it**: `rename_editor_layer` (`store.rs:1886`) has exactly one mention repo-wide, its own definition. `reparent_editor_layer` (`:1895`), `remove_editor_layer` (`:1527`) and `move_slot_to_layer` (`:1915`) likewise have no UI callers. Pure UI wiring onto shipped, tested core |
| ATTR-FIELD-LYR-ENABLE-XFORM | — | missing | a | new — N3 | No per-layer transform lock. Editor-only state — the editor payload is open, so no contract change |
| ATTR-FIELD-LYR-ENABLE-VIS | — | missing | a | new — N3 | **No layer visibility toggle exists.** Not to be confused with the 12 **world-layer** checkboxes (`world_layer_prefs.rs:61-76`), which are localStorage basemap prefs on a different object |

## Scenario — SCN (11)

Most absences here are **enforced, not overlooked**: every environment write goes through
`author_env` (`eden_chrome.rs:271`), which refuses any key outside `CARRIED_ENV_KEYS` (5, pinned by
a test asserting `len() == 5` at `:4648-4652`) or `AUTHORED_FLOW_KEYS` (4, pinned `:4700-4711`).

| eden_id | tbd_id | parity | build_class | ticket | gap_notes |
|---------|--------|--------|:---:|----------|-----------|
| ATTR-FIELD-SCN-TITLE | TOP-TITLE-001 | match | a | T-049 ✅ | Top-strip input (`eden_chrome.rs:1047-1063`) → `meta.title` → compiled (`mission/compile.rs:251`). Caveat: `RowMirror` PATCHes only `time_of_day` + `weather` (`:501-508`), so the editor title never reaches the library row |
| ATTR-FIELD-SCN-AUTHOR | — | na | d | — | Server-assigned from the authenticated account; `ModMeta.author` (`flatten.rs:263`) already reaches the game. A hand-typed author field would be a spoofing surface, not a gap |
| ATTR-FIELD-SCN-PICTURE | — | missing | a | new — N1 | DTO and API already exist (`dto.rs:850`, `handlers/missions.rs:658`); **no editor control and no frontend caller.** No contract change needed |
| ATTR-FIELD-SCN-OVERVIEW-TEXT | — | missing | a | new — N1 | **The clearest hole in the sweep.** `PATCH /missions/:id` accepts `briefing` (`handlers/missions.rs:657`) and `meta.briefing` reaches the doc via hydrate (`store.rs:1658-1663`) — and **nothing in the SPA can edit it**, on the create dialog or in the editor |
| ATTR-FIELD-SCN-DLC | — | na | d | — | Reforger declares workshop dependencies in the mod manifest, not per mission |
| ATTR-FIELD-SCN-REQUIRE-DLC | — | na | d | — | Same |
| ATTR-FIELD-SCN-TIME | TOP-SETTINGS-001 | match | a | shipped (pre-T-049 chrome) | `<input type="time">` (`eden_chrome.rs:3850-3868`) → carried key `time` → `environment.dateTime` → `ModEnvironment.date_time` (`flatten.rs:270`). Two surfaces, both mirror to the row |
| ATTR-FIELD-SCN-WEATHER | TOP-SETTINGS-001 | match | a | shipped (pre-T-049 chrome) | `<select>` of 4 presets (`:3877-3895`) → `weatherPreset` → `ModEnvironment.weather_preset` (`:272`) |
| ATTR-FIELD-SCN-FOG | — | missing | c | new — N8 · wb | Not independent — only the `dense_fog` preset. Refused by `keys_nothing_reads_are_not_authored` (`eden_chrome.rs:4622-4629`). Blocked on a **reader**, not on the schema |
| ATTR-FIELD-SCN-WIND | — | missing | c | new — N8 · wb | The sharpest case of *"a schema field is not a reader"*: `$defs/environment.windDirDeg` **is declared**, and `ModEnvironment` (`flatten.rs:268-275`) does not even serialise it |
| ATTR-FIELD-SCN-VIEW-DIST | ENV-SETTINGS-002 | missing | c | new — N8 · wb (+ N9 cleanup) | **Correction 2 — was `partial`.** Built, then **removed by T-193** (`b30f5490`); refused at `eden_chrome.rs:4624`. Residual dead DTO fields `dto.rs:991-992` are still parsed at `editor_ops.rs:205-212` |

## Trigger — TRG (13)

All absent. `mission.schema.json` has 2 `trigger` hits, both prose inside `description` strings;
none of the 30 `$defs` is a trigger. **Nearest analogue, and it is not a trigger:** `$defs/zone` +
`$defs/zoneRules` (16 keys), shipped as a draw tool at T-582. `INFERRED:` zones give a typed area
with declarative rules but no activation/condition/timer/effects model.

| eden_id | tbd_id | parity | build_class | ticket | gap_notes |
|---------|--------|--------|:---:|----------|-----------|
| ATTR-FIELD-TRG-VARNAME | RIGHT-STUB-003 | na | d | — | Scripting handle |
| ATTR-FIELD-TRG-TEXT | RIGHT-STUB-003 | missing | b | T-079 (a) · wb | `$defs/zone.label` is the shape of the answer; no trigger def exists to hang it on |
| ATTR-FIELD-TRG-POSITION | RIGHT-STUB-003 | missing | b | T-079 (a) · wb | Geometry is the solved half — `$defs/shape` (circle\|polygon) + the T-582 draw tool |
| ATTR-FIELD-TRG-ROTATION | RIGHT-STUB-003 | missing | b | T-079 (a) · wb | Same |
| ATTR-FIELD-TRG-SIZE | RIGHT-STUB-003 | missing | b | T-079 (a) · wb | Same |
| ATTR-FIELD-TRG-SHAPE | RIGHT-STUB-003 | missing | b | T-079 (a) · wb | `$defs/shape` has no rectangle; Eden's `IsRectangle` has no home |
| ATTR-FIELD-TRG-TYPE | RIGHT-STUB-003 | missing | c | T-079 (a) · wb | Trigger type (None/Guarded/Switch/…) presupposes an Enfusion runtime trigger system |
| ATTR-FIELD-TRG-ACTIVATION | RIGHT-STUB-003 | missing | c | T-079 (a) · wb | Activating side/party. No runtime |
| ATTR-FIELD-TRG-ACTIVATION-TYPE | RIGHT-STUB-003 | missing | c | T-079 (a) · wb | Present / Not Present / Detected By. No runtime |
| ATTR-FIELD-TRG-CONDITION | RIGHT-STUB-003 | missing | c | T-079 (a) · wb | Eden's is an SQF expression. TBD's answer must be a **structured condition model**, not an eval'd string — an evaluator is out of scope for the same reason `OBJ-INIT` is `na` |
| ATTR-FIELD-TRG-ON-ACTIVATION | RIGHT-STUB-003 | missing | c | T-079 (a) · wb | Same: a structured effect list, not code. `$defs/winConditions` is the only effect vocabulary today and it is mission-global |
| ATTR-FIELD-TRG-REPEATABLE | RIGHT-STUB-003 | missing | c | T-079 (a) · wb | No runtime |
| ATTR-FIELD-TRG-TIMER | RIGHT-STUB-003 | missing | c | T-079 (a) · wb | No runtime |

## Waypoint — WP (9)

All absent, and more deeply than the id list suggests: `grep -rin waypoint` across the SPA,
`crates/` and the schema returns **exactly 2 hits** — the strings `"waypoint"`/`"waypoint2"` inside
the marker **icon alias enum**. Glyph names, not entities.

**The scoping fact for T-079:** a waypoint orders an AI group, and `TBD_SpawnManager.c:963,1166`
spawns every slot body with **AI disabled**. Waypoints today would have no subject.

| eden_id | tbd_id | parity | build_class | ticket | gap_notes |
|---------|--------|--------|:---:|----------|-----------|
| ATTR-FIELD-WP-TYPE | — | missing | c | T-079 (b) · wb | MOVE/SAD/GUARD/… — each is an AI behaviour to implement, not a field to store |
| ATTR-FIELD-WP-DESCRIPTION | — | missing | b | T-079 (b) · wb | A string on a waypoint entity; no waypoint `$def` exists |
| ATTR-FIELD-WP-ORDER | — | missing | b | T-079 (b) · wb | Sequence index; storage-only once the entity exists |
| ATTR-FIELD-WP-POSITION | — | missing | b | T-079 (b) · wb | 2D map is the ideal surface; needs the entity |
| ATTR-FIELD-WP-COMBAT-MODE | — | missing | c | T-079 (b) · wb | Per-waypoint AI override; `combatMode` = 0 hits anywhere |
| ATTR-FIELD-WP-BEHAVIOUR | — | missing | c | T-079 (b) · wb | 0 hits as a field |
| ATTR-FIELD-WP-FORMATION | — | missing | c | T-079 (b) · wb | 0 hits in `apps/mod` |
| ATTR-FIELD-WP-SPEED | — | missing | c | T-079 (b) · wb | 0 hits as a field |
| ATTR-FIELD-WP-CONDITION | — | missing | c | T-079 (b) · wb | Completion condition — same structured-model argument as `TRG-CONDITION`. Eden also allows completion via connected triggers, so this is coupled to the trigger slice |

## Composition metadata — COMP (3)

Not entity attributes — save-dialog metadata. Zero hits repo-wide for `save_composition` /
`user_composition` / `custom_composition`. The only frontend `composition` occurrences are
`asset_catalog.rs:345-844`, a **path-string classifier** that prefixes `comp:` when an Arma resource
path contains `"Composition"` — it **consumes** Bohemia's shipped prefabs; it does not author one.

| eden_id | tbd_id | parity | build_class | ticket | gap_notes |
|---------|--------|--------|:---:|----------|-----------|
| ATTR-FIELD-COMP-TITLE | — | missing | a | T-078 | Composition metadata is a user artifact, never part of the compiled mission → no contract blocker. Needs a table + API + SPA, all factory-side |
| ATTR-FIELD-COMP-AUTHOR | — | missing | a | T-078 | Same. Should be server-assigned like `SCN-AUTHOR`, not typed |
| ATTR-FIELD-COMP-CATEGORY | — | missing | a | T-078 | Same |

---

# Part 3 — Shell & data (rows outside the two catalogues, 15)

These come from [`feature_inventory.md`](../feature_inventory.md) or are TBD-only surfaces. They
have **no `eden/` catalogue id** and are preserved from the previous version of this table so
nothing is lost. The four `TOOLBAR-*` ids below were **minted by that version**, not by
`interactions.md` — which literalises only `TOOLBAR-NEW-001` and `TOOLBAR-TUTORIAL-001`. Treat them
as local ids until the toolbar gets a real catalogue pass.

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| TOOLBAR-INTEL-001 *(minted)* | TOP-SETTINGS-001 | partial | — | Scenario attrs partial — see the `SCN-*` family for the per-field census |
| TOOLBAR-MAP-001 *(minted)* | MAP-VIEW-001 | partial | — | TBD is 2D-only always; Eden's `M` toggles 3D ⇄ 2D |
| TOOLBAR-GRID-MOVE-001 *(minted)* | — | missing | new — P-8 | Snap grid. Same subject as `KEY-GRID-001`; **one slice, two ids** |
| TOOLBAR-UNDO-001 *(minted)* | TOP-UNDO-001 | match | T-052 ✅ | Cmd/Ctrl+Z / Shift+Z / Ctrl+Y keyboard + toolbar buttons |
| Eden:ATTR-ARSENAL-001 | ATTR-TAB-004 | partial | T-068.4 ✅ | Dumb loadout export (4 dropdowns + JSON download); smart Forge is T-068.10 |
| SEL-ORBAT-DBL-001 | SEL-ORBAT-DBL-001 | match | T-054 ✅ | ORBAT slot row dbl-click → Attributes (`orbat_manager.rs:1190`). Inherits `ATTR-OPEN-001`'s residual (non-slot kinds do not open); multi-select opens multi-edit (T-649), not suppression |
| MAP-TERRAIN-001 | MAP-TERRAIN-001 | match | T-049 ✅ | `meta.terrain` → viewport (key-remount, Everon/Arland bounds) |
| ENV-SETTINGS-002 | TOP-SETTINGS-001 | **missing** | new — N8 · wb (+ N9) | **Correction 2 — was `partial`, "Thermals + view dist in dialog". Neither is in the dialog.** T-193 (`b30f5490`) removed both; `eden_chrome.rs:4622-4629` now asserts they are *not* authorable. Same subject as `ATTR-FIELD-SCN-VIEW-DIST`. `feature_inventory.md:1732` still records Status `working` and is stale for the same reason |
| DATA-HYD-TITLE-001 | TOP-TITLE-001 | match | T-049 ✅ | `applyMissionRowMeta` hydrates title/terrain/env on load (no PATCH-back) |
| SEL-MAP-003 | SEL-MAP-003 | match | — | Marquee |
| XFORM-DEL-001 | XFORM-DEL-001 | match | — | Delete. **Known lead, not a finding:** `delete_selection` calls `core.remove_slots(ids)` only, while the selection can legitimately hold vehicle ids — `INFERRED:` `Delete` with a vehicle selected removes nothing for that vehicle. Not verified in-browser (`interactions_sweep.md` §6.5) |
| TOP-SAVE-001 | TOP-SAVE-001 | partial | — | Semver versions vs Eden save |
| TOP-EXPORT-001 | TOP-EXPORT-001 | match | — |  |
| — | TBD-LAYER-001 | tbd_only | — | Workflow folders |
| — | TBD-CONFLICT-001 | tbd_only | — | IndexedDB conflict |

> **Other TBD-only surfaces with no Eden id** (named here rather than given fake rows): zones /
> play areas (T-582), the ORBAT Manager modal (T-071/T-180), semver mission versions, the compiled
> mission export, and the SZ payload estimate in the toolbelt.

---

## Summary

### By parity — all 191 rows

| Source | match | partial | missing | deferred | na | tbd_only | total |
|---|---:|---:|---:|---:|---:|---:|---:|
| Interactions (83) | 11 | 8 | 51 | 4 | 9 | 0 | **83** |
| Attributes (93) | 6 | 5 | 60 | 0 | 22 | 0 | **93** |
| Shell & data (15) | 7 | 4 | 2 | 0 | 0 | 2 | **15** |
| **total** | **24** | **17** | **113** | **4** | **31** | **2** | **191** |

Rows and columns both sum to 191. The old table's counts (5 `match` for "core map edit", "10+"
`missing` for attributes, and so on) were estimates over a sample and are **not** carried forward.

`tbd_only` is 0 across both catalogues **by construction** — a sweep of Eden's id space has no id
under which to file a TBD addition. The TBD-only surfaces are listed in Part 3.

### Interactions by domain (83)

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
| SEL / LAYER / ATTR / CTX | 12 | 2 | 1 | 9 | 0 | 0 |
| KEY | 4 | 0 | 0 | 4 | 0 | 0 |
| ACTION | 10 | 3 | 0 | 5 | 0 | 2 |
| STATUS | 7 | 3 | 0 | 1 | 0 | 3 |
| **total** | **83** | **11** | **8** | **51** | **4** | **9** |

### Attributes by build class — the headline (93)

| class | count | share | program |
|---|---:|---:|---|
| **(a)** SPA-buildable today | **22** | 23.7 % | factory, `executor: claude-code` |
| **(b)** schema-blocked | **20** | 21.5 % | `executor: workbench` |
| **(c)** mod-blocked | **29** | 31.2 % | `executor: workbench` |
| **(d)** `na` | **22** | 23.7 % | closed |
| | **93** | | |

**Read this as: 22 factory · 49 workbench · 22 closed.** Of the 22 factory ids **six are already
shipped**, so the genuinely new factory attribute work is **16 ids**, concentrated in five small
slices (markers core, mission presentation, comments, layer flags + layer rename, composition
metadata).

### Attributes — parity × build class

| | (a) | (b) | (c) | (d) | total |
|---|---:|---:|---:|---:|---:|
| match | 6 | 0 | 0 | 0 | **6** |
| partial | 2 | 2 | 1 | 0 | **5** |
| missing | 14 | 18 | 28 | 0 | **60** |
| na | 0 | 0 | 0 | 22 | **22** |
| **total** | **22** | **20** | **29** | **22** | **93** |

`match` (6): `OBJ-POSITION`, `OBJ-ROTATION`, `GRP-CALLSIGN`, `SCN-TITLE`, `SCN-TIME`, `SCN-WEATHER`.
`partial` (5): `OBJ-TYPE`, `OBJ-ROLE-DESC`, and the three T-216 casualties `OBJ-RANK`,
`OBJ-CALLSIGN`, `OBJ-STANCE`.
**No id was upgraded to `match` on editor evidence alone** — three that a modal-only survey would
have scored `match` are `partial` because of the drop ledger, and `LYR-NAME` is `missing` because
its mutator is unreachable.

### Attributes by family (93)

| family | n | (a) | (b) | (c) | (d) |
|---|---:|---:|---:|---:|---:|
| OBJ | 31 | 4 | 5 | 9 | 13 |
| TRG | 13 | 0 | 5 | 7 | 1 |
| SCN | 11 | 5 | 0 | 3 | 3 |
| GRP | 10 | 1 | 1 | 4 | 4 |
| MRK | 10 | 3 | 6 | 0 | 1 |
| WP | 9 | 0 | 3 | 6 | 0 |
| CMT | 3 | 3 | 0 | 0 | 0 |
| LYR | 3 | 3 | 0 | 0 | 0 |
| COMP | 3 | 3 | 0 | 0 | 0 |

**OBJ and TRG carry two thirds of the workbench load.** CMT / LYR / COMP are pure factory because
Eden itself defines them as editor-only. MRK splits cleanly — the four schema-carried fields are
factory, the six style fields are one schema widening.

### Disposition — interaction ids (83)

| disposition | ids |
|---|---:|
| already shipped (`match`) | 11 |
| genuinely `na` for a 2D web editor | 9 |
| absorbed by an **existing open** ticket | 41 |
| needs a **new** slice (P-1…P-11) | 16 |
| shipped-partial or deferred, no ticket needed | 6 |
| **total** | **83** |

The 6 needing no owner: `RIGHT-SUBMODE-001` (shipped T-180.5), `PLACE-005` / `WIDGET-AREA-001` /
`WIDGET-AREA-SCALE-001` (T-582 shipped, follow-on only), `TOOLBAR-NEW-001` (exists on the Library
route), `TOOLBAR-TUTORIAL-001` (deferred).

Per-ticket absorption: T-079 **14** · T-076 **6** · T-078 **6** · T-084 **4** · T-075 **3** ·
T-073 **2** · T-082 **2** · T-072 **1** · T-077 **1** · T-069/T-213 **1** · T-146 **1** = 41.
**T-079 is three programs in one ticket and must be split before dispatch**
(`interactions_sweep.md` §5.3); T-082 will silently absorb 20 workbench ids unless scoped to
build-class (a).

### The five keyboard/pointer collisions

Recorded here because they are cross-cutting and not visible from any single row —
full table at `interactions_sweep.md` §4.4.

| # | Eden | TBD today | Severity | Blocks |
|---|---|---|---|---|
| 1 | `Space` cycles the transformation widget | `Space` centres on the selection | hard | `WIDGET-CYCLE-001`, `KEY-WIDGET-001` → T-075. Eden's own `1`–`5` direct keys are free, which dissolves it |
| 2 | `F` centres on the selected entity | unbound; TBD uses `Space` | soft | Bind `F` as an alias in the T-075 slice |
| 3 | `Backspace` hides all editor chrome | `Backspace` deletes the selection | hard **and dangerous** | `KEY-HIDE-UI-001` → P-9 |
| 4 | `RMB` opens the context menu; `Shift+RMB` drops a quick waypoint | RMB pans; `contextmenu` unconditionally suppressed | **hard, structural** | **7 ids at once** — `PLACE-COMMENT-001`, `CREW-SEAT-001`, `CONN-START-001`, `CTX-FORMATION-001`, `ATTR-MULTI-001`, `COMP-SAVE-001`, `KEY-WP-001`/`ACTION-WP-QUICK-001`. MMB already pans, so RMB can be freed. **The highest-leverage single change in the sweep** → P-3 |
| 5 | `Ctrl` held = multi-place; `Ctrl`+drag char→char = group | `Ctrl/Cmd+LMB` = additive select | hard, three-way | `PLACE-004` (T-072) and `CONN-GROUP-001`'s map half (P-4). Disambiguate by press context, and **land both together** or the second re-opens the first |

⚠ `Ctrl+R` / `Ctrl+T` / `Ctrl+F` are owned by the **browser** (reload / new tab / find). Do not copy
the first two; `Ctrl+F` is worth `preventDefault`-ing on the editor route (P-12).

### Carried forward unresolved — do not launder these into parity values

| Item | State |
|---|---|
| The ~13 unnamed toolbar buttons | **`UNKNOWN` as ids.** Named in prose at `interactions.md:371`, never assigned ids. Four were minted locally (Part 3); the rest cannot be triaged until a docs pass mints them from the screenshot corpus |
| `ATTR-FIELD-OBJ-STAMINA` | **`UNKNOWN:`** whether Reforger exposes a per-character stamina toggle — a Workbench API question, not answerable by code search |
| The `SYS` family | Declares **no ids** (`attributes.md:223-225`). T-079's "systems" third is un-enumerated and cannot be sized until it gets its own catalogue pass |
| `$defs/zoneRules`' 16 keys | No `ATTR-FIELD-*` id, so outside both sweeps — but they are the closest thing TBD has to trigger semantics and belong in whatever scopes T-079a |
| Zones as trigger areas | **`INFERRED:`** zones give a typed area with declarative rules but no activation/condition/timer/effects model. Do not close the `TRG-*` ids on zone evidence |
| `Delete` with a vehicle selected | **`INFERRED:`** removes nothing for that vehicle while clearing the selection. Recorded as a lead, not verified in-browser |
| T-069 / T-213 spec premises | Both cite a `state/schema.ts` deleted at T-159.29.3. **Rewrite the specs before promoting**, and fix T-213's line cite (`eden_chrome.rs:1528` → `:3337`) |

**Phase 2 doc coverage:** all 83 interaction ids in [`interactions.md`](./interactions.md); all 93
attribute ids in [`attributes.md`](./attributes.md); full UI anatomy in
[`ui_anatomy.md`](./ui_anatomy.md); wiki scrape 28/28 pages; 75 real Eden frames in
[`.ai/artifacts/eden_screenshots/`](../../../../.ai/artifacts/eden_screenshots/) across 8 batches. **Second pass (2026-06-20):**
comments, crew, clipboard actions, WP attach, multi-edit attrs, TRG/WP field tables.
**Third pass (2026-08-01):** this rewrite — sample → census.
