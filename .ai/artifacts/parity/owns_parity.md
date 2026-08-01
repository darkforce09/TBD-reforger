# `owns` derivation for the parity tickets, and the combined packing

**2026-08-01.** Companion to [`owns_and_waves.md`](owns_and_waves.md), which derived `owns` for
T-631…T-660. This file covers the **parity-sweep** tickets: the new attribute slices
([`attributes_sweep.md`](attributes_sweep.md) §5), the new interaction slices
([`interactions_sweep.md`](interactions_sweep.md) §5), and the eight existing `idea`/`deferred`
tickets the sweeps map ids onto. §5 then packs **both** sets into one wave sequence.

**Method, format and evidence grades are the sibling's** — `owns_and_waves.md` §1. The two tables
merge without translation.

**Written incrementally.** A previous agent on this job died on a session limit with everything in
its head and nothing on disk.

---

## 1. Method

### Inherited from `owns_and_waves.md` §1, unchanged

- `owns` = **files the ticket will MODIFY**, not read. Repo-root-relative,
  semicolon-space separated, exactly as `docs/platform/wave_plan.tsv` column 4 writes them.
- Confidence: `high` = read the exact code that changes, `file:line` cited · `medium` = found the
  module and the symbol, not every edit site · `low` = inferred from the description.
- Every inference is prefixed `INFERRED:`. Every count states its command **and what the surviving
  hits actually were**.
- `NEW:` marks a file the ticket creates. A new file collides with nothing, which materially
  improves packing.
- Frontend paths abbreviate `apps/website/frontend/src/` as **`FE/`** in the *Why* column only; the
  `owns` column is always written out in full.

### The one thing this file does differently: everything is POST-SPLIT

Wave 0 is the `eden_chrome.rs` split (`owns_and_waves.md` §7.3 — ten modules from one, 5,119 lines).
**Every ticket below that would have touched `eden_chrome.rs` names the post-split module instead.**

The mapping I applied, and the symbol that decides it:

| Post-split module | Symbols it receives (current `eden_chrome.rs` lines) |
|---|---|
| `eden_layout.rs` | `STRIP_TOP_PX` `:38`, `DOCK_LEFT_PX` `:40`, `DOCK_RIGHT_PX` `:42`, `TOOLBELT_BAND_PX` `:45`, shared style consts `:53-79` |
| `eden_top_strip.rs` | `MENUS` `:112`, `MirroredField` `:496`, `MIRROR_TIME` `:501`, `MIRROR_WEATHER` `:506`, `RowMirror` `:697-829`, `TopCommandStrip` `:831-1372` |
| `eden_env.rs` | `CARRIED_ENV_KEYS` `:231`, `author_env` `:271`, flow helpers `:377-482` |
| `eden_tree.rs` | `guide_spans` `:1383`, `ROW_H` `:1525`, `single_row` `:1532`, `virtual_tree` `:1711` |
| `eden_dock_left.rs` | `DockLeft` `:2813-2870` |
| `eden_dock_right.rs` | `PaletteKind` `:1833`, `palette_rows` `:1863`, `EDEN_SIDE_CHIPS` `:2871`, `DockRight` `:2961-3379` |
| `eden_vehicles_panel.rs` | `placed_vehicles_panel` `:1985` (wasm) / `:2213` (native) |
| `eden_zones.rs` | `zones_panel` `:2228`/`:2790`, `ZoneShape` `:3660`, `circle_from_clicks` `:3626` |
| `eden_toolbelt.rs` | `TOOLBELT` `:64`, `TOOL_ACTIVE` `:72`, `TOOL_DISABLED` `:74`, `fmt_coord` `:80`, `BottomToolbelt` `:3672-3786` |
| `eden_settings.rs` | `MissionSettingsDialog` `:3788-3926`, `render_flow_section` `:3927`, `render_prefs_section` `:4057` |

**Where I could not tell, I said so and graded `low`** rather than guessing — a wrong module name
silently breaks the packing, and this program has five confirmed errors of exactly that family.

One structural fact worth pinning before the tables: **`virtual_tree` has exactly one call site.**

```
$ grep -rn 'virtual_tree(' apps/website/frontend/src --include='*.rs'
apps/website/frontend/src/eden_chrome.rs:1711    ← the definition
apps/website/frontend/src/eden_chrome.rs:2844    ← the only call, inside DockLeft
```

So a per-row control (a layer eye, a lock glyph) is an `eden_tree.rs` edit and **not** an
`eden_dock_left.rs` edit — the flags arrive inside the `nodes` the dock already passes. That
distinction moves two tickets out of collision with each other.

### `main.rs` — the collision the file graph hides, restated

`apps/website/frontend/src/main.rs` carries **57** module declarations
(`grep -c '^mod \|^pub mod ' main.rs` → 57). Rust has no implicit module discovery, so **every
ticket creating a new module adds a line there.** The sibling's recommendation — have the wave-0
split ticket pre-declare empty stubs — applies to my new modules too, and §5 assumes it. Where it
matters I say so per ticket.

### Two corrections found while deriving

**1. `LYR-NAME` is graded `match` in `attributes_sweep.md:212`. The core mutator exists; nothing
calls it.**

```
$ grep -rn 'add_editor_layer\|rename_editor_layer\|reparent_editor_layer\|remove_editor_layer' \
    apps/website/frontend/src crates/ --include='*.rs' | grep -v 'doc/store.rs'
outliner.rs:131          ← doc comment
main.rs:30               ← doc comment
editor_ops.rs:842        ← doc comment
editor_ops.rs:1136       ← core.add_editor_layer(DEFAULT_LAYER_ID, …)   THE ONLY REAL CALL
doc/apply_faction.rs:659 ← core test
doc/place_orbat.rs:254   ← core test
```

I read all six. **`rename_editor_layer` `:1886`, `reparent_editor_layer` `:1895`,
`remove_editor_layer` `:1527` and `move_slot_to_layer` `:1915` have zero callers of any kind**;
`add_editor_layer` has exactly one, seeding the default layer at `editor_ops.rs:1136`. And
`DockLeft` (`eden_chrome.rs:2813-2870`, 58 lines read in full) renders `virtual_tree` plus five
`disabled=true` decoration buttons — **there is no create, rename, delete or reparent control in the
editor at all.** `interactions_sweep.md:514` (P-6) is right and `attributes_sweep.md:212` is
over-generous; T-037 shipped the *core*, not the UI.

**2. T-076's "must include the T-216 vehicle-roster compile drop" makes it cross-boundary.**
`flatten.rs:2631` states the delta for the vehicle roster as *"document root + a new `$def`"*, key
`vehicles`, and `:2634-2640` proves the already-declared `entities[]` cannot carry it (`$defs/alias`
is `^(kit|comp|veh|preset|layer|prop|item):[a-z0-9_]+$` and the mod alias registry holds exactly one
`veh:` row). So the compile half of T-076 modifies `packages/tbd-schema/schema/mission.schema.json`
→ **`executor: workbench`**. The crew *UI* half is factory-safe. **Split T-076 or it cannot be
dispatched.** See §4.

---

## 2. `owns` for the new slices

### 2.1 New attribute slices — `attributes_sweep.md` §5

Four slices. The sweep proposes five (N1, N2, N3, N9 plus the marker slice, which is an existing
ticket and lives in §3). **N2 is already derived** — see the note below the table.

#### N1 — Mission presentation: briefing text + thumbnail (`SCN-OVERVIEW-TEXT`, `SCN-PICTURE`)
```
apps/website/frontend/src/eden_settings.rs; apps/website/frontend/src/eden_top_strip.rs; apps/website/frontend/src/create_mission_dialog.rs
```
| Path | Why | Conf |
|---|---|---|
| `FE/eden_settings.rs` *(post-split)* | Eden's equivalent surface is Scenario Attributes; TBD's is `MissionSettingsDialog` (`eden_chrome.rs:3788`). I read `:3788-3850`: it renders Terrain (readonly), Time, Weather, then flow + render prefs. A briefing `<textarea>` and a thumbnail field belong in the same `overflow-y-auto` column at `:3834+` | **medium** |
| `FE/eden_top_strip.rs` *(post-split)* | `briefing` and `thumbnail_url` are **`missions` row columns, not `meta.*`** — `handlers/missions.rs:657` / `:658` in the PATCH body struct. The editor's only PATCH path is the `RowMirror` debounce machine (`eden_chrome.rs:496-829`, error surface `:797`), and it mirrors exactly two columns: `MIRROR_TIME` `:501` (`time_of_day`) and `MIRROR_WEATHER` `:506` (`weather`). Authoring briefing in the editor adds a third | **medium** |
| `FE/create_mission_dialog.rs` | 227 lines, one component `CreateMissionDialog` `:27`. The sweep names the create dialog as one of the two surfaces that cannot set it | **medium** |

