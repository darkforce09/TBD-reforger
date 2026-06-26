# T-064 — Virtualized outliner @ 100k–360k+ leaves

**Status:** **shipped + verified** — FE build/lint clean; manual @ ~367k: outliner visible on first paint, scrollable virtual slot list, no tab freeze (2026-06-24).  
**Git tag on ship:** **T-064** (`8838e58`, 2026-06-24)  
**Authority:** [MC ROADMAP](ROADMAP.md) §Map performance · [agent_execution.md](agent_execution.md) §ACTIVE SLICE · [t059_bulk_paste_operations.md](t059_bulk_paste_operations.md) (outliner cap origin) · [t063_spatial_index.md](t063_spatial_index.md) (prior slice)

**Prerequisites:** T-063 shipped (`078960e`). Repro mission: `70a36667-612f-40c5-ad56-3fb8e0613a17` (~367k slots).

---

## In one sentence

**The left sidebar used to either freeze the tab (render all rows) or hide them (500-row cap); T-064 only paints the ~20 visible rows so you can scroll through 360k units without a DOM explosion.**

---

## Problem (pre-T-064)

Pan/zoom (T-057), drag-move (T-061), pick/marquee (T-063), and bulk paste (T-059 batch append) are fast @ 367k. **The outliner sidebar was not.**

[`TreeView.tsx`](../../frontend/src/features/mission-creator/layout/tree/TreeView.tsx) recursively mounted **every** visible tree node as real DOM `<li>` elements. At 360k slot leaves that hard-froze the tab.

T-059 added a band-aid — `OUTLINER_LEAF_CAP = 500`: folders/squads over the cap showed a count label with **no scrollable rows**. T-064 removes that cap via virtualization.

**Scroll container:** [`LeftSidebar.tsx`](../../frontend/src/features/mission-creator/layout/LeftOutliner/LeftSidebar.tsx) uses **one** `overflow-y-auto` div for ORBAT + Editor Layers.

---

## Goal

Replace the 500-row cap with **real list virtualization** so users can expand and **scroll** through 100k–360k+ slot rows with only viewport-sized DOM.

**Out of scope:** T-065 LOD, T-066 worker, T-067 chunks, full `docToSnapshot` elimination on paste/hydrate, Asset Browser virtualization, backend/Y.Doc schema changes.

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Virtualization lib | **`@tanstack/react-virtual@^3.14.3`** |
| Where to virtualize | **One combined virtual list in `LeftSidebar`** |
| Slot representation | **`virtualSlotIds?: ID[]` on `TreeNodeData`** — never materialize 360k `children` |
| Threshold | **`VIRTUAL_SLOT_THRESHOLD = 50`** (replaces `OUTLINER_LEAF_CAP = 500`) |
| Row height | **`ROW_HEIGHT = 28`** px |
| Flatten strategy | **Segment index** — `OutlinerSegment[]` + `rowAt(index)`; never `FlatRow[360000]` |
| Expanded state | **Lifted to `LeftSidebar`**; lazy `useState` initializer (seed-once; see T-064.1) |
| Scroll element | **Callback-ref state** `scrollEl` passed to virtualizer (see T-064.1) |
| Paste selection cap | **`BULK_SELECT_CAP = 500`** unchanged |

---

## Shipped implementation

### Dependency

| Package | Role |
|---------|------|
| `@tanstack/react-virtual@^3.14.3` | Virtual list over sidebar scroll container |

### New files (`layout/tree/`)

| File | Role |
|------|------|
| **`TreeRow.tsx`** | Row chrome shared by recursive `TreeView` (Asset Browser) and `VirtualOutliner` |
| **`treeRowModel.ts`** | `OutlinerSegment`, `TreeRowModel`, `ROW_HEIGHT=28`, `INDENT_PX=14` |
| **`flattenOutliner.ts`** | `buildOutlinerSegments`, `totalRowCount`, `rowAt` |
| **`VirtualOutliner.tsx`** | `useVirtualizer` + section headers / empty / root-drop + `<TreeRow>` |

### Refactors

| File | Change |
|------|--------|
| **`TreeView.tsx`** | `<TreeRow>` extraction; `virtualSlotIds?: ID[]`; API unchanged for Asset Browser |
| **`EditorLayersSection.tsx`** | `VIRTUAL_SLOT_THRESHOLD=50`; `useEditorLayersOutliner(md)` + `TREE_MIME` |
| **`OrbatSection.tsx`** | Virtual pattern; `useOrbatOutliner()` |
| **`LeftSidebar.tsx`** | `scrollEl` callback-ref state; `VirtualOutliner`; hardened `collectDefaultExpanded` |

**Constraints honored:** no backend/Y.Doc/map pick/AssetBrowser/`BULK_SELECT_CAP` changes; `OUTLINER_LEAF_CAP` removed.

---

## T-064.1 — scroll-ref hotfix (shipped same tag)

**Symptom (pre-fix):** Outliner showed only the header until first map selection.

**Root cause:** `VirtualOutliner` is a child of the scroll div. On first render `scrollRef.current === null` → `useVirtualizer` returned zero virtual items.

**Fix (3 files):**

- **`VirtualOutliner.tsx`** — `scrollElement: HTMLDivElement | null`; `getScrollElement: () => scrollElement`
- **`LeftSidebar.tsx`** — `const [scrollEl, setScrollEl] = useState(...)`; `<div ref={setScrollEl}>`; hardened `collectDefaultExpanded` (includes `virtualSlotIds?.length`)
- **`OrbatSection.tsx`** — inline ORBAT slot leaves get `icon: User` (cosmetic parity)

**Expanded seeding note:** Spec suggested `useEffect` + seed-once `useRef` guard; React Compiler lint rejects `set-state-in-effect` / ref writes during render. **Shipped:** lazy `useState(() => collectDefaultExpanded(...))` only — correct because `LeftSidebar` mounts after `docStatus === 'ready'` (`MissionCreatorPage`), so trees are populated on first render. Inherently seed-once; manually collapsed folders stay collapsed across edits.

---

## Acceptance (T-064)

| Check | Result |
|-------|--------|
| `npm run build` + `npm run lint` | **Clean** |
| Outliner on first paint @ ~367k | **Pass** — no map click required (T-064.1) |
| Expand + scroll 367k virtual slots | **Pass** — no tab freeze |
| Click / dbl-click / DnD / rename / delete | **Pass** |
| Collapse folder → edit → stays collapsed | **Pass** |
| Map pan/pick (T-063) | **Unchanged** |
| Asset Browser | **Unchanged** |

---

## Out of scope (later tags)

- T-065 LOD / clustering, T-066 worker, T-067 spatial chunks
- Scroll-into-view when map-selecting off-screen row (stretch)

---

## After T-064

- **Active:** T-067.0 — [`t067_spatial_chunks.md`](t067_spatial_chunks.md) (spec ready)
- Eden **T-068+** after scale milestones

---

## Documentation sync (Cursor — T-064 ship) — **complete** (2026-06-24)

Synced: `agent_execution.md`, `ROADMAP.md`, `CLAUDE.md`, `feature_inventory.md` (PERF-OUTLINER-001 + KEY-COPY-001), `mission-editor.md`, `TAGS.md`, `docs/frontend/ROADMAP.md`, `docs/AGENT_COMMIT_CHECKLIST.md`, spec footers (`t056`–`t063`, `t070`, etc.).

---

## Claude Code prompt archive (T-064 / T-064.1 — completed)

Historical — do not re-run unless regressing outliner perf or blank-on-load. See git diff / §Shipped implementation above.
