# T-068.3 — Wire Factions palette to live registry

**Ticket:** T-068 · **Slice:** T-068.3  
**Status:** **shipped** @ `da78452` (git tag **T-068.3**)  
**Executor:** claude-code  
**Authority:** [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md)

---

## In one sentence

Replace mock Factions catalog with `GET /registry` → tree builder → existing T-055 search + DnD.

---

## Problem (historical)

Previously [`AssetBrowser.tsx`](../../../apps/website/frontend/src/features/mission-creator/layout/RightInspector/AssetBrowser.tsx) imported static `assetCatalogMock.ts` (deleted by this ticket) (fake ids). **Fixed @ T-068.3** — live `GET /registry` feed.

---

## Goal

1. `buildCatalogTree.ts` — flat `RegistryItem[]` → Faction → Category tree; leaf `id` = `resource_name`; label = `display_name`; filter `kind === 'character'` for palette leaves.
2. `useRegistry()` in `hooks/queries.ts` — TanStack Query; optional `If-None-Match` for 304.
3. Wire `AssetBrowser` — loading/error/empty states; preserve `filterCatalog` (T-055).
4. DnD: `AssetDropPayload.assetId` = `resource_name`; `role` = display_name; `kind: 'slot'`.
5. **Delete** `assetCatalogMock.ts` (runtime import removed).
6. JSDoc on `AssetDropPayload` / `Slot.assetId` — stores `resource_name`.

---

## Out of scope

- Vehicles/Markers/Objectives stub tabs
- Loadout UI (T-068.4)
- T-067 perf regression fixes

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Tree scope | `character` kind only in palette |
| Search | Client-side T-055 only |
| Icons | Lucide fallback when `icon_url` empty |

---

## Tasks

1. `frontend/src/features/mission-creator/registry/buildCatalogTree.ts`
2. `frontend/src/hooks/queries.ts` — `useRegistry`
3. `AssetBrowser.tsx` — wire feed
4. Delete `assetCatalogMock.ts`
5. `tactical-map/types.ts` — JSDoc

---

## Verify

```bash
cd apps/website/frontend && npm run build && npm run lint
if rg -q 'assetCatalogMock' src/features/mission-creator --glob '!**/*.test.*'; then echo "FAIL: mock still imported"; exit 1; fi
test ! -f src/features/mission-creator/layout/RightInspector/assetCatalogMock.ts
rg -q 'buildCatalogTree' src/features/mission-creator
rg -q 'useRegistry' src/hooks/queries.ts
```

---

## Verification gate (mandatory)

### Automated (exit 0)

```bash
cd apps/website/frontend && npm run build && npm run lint
if rg -q 'assetCatalogMock' src/features/mission-creator --glob '!**/*.test.*'; then echo "FAIL: mock still imported"; exit 1; fi
test ! -f src/features/mission-creator/layout/RightInspector/assetCatalogMock.ts
```

### Manual (browser — paste DevTools evidence)

Prerequisites: `make db-up && make api && make web`; dev-login `mission_maker`; open `/missions/:id/edit`.

| ID | Step | Pass condition | Evidence to paste |
|----|------|----------------|-------------------|
| M1 | Network | `GET /api/v1/registry` → **200** with JSON body | Network tab status + response size |
| M2 | Factions tree | NATO/CSAT folders visible; **not** instant mock labels from old static file | Screenshot or DOM text snippet |
| M3 | Search `medic` | Medic row visible under NATO (T-055) | Filter result description |
| M4 | Search `nato` | Full NATO subtree expands | Filter result description |
| M5 | Drag rifleman | Slot placed on map | — |
| M6 | Store `assetId` | After drag, paste leaf `resource_name` from tree/API **and** confirm slot row in Outliner shows new entity (paste slot label + count) | Pasted GUID string + OBJ before/after |
| M7 | Mock removed | No runtime request to static mock data | Network tab shows only `/registry` |

**Acceptance criteria (all PASS):** A1 build/lint clean · A2 no mock import · A3 `assetCatalogMock.ts` deleted · M1–M7 manual rows.

### Verify paste (required)

Automated output + M1–M7 table with PASS + pasted `assetId` value.

---

## Depends on / Unblocks

- **Depends on:** T-068.2 (**shipped** — `GET /api/v1/registry` @ `4c609fe`, types @ `frontend/src/types/models/registry.ts`, 21-row dev seed)
- **Unblocks:** T-068.6; closes **RIGHT-CAT-001**

---

## Backend contract (already shipped — do not reimplement)