**Not claimed, deliberately:** `dto.rs` — `MissionDetail.briefing` `:846` and `.thumbnail_url` `:850`
already exist. `apps/website/api/src/handlers/missions.rs` — the PATCH body already accepts both.
**No backend change; no schema change.** That is what makes this the highest value-per-unit-effort
row in the sweep.

> **Product call the operator should make before this is filed.** If briefing is authored in the
> **Mission Library dossier** instead of the editor, `owns` becomes
> `apps/website/frontend/src/mission_overview.rs` (1,412 lines) alone and the ticket becomes
> **collision-free against the entire program**. The editor-side design above collides with three
> tickets on `eden_top_strip.rs`. Both are defensible; the library version packs better.

#### N3 — Editor layer flags: visibility + transform lock (`LYR-ENABLE-VIS`, `LYR-ENABLE-XFORM`)
```
crates/map-engine-core/src/doc/store.rs; apps/website/frontend/src/editor_ops.rs; apps/website/frontend/src/outliner.rs; apps/website/frontend/src/eden_tree.rs
```
| Path | Why | Conf |
|---|---|---|
| `crates/…/doc/store.rs` | `add_editor_layer` `:1872` writes exactly `id` / `name` / `parentId` / `entityIds` — read in full. Two booleans plus their setters go beside `rename_editor_layer` `:1886` | **high** |
| `FE/editor_ops.rs` | `layer_rows` `:824` builds `LayerRow` from the doc and reads no flag (its own comment `:819-823` explains there is no public `editor_layers` accessor). **Visibility is enforced here, not in the renderer:** every slot upload goes through `core.materialize()` — `:368`, `:637`, `:941` — so a hidden layer's slots are filtered before the SoA reaches the engine. Transform lock refuses in `move_entities`/`attrs_update_position` `:667` | **high** |
| `FE/outliner.rs` | `LayerRow` `:45`, `OutlinerNode` `:75`, `build_outliner` `:105` — the flags have to reach the row renderer through this struct | **high** |
| `FE/eden_tree.rs` *(post-split)* | The eye/lock glyphs are per-row controls: `single_row` `:1532` (which already takes a per-call `orbat_refile: bool`) and `virtual_tree` `:1711` | **medium** |

**`eden_dock_left.rs` deliberately NOT claimed** — the flags ride inside `nodes`, which `DockLeft`
already passes at `eden_chrome.rs:2844`; no new parameter, so no edit. `crates/map-engine-render/`
is **not** claimed either: filtering at `materialize()` is cheaper than a render-side visibility
mask and keeps this ticket out of a 6,302-line file.

#### N9 — Cleanup: dead `view_distance` / `thermals` DTO fields
```
apps/website/frontend/src/dto.rs; apps/website/frontend/src/editor_ops.rs
```
| Path | Why | Conf |
|---|---|---|
| `FE/dto.rs` | `MissionEnv.view_distance` `:991`, `.thermals` `:992`, and their `Default` arms `:1007-1008`. No writer since T-193 (`b30f5490`) | **high** |
| `FE/editor_ops.rs` | Still parsed in `read_env` — `:205-208` (`viewDistance`, default 1600) and `:209-212` (`thermals`) | **high** |

`grep -rn 'view_distance\|thermals' apps/website/frontend/src --include='*.rs'` → **14 hits, all
read.** Four are the declarations/defaults above, two are the parse, and the remaining eight are in
`eden_chrome.rs` (`:206`, `:218`, `:220`, `:289`, `:4599`, `:4612`, `:4624`, `:4825`) — **every one
is a doc comment or a test asserting the keys are NOT authored.** The `keys_nothing_reads_are_not_authored`
test at `:4624` stays green when the DTO fields go, because it asserts absence. **No consumer
renders either value.** This is a genuine dead-code removal, and it is the smallest ticket in
either program.

#### N2 — Editor comments — **already derived; do not file twice**

`attributes_sweep.md:455` files N2 as *"No ticket anywhere — was T-651 in the draft set only."*
That is the same ticket. **T-651 (`owns_and_waves.md` §2 Group F) already carries an `owns` list**,
and the three ids `CMT-TITLE` / `CMT-TOOLTIP` / `CMT-POSITION` are exactly its content. Adopt
T-651; translated to post-split its `owns` is:

```
crates/map-engine-core/src/doc/store.rs; apps/website/frontend/src/editor_ops.rs; apps/website/frontend/src/outliner.rs; apps/website/frontend/src/eden_dock_right.rs; apps/website/frontend/src/mission_editor.rs
```

The sibling's `eden_chrome.rs` row was justified as *"RMB-empty → Place Comment; the context menu
lives with `MENUS` / `DockRight`"*. **Post-split that is now wrong in a useful way:** the context
menu is P-3's new module, not `MENUS`, so T-651's chrome path should be `eden_dock_right.rs` (a
Comments palette entry) **and T-651 gains a hard dependency on P-3**. Recorded in §5.

---

### 2.2 New interaction slices — `interactions_sweep.md` §5.2

Ten proposed slices covering 16 ids. Three of them duplicate tickets the sibling already derived;
those are marked **⇄ merge** and derived once, here, in post-split terms.

#### P-3 — Right-click context menu *(the unblocking slice)*
```
apps/website/frontend/src/mission_editor.rs; apps/website/frontend/src/context_menu.rs; apps/website/frontend/src/main.rs
```
| Path | Why | Conf |
|---|---|---|
| `FE/mission_editor.rs` | Two edits, both read in full. **(a)** `onpointerdown` `:1402` is `if ev.button() == 1 \|\| ev.button() == 2 {` — one branch pans on **both** MMB and RMB; freeing RMB means narrowing it to `== 1`. **(b)** `oncontextmenu` `:1844-1847` is a three-line closure whose entire body is `ev.prevent_default()` — it must instead open the menu at the event px. The comment at `:1392-1394` records *why* it was blanket-suppressed ("so an RMB-drag isn't interrupted"), which is exactly the reason that dies with (a) | **high** |
| `NEW: FE/context_menu.rs` | Item model, hit target resolution, keyboard dismissal. `INFERRED:` a new module per the flat one-concern-per-file convention (`select_tool.rs`, `world_layer_prefs.rs`, `mission_size.rs`). Verbatim item lists exist in `eden_screenshots/batch01_context_menu.md:119-292` | **low** |
| `FE/main.rs` | One `mod context_menu;` line — **removable** if wave 0 pre-declares the stub | **high** |

**Not claimed:** any `eden_*` module. A context menu is a floating overlay mounted beside the other
overlays in `mission_editor.rs` (`:2060-2110`), not dock chrome. Claiming an `eden_*` path here
would collide P-3 with the docks for no reason, and P-3 gates six other tickets — it must not wait.

#### P-6 — Outliner layer authoring (5 ids)
```
apps/website/frontend/src/editor_ops.rs; apps/website/frontend/src/eden_dock_left.rs; apps/website/frontend/src/eden_tree.rs
```
| Path | Why | Conf |
|---|---|---|
| `FE/editor_ops.rs` | The wrappers that do not exist. Today the file has **one** layer command, `set_active_layer` `:1025`, plus internal `layer_rows` `:824` / `ensure_layer` `:1125` / `refresh_docks` `:961`. Create/rename/delete/reparent/refile wrappers onto the five core mutators land here, and each needs a `refresh_docks()` | **high** |
| `FE/eden_dock_left.rs` *(post-split)* | The "+" create button and the root dropzone go in the `DockLeft` header (`eden_chrome.rs:2838-2841`) — there is no control there today | **high** |
| `FE/eden_tree.rs` *(post-split)* | Inline rename, hover row actions, and *"folder click selects children + descendants"* are all `single_row` `:1532` / `virtual_tree` `:1711` behaviour | **high** |

**`crates/…/doc/store.rs` deliberately NOT claimed.** All five mutators already exist and are
tested — `add_editor_layer` `:1872`, `rename_editor_layer` `:1886`, `reparent_editor_layer` `:1895`
(cycle-guarded), `remove_editor_layer` `:1527` (subtree + reseed), `move_slot_to_layer` `:1915`.
**This is pure UI wiring onto shipped core**, which is why the sweep calls it the best
value-per-line available, and not claiming `store.rs` keeps it off T-651's and N3's critical path.

