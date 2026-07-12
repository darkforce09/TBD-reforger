# T-068.10.2 verify log — exporter reclassify + kind taxonomy v3

**Date:** 2026-07-12 · **Executor:** Claude Code (Fable 5), operator-approved full-program
session (docs lane included) · **Branch:** `main` ·
**Spec:** `docs/specs/Mission_Creator_Architecture/t068_10_2_exporter_reclassify.md` ·
**Contract:** `.ai/artifacts/t068_10_2_census.md` (H_pred freeze @ `52e938fd`)

## Status

**PASS — all gates green.** Export run 6 (items **1,857**, edges **4,685**) committed as the
workbench envelopes; DB imported + pruned; G1–G7 + G5-sample + addon-provenance PASS with
per-item evidence. One OPEN (non-blocking, documented): EntityCatalog Tier-B parse 0/30 —
`arsenal_type` absent this pass, zero kind impact by construction (recommend follow-up
T-068.10.2.1). Kind histogram (live DB == envelope, exact):
ammo 101 · attachment 26 · character 354 · crate 311 · gear_armored_vest 12 ·
gear_backpack 43 · gear_binoculars 7 · gear_boots 6 · gear_explosive 6 · gear_glasses 3 ·
gear_gloves 6 · gear_handgun 5 · gear_helmet 92 · gear_item 80 · gear_jacket 61 ·
gear_launcher 18 · gear_pants 33 · gear_primary 84 · gear_throwable 15 · gear_vest 34 ·
magazine 125 · optic 30 · other 100 · vehicle 218 · vehicle_weapon 87.

## Gates (final, evidence in artifacts)

| # | Assertion | Result |
|---|-----------|--------|
| A1 | `npm run validate` (v3 goldens + addon strict) | PASS — 123 checks, addon 1857/1857 |
| A2 | codegen zero drift | PASS @ `bd74fb54` |
| A3 | `cargo check` + `make test-it` (fresh DB runs 0004) + tsc | PASS — 0 failed |
| G1 | per-item kind vs census v5 + plugin ruleId sidecar | PASS — 0 unexplained, 25 justified, 14 inheritance-kept, 23 dropped/denied as predicted ([gates output](t068_10_2_gates_output.txt)) |
| G1b | histogram == per-item expectation | PASS — exact |
| G2 | `gear_primary` pollution | PASS — 0 rows (down from 41 polluted in v2 data) |
| G3 | abstract flags per item | PASS — 346/346 visible, 0 flips |
| G4/G4b | conservation + edge referential integrity | PASS — 1857+23 == 1880; 0 dangling |
| G5 | `character_default_weapon` | PASS — 673 edges / 274 characters; 20/20 slot-exact sample, seed 68102 ([sample](t068_10_2_g5_sample.txt)) |
| G6 | DB == envelope + idempotency + prune | PASS — histograms exact; re-run 0/0/0; prune removed exactly the 23 predicted rows (+16 stale edges) |
| G7 | double-export byte identity | PASS — items + compat identical modulo `generatedAt` |

## Compile + export cycle (ops notes for future rounds)

- **Launch recipe (proven):** `setsid steam -applaunch 1874910 -gproj
  'Z:\home\Samuel\Projects\TBD-Reforger\apps\mod\tbd-framework\addon.gproj'
  -addonsDir 'Z:\home\Samuel\ArmaReforger-Base'` — plain `-gproj` alone CANNOT resolve the
  vanilla data dependency (GUID 58D0FB3206B6F859 registered only in the GUI project list);
  `-addonsDir` supplies it. Plain `-applaunch` (no gproj) sticks at the project picker.
- After each restart: `mcp-daemon.sh restart` ONCE (a stale daemon holds a dead WB socket;
  restarting it per-poll perpetually cold-starts the 35 s index — restart once, then call).
- Kill cycle: resolve PIDs via `ps -eo pid,args | grep -E 'AppId=1874910|enfMain'` — `pkill -f`
  self-matches the calling shell (T-150 note confirmed) and a leftover `enfMain` blocks both
  the Net API port and new launches.
- EnfScript compile errors fixed across rounds: `typename.GetParent` does not exist (→
  IsInherited probe against the 14 known area bases); a second `foreach` over one map-value
  variable is rejected (→ single merged loop); grenade/throwable default slots are
  `CharacterGrenadeSlotComponent` (≠ `*WeaponSlotComponent` suffix) — round-5 fix, +248 edges.

## Gates (design)

## Unknown ledger resolution (spike, all closed before classifier code)

