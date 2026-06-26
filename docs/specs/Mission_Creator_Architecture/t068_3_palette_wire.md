# T-068.3 — Wire Factions palette to live registry

**Ticket:** T-068 · **Slice:** T-068.3  
**Status:** Spec ready — code pending  
**Executor:** claude-code  
**Authority:** [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md)

---

## In one sentence

Replace mock Factions catalog with `GET /registry` → tree builder → existing T-055 search + DnD.

---

## Problem

[`AssetBrowser.tsx`](../../../apps/website/frontend/src/features/mission-creator/layout/RightInspector/AssetBrowser.tsx) imports [`assetCatalogMock.ts`](../../../apps/website/frontend/src/features/mission-creator/layout/RightInspector/assetCatalogMock.ts).

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

- **Depends on:** T-068.2
- **Unblocks:** T-068.6; closes **RIGHT-CAT-001**

---

## Documentation sync (Cursor)

After merge: `feature_inventory.md` RIGHT-CAT-001 → working; `eden/gap_analysis` Factions feed → match.

---

## Claude Code prompt — T-068.3

```
Read CLAUDE.md §Status. Active slice: T-068.3.
Implement ONLY docs/specs/Mission_Creator_Architecture/t068_3_palette_wire.md
Do not edit documentation. Branch: ticket/T-068
Verify: FE build/lint; mock-import grep gate; complete M1–M7 manual table with assetId proof
Return: Verify paste block per program hub template.
```