#### P-10 — Clipboard completion: `Ctrl+X` cut, `Ctrl+Shift+V` paste-at-original
```
apps/website/frontend/src/mission_editor.rs
```
| Path | Why | Conf |
|---|---|---|
| `FE/mission_editor.rs` | Two match arms in the editor keydown, read in full at `:1019-1031`. The existing arms are `"KeyC" if modk && !alt && !shift` `:1020`, `"KeyV" if modk && !alt && !shift` `:1023`. **Both primitives already exist** — `copy_selection` `editor_ops.rs:394`, `delete_selection` `:327`, and `paste_at_cursor(None, None)` `:436` is literally paste-at-original because the anchor is optional | **high** |

**Single-file ticket. `editor_ops.rs` is NOT claimed** — nothing there changes. That makes P-10 one
of only three tickets in the combined program that touch exactly one file.

#### P-11 — Scale readout in the toolbelt (`STATUS-ZOOM-001`)
```
apps/website/frontend/src/eden_toolbelt.rs; apps/website/frontend/src/mission_editor.rs
```
| Path | Why | Conf |
|---|---|---|
| `FE/eden_toolbelt.rs` *(post-split)* | The readout row. `BottomToolbelt` `:3672` and its two mono groups: CUR/SEL + X/Y/Z at `:3720-3740`, OBJ/SEL/SZ at `:3741-3766`. A scale span is a fourth cell in the second group | **high** |
| `FE/mission_editor.rs` | **The component has no zoom input.** I read the full signature `:3672-3682`: it takes `cursor`, `sel_count`, `obj_count`, `selected_ids`, optional `sz_bytes` — no camera. `RenderEngine::zoom()` exists (`crates/map-engine-render/src/engine.rs:1720`) but is only reachable from the rAF sampler in `mission_editor.rs` (mount at `:2070`), so this ticket adds a signal there and a prop here | **high** |

Two files, and the second one is `mission_editor.rs` — so P-11 is **not** the free single-file win
the sweep implies. Worth stating; the sweep's *"one `<span>`"* is the toolbelt half only.

#### P-9 — Backspace collision + hide-UI (`KEY-HIDE-UI-001`)
```
apps/website/frontend/src/mission_editor.rs
```
| Path | Why | Conf |
|---|---|---|
| `FE/mission_editor.rs` | Drop the alias: the arm is `"Delete" \| "Backspace" if !modk =>` at `:1027-1029`, one pattern edit. Bind hide-chrome in the same keydown; the chrome mounts are all siblings in this file (`:2060-2110`), so a `chrome_hidden` signal gates them here | **high** |

**Single file.** This is one of the two wave-1 quick wins in §5.

#### P-1 / P-2 / P-7 — Placement + Attributes entry points ⇄ **merge with T-647**
```
apps/website/frontend/src/mission_editor.rs; apps/website/frontend/src/editor_ops.rs
```
| Path | Why | Conf |
|---|---|---|
| `FE/mission_editor.rs` | P-7 is a **one-call-site swap**, verified: the dblclick handler at `:1963` calls `crate::select_tool::pick(&cam, &c.materialize(), px, py)`, while the click path already uses `st::pick_slot_or_vehicle` at `:1537` and `:1733`. Both exist (`select_tool.rs:128` and `:150`). P-1/P-2 are the `onpointerup` `:1644` and dblclick `:1934` seams | **high** |
| `FE/editor_ops.rs` | `open_attributes` `:568` must accept vehicle/zone ids, and P-1's click-then-click changes the arm/consume lifecycle around `place_at` `:2191` | **high** |

**Identical `owns` to the sibling's T-647.** These are the same seam — "what a click or dbl-click on
the map means". **File one ticket, not two.** T-647 already carries `PLACE-003`; folding
`PLACE-001` and `ATTR-OPEN-001` into it costs nothing and saves a wave.

#### P-5 — Select All in view (`SEL-ALL-001`) ⇄ **merge with T-649**
```
apps/website/frontend/src/mission_editor.rs; apps/website/frontend/src/select_tool.rs
```
| Path | Why | Conf |
|---|---|---|
| `FE/mission_editor.rs` | `Ctrl+A` joins the keydown arms at `:1019-1031` | **high** |
| `FE/select_tool.rs` | Eden scopes Select All to the viewport; `marquee_ids` `:276` and `marquee_ids_with_vehicles` `:308` are exactly a viewport-rect query | **high** |

**A strict subset of the sibling's T-649**, which claims these two plus `attributes.rs` and
`editor_ops.rs`. **Do not file P-5 separately** — it would collide with T-649 on both paths and buy
nothing.

#### P-4 — Map-surface grouping: `Ctrl` + drag character→character (`CONN-GROUP-001` map half)
```
apps/website/frontend/src/mission_editor.rs; apps/website/frontend/src/editor_ops.rs
```
| Path | Why | Conf |
|---|---|---|
| `FE/mission_editor.rs` | A modifier branch in `onpointermove` / `onpointerup` `:1644`; the drop target is a second `pick_slot_or_vehicle` at release | **medium** |
| `FE/editor_ops.rs` | Regrouping is a squad-membership write. `orbat_add_vehicle` `:1616` is the nearest existing shape; the core side already exists (`store.rs:835` `attach_vehicle`, `:880` `detach_vehicle`) | **medium** |

**Ship with T-072** — `interactions_sweep.md:516` records that they overload the same modifier
(collision 5), and T-072's own scope is `place_at`'s `take()` at `editor_ops.rs:2204` (read: the
`let Some(pending) = ctx.pending.borrow_mut().take() else` inside `place_at` `:2191`). Same two
files. **Note the sibling records T-647 as superseding T-072** — so P-4 folds into T-647 as well.

#### P-8 — Snapping grids: translation grid toggle + step (`KEY-GRID-001`) ⇄ **merge with T-648**
```
apps/website/frontend/src/select_tool.rs; apps/website/frontend/src/mission_editor.rs
```
| Path | Why | Conf |
|---|---|---|
| `FE/select_tool.rs` | `GRID_CELL_M` `:33` already exists as `MissionDocCore::GRID_CELL_M`, but it is a **spatial-index cell size**, used only at `:338` and `:372` to build a `PointIndex` — **not a snap step.** I read both. The snap quantiser goes beside `drag_delta` `:201` | **high** |
| `FE/mission_editor.rs` | The grid toggle key, in the keydown at `:1019-1031` | **high** |

**Both paths are already in the sibling's T-648** (`Shift`-rotate, snap grid, widget + `Space`
cycle), whose `owns` additionally has `editor_ops.rs`. **Fold P-8 into T-648.** The sweep is right
that nothing snap-related exists — and the README's `snap` warning applies verbatim: the 37
word-boundary hits are all `let snap = read_snapshot()`.

#### P-12 — `Ctrl+F` focuses asset search *(no `interactions.md` id)*
```
apps/website/frontend/src/mission_editor.rs; apps/website/frontend/src/eden_dock_right.rs
```
| Path | Why | Conf |
|---|---|---|
| `FE/mission_editor.rs` | Needs `preventDefault` on the editor route so the browser find bar does not open — the keydown at `:1019-1031` | **medium** |
| `FE/eden_dock_right.rs` *(post-split)* | The focus target. Two search inputs exist, both in `DockRight`: `type="search"` at `eden_chrome.rs:3112` (Factions/Objects, `search` `:2990` / `object_search` `:3012`) and `:3253` (`vehicle_search` `:3009`). `Ctrl+F` must pick the one matching the active tab | **medium** |

The sweep suggests folding this into P-5 or T-146. **Fold into T-084** instead — same file, same
concern (asset search), and T-084 is already the cheapest ticket in the sweep.

---

## 3. `owns` for the existing tickets to promote/scope

All eight, derived **as they would be scoped** after the sweeps' corrections. Post-split module
names throughout.

### T-069 ⊕ T-213 — Markers on map — **merge into one ticket**

`attributes_sweep.md:443` (G1) and `interactions_sweep.md:493` both say these are the same job.
Scope to the four schema-carried fields only: `MRK-TYPE`, `MRK-TEXT`, `MRK-POSITION` → `{x, z, icon,
label}`. Marker *style* (size/rotation/shape/brush/colour/alpha) is a `$defs/marker` widening →
workbench, §4.