| ID | Outcome | Evidence |
|----|---------|----------|
| U1 | **Container capacity IS serialized** when overridden: storage `MaxCumulativeVolume` (100…10000 cm³ observed) + `m_fMaxWeight` (0…100 kg observed) in vanilla container prefab texts; class-default cases export absent. → schema fields `max_volume_cm3` / `max_weight_kg` | grep over 160 cached storage-bearing prefab texts (census cache) |
| U2 | **Non-blocking partial.** `SCR_EArsenalItemMode` flags ≥8 verbatim (WEAPON_VARIANTS=8, AMMUNITION=16, CONSUMABLE=32, ATTACHMENT=64, SUPPORT_STATION=128, PYLON=256); first 3 garbled in pak text extraction and unreadable via `wb_script_editor` (openFile can't address pak scripts). ItemMode is consumed nowhere in the pipeline — `arsenal_type` uses `SCR_EArsenalItemType` (fully verbatim, 22 flags) | pak read + wb_script_editor probes |
| U3 | **Shape resolved:** `SCR_ArsenalItem extends SCR_BaseEntityCatalogData` with `GetItemType() → SCR_EArsenalItemType`, `GetItemMode()`, `GetItemResourceName()` (api_search). Container VAR names probed at runtime by the plugin with logged fallbacks (`m_aEntityEntryList`/`m_sEntityPrefab`/`m_aEntityDataList`/`m_eItemType` first); coverage measured per export (`catalogEntries`/`catalogHits` counters). Catalog misses degrade to zero refinements — H_pred unaffected by construction | api_search + plugin design |
| U4 | **14/15 `LoadoutAreaType` subclasses named** (+`LoadoutWatchArea`, `LoadoutHandSlotArea` discovered this pass). The 15th is unused by all 1,880 items — census `unknown_areas == 0`; classifier maps unknown areas → `other` + counted, never mislabeled | api_search enumeration + census |

## Census (Step 0 — committed BEFORE classifier code @ `52e938fd`)

- 1,880/1,880 items disposed; Σ conservation asserts pass; `abstract` = 352;
  `weapon_unsplit` = 0; `unknown_areas` = 0; read errors = 0.
- H_pred frozen in `.ai/artifacts/t068_10_2_census.md` §H_old → H_pred (26-kind table).
- Pre-authorized G1 deltas: 8 named flare rows may refine `gear_launcher → gear_throwable`
  via the catalog pass; deployable weapon parts stay `gear_backpack` (engine wear area);
  `Weapon_Base` → `vehicle_weapon` abstract; 35 inert unresolved ancestor refs (all
  justified in census §Caveats — zero dispositions depend on them).

## Schema v3 + plumbing (@ `bd74fb54`)

- `npm run validate` — ALL PASS incl. new addon-provenance strict check
  (`registry-items.sample.json` 2/28 items carry addon; workbench v2 envelope 0/1880 vacuous).
- `make schema-codegen` — regenerated TS + Rust contracts; re-run drift: clean.
- `cargo check` — clean. `make test-it` — **0 failed** (fresh `rust_it` DB runs migration
  `0004_registry_items_v3.sql`). `npx tsc --noEmit` — clean.

## Classifier rewrite (this slice's mod change)

`TBD_RegistryScan.c` (+ emission in `TBD_RegistryItemsExportPlugin.c`):

- Rule order = census §Rules (R0, R2 wear-exact incl. ArmoredVest/Watch/HandSlot maps with
  typename-ancestry fallback for mod subclasses, R3 gadget inheritance, R4 GrenadeMove,
  R5 explosive components, R7a vanilla weapon-family ancestry, R6 statics
  (compartment/rocket-ejector/not-carryable), R7b MuzzleInMag, R7c path fragments,
  R7d unsplit → counted, R9 fallthrough → other + counted). `else → gear_primary` DELETED.
- Tier-B EntityCatalog pass: arsenal_type metadata + bounded refinements (other/unsplit/
  flare-delta only — never contradicts component kinds).
- New: `character_default_weapon` edges from `CharacterWeaponSlotComponent.WeaponTemplate`;
  `abstract` flag; `weight_kg`/`volume_cm3`/`max_weight_kg`/`max_volume_cm3`; per-item
  `addon` (scanner already tracked it, now emitted); tier counters
  (`tierA+tierB+tierC+other == items` printed every run).

## Compile + export cycle

(filled after the Workbench run)

## Gates

(filled after export + import: G1 per-item + G1b histogram, G2 pollution=0, G3 abstract,
G4 conservation + referential integrity, G5 default-weapon sample, G6 DB equality +
idempotent re-import, G7 double-export byte identity)
