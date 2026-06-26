# T-057 — Map performance hotfix (≥55 fps pan/zoom @ 200+ slots)

**Status:** shipped (T-057)
**Git tag on ship:** T-057
**Authority:** [MC ROADMAP](ROADMAP.md) §Map performance (contract + scale program) · [engineering_plan.md](engineering_plan.md) §4.4 · [agent_execution.md](agent_execution.md)

---

## Goal

Restore the engineering-plan performance contract: **60 fps pan/zoom with 200+ pickable
slot icons** on the flat grid. A 2026-06 regression dropped this to **~9 fps at ~100–200
slots**. T-057 is an **interrupt hotfix** that must land before Eden **T-068+** or
the T-059..T-067 scale program.

**Acceptance:** ≥55 fps sustained pan/zoom with 200+ slots (read from the existing
`FpsCounter`, green ≥55); click-select, Ctrl/Cmd-toggle, marquee, drag-move,
double-click→Attributes, Ctrl+C/V copy-paste, Spacebar center, Delete all behave exactly as
before; `cd frontend && npm run build && npm run lint` clean.

Deck.gl already draws every icon on the GPU (`IconLayer`) — it can draw 100k+. The
bottleneck is the **React/DOM layer**, not the renderer.

---

## Root causes (confirmed in code; matches MC ROADMAP §Map performance table)

| # | Bottleneck | Evidence | Fix |
|---|------------|----------|-----|
| 1 | `onHover` → `setCursor` re-renders the **entire** `MissionCreatorPage` (both Outliner trees, asset palette, toolbelt) on **every pointer move** | `MissionCreatorPage` `onCursorMove={setCursor}` → `useState` cursor | Route cursor through the engine's transient store; only `BottomToolbelt` re-renders |
| 2 | Deck runs a **GPU hover pick pass** over all icons every move, only to feed cursor coords | `<DeckGL onHover>` + `IconLayer pickable:true` | Remove `onHover`; compute cursor by unprojecting the mouse ourselves (rAF-throttled) |
| 3 | Pan re-renders `TacticalMap` every frame | `useOrthographicView` holds `viewState` in `useState`; `useSelectTool` pan calls `onViewStateChange` per `pointermove` | rAF-coalesce pan to one `setViewState` per frame; layers already memoized |
| 4 | `getCursor={({isHovering})…}` keeps hover-picking alive | `TacticalMap` `getCursor` 2nd path | Constant `getCursor` (no `isHovering`) |

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Cursor channel | The X/Y/Z read-out is consumed **only** by `BottomToolbelt`. Move it into the engine's transient store (`useMapStore.cursor`, alongside `selection`/`drag`/`marquee`) so the page never re-renders on mouse move. |
| Cursor source | Self-unproject the mouse (`view.makeViewport(...).unproject([px,py])` — same math as `onDrop`), **not** Deck's hover pick. `z: 0` on the flat map (until Phase 2 DEM, unchanged from T-050). |
| Throttle | Cursor emit is **rAF-throttled** (one update per frame max), reading the latest `viewState` via a ref. |
| Off-map "—" | `onPointerLeave` → `setCursor(null)`. Over a docked panel the pointer leaves the map container, so the toolbelt shows `—` (matches prior behavior). |
| Hover feedback | Picking is kept **only** for click / dbl-click / marquee / drag-start. The pointer no longer changes to a "pointer" cursor over an icon (Deck `isHovering` is no longer computed per move). **Accepted minor regression** — ROADMAP-sanctioned ("pick only on click/drag"). `getCursor` is a constant `'crosshair'`. |
| Pan | rAF-coalesce `setViewState` so high-rate mice can't outrun the display; `TacticalMap`'s layers are memoized so a coalesced re-render reuses layer instances (no per-frame rebuild). |
| Memo | `React.memo` the tree/panel-bearing children + the `TacticalMap` engine so a non-cursor page re-render (modal open/close, dirty flag) can't cascade into the Outliner trees or map. |
| Scope | Perf only. No schema/compiler/backend change. All interactions unchanged. |

---

## Implementation specification

### a. `frontend/src/features/tactical-map/state/useMapStore.ts`

Add a transient UI field next to `drag`/`marquee`:

```ts
/** Live cursor world position (meters) for the toolbelt read-out; null off-map. Transient
 *  UI state — never written to the Y.Doc. Set rAF-throttled from TacticalMap. */
cursor: { x: number; y: number; z: number } | null
setCursor: (cursor: { x: number; y: number; z: number } | null) => void
```

Initial `cursor: null`; `setCursor: (cursor) => set({ cursor })`; include `cursor: null` in
`reset()`.

### b. `frontend/src/features/tactical-map/TacticalMap.tsx`

- **Remove** the `onHover` callback and the `<DeckGL onHover={…}>` prop.
- **`getCursor`** → constant: `getCursor={() => 'crosshair'}` (no `isHovering` → Deck stops
  per-move hover picking).
