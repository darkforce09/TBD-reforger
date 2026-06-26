# FEDS — Feature Entry Documentation Schema

**Document:** `reference/feds_schema.md`  
**Status:** Normative. All entries in `feature_inventory.md` and `eden/interactions.md` **must** follow this schema. Gap analysis `eden/gap_analysis.md` links rows by `id` only.

---

## Purpose

FEDS exists so an agent (or engineer) can implement a feature **without guessing**:

- **Goal** — why it exists  
- **Trigger** — exact user/system event  
- **Procedure** — numbered internal steps  
- **Acceptance** — verifiable smoke tests  
- **Evidence** — code path (TBD) or cited source (Eden)

If a field cannot be confirmed, write `UNVERIFIED` and what must be checked.

---

## Feature ID taxonomy

```
{DOMAIN}-{SUBDOMAIN}-{NNN}
```

| Domain | Scope |
|--------|--------|
| `SHELL` | Route, layout, chrome, load lifecycle |
| `MAP` | Canvas viewport, grid, camera, cursor |
| `SEL` | Selection (click, marquee, modifiers, modes) |
| `XFORM` | Move, rotate, snap, align, sync, delete |
| `PLACE` | Drag-from-palette, spawn defaults, click-place |
| `LEFT` | Left sidebar (ORBAT, layers, tabs) |
| `RIGHT` | Asset palette / browser |
| `TOP` | Command strip, menus, save/export |
| `BOTTOM` | Toolbelt, coordinates HUD |
| `ATTR` | Attributes modal / entity properties |
| `KEY` | Keyboard shortcuts |
| `DATA` | Persistence, hydrate, compiler, undo |
| `ENV` | Time, weather, view distance, fog |
| `ORBAT` | Factions, squads, groups, slots (export truth) |
| `LAYER` | Editor workflow folders (TBD Y.Doc `editorLayers`) |
| `WP` | Waypoints |
| `TRG` | Triggers / modules |
| `MRK` | Markers |
| `VEH` | Vehicles |
| `MEAS` | Ruler, elevation, distance |
| `FILE` | New/open/save/session menus |
| `COMP` | Custom compositions (save/edit/workshop) |
| `TOOLBAR` | Workspace toolbar buttons |
| `WIDGET` | Transformation widget variants |
| `CTX` | Entity context menu entries |
| `CONN` | Connection / sync types |
| `MENU` | Menu bar items |
| `RIGHT` | Asset browser (use `RIGHT-MODE`, `RIGHT-SEARCH` subdomains) |

**Subdomain** is a short grouping token (`MAP`, `MOD`, `TREE`, `PAL`, …). **NNN** is zero-padded `001`…`999` per subdomain.

**Extended ID patterns (Eden `07`):**