```
apps/website/frontend/src/eden_dock_right.rs; apps/website/frontend/src/editor_ops.rs; apps/website/frontend/src/mission_editor.rs; crates/map-engine-core/src/doc/store.rs; crates/map-engine-render/src/draw_order.rs; crates/map-engine-render/src/engine.rs
```
| Path | Why | Conf |
|---|---|---|
| `FE/eden_dock_right.rs` *(post-split)* | The whole stub. Tab button `{tab_btn(2, "Markers")}` at `eden_chrome.rs:3046`; the body is the `_ =>` fallthrough arm whose entire content is `"Marker placement lands in T-069."` at `:3337`. `PaletteKind` `:1833` has exactly three variants (Character / Vehicle / Object; arms `:1944-1950`) and needs a fourth | **high** |
| `FE/editor_ops.rs` | `grep -c marker editor_ops.rs` → **0**. Not one marker command exists in the SPA's doc-mutation surface. Arm / place / move / remove wrappers plus `refresh_docks` `:961` | **high** |
| `FE/mission_editor.rs` | A marker place consumes on canvas release — `onpointerup` `:1644` with the on-canvas guard `:1668-1671` — and the marker buffer joins the per-frame upload | **medium** |
| `crates/…/doc/store.rs` | `markersById` is a declared root (`:355`) with `marker_any(id, x, z, icon, label)` `:3048` and `marker_row_id` `:3033` already written. But the only two mutators — `set_faction_briefing_marker` `:2030` and `remove_faction_briefing_marker` `:2070` — are **faction-briefing scoped and have zero product callers** (`attributes_sweep.md:189`, re-verified). Free placement needs generic add/move/remove | **high** |
| `crates/map-engine-render/src/draw_order.rs` | **632 lines, and it is where every render lane is declared.** `LaneRole` `:9`; the mission lanes are `MissionZones` `:55`, `SquadLinks` `:57`, `MissionVehicles`, `Slots`. **There is no marker lane.** Adding `MissionMarkers` touches the variant, `lane_order` `:107`, the `ALL` list `:147`, and the `role_id` round-trip `:237`/`:258` — plus the ordering assertions at `:581-585` | **medium** |
| `crates/map-engine-render/src/engine.rs` | The upload entry point. `upload_icon_lane(kind, bytes, visible)` `:3555` is a **closed** `match kind { 0 => WorldTrees, 1 => WorldProps, 2 => WorldBadges, _ => return }` — world-asset scoped, so markers cannot ride it unchanged. I read all **10** `marker` hits in this 6,302-line file: nine belong to `feed_cluster_markers` `:3741` (supercluster discs) and one is a comment at `:1499` about a debug orientation marker. **None is a mission marker** | **medium** |

> **Two dead premises to fix before filing.** `T-213`'s spec cites a `state/schema.ts` that no
> longer exists — the React tree was deleted at T-159.29.3 (`attributes_sweep.md` §4.4). `T-069`'s
> spec premise is confirmed dead (§4.5). **Rewrite both specs, then merge.** Keep T-069's id (it
> owns the in-code stub message at `eden_chrome.rs:3337` and the test that pins it at `:4499-4502`).

### T-076 — Vehicle crew UI — **split; only 076a is dispatchable**

`interactions_sweep.md:489` requires T-076 to include the T-216 vehicle-roster compile drop. **That
makes half of it cross-boundary.** See §1 correction 2.

**T-076a — crew authoring UI (factory-safe)**
```
apps/website/frontend/src/eden_vehicles_panel.rs; apps/website/frontend/src/editor_ops.rs; crates/map-engine-core/src/doc/store.rs; apps/website/frontend/src/eden_dock_right.rs
```
| Path | Why | Conf |
|---|---|---|
| `FE/eden_vehicles_panel.rs` *(post-split)* | `placed_vehicles_panel` `eden_chrome.rs:1985` (wasm) / `:2213` (native) — it already renders `VehicleRow` + `VehicleCargoRow` per placed vehicle. `CREW-PANEL-001` is a seat list in the same panel | **high** |
| `FE/editor_ops.rs` | The vehicle command surface, listed in full: `begin_place_vehicle` `:1044`, `orbat_add_vehicle` `:1616`, `vehicle_rows` `:1751`, `set_vehicle_cargo` `:1811`, `vehicle_points` `:1833`, `set_vehicle_heading` `:1841`, `move_vehicles` `:1877`, `is_vehicle_id` `:1902`, `remove_vehicle` `:1908`. **Nine functions, no crew concept.** Board / unboard / seat-assign are new | **high** |
| `crates/…/doc/store.rs` | Same on the core side, read in full: `add_vehicle` `:747`, `set_vehicle_faction` `:784`, `set_vehicle_cargo` `:812`, `attach_vehicle` `:835` (vehicle→**squad**, not crew→vehicle), `remove_vehicle` `:858`, `detach_vehicle` `:880`, `set_vehicle_position` `:1118`. A crew map is new doc state | **high** |
| `FE/eden_dock_right.rs` *(post-split)* | `RIGHT-CREW-001` — the with/without-crew placement toggle sits beside the Vehicles search input at `eden_chrome.rs:3253` | **medium** |

**T-076b — vehicle roster reaches the game (`executor: workbench`, excluded — §4).**

### T-077 — Alt + empty vehicle (`PLACE-CREW-001`)
```
apps/website/frontend/src/mission_editor.rs; apps/website/frontend/src/editor_ops.rs
```
| Path | Why | Conf |
|---|---|---|
| `FE/mission_editor.rs` | **`Alt` is genuinely free.** `grep -rnw 'alt_key' apps/website/frontend/src --include='*.rs'` → **3 hits, all read**: `mission_history.rs:490` (an undo guard that *excludes* alt), `mission_editor.rs:1020` and `:1023` (the same exclusion on Ctrl+C / Ctrl+V). **Not one is a placement modifier.** The flag is read where the place is consumed, `onpointerup` `:1644` | **high** |
| `FE/editor_ops.rs` | A with/without-crew flag threaded `begin_place_vehicle` `:1044` → `place_at` `:2191` (the `take()` is at `:2204`) | **high** |

**Depends on T-076a** — there must be a crew to suppress. **And the sibling records T-647 as
superseding T-077** (`owns_and_waves.md` §2, T-647). T-647's `owns` is byte-identical to this row.
**Fold T-077 into T-647 and delete the row** — filing both guarantees a collision on two files for
one modifier.

### T-078 — Custom compositions — ⇄ **merge with T-650**

`interactions_sweep.md:491` gives T-078 six ids; `attributes_sweep.md:447` (G5) adds the three
`COMP-*` metadata fields. The sibling's **T-650 "Compositions: save and place"** is the same
ticket. Post-split, T-650's `owns` becomes:

```
apps/website/frontend/src/eden_dock_right.rs; apps/website/frontend/src/editor_ops.rs; crates/map-engine-core/src/doc/store.rs
```
| Path | Why | Conf |
|---|---|---|
| `FE/eden_dock_right.rs` *(post-split)* | `RIGHT-MODE-002` is a new palette mode — `PaletteKind` `:1833`, arms `:1944-1950`, tab strip `:3041-3046`. **Same three symbols the marker ticket, T-079a and T-079c all need** | **medium** |
| `FE/editor_ops.rs` | Save-selection is a read plus a write; place is a multi-slot paste — `copy_selection` `:394` / `paste_at_cursor` `:436` are the shape to reuse | **medium** |
| `crates/…/doc/store.rs` | Storage, **if in-document**. Zero repo-wide hits for `save_composition` / `user_composition` / `custom_composition`; the only `composition` occurrences in the frontend are `asset_catalog.rs:345-844`, a path-string classifier (`derive_object_alias` `:352` prefixes `comp:` by whether the Arma path contains `"Composition"`) — it **consumes** Bohemia's compositions, it does not author one | **low** |

> **The `COMP-TITLE` / `-AUTHOR` / `-CATEGORY` metadata forces the open question closed.**
> `attributes_sweep.md:447` scopes G5 as *"Table + API + SPA"* — a title, a server-assigned author
> and a category are **user-scoped rows, not mission-document state.** If the operator accepts that
> framing, T-650 loses `store.rs` and gains
> `NEW: apps/website/api/src/handlers/compositions.rs` + `NEW: apps/website/api/migrations/<n>_compositions.sql`
> + `apps/website/api/src/models/mod.rs` + `apps/website/frontend/src/dto.rs`. That version is
> **better for packing** (two of four paths are new files) and is still `claude-code` — the Rust
> workspace, not `packages/` or `apps/mod/`. Graded **low** either way; this is a design decision,
> not a search result.

### T-079 — split four ways

`interactions_sweep.md:492` — 14 ids, the largest single absorber, three unrelated entity families.
Split per §5.3 of that sweep. **T-079b is excluded (§4); T-079c cannot be sized.**