- **Self-computed cursor**, rAF-throttled, on the gesture-host container:
  - Keep `viewState` in a ref (`viewStateRef`) updated each render so the rAF closure reads
    the latest camera.
  - `emitCursor(e)`: store the latest client coords; if no rAF pending, schedule one that
    builds a viewport (`view.makeViewport({ width, height, viewState: viewStateRef.current })`),
    unprojects `[clientX-left, clientY-top]`, and calls `onCursorMove({ x, y, z: 0 })`.
  - Compose the container `onPointerMove` to call `selectTool.onPointerMove(e)` **then**
    `emitCursor(e)`.
  - `onPointerLeave` → cancel any pending rAF and `onCursorMove(null)`.
  - Cleanup the rAF on unmount.

### c. `frontend/src/features/tactical-map/tools/useSelectTool.ts`

rAF-coalesce the **pan** branch of `onPointerMove`: instead of calling `onViewStateChange`
synchronously every event, stash the computed target in a ref and schedule a single rAF that
flushes the latest target via `onViewStateChange`. Cancel the pending rAF on `pointerup`
(and when a non-pan gesture takes over). Move/marquee branches are unchanged (they write to
the store, which only re-renders the icon layer's data — cheap).

### d. `frontend/src/features/mission-creator/MissionCreatorPage.tsx`

- Delete the `cursor` `useState`, the `cursorRef`, and its mirroring `useEffect`.
- `onCursorMove={(c) => useMapStore.getState().setCursor(c)}` — a stable inline that does
  **not** subscribe the page to `cursor`.
- Ctrl+V paste anchor reads `useMapStore.getState().cursor` (was `cursorRef.current`).
- `<BottomToolbelt />` — drop the `cursorWorld` prop.
- Stabilize `AttributesModal`'s `onOpenChange` with `useCallback`.

### e. `frontend/src/features/mission-creator/layout/BottomToolbelt.tsx`

- Drop the `cursorWorld` prop; read `const cursorWorld = useMapStore((s) => s.cursor)`.
- Everything else unchanged (SEL/CUR logic, formatting).

### f. `React.memo` hardening

Wrap with `React.memo` (default-export or named, matching each file's style):
`TacticalMap`, `LeftSidebar`, `AssetPalette`, `TopCommandStrip`, `BottomToolbelt`,
`AttributesModal`. Props to these are referentially stable (`md`, `undo`, `setAttributesId`,
the now-`useCallback`'d `onOpenChange`).

---

## What is intentionally unchanged

- Every interaction: left-click select, Ctrl/Cmd-click toggle (T-053), marquee box-select,
  drag-move + undo (T-036), double-click→Attributes on map & Outliner (T-054), Ctrl+C/V
  paste-at-cursor (T-056), Spacebar center, Delete.
- The Y.Doc schema, compiler, export, and all backend contracts.
- `BottomToolbelt`'s SEL (selected slot) vs CUR (cursor) read-out and the off-map `—`.

**Only behavioral change:** the mouse pointer no longer switches to a "pointer" glyph when
hovering an icon (no per-move hover pick). Click/dbl-click/marquee/drag picking is untouched.

---

## Verification

1. `cd frontend && npm run build && npm run lint` — clean.
2. `make web`; `dev-login` as `mission_maker`; open `/missions/:id/edit`.
3. **Generate 200+ slots** (verification only — *not committed*): copy-paste doubling (drop
   one slot, Ctrl+C, Ctrl+V repeatedly 1→2→4…→256), or a throwaway `import.meta.env.DEV`
   effect looping `addSlot(md, …)` ~250× (removed before commit).
4. Pan (middle/right-drag) and zoom while watching `FpsCounter`: **≥55 fps sustained** (was
   ~9). Idle hover also stays green.
5. Regression pass (all unchanged): select / Ctrl-toggle / marquee / drag-move+undo /
   dbl-click→Attributes (map + Outliner) / Ctrl+C/V / Space / Delete. BottomToolbelt shows
   CUR X/Y/Z while hovering, SEL X/Y/Z with one slot selected, `—` off-map.

---

## Docs synced (same commit)

`CLAUDE.md` §Status (T-057 bullet + latest-feature line), MC [ROADMAP.md](ROADMAP.md)
§Map performance (T-057 row → done), [docs/TAGS.md](../../docs/TAGS.md),
[docs/frontend/ROADMAP.md](../../docs/frontend/ROADMAP.md),
[frontend/docs/pages/mission-editor.md](../../frontend/docs/pages/mission-editor.md),
[agent_execution.md](agent_execution.md) Decisions log + todo status.

## After T-057

**T-058** — toolbelt entity-count telemetry. **T-059** bulk paste ✅ (**360k @ 100+ fps**). **T-060** ✅ (`b1fd25a`). **T-061** ✅ drag-move good enough @ 360k. **T-062** ✅ incremental bindings @ 360k. **T-062.2** ✅ editor session. **T-062.1** ✅ chunked IDB load. **T-062.1.1** ✅ Save orbat dedup. **T-063** ✅ spatial index. **T-064** ✅ virtualized outliner. **T-065** ✅ cluster/LOD. **T-066** ✅ worker compile. **T-067** ✅ spatial chunks — [`t067_spatial_chunks.md`](t067_spatial_chunks.md). **Eden T-068+**.
