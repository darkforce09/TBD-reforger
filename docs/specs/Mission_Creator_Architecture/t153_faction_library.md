# T-153 — Faction Library (reusable factions + palette swap)

**Status:** **shipped** (backend + frontend; tags **T-153.0**/**T-153.1** on the two
commits) · **Executor:** claude-code (operator-approved Mode C session, docs lane
included) · **Verify:** [`.ai/artifacts/t153_verify_log.md`](../../../.ai/artifacts/t152_verify_log.md) ·
**Authority:** [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md) (Mode C)

## In one sentence

Operator-authored, cross-mission reusable factions — side (BLUFOR/OPFOR/INDFOR/CIV) →
faction ("US Army 1980s") → ORBAT role templates (registry character + optional SlotLoadout
v2) and a vehicle pool — replace the raw vanilla registry character dump in the Mission
Creator Factions palette entirely (closes the T-074 intent).

## Shipped shape

- **Contract:** `faction-library.schema.json` (+ golden with real census-envelope GUIDs;
  codegen TS + Rust; numeric const expressed as integer bounds — quicktype Rust emitter
  limitation).
- **Backend:** migration `0006_user_factions.sql` (owner_id = discord_id; side/name
  projections; unique (owner, name); `doc` jsonb); `/api/v1/factions` CRUD
  (mission_maker+, owner-scoped, contract-validated on every write). Gates T1–T8 in
  `tests/factions.rs`.
- **Frontend:** `useFactionLibrary`/`useSaveFaction`/`useDeleteFaction`;
  **FactionManagerDialog** (side/name; role CRUD with character picker + per-role loadout
  via the extracted **ArsenalPicksPanel** — the same 14-row grouped/sorted/variant-filtered
  pipeline the Arsenal tab uses; vehicle CRUD); **buildFactionTree** palette (side folders
  with Aegis accents: BLUFOR primary / OPFOR error / INDFOR success / CIV outline → faction
  → Roles/Vehicles); AssetBrowser rewritten onto the library (vanilla dump deleted —
  `buildCatalogTree.ts` removed); role drags carry `assetId + tag + loadout + factionRef`;
  drops `ensureFaction`/`ensureSquadFor` (first real user of `Faction.key` beyond the
  hardcoded BLUFOR) and store the pre-authored loadout on the new slot. Vehicle leaves are
  listed with a T-070 badge — map placement explicitly out of scope here.
- **Compile twins untouched:** assetId→kit and Slot.loadout paths identical.

## Verification

`make test-it` 22 suites ok (T1–T8: 403 tier, 400 schema-details, 201 golden, 409 dup,
owner-scoped list/get, full-doc update, 204+404 delete). Frontend vitest **340/340**
(buildFactionTree gates incl. the structural vanilla-purge assert — the tree is a pure
function of the library, no registry enumeration), build + tsc clean, lint pre-existing
`router.tsx` only. Live smoke: `POST /factions` from the committed golden → 201 → listed
(OPFOR · Soviet Army 1980s · 2 roles).

## Out of scope / follow-ups

Vehicle map placement (**T-070**) · faction emblem upload · sharing/visibility beyond
owner · doc-faction ↔ library sync beyond drop-time ensure (T-071 ORBAT manager remains
the mission-side authoring lane).