**T-079a — triggers (`RIGHT-MODE-003`, `CONN-TRG-OWNER-001`)**
```
apps/website/frontend/src/eden_dock_right.rs; apps/website/frontend/src/eden_zones.rs; apps/website/frontend/src/editor_ops.rs; crates/map-engine-core/src/doc/store.rs
```
| Path | Why | Conf |
|---|---|---|
| `FE/eden_dock_right.rs` *(post-split)* | `RIGHT-MODE-003` — another `PaletteKind` `:1833` variant + tab strip `:3041-3046` | **medium** |
| `FE/eden_zones.rs` *(post-split)* | Trigger geometry rides the **shipped** T-582 zone draw tool: `zones_panel` `:2228`, `ZoneShape` `:3660`, `circle_from_clicks` `:3626`, `zone_rule_fields` `:3443`. `attributes_sweep.md:445` says exactly this | **medium** |
| `FE/editor_ops.rs` | The zone-draw command surface is here and is large — `zone_draw_armed` `:2325`, `begin_zone_draw` `:2350`, `begin_zone_reshape` `:2378`, `cancel_zone_draw` `:2400`, `zone_draw_pop_vertex` `:2412`, `advance_zone_draw` `:2436`, `close_zone_polygon` `:2499`, `zone_rows` `:2579`, `set_zone_rule` `:2688`, `delete_zone` `:2717`. A trigger area is a second consumer of all of it | **medium** |
| `crates/…/doc/store.rs` | A trigger entity map, on the `markersById` `:355` / zones precedent | **medium** |

**Only the editor half.** The activation/effects model is a new Enfusion runtime → workbench (§4).

**T-079c — systems / modules (`RIGHT-MODE-005`)**
```
apps/website/frontend/src/eden_dock_right.rs
```
| Path | Why | Conf |
|---|---|---|
| `FE/eden_dock_right.rs` *(post-split)* | One more `PaletteKind` variant. **That is all that can be derived.** `attributes_sweep.md:536`: *"The `SYS` family declares no ids in `attributes.md:223-225`. T-079's 'systems' third is un-enumerated — it needs its own catalogue pass before it can be sized."* | **low** |

**Recommendation: do not file T-079c yet.** A ticket whose entire derivable content is one enum
variant is a placeholder, and it would consume an `eden_dock_right.rs` wave slot that four real
tickets are queued for.

