# Mission Creator — Eden Editor UX Spec
**Document:** `04_eden_ux_spec.md`
**Status:** Human-readable UX contract. **Execution authority is `05_agent_execution_plan.md`** —
this file restates the locked UX so engineers and reviewers can read the *intended behaviour*
without parsing the phase plan. If this file and `05_agent_execution_plan.md` disagree, **05 wins**.

> **Visual target:** **Arma 3 Eden Editor** layout + interactions, **modernized with Aegis glass**
> (macOS). **Not** the historical HTML mockups under `Mission_Creator_Mock_Up/` or
> `macOS_Blueprints/` — those explorations contradict each other and the Decisions log below.

---

## Layout

Fullscreen editor — the platform `Sidebar` + `TopNav` are **hidden** on `/missions/:id/edit`.
Left and right panels are **docked flush** to the viewport edges; the map fills the space between.

```
┌─────────────────────────────────────────────────────────────┐
│ TopCommandStrip (h-12) — NO platform TopNav/Sidebar          │
├──────────┬──────────────────────────────────────┬───────────┤
│ Left     │         TacticalMap                  │ Right     │
│ w-64     │         ml-64 mr-80                  │ w-80      │
│ ORBAT +  │         topo + grid overlay          │ Asset     │
│ Layers   │         [BottomToolbelt in map area] │ Palette   │
└──────────┴──────────────────────────────────────┴───────────┘
```

- **Left sidebar (`w-64` / 256px):** one scroll with **ORBAT** (Factions→Squads→Slots) on top and
  **Editor Layers** (workflow folders) below; stub sections for Waypoints / Zones / Logic until Phase 8.
- **Right Asset Palette (`w-80` / 320px):** **always visible** — never swaps to an inspector.
  Tabs (Factions / Vehicles / Markers / Objectives); 2-col grid cards drill down into an
  Eden-style nested tree (Men → Rifleman). Drag a leaf onto the map to place it.
- **Top Command Strip:** mission title (inline edit), File/Edit/View/Mission/Environment menu stubs,
  Eden-style **time slider/scrub** + weather, Undo/Redo, Export (disabled until Phase 9), settings gear.
- **Bottom Toolbelt:** Select / Ruler / LoS + live X/Y/Z readout in JetBrains Mono, centered in the map area.
- **Attributes Modal:** opens on **double-click** of a single entity (Transform / Identity / States / Arsenal tabs).

---

## Interaction contract

| User action | System response |
|-------------|-----------------|
| Drag asset from palette | Place entity on map; file in active Editor Layer |
| Single-click entity | Select + highlight + outliner sync (**no** camera move) |
| Double-click entity | Open **AttributesModal** (Transform, Identity, States, Arsenal tabs) |
| Click-drag entity on map | Move entity; one undo step on release |
| Left-drag on empty map | Marquee box-select |
| Middle-mouse / right-drag | Pan/zoom map |
| Click-drag selected group | Move all selected together; one undo step |
| Spacebar | Center camera on selection |
| Delete / Backspace | Delete selected entities (undoable) |
| Click empty map | Clear selection only |
| Click outliner row | Select entity (**no** camera move until Spacebar) |

---

## Decisions log (human-confirmed — do not re-litigate without user approval)

| Topic | Decision |
|-------|----------|
| **Visual target** | **Arma 3 Eden Editor** layout + interactions, **modernized with Aegis glass** (macOS). Not HTML mockups. |
| **Platform chrome** | **Hide** platform `Sidebar` + `TopNav` on `/missions/:id/edit` — true fullscreen Eden-style editor (dedicated layout escape in `AppLayout` or editor wrapper). |
| **Left sidebar** | **Both sections visible** in one scroll: **ORBAT** (Factions→Squads→Slots) on top, **Editor Layers** (workflow folders) below. Stub sections for Waypoints/Zones/Logic until Phase 8. |
| **Right palette** | **Docked flush right** — mirror left sidebar (~`w-80` / 320px), full height below top bar, no floating gap. Map sits between two glass panels. |
| **Inspector** | Asset Palette always visible. **Attributes modal on double-click only** (no right-panel inspector swap). |
| **Map pan** | **Middle-mouse or right-drag** = pan/zoom. **Left-drag on empty map** = marquee box-select. |
| **Multi-select** | **Marquee box** is the primary multi-select method. Shift+click additive toggle is optional bonus, not required for v1. |
| **Center camera** | **No auto flyTo on click.** Select unit → press **Spacebar** to center camera on selection (map or outliner). |
| **Delete** | **Delete/Backspace** removes selected entities; **undoable** (one transaction). No confirmation dialog. |
| **Load conflict** | When API `json_payload` and local IndexedDB disagree → **prompt user** to choose which to keep. |
| **Autosave** | **Debounced autosave** overwrites a single server **draft** on the mission. **Undo** = in-session. Manual **Save Version** creates semver snapshots for future Visual-Git/history. |
| **Time of day** | Match **Arma 3 Eden** environment control (slider/scrub in environment UI — not preset-only dropdowns). Expose quick readout in top bar; fine control in Mission Settings. |
| **Phase order** | **Commit/finish uncommitted tree wiring FIRST** (pre-3.5), then DOC-0 → 3.5 → 7b → 7a → 9. |

---

## Fullscreen layout escape (implementation note)

The editor must render **without** the platform `Sidebar` + `TopNav`. `AppLayout`
(`frontend/src/components/layout/AppLayout.tsx`) currently always renders both; its `fullBleed`
route handle only changes the `<main>` padding/overflow, not the chrome. Phase 3.5 adds a chrome
escape — e.g. an `editorFullscreen` (or reused `fullBleed`-strict) route handle that suppresses
`<Sidebar/>` + `<TopNav/>`, **or** the editor route mounts outside the `AppLayout` `<Outlet/>`
entirely. Either way, `/missions/:id/edit` owns the full viewport.
