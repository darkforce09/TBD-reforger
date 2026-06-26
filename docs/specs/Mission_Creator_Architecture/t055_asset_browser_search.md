# T-055 — Asset browser search

**Status:** shipped (T-055)  
**Git tag on ship:** T-055  
**Authority:** [MC ROADMAP](ROADMAP.md) · [eden/gap_analysis.md](eden/gap_analysis.md) (`RIGHT-SEARCH-001`) · [feature_inventory.md](feature_inventory.md) RIGHT-SEARCH-001

---

## Goal

Add Eden's **asset-browser search field**. The right palette's Factions tab shows a nested
Faction → Category → Class tree (`AssetBrowser` over `ASSET_CATALOG`); finding a unit today
means hand-expanding folders. Eden puts a search box at the top of the asset browser that
filters the catalog to matching assets.

| ID | Gap | Deliverable |
|----|-----|-------------|
| **RIGHT-SEARCH-001** | `RIGHT-SEARCH-001` | Search field filters the Asset Browser tree live by asset/folder name |

**Out of scope:** `class:` classname-prefix search (`RIGHT-SEARCH-002`, P2); BLUFOR/OPFOR
submode (`RIGHT-SUBMODE-001`, T-074); the stub tabs (Vehicles/Markers/Objectives — no catalog
yet); registry feed; backend changes.

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Scope | Search lives in **`AssetBrowser` (Factions tab only)** — the only live catalog. Stub tabs stay stub; a palette-global search would mislead |
| Match | **Case-insensitive substring** on node `label`. Leaf match keeps the leaf; folder keeps if its own label matches (→ full subtree) or any descendant matches (→ filtered children) |
| Auto-expand | Retained folders are forced `defaultExpanded: true`; the `TreeView` is **keyed** on the query so its mount-time `collectExpanded` re-runs and reveals matches |
| Empty result | "No assets match …" message (Aegis `text-outline`), same style as the palette stub copy |
| Clear | `X` button (when non-empty) + **Escape** in the input → clears the query |
| Diff size | **One real file** (`AssetBrowser.tsx`); `TreeView`, `ASSET_CATALOG`, `AssetPalette` untouched |

### Rationale

- `TreeView` seeds its expanded set **once at mount** via `collectExpanded` in a `useState`
  initializer (`tree/TreeView.tsx` L70); it does not recompute when `nodes` change. Keying
  `<TreeView key={query.trim() || 'all'}>` remounts it per query change so the filtered tree
  (whose retained folders are `defaultExpanded`) renders expanded; an empty query uses the
  stable `'all'` key so normal manual expand/collapse persists. The search `<input>` lives
  outside `TreeView`, so the remount never steals focus. Trivial cost on the small mock tree;
  no `TreeView` change.
- Typing in the box is safe: `MissionCreatorPage`'s global keydown handler already ignores
  `INPUT`/`SELECT`/`TEXTAREA`/contentEditable focus, so Delete/Space/undo don't fire.

---

## Implementation specification

**File:** [`frontend/src/features/mission-creator/layout/RightInspector/AssetBrowser.tsx`](../../frontend/src/features/mission-creator/layout/RightInspector/AssetBrowser.tsx)

1. `const [query, setQuery] = useState('')`.
2. Module-scope recursive filter:
   ```ts
   function filterCatalog(nodes: TreeNodeData[], q: string): TreeNodeData[] {
     const out: TreeNodeData[] = []
     for (const n of nodes) {
       const selfMatch = n.label.toLowerCase().includes(q)
       if (n.children) {
         const kids = filterCatalog(n.children, q)
         if (selfMatch) out.push({ ...n, defaultExpanded: true })           // full subtree
         else if (kids.length) out.push({ ...n, defaultExpanded: true, children: kids })
       } else if (selfMatch) {
         out.push(n)
       }
     }
     return out
   }
   ```
3. `const nodes = useMemo(() => { const q = query.trim().toLowerCase(); return q ? filterCatalog(ASSET_CATALOG, q) : ASSET_CATALOG }, [query])`.
4. Search input above the tree (Aegis `controlClass` vocabulary from `fields.tsx`: border
   `outline-variant/40`, bg `surface-container-lowest/60`, `focus:border-primary/60`), with a
   left `Search` icon (`pl-8`) and a clear `X` button when `query` is non-empty; `Escape`
   clears.
5. Body: `nodes.length === 0` → empty message; else
   `<TreeView key={query.trim() || 'all'} nodes={nodes} selectedId={selectedId} onSelect={setSelectedId} onNodeDragStart={onNodeDragStart} />`.

`onNodeDragStart` unchanged → filtered leaves still drag the same `ASSET_DND_MIME` /
`AssetDropPayload` (`{ assetId, role, kind:'slot' }`).

---

## Files to change (checklist)

| File | Change |
|------|--------|
| `frontend/src/features/mission-creator/layout/RightInspector/AssetBrowser.tsx` | search input + `filterCatalog` + keyed `TreeView` + empty state |

**No backend change. No store/schema change. No `TreeView` / `ASSET_CATALOG` / `AssetPalette` change.**

---

## Verification

```bash
cd frontend && npm run build && npm run lint
```

### Manual test plan (`make web`, dev-login `mission_maker`, `/missions/:id/edit`, right palette → Factions)

1. Type `medic` → tree narrows to NATO ▸ Men ▸ Medic (ancestors auto-expanded).
2. Type `nato` → whole NATO subtree shown (folder-name match).
3. Type `zzz` → "No assets match" message.
4. Clear (X or Esc) → full catalog restored; manual expand/collapse works as before.
5. Drag a filtered leaf (e.g. searched "Hunter") onto the map → slot places (DnD intact).
6. Typing in the box does not delete the selected map entity or trigger Space-center.

---

## Documentation sync (same commit — T-055)

Use [`docs/AGENT_COMMIT_CHECKLIST.md`](../../docs/AGENT_COMMIT_CHECKLIST.md).

| Doc | Change |
|-----|--------|
| **This file** | Status → **shipped** |
| [`CLAUDE.md`](../../CLAUDE.md) §Status | T-055 bullet + bump `latest feature work` line + Next-slices line |
| [`feature_inventory.md`](feature_inventory.md) | **New `RIGHT-SEARCH-001` FEDS entry** (Status working; Evidence `AssetBrowser.tsx`) |
| [`agent_execution.md`](agent_execution.md) | Decisions log row **Asset browser search (T-055)**; refresh one-line prompt Next-slices |
| [`ROADMAP.md`](ROADMAP.md) | DONE T-055 section + spec index row; Eden execution-order line; refresh Next |
| [`eden/gap_analysis.md`](eden/gap_analysis.md) | T-055 → ✅ shipped T-055; RIGHT-SEARCH-001 table row → built |

**Do not update:** archive stitch, Eden wiki artifacts, historical CLAUDE bullets.

---

## Git strategy

**One T-055 commit** on `main`: code + doc finalize + CLAUDE §Status. Co-Authored-By when
using AI. **Do not commit until the user asks.**

---

## Related

- Prior: [t054_attributes_entry_points.md](t054_attributes_entry_points.md)
- Next Eden backlog: see [`docs/TICKET_LEAD.md`](../../docs/TICKET_LEAD.md) (T-068+).
