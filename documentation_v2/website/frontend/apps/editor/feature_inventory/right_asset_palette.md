**Status:** live

## RIGHT — Asset palette

#### RIGHT-TABS-001 — Palette tab strip

| Field | Value |
|-------|-------|
| **Domain** | RIGHT |
| **Goal** | Eden asset category tabs |
| **Trigger** | Click Factions/Vehicles/Markers/Objectives |
| **Preconditions** | Palette open |
| **Procedure** | Local `tab` state; Factions → `AssetBrowser`; others → stub text |
| **Postconditions** | Tab content switches |
| **Inputs** | LMB tab |
| **Outputs** | UI |
| **Edge cases** | Only Factions functional |
| **Acceptance** | `- [x] Factions shows tree` `- [ ] Other tabs show placeholder` |
| **Eden parity** | Eden:RIGHT-TABS-001 |
| **Status** | partial |
| **Evidence** | `AssetPalette.tsx` |

#### RIGHT-CAT-001 — Factions registry catalog tree

| Field | Value |
|-------|-------|
| **Domain** | RIGHT |
| **Goal** | Browse placeable units |
| **Trigger** | Factions tab active |
| **Preconditions** | `mission_maker+`; `GET /api/v1/registry` returns rows |
| **Procedure** | `useRegistry()` → `buildCatalogTree` (character rows only) → `TreeView`; T-055 `filterCatalog`; leaf drag `ASSET_DND_MIME` with **`resource_name`** as `assetId` |
| **Postconditions** | Draggable leaves with full Enfusion **`resource_name`** in DnD payload |
| **Inputs** | Expand/select/drag; search filters tree |
| **Outputs** | DnD payload → `addSlot(..., { assetId: resource_name, role: display_name })` |
| **Edge cases** | Loading spinner-only (no tree flash); error + Retry; empty modpack message |
| **Acceptance** | `- [x] NATO tree from API` `- [x] Leaf drags to map with resource_name GUID` |
| **Eden parity** | Eden:RIGHT-CAT-001 |
| **Status** | **working** |
| **Ticket** | T-068.3 |
| **Evidence** | `registry/buildCatalogTree.ts`, `AssetBrowser.tsx`, `hooks/queries.ts` `useRegistry()` |

#### RIGHT-SEARCH-001 — Asset browser search

| Field | Value |
|-------|-------|
| **Domain** | RIGHT |
| **Goal** | Eden asset search field |
| **Trigger** | Type in the Asset Browser search box (Factions tab) |
| **Preconditions** | Palette open, Factions tab |
| **Procedure** | T-055: `AssetBrowser` `filterCatalog(catalog, q)` on registry-built tree (case-insensitive label substring; folder kept on self-match → full subtree, else on descendant match → filtered children; retained folders `defaultExpanded`); `TreeView` keyed on query so mount-time expand re-runs |
| **Postconditions** | Tree narrows to matches (ancestors expanded); empty → "No assets match"; clear (X/Esc) restores |
| **Inputs** | Text query; Esc / X to clear |
| **Outputs** | Filtered `TreeView`; filtered leaves still draggable (`ASSET_DND_MIME`) |
| **Edge cases** | Whitespace-only = no filter; stub tabs have no search (no catalog); no `class:` prefix (RIGHT-SEARCH-002, P2) |
| **Acceptance** | `- [x] Query filters tree` `- [x] Folder-name match shows subtree` `- [x] Clear restores` |
| **Eden parity** | Eden:RIGHT-SEARCH-001 |
| **Status** | working |
| **Ticket** | T-055 |
| **Evidence** | `AssetBrowser.tsx` |

#### RIGHT-STUB-001 — Vehicles tab placeholder

| Field | Value |
|-------|-------|
| **Domain** | RIGHT |
| **Goal** | Future vehicle catalog |
| **Trigger** | Vehicles tab |
| **Preconditions** | — |
| **Procedure** | Static message "catalog arrives with the asset registry feed" |
| **Postconditions** | No catalog |
| **Inputs** | Tab click |
| **Outputs** | UI text |
| **Edge cases** | Phase 5-6 |
| **Acceptance** | `- [ ] Stub message shown` |
| **Eden parity** | Eden:VEH-CAT-001 |
| **Status** | stub |
| **Evidence** | `AssetPalette.tsx` |

#### RIGHT-STUB-002 — Markers tab placeholder

| Field | Value |
|-------|-------|
| **Domain** | RIGHT |
| **Goal** | Future marker catalog |
| **Trigger** | Markers tab |
| **Preconditions** | — |
| **Procedure** | Same stub message |
| **Postconditions** | — |
| **Inputs** | Tab |
| **Outputs** | UI |
| **Edge cases** | — |
| **Acceptance** | `- [ ] Stub shown` |
| **Eden parity** | Eden:MRK-CAT-001 |
| **Status** | stub |
| **Evidence** | `AssetPalette.tsx` |

#### RIGHT-STUB-003 — Objectives tab placeholder

| Field | Value |
|-------|-------|
| **Domain** | RIGHT |
| **Goal** | Future objectives catalog |
| **Trigger** | Objectives tab |
| **Preconditions** | — |
| **Procedure** | Same stub message |
| **Postconditions** | — |
| **Inputs** | Tab |
| **Outputs** | UI |
| **Edge cases** | — |
| **Acceptance** | `- [ ] Stub shown` |
| **Eden parity** | Eden:WP-CAT-001 |
| **Status** | stub |
| **Evidence** | `AssetPalette.tsx` |

---