**T-079d — connection graph (`CONN-START-001`, `CONN-SYNC-001`, `CONN-DEL-001`, `ACTION-FORM-001`, `CTX-FORMATION-001`)**
```
apps/website/frontend/src/mission_editor.rs; apps/website/frontend/src/editor_ops.rs; apps/website/frontend/src/context_menu.rs; crates/map-engine-core/src/doc/store.rs; crates/map-engine-render/src/draw_order.rs
```
| Path | Why | Conf |
|---|---|---|
| `FE/mission_editor.rs` | Drag-from-entity-to-entity is a third gesture mode in `onpointerdown` `:1395` / `onpointerup` `:1644` | **medium** |
| `FE/editor_ops.rs` | Link create / sync / delete mutators | **medium** |
| `FE/context_menu.rs` *(P-3's new module)* | `CTX-FORMATION-001` and `CONN-DEL-001` are context-menu items by definition | **medium** |
| `crates/…/doc/store.rs` | A link map | **medium** |
| `crates/map-engine-render/src/draw_order.rs` | **The lane may already exist.** `LaneRole::SquadLinks` `:57` is *"T-180.4 — squad leader→member hairline links (under slot rings)"*, and its doc comment at `:49` groups it with `Contours` / `ForestOutline` as a flat LineList lane — **exactly the primitive a connection graph needs.** Either it is reused (drop this path, and `engine.rs` too) or a `MissionConnections` peer is added here | **medium** |

**Hard dependency on P-3** (`interactions_sweep.md:528`).

### T-082 — Full attribute fields — **scoped to build-class (a), or it absorbs 20 workbench ids**
```
apps/website/frontend/src/attributes.rs; apps/website/frontend/src/editor_ops.rs
```
| Path | Why | Conf |
|---|---|---|
| `FE/attributes.rs` | 367 lines. `identity_tab` `:322` — I read it: `text_field("Role", …, "Rifleman", …)` at `:326` and `text_field("Tag", …, "MED · ENG · SL…", …)` at `:330`, both writing through `attrs_update_slot`. `OBJ-ROLE-DESC` needs a *separate* description field (today `role` doubles as both); `OBJ-TYPE` needs a type control that does not exist. `text_field` `:233`, `TABS` `:16` | **high** |
| `FE/editor_ops.rs` | **`read_attrs` `:631` builds `SlotAttrs` from `core.materialize()` `:637` and never reads `assetId`** — which is exactly why the type cannot be changed in the modal, even though the core mutator exists (`store.rs:1276`). Writes go through `attrs_update_slot` `:796` | **high** |

**Scope, stated so it cannot drift:** `OBJ-TYPE` + `OBJ-ROLE-DESC` **only** (2 of the 31 `OBJ` ids).
`ATTR-MULTI-001` / `ATTR-MULTI-CHK-001` route to **T-649**, which already owns `attributes.rs`
*and* the `editor_ops.rs:583-585` / `:605-607` suppression guards. `OBJ-UNIT-NAME`,
`OBJ-CALLSIGN`, `OBJ-RANK`, `OBJ-STANCE` are class (b)/(c) → workbench.

> ⚠ **T-082 collides with T-649 on both of its paths.** They cannot share a wave. If the operator
> prefers one ticket, fold T-082 into T-649 — the combined `owns` is unchanged from T-649's, and
> the program loses a wave slot for free.

### T-084 — Classname / mod prefix search — the cheapest ratio in the sweep
```
apps/website/frontend/src/asset_catalog.rs; apps/website/frontend/src/eden_dock_right.rs
```
| Path | Why | Conf |
|---|---|---|
| `FE/asset_catalog.rs` | `pub fn filter_catalog(nodes, query)` `:396`, read in full (`:390-414`): it lowercases the query and does a **label substring** match with a self-match/descendant-match rule. **There is no prefix parsing of any kind.** All four ids (`RIGHT-SEARCH-002/003/004/005`) are pattern parsing in front of this one function | **high** |
| `FE/eden_dock_right.rs` *(post-split)* | Three call sites, all inside `DockRight`: `:3163` (Objects), `:3218` (Factions), `:3306` (Vehicles); the two `type="search"` inputs are `:3112` (shared Factions/Objects, signals `search` `:2990` / `object_search` `:3012`) and `:3253` (`vehicle_search` `:3009`). Placeholder/hint copy changes here | **medium** |

**Fold P-12 (`Ctrl+F` focuses asset search) in** — same file, same concern. That adds
`apps/website/frontend/src/mission_editor.rs` for the `preventDefault`, and T-084 becomes three
files. Four ids plus the highest-frequency Eden shortcut, in one ticket.

---

## 4. Excluded as workbench — and why

Rule applied, per the brief: **anything modifying
`packages/tbd-schema/schema/mission.schema.json` or `apps/mod/tbd-framework/` is program 2.** No
`owns` derived. The blocking evidence is cited so no one re-litigates it.

| Excluded | ids | Blocking evidence |
|---|---|---|
| **Marker style / Area markers** | `MRK-SIZE`, `-ROTATION`, `-SHAPE`, `-BRUSH`, `-COLOR`, `-ALPHA` (6) | `$defs/marker` is exactly `{x, z, icon, label}` and closed. Icon-vs-Area is Eden's whole second marker model. Must ship **after** the T-069/T-213 core or it converts a factory ticket into a workbench one (`attributes_sweep.md:444`) |
| **T-076b — vehicle roster to the wire** | — | `flatten.rs:2631`: the delta is *"document root + a new `$def`"*, key `vehicles`. `:2634-2640` proves the declared `entities[]` cannot carry it — `$defs/alias` is `^(kit\|comp\|veh\|preset\|layer\|prop\|item):[a-z0-9_]+$` and `apps/mod/tbd-framework/Data/registry.json` holds **one** `veh:` row, so alias substitution would be the T-200 silent-substitution defect with a 10-tonne vehicle |
| **T-079b — waypoints** | 9 `WP-*` + `RIGHT-MODE-004`, `CONN-WP-ACT-001`, `CONN-WP-ATTACH-001`, `CONN-RAND-START-001`, `KEY-WP-001`, `ACTION-WP-QUICK-001` | **Blocked on AI units existing**, not on schema. `TBD_SpawnManager.c:963,1166` spawns every body with AI disabled — waypoints have no subject (`attributes_sweep.md:446`, the sweep's own "single biggest scoping correction available to T-079") |
| **T-079a/b runtime** — trigger activation + effects | 12 of 13 `TRG-*` | A new Enfusion runtime in `apps/mod/tbd-framework/`. Only the **geometry + palette mode** half is derived in §3 |
| **Group AI state** | `GRP-COMBAT-MODE`, `-BEHAVIOUR`, `-FORMATION`, `-SPEED-MODE` (4) | Same AI gate as waypoints. `combatMode` / `speedMode` word-boundary = **0** in both trees; `formation` = 1 frontend hit, and it is prose in `editor_ops.rs:1324` |
| **N4 — T-216 follow-on** | `OBJ-CALLSIGN`, `OBJ-RANK`, `OBJ-STANCE` + TBD-only `tag`, `leaderSlotId` | The contract delta is pre-written at `flatten.rs:2620-2632` — five `$defs/slot` / `$defs/group` keys. `stance` additionally needs an Enfusion spawn-pose call (`stance` word-boundary in `apps/mod` = **0**). Related ticket: T-242 |
| **N5 — placement scatter** | `OBJ-PLACEMENT-RADIUS`, `OBJ-SHAPE`, `GRP-PLACEMENT-RADIUS` (3) | `$defs/slot` and `$defs/group` both closed. Cheapest coherent (b) slice, but still a widening |
| **N6 — vehicle states** | `OBJ-LOCK`, `-FUEL`, `-AMMO` (3) | `$defs/entity` closed; no reader. `fuel` word-boundary in `apps/mod` = 1, `lock` = 4 and none is an authored vehicle lock |
| **N7 — entity states** | `OBJ-HEALTH`, `-ALLOW-DAMAGE`, `-SHOW-MODEL`, `-SIZE`, `-STAMINA` (5) | Gated on `entities[]` acquiring a consumer — `mission.schema.json:72` records that nothing on any shipped build reads it. `OBJ-STAMINA` additionally carries an unresolved Workbench API question |
| **N8 — environment readers** | `SCN-FOG`, `-WIND`, `-VIEW-DIST` (3) | Refused **by test**: `eden_chrome.rs:4624` iterates `["viewDistance", "thermals", "windDirDeg", "fog", "wind"]` and asserts none is carried. The mod reader comes first, the control second. `windDirDeg` is in the schema and `ModEnvironment` (`flatten.rs:268-275`) does not even serialise it |
| **`OBJ-UNIT-NAME`** | 1 | `$defs/slot` closed; rides the T-216 follow-on |
| **T-654** *(sibling's)* | — | Already excluded by `owns_and_waves.md` §6 for the same rule. Restated here so the two lists agree |

**Total excluded: 49 attribute ids + 15 interaction ids.** That matches `attributes_sweep.md`
§6's *"`executor: workbench` — a second program (49 ids)"* exactly, which is a useful independent
check on both derivations.

**Two near-misses that are NOT excluded, stated so they are not wrongly moved:**

- **T-078 / T-650 gaining `apps/website/api/` + a migration** is still the Rust workspace and still
  `claude-code`. Not `packages/`, not `apps/mod/`.
- **T-069/T-213 gaining `crates/map-engine-render/`** is likewise in-workspace. A render lane is
  not a contract change; `$defs/marker` already declares `{x, z, icon, label}` and the icon enum is
  closed at 64 aliases, so the marker **core** needs no schema edit at all. That is the whole reason
  it is the one absent-entity family that is factory-dispatchable today.

---

## 5. The combined packing

### 5.1 The roster — 43 rows

**28** from `owns_and_waves.md` (27 code-bearing tickets with T-641 split into a/b) · **14** derived
here · **1** wave-0 split ticket. `T-652`, `T-653`, `T-654` carry no `owns` and take no slot.

**Nine slices were merged away rather than filed**, each because its `owns` was a subset of, or
identical to, an existing row. Filing them separately would have added waves and bought nothing:

| Merged | Into | Reason |
|---|---|---|
| P-9 (Backspace + hide-UI) | **W1-UNBLOCK** | Same file as the RMB unblock — `mission_editor.rs` |
| P-5 (Select All) | T-649 | Strict subset: `mission_editor.rs` + `select_tool.rs` |
| P-1 / P-2 / P-7 | T-647 | Byte-identical `owns`; same seam |
| P-4 (map grouping) | T-647 | Same two files, same modifier as T-072 (which T-647 supersedes) |
| P-8 (snapping) | T-648 | Both paths already in T-648 |
| P-12 (`Ctrl+F`) | **deferred** | See §5.4 — it is the 18th `mission_editor.rs` claimant and costs a whole wave |
| N2 (editor comments) | T-651 | Same ticket under another name |
| T-078 (compositions) | T-650 | Same ticket |
| T-077 (Alt empty vehicle) | T-647 | Sibling already records the supersession |
| T-079b (waypoints) | **excluded** | Blocked on AI units existing (§4) |
| T-079c (systems) | **not filed** | Un-enumerable; one enum variant is not a ticket (§3) |

### 5.2 The floor

A file with *n* claimants forces at least *n* waves — it admits one agent per wave. Counted over the
**42 code-bearing rows** (43 minus the wave-0 split):

| File | Claimants | Share | Sibling | Mine |
|---|---|---|---|---|
| **`apps/website/frontend/src/mission_editor.rs`** | **17** | **40%** | 11 | 6 |
| **`apps/website/frontend/src/editor_ops.rs`** | **16** | 38% | 8 | 8 |
| `apps/website/frontend/src/eden_dock_right.rs` *(post-split)* | 7 | 17% | 3 | 4 |
| `crates/map-engine-core/src/doc/store.rs` | 7 | 17% | 2 | 5 |
| `apps/website/frontend/src/eden_top_strip.rs` *(post-split)* | 5 | 12% | 4 | 1 |
| `apps/website/frontend/src/eden_toolbelt.rs` *(post-split)* | 5 | 12% | 4 | 1 |
| `apps/website/frontend/src/main.rs` *(one `mod` line each)* | 5 | 12% | 4 | 1 |
| `apps/website/frontend/src/select_tool.rs` | 4 | 10% | 4 | 0 |
| `crates/map-engine-core/src/mission/validate.rs` *(new)* | 4 | 10% | 4 | 0 |
| `apps/website/frontend/src/eden_tree.rs` *(post-split)* | 3 | 7% | 1 | 2 |
| `apps/website/frontend/src/eden_dock_left.rs` *(post-split)* | 3 | 7% | 2 | 1 |
| `apps/website/frontend/src/context_menu.rs` *(new)* | 3 | 7% | 1 | 2 |
| 8 files at 2 | `attributes` · `outliner` · `asset_catalog` · `dem_vectors` · `dem/sample` · `los_tool` · `engine` · `draw_order` | — | — | — |
| 17 files at 1 | incl. `eden_layout` · `eden_zones` · `eden_settings` · `eden_vehicles_panel` · `dto` · `ui` | — | — | — |

```
floor = max(claimants per file) = 17     (mission_editor.rs)
next constraint down            = 16     (editor_ops.rs)
then                            =  7     (eden_dock_right.rs, doc/store.rs — tied)
+ wave 0, which must run alone   = 18 waves total
```

**43 tickets ÷ 18 waves = mean 2.39 tickets/wave.** Over the 17 code waves alone: **2.47**.

### 5.3 The packing — 18 waves, barrier between

Dependency- and priority-ordered. **Zero single-agent waves except wave 0**, which is alone by
construction.

| Wave | Tickets | n | Files locked |
|---|---|---|---|
| **0** | **T-630.5** *(the `eden_chrome.rs` split)* | 1 | `eden_chrome` → 10 modules + `main.rs`. **Alone.** |
| **1** | **W1-UNBLOCK** · N9 · T-639 | 3 | `mission_editor` \| `dto`+`editor_ops` \| `lod_gates`+`dem_vectors` |
| **2** | **P-3** · N3 · T-640 | 3 | `mission_editor`+**new** `context_menu` \| `store`+`editor_ops`+`outliner`+`eden_tree` \| `dem_vectors`+`contours`+`vector_compose` |
| **3** | T-631 · T-076a · T-641a | 3 | `mission_editor` \| `eden_vehicles_panel`+`editor_ops`+`store`+`eden_dock_right` \| `labels`+`peaks` |
| **4** | T-635 · P-6 · T-656 | 3 | `mission_editor` \| `editor_ops`+`eden_dock_left`+`eden_tree` \| **new** `validate`+`mission/mod` |
| **5** | T-636 · T-646 | 2 | `eden_toolbelt`+`mission_editor` \| `asset_catalog`+`eden_dock_right`+`editor_ops` |
| **6** | T-647 · T-641b | 2 | `mission_editor`+`editor_ops` \| `eden_toolbelt` — T-641b unblocked by T-636 @ w5 |
| **7** | T-638 · T-659 · T-657 | 3 | `eden_layout`+`mission_editor`+`select_tool` \| `eden_top_strip`+`editor_ops` \| `validate` |
| **8** | T-642 · T-650 · T-658 | 3 | `eden_toolbelt`+`mission_editor`+`select_tool`+**new** `ruler_tool` \| `eden_dock_right`+`editor_ops`+`store` \| `validate` |
| **9** | T-643 · T-079a · T-660 | 3 | `eden_toolbelt`+`mission_editor`+**new** `los_tool`+`dem/sample` \| `eden_dock_right`+`eden_zones`+`editor_ops`+`store` \| `validate`+`wire_safety` |
| **10** | T-648 · T-644 | 2 | `mission_editor`+`select_tool`+`editor_ops` \| `los_tool`+`engine`+`dem/sample` — T-644 unblocked by T-643 @ w9 |
| **11** | T-655 · T-645 | 2 | **new** `validation_panel`+`mission_editor` \| `editor_ops`+`eden_top_strip`+**new** `place_helpers` |
| **12** | T-649 · T-632 | 2 | `mission_editor`+`select_tool`+`attributes`+`editor_ops` \| `eden_dock_right` |
| **13** | P-10 · T-082 | 2 | `mission_editor` \| `attributes`+`editor_ops` |
| **14** | T-651 · T-633 | 2 | `store`+`editor_ops`+`outliner`+`mission_editor`+`context_menu` \| `eden_top_strip`+`ui` |
| **15** | P-11 · T-634 | 2 | `eden_toolbelt`+`mission_editor` \| `eden_top_strip` |
| **16** | **T-069⊕T-213** · T-637 | 2 | `eden_dock_right`+`editor_ops`+`mission_editor`+`store`+`draw_order`+`engine` \| `eden_dock_left`+`eden_tree` |
| **17** | T-079d · N1 · T-084 | 3 | `mission_editor`+`editor_ops`+`context_menu`+`store`+`draw_order` \| `eden_settings`+`eden_top_strip`+`create_mission_dialog` \| `asset_catalog`+`eden_dock_right` |

**18 waves · 43 tickets · mean 2.39 · zero single-agent code waves.**

### 5.4 Four things this packing depends on — state them or it breaks

**1. Wave 0 must pre-declare the `mod` stubs.** Five tickets add a line to `main.rs`
(T-642 w8, T-643 w9, T-645 w11, T-655 w11, P-3 w2) — and **T-645 and T-655 share wave 11**. Without
the stubs they collide on one line and wave 11 splits in two, taking the program to 19. The sibling
recommended this for four tickets; it is now load-bearing for five. `main.rs` is 148 lines with 57
`mod` declarations; the stubs cost five lines.

**2. Wave 1's "two quick wins" are one ticket, not two.** The brief names them separately, but
`mission_editor.rs:1027` (the `"Delete" | "Backspace" if !modk` arm) and `:1402` / `:1844-1847`
(the MMB-or-RMB pan branch and the blanket `prevent_default`) are the **same file**. Two agents
cannot share it. Filed as one row, **W1-UNBLOCK**, they cost one slot; filed as two they cost two
waves for perhaps twelve lines of diff.

**3. RMB gates six tickets, and this packing frees it in wave 1.** `interactions_sweep.md:515`
lists P-3 as unblocking `CREW-SEAT-001`, `CONN-START-001`, `CTX-FORMATION-001`, `ATTR-MULTI-001`,
`COMP-SAVE-001` and `KEY-WP-001`. In the table above P-3 runs at **wave 2** and every dependent
runs at wave 6 or later — T-647 (w6), T-650 (w8), T-651 (w14), T-069⊕213 (w16), T-079d (w17).
**No dependent precedes its gate.** Verified row by row.

**4. P-12 (`Ctrl+F`) is deferred, and that is a deliberate one-wave purchase.** It would be the
**18th** `mission_editor.rs` claimant and therefore the 19th wave, for a shortcut that carries no
`interactions.md` id (it comes from the screenshot corpus, `batch02:586`). If the operator wants it,
the cheapest home is **W1-UNBLOCK at wave 1** — that ticket already owns the keydown and the
`preventDefault`, and only the focus-target plumbing into `eden_dock_right.rs` would be new. Filing
it as its own row costs a full wave; folding it into T-084 costs a full wave. Folding it into
wave 1 costs one file.

### 5.5 Where this packing differs from the sibling's, and why

The sibling's §5.2 put T-632 at wave 0, T-633 at wave 1 and T-655 at wave 10. Here T-632 is wave 12,
T-633 is wave 14, T-655 is wave 11. **Nothing moved on merit** — they moved because 14 new rows
entered the same file graph and the `eden_chrome.rs` split redistributed the chrome claimants. The
one deliberate ordering change is that **the unblock wave went first**, which the sibling's set had
no reason to model.

The sibling's §5.2 also flagged a product-order violation it chose to state rather than hide:
§D.3 wants **T-655 (validation panel) to ship first** and both packings put it late (their wave 10,
my wave 11). The file graph would permit T-655 as early as wave 3. **That call is unchanged and
still open** — it is a product decision, not a scheduling one, and it costs zero waves either way.

---

## 6. The new binding constraint

```
Before the split (sibling, 27 tickets) : eden_chrome.rs        14 claimants → 14 waves
After  the split (sibling, 27 tickets) : mission_editor.rs     11 claimants → 11 waves
After  the split (combined, 42 tickets): mission_editor.rs     17 claimants → 17 waves (+ wave 0)
```

**`apps/website/frontend/src/mission_editor.rs` is the new binding constraint — 17 of 42
code-bearing tickets, 40%.** The sibling predicted this file would inherit the bottleneck the
moment `eden_chrome.rs` stopped being it, and adding the parity program made it worse, not better:
six of my fourteen rows claim it, because **every map gesture and every keyboard binding in the
editor lives in one file** — `onwheel` `:1333`, `onpointerdown` `:1395`, `onpointermove` `:1437`,
`onpointerup` `:1644`, `oncontextmenu` `:1844`, `ondblclick` `:1934`, `onresize` `:1972`, and the
editor keydown at `:1005-1030`. Eden parity is, almost by definition, input work.

**The important number is the second one, and it changes the recommendation.**

```
editor_ops.rs = 16 claimants (38%)
```

The sibling's §7.2 costed a `mission_editor.rs` gesture-host extraction at **14 → 8 waves (−43%)**
against its 27-ticket set. **Against the combined 42, that arithmetic no longer holds.** Splitting
`mission_editor.rs` alone moves the floor from **17 to 16** — `editor_ops.rs` is one claimant
behind it — which is **−6%, not −43%**. One wave, for a split the sibling itself graded *"materially
riskier"* than the `eden_chrome.rs` one (a live gesture state machine with deliberately leaked
closures at `:1998`/`:1999` and a documented single-pointer invariant).

**So the recommendation inverts: split both or split neither.**

```
split mission_editor.rs only        : floor 16  (−6%)     ← not worth the risk
split editor_ops.rs only            : floor 17  ( 0%)     ← buys literally nothing
split BOTH                          : floor ~8  (−53%)   ← the whole prize
```

with the tier-2 floor at **7** — `eden_dock_right.rs` and `crates/map-engine-core/src/doc/store.rs`
are tied, so no amount of further splitting takes this program below about seven waves while those
two files are each one file.

`editor_ops.rs` is the better first cut of the two, and the sibling's *"do not split `editor_ops.rs`
yet"* was correct **for its 27 tickets** and is wrong for these 42. Its 68 `pub fn` already cluster
by concern along lines the parity tickets exposed: **clipboard** (`copy_selection` `:394`,
`paste_at_cursor` `:436`, `delete_selection` `:327`) · **placement** (`begin_place*` `:1037-1049`,
`place_at` `:2191`) · **attributes** (`read_attrs` `:631`, `attrs_update_position` `:667`,
`attrs_update_slot` `:796`, and the two suppression guards `:583-585` / `:605-607`) · **layers**
(`layer_rows` `:824`, `set_active_layer` `:1025`, `ensure_layer` `:1125`) · **vehicles** (nine
functions, `:1044`-`:1908`) · **zones** (ten functions, `:2325`-`:2717`). Six groups, and the
16 claimants distribute across them roughly 3 / 4 / 3 / 2 / 2 / 2 — **max 4**, which is the number
that matters. It is a mechanical extraction with no state machine in it, and unlike the gesture host
it cannot break a live invariant.

**Recommended order, costed:**

| Step | Floor after | Risk |
|---|---|---|
| Wave 0 — `eden_chrome.rs` split *(already planned)* | 17 | none; pure move |
| Then — `editor_ops.rs` split by concern | 17 *(no gain alone)* | low; no state machine |
| Then — `mission_editor.rs` gesture-host + keydown extraction | **~8** | **high**; leaked closures, single-pointer invariant |

The middle row buys nothing on its own and everything in combination. **Do not let it be dropped on
the grounds that it shows a zero.**

---

## Appendix — machine-readable `owns` (paste-ready column 4)

Tab-separated `ticket <TAB> owns`, semicolon-space separated, repo-root-relative. **Post-split
paths.** Wave numbers deliberately omitted; §5.3 has them. The sibling's appendix covers
T-631…T-660 pre-split — **where the two disagree, this one is post-split and wins.**

```
T-630.5	apps/website/frontend/src/eden_chrome.rs; apps/website/frontend/src/eden_layout.rs; apps/website/frontend/src/eden_top_strip.rs; apps/website/frontend/src/eden_env.rs; apps/website/frontend/src/eden_tree.rs; apps/website/frontend/src/eden_dock_left.rs; apps/website/frontend/src/eden_dock_right.rs; apps/website/frontend/src/eden_vehicles_panel.rs; apps/website/frontend/src/eden_zones.rs; apps/website/frontend/src/eden_toolbelt.rs; apps/website/frontend/src/eden_settings.rs; apps/website/frontend/src/main.rs
W1-UNBLOCK	apps/website/frontend/src/mission_editor.rs
N1	apps/website/frontend/src/eden_settings.rs; apps/website/frontend/src/eden_top_strip.rs; apps/website/frontend/src/create_mission_dialog.rs
N3	crates/map-engine-core/src/doc/store.rs; apps/website/frontend/src/editor_ops.rs; apps/website/frontend/src/outliner.rs; apps/website/frontend/src/eden_tree.rs
N9	apps/website/frontend/src/dto.rs; apps/website/frontend/src/editor_ops.rs
P-3	apps/website/frontend/src/mission_editor.rs; apps/website/frontend/src/context_menu.rs
P-6	apps/website/frontend/src/editor_ops.rs; apps/website/frontend/src/eden_dock_left.rs; apps/website/frontend/src/eden_tree.rs
P-10	apps/website/frontend/src/mission_editor.rs
P-11	apps/website/frontend/src/eden_toolbelt.rs; apps/website/frontend/src/mission_editor.rs
T-069	apps/website/frontend/src/eden_dock_right.rs; apps/website/frontend/src/editor_ops.rs; apps/website/frontend/src/mission_editor.rs; crates/map-engine-core/src/doc/store.rs; crates/map-engine-render/src/draw_order.rs; crates/map-engine-render/src/engine.rs
T-076a	apps/website/frontend/src/eden_vehicles_panel.rs; apps/website/frontend/src/editor_ops.rs; crates/map-engine-core/src/doc/store.rs; apps/website/frontend/src/eden_dock_right.rs
T-079a	apps/website/frontend/src/eden_dock_right.rs; apps/website/frontend/src/eden_zones.rs; apps/website/frontend/src/editor_ops.rs; crates/map-engine-core/src/doc/store.rs
T-079d	apps/website/frontend/src/mission_editor.rs; apps/website/frontend/src/editor_ops.rs; apps/website/frontend/src/context_menu.rs; crates/map-engine-core/src/doc/store.rs; crates/map-engine-render/src/draw_order.rs
T-082	apps/website/frontend/src/attributes.rs; apps/website/frontend/src/editor_ops.rs
T-084	apps/website/frontend/src/asset_catalog.rs; apps/website/frontend/src/eden_dock_right.rs
```

**Empty deliberately** — `T-213` (merged into T-069), `T-077` / `T-072` (superseded by T-647),
`T-078` (merged into T-650), `T-079b` (excluded, §4), `T-079c` (not filed, §3), `T-076b`
(workbench). An empty column 4 on those rows must not be read as "not yet derived".

**The sibling's rows, retargeted to post-split modules** (only the six that named `eden_chrome.rs`
change; the rest are unchanged from `owns_and_waves.md`'s appendix):

```
T-632	apps/website/frontend/src/eden_dock_right.rs
T-633	apps/website/frontend/src/eden_top_strip.rs; apps/website/frontend/src/ui.rs
T-634	apps/website/frontend/src/eden_top_strip.rs
T-636	apps/website/frontend/src/eden_toolbelt.rs; apps/website/frontend/src/mission_editor.rs
T-637	apps/website/frontend/src/eden_dock_left.rs; apps/website/frontend/src/eden_tree.rs
T-638	apps/website/frontend/src/eden_layout.rs; apps/website/frontend/src/mission_editor.rs; apps/website/frontend/src/select_tool.rs
T-641b	apps/website/frontend/src/eden_toolbelt.rs
T-642	apps/website/frontend/src/eden_toolbelt.rs; apps/website/frontend/src/mission_editor.rs; apps/website/frontend/src/select_tool.rs; apps/website/frontend/src/ruler_tool.rs
T-643	apps/website/frontend/src/eden_toolbelt.rs; apps/website/frontend/src/mission_editor.rs; apps/website/frontend/src/los_tool.rs; crates/map-engine-core/src/dem/sample.rs
T-645	apps/website/frontend/src/editor_ops.rs; apps/website/frontend/src/eden_top_strip.rs; apps/website/frontend/src/place_helpers.rs
T-646	apps/website/frontend/src/asset_catalog.rs; apps/website/frontend/src/eden_dock_right.rs; apps/website/frontend/src/editor_ops.rs
T-650	apps/website/frontend/src/eden_dock_right.rs; apps/website/frontend/src/editor_ops.rs; crates/map-engine-core/src/doc/store.rs
T-651	crates/map-engine-core/src/doc/store.rs; apps/website/frontend/src/editor_ops.rs; apps/website/frontend/src/outliner.rs; apps/website/frontend/src/mission_editor.rs; apps/website/frontend/src/context_menu.rs
T-659	apps/website/frontend/src/eden_top_strip.rs; apps/website/frontend/src/editor_ops.rs
```

`main.rs` is **omitted from every row above** on the assumption that wave 0 pre-declares the five
`mod` stubs (§5.4 item 1). If it does not, add
`apps/website/frontend/src/main.rs` to P-3, T-642, T-643, T-645 and T-655 — and split wave 11.

---

## What was not verified

Stated so nothing here is mistaken for checked fact.

- **Three sibling rows look under-claimed post-split, and I did not widen them.**
  `owns_and_waves.md` §5.3 assigns each chrome ticket exactly **one** post-split module, but §2 of
  the same document names symbols spanning several: **T-636** also names `TOOLBELT_BAND_PX` `:45`
  (→ `eden_layout.rs`), **T-637** also names `DockRight` `:2961-3379` (→ `eden_dock_right.rs`), and
  **T-638** collapses both docks, not just the constants (→ `eden_dock_left.rs` +
  `eden_dock_right.rs`). I followed §5.3 because the brief names it as the post-split claimant
  table. **If the operator widens those three rows, waves 5, 7, 12 and 16 need re-checking** —
  T-637 would then collide with T-069⊕213 at wave 16, and T-638 with T-632 nowhere but T-650 at
  wave 8 is close. This is the single most likely place this packing breaks.
- **`eden_settings.rs` for N1 is a product call, not a search result.** The library-dossier
  alternative (`mission_overview.rs` alone, collision-free) is graded equally defensible in §2.1.
- **`context_menu.rs`, and P-3's design generally, is `low`.** No ticket has named the module. Three
  rows claim it (P-3, T-651, T-079d), so a wrong guess costs a re-pack, not a wave.
- **The marker render path is `medium`, and it is the one I would check first.** I claimed
  `draw_order.rs` + `engine.rs` because `upload_icon_lane` `:3555` is a closed three-arm match and
  no `LaneRole` marker variant exists. If markers can be injected into the slot SoA that
  `editor_ops.rs` already uploads via `core.materialize()`, **both crate paths drop** and
  T-069⊕213 stops touching `map-engine-render` at all. I did not trace `slots_gpu` far enough to
  settle it.
- **T-079c is unsized, not small.** `attributes_sweep.md:536` records that the `SYS` family declares
  no ids at all. Its one-variant `owns` is a lower bound with no upper bound.
- **T-650's storage location remains the sibling's open question**, and the `COMP-*` metadata makes
  it sharper rather than settling it (§3).
- I did **not** re-verify the sibling's 28 rows against source. Their line cites are taken as given;
  only their *module assignment* was re-derived, and only where `eden_chrome.rs` was named.