| Pattern | Use |
|---------|-----|
| `RIGHT-MODE-00N` | Asset browser F1–F6 modes |
| `RIGHT-SEARCH-00N` | Search syntax behaviors |
| `COMP-00N` | Custom compositions |
| `TOOLBAR-00N` | Toolbar buttons |
| `WIDGET-00N` | Transformation widget |
| `CTX-00N` | Context menu entries |
| `CONN-00N` | Connection types |
| `ATTR-FIELD-{TYPE}-{NAME}` | Single attribute field in `07b` (e.g. `ATTR-FIELD-OBJ-ALLOWDAMAGE-001`) |
| `MENU-{MENU}-{ITEM}` | Menu bar items |
| `ACTION-{NAME}` | Engine action from [Eden Actions](https://community.bistudio.com/wiki/Eden_Editor:_Actions) wiki |

**One behavior = one ID.** Do not bundle “selection” into one row — split click, marquee, Shift-add, Ctrl-toggle, etc.

**Attribute catalog rule:** Attribute **fields** live in `eden/attributes.md` as `ATTR-FIELD-*` rows. `eden/interactions.md` references them; do not inline 200+ fields in the interaction reference.

---

## Glossary (define once per inventory doc)

Each of `06` and `07` opens with a glossary. Terms **must not** be conflated across docs.

### TBD (`06`) — required terms

| Term | Definition |
|------|------------|
| **Entity** | Any placeable mission object (slot, vehicle, marker, objective, …) stored in Y.Doc entity maps |
| **Slot** | A placed unit in `slotsById`; export row in `orbat[].slots[]` |
| **ORBAT** | Export-truth hierarchy: Faction → Squad → Slot (`compile.ts` → `orbat[]`) |
| **Editor Layer** | Workflow folder in `editorLayers`; does **not** map 1:1 to Eden Layer; filed in `editor` block for reload |
| **Active layer** | `activeLayerId` — drop target for new placements |
| **Selection** | `useMapStore.selection: { kind, ids[] }` |
| **Squad** | ORBAT grouping of slots (`squadsById`); auto-created on first placement |
| **Faction** | ORBAT top level (`factionsById`) |

### Eden (`07`) — required terms

| Term | Definition | Source |
|------|------------|--------|
| **Entity** | Object, group, trigger, waypoint, system, or marker in the scenario | [Eden Terminology](https://community.bistudio.com/wiki/Eden_Editor:_Terminology) |
| **Asset** | Browser entry before placement | same |
| **Entity List** | Left panel listing all scenario entities | same |
| **Layer** | Folder containing entities; hide/show via attributes | [Eden Layer](https://community.bistudio.com/wiki/Eden_Editor:_Layer) |
| **Group** | Multiple units with a leader | Terminology |
| **Sync / Connect** | General connection between entities (modules, tasks, triggers) | [Connecting](https://community.bistudio.com/wiki/Eden_Editor:_Connecting) |
| **View** | 3D or map camera workspace | Terminology |

---

## Mandatory feature block

Copy this structure for **every** feature. Heading level: `#### {ID} — {Short name}`.

```markdown
#### {ID} — {Short name}

| Field | Value |
|-------|-------|
| **Domain** | e.g. SEL-MAP |
| **Goal** | User outcome or Eden parity reason |
| **Trigger** | Exact event (e.g. "LMB down on empty map, drag >4px, release") |
| **Preconditions** | Tool, permissions, doc loaded, selection state, … |
| **Procedure** | 1. … 2. … 3. … (UI → handler → state → feedback) |
| **Postconditions** | State after success |
| **Inputs** | Mouse buttons, keys, modifiers, drag MIME types |
| **Outputs** | Visual feedback + Y.Doc / API writes |
| **Edge cases** | Cancel paths, empty selection, limits, conflicts |
| **Acceptance** | `- [ ]` verifiable bullets |
| **Eden parity** | `Eden:{ID}` \| `N/A ({reason})` \| `TBD-only` |
| **Status** | *(06 only)* `working` \| `partial` \| `stub` \| `disabled` \| `not_built` |
| **Ticket** | *(06, optional)* Shipped `T-0xx` when a registry ticket owns the slice (e.g. `T-049`) |
| **Evidence** | *(06)* file paths + symbols; *(07)* URL or `UNVERIFIED` |
| **UI Surface** | *(07 required)* `MenuBar` \| `Toolbar` \| `AssetBrowser` \| `EntityList` \| `View` \| `AttributesDialog` \| `ScenarioAttributes` \| `ContextMenu` \| `StatusBar` \| `ConnectionLine` \| `—` |
| **Feature kind** | *(07 required)* `interaction` \| `ui_chrome` \| `attribute_field` \| `connection_type` \| `browser_mode` \| `engine_action` |
| **Wiki anchor** | *(07 required)* Full URL + `#Section_Heading` (exact scrape source) |
| **Shortcut** | Explicit key combo or `—` |
| **Parent ID** | Optional grouping (e.g. `TOOLBAR-SNAP-001` under `TOOLBAR-GRID-001`) |
```

### Field rules

| Field | Rule |
|-------|------|
| **Trigger** | Use explicit modifiers: `Ctrl`, `Shift`, `Alt`, `LMB`, `RMB`, `MMB` — never "modifier click" |
| **Procedure** | Must be code-backed in 06; wiki/manual-backed in 07 |
| **Acceptance** | At least one checkbox; complex features need 3+ |
| **Evidence** | 06: `path/to/file.ts` → `functionName`; 07: full wiki URL |
| **Eden parity** | 06 links to 07 id when applicable |

---

## Writer rules (agents)

1. **Grep/read before write** — 06 `Procedure` must match implementation.  
2. **No guessed Eden** — 07 without cite → `Evidence: UNVERIFIED`.  
3. **Never conflate** ORBAT, Editor Layer, Eden Layer, Eden Group.  
4. **Visible stubs count** — disabled buttons and "(soon)" menus get IDs with `stub` or `disabled`.  
5. **08 uses IDs only** — no free-text feature names in gap rows.

---

## Optional YAML index

At the top of `06` or `07`, an optional machine index:

```yaml
# feature-index (optional)
features:
  - id: SEL-MAP-003
    name: Marquee box-select
    domain: SEL
    status: working   # 06 only
```

The markdown FEDS block remains authoritative.

---

## Gap analysis row format (`eden/gap_analysis.md`)

```markdown
| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|--------|-----------|
| SEL-MAP-003 | SEL-MAP-003 | match | — | |
| SEL-MOD-001 | SEL-MOD-001 | match | T-053 | Ctrl+LMB additive select |
```

**Parity:** `match` | `partial` | `missing` | `deferred` | `na` | `tbd_only`

**Ticket:** Registry `T-0xx` when queued or shipped; `—` when not ticketed. Open Eden backlog: [`docs/TICKET_LEAD.md`](../../../docs/TICKET_LEAD.md). Deferred map/DEM infra: **T-090** (aligned tiles), **T-091** (DEM + Z-axis). Terrain base at scale: **T-110** ([`t110_terrain_base_mission_layers.md`](../t110_terrain_base_mission_layers.md)).

---

## Document map

| File | Role |
|------|------|
| `reference/feds_schema.md` | This file — normative schema (FEDS v2) |
| `feature_inventory.md` | TBD implementation inventory |
| `eden/ui_anatomy.md` | Eden workspace layout — what you see per panel |
| `eden/attributes.md` | Per-entity-type attribute fields (`ATTR-FIELD-*`) |
| `eden/interactions.md` | Eden interaction reference (wiki-anchored) |
| `eden/gap_analysis.md` | ID-linked parity matrix + backlog |
| `eden/wiki_manifest.yaml` | Wiki pages to scrape + status |
| `artifacts/eden-wiki/` | Scraper raw page cache |
| `artifacts/eden-feds-draft.jsonl` | Scraper draft FEDS candidates |