| Item | Value |
|------|-------|
| Route | `GET /api/v1/registry` on **`mm`** group (`RequireMinRole("mission_maker")`) |
| Response | `{ data: RegistryItem[], etag, modpack_id, modpack_version }` |
| Row fields | `resource_name`, `display_name`, `category` (slash path e.g. `NATO/US_Army/Rifleman`), `kind`, optional `icon_url` |
| Caching | Weak `etag`; send `If-None-Match` → **304** (optional in `useRegistry`; not required for PASS) |
| Dev data | `make seed` → 21 rows (8 `character`, 4 gear kinds for later T-068.4) |
| Types | [`frontend/src/types/models/registry.ts`](../../../apps/website/frontend/src/types/models/registry.ts) |

**Tree builder input:** filter `data` where `kind === 'character'` only. Split each row's `category` on `/` to nest folders; leaf `id` = **`resource_name`** (full `{GUID}Prefabs/.../File.et`), leaf `label` = **`display_name`**.

---

## Documentation sync (Cursor)

**Done @ T-068.3 merge:** `feature_inventory` **RIGHT-CAT-001** → working; `eden/gap_analysis` Factions feed → match; `mission-editor.md` + MC ROADMAP palette row.

---

## Claude Code prompt — T-068.3

```
Read CLAUDE.md §Status. Active slice: T-068.3.
Implement ONLY docs/specs/Mission_Creator_Architecture/t068_3_palette_wire.md
Do not edit documentation. Branch: ticket/T-068 (checkout from main if needed)

CONTEXT — T-068.2 already on main (@ 4c609fe):
  GET /api/v1/registry — mission_maker+ JWT; response { data, etag, modpack_id, modpack_version }
  Types: apps/website/frontend/src/types/models/registry.ts (RegistryItem, RegistryResponse)
  Dev seed: make db-up && make seed → 21 rows; 8 characters with real resource_name GUIDs
  AssetBrowser.tsx still imports assetCatalogMock.ts — YOUR job is to replace that feed

PREFLIGHT — verify stack before coding:
  make db-up && make seed
  make api   # restart if stale — go run does not hot-reload handlers
  make web
  Dev-login mission_maker → confirm GET /api/v1/registry returns 200 + data.length >= 10

LOCKED (do not deviate):
  - Leaf tree id = resource_name (full Enfusion ResourceName string, NOT mock ids like a-nato-rifleman)
  - Palette shows kind === 'character' only (gear_* rows exist for T-068.4 Arsenal dropdowns)
  - DnD payload: AssetDropPayload { assetId: resource_name, role: display_name, kind: 'slot' }
  - Preserve T-055 filterCatalog + TreeView key={query} search behavior unchanged
  - Delete assetCatalogMock.ts; zero runtime imports of assetCatalogMock in mission-creator/
  - icon_url empty → Lucide User/Folder fallback (match mock visual weight)
  - Do NOT touch Arsenal tab stub (T-068.4), compiler, backend, or mod

IMPLEMENT (exact files from spec):
  1. features/mission-creator/registry/buildCatalogTree.ts
     — flat RegistryItem[] → TreeNodeData[]; split category on '/'; stable folder ids from path prefix
  2. hooks/queries.ts — useRegistry()
     — useQuery(['registry'], () => api.get<RegistryResponse>('/registry'))
     — enabled when useAuthed(); surface isLoading/isError to AssetBrowser
  3. layout/RightInspector/AssetBrowser.tsx
     — replace ASSET_CATALOG with buildCatalogTree(useRegistry().data ?? [])
     — loading spinner / error retry / empty modpack message
     — keep filterCatalog, onNodeDragStart, ASSET_DND_MIME unchanged except assetId source
  4. DELETE layout/RightInspector/assetCatalogMock.ts
  5. features/tactical-map/types.ts — JSDoc: assetId stores resource_name (Enfusion ResourceName)

OUT OF SCOPE: Vehicles/Markers tabs, Loadout UI, useRegistry ETag/304 (optional nice-to-have), perf work

VERIFY — all must exit 0 / PASS before returning:
  cd apps/website/frontend && npm run build && npm run lint
  rg -q 'assetCatalogMock' src/features/mission-creator --glob '!**/*.test.*' && exit 1 || true
  test ! -f src/features/mission-creator/layout/RightInspector/assetCatalogMock.ts
  rg -q 'buildCatalogTree' src/features/mission-creator
  rg -q 'useRegistry' src/hooks/queries.ts

MANUAL (browser — paste evidence in verify block):
  M1: Network GET /api/v1/registry → 200
  M2: Factions tree shows NATO/CSAT from API categories (not mock vehicle/object subtrees from old mock)
  M3: Search "medic" → medic row visible (T-055)
  M4: Search "nato" → full NATO subtree
  M5: Drag US Rifleman (or equivalent) → slot on map
  M6: Paste dropped slot's assetId — must match /^\\{[0-9A-F]{16}\\}/ (full resource_name from tree)
  M7: Network tab — no static mock fetch; only /registry

Return: Verify paste per program hub template — automated output + A1–A3 table + M1–M7 with PASS + pasted assetId GUID string.
```
