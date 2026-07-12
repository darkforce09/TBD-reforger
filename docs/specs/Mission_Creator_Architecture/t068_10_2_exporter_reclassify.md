# T-068.10.2 — Registry exporter reclassify + kind taxonomy v3

**Ticket:** T-068 · **Slice:** T-068.10.2 · **Status:** ready (ACTIVE) · **Executor:** claude-code
(operator-approved full-program session, docs lane included) ·
**Authority:** [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md) ·
**Census (H_pred freeze):** [`.ai/artifacts/t068_10_2_census.md`](../../../.ai/artifacts/t068_10_2_census.md) ·
**Taxonomy:** [`.ai/artifacts/ace_arsenal_taxonomy_map.md`](../../../.ai/artifacts/ace_arsenal_taxonomy_map.md)

---

## In one sentence

Make the T-150 scanner's `kind` classification Reforger-correct and mod-agnostic (engine
components + BI EntityCatalog, no `gear_primary` catch-all), add per-item `abstract` /
`arsenal_type` / `weight_kg` / `volume_cm3` / `addon` fields and `character_default_weapon`
edges, re-export, and re-import — with the post-export histogram matching the frozen census
prediction exactly.

## Problem

`gear_primary` (125 rows) mixes rifles with grenades, smokes, flares, demo charges, mortars,
cannons and rocket pods because the classifier's weapon branch ends in `else → gear_primary`
(`TBD_RegistryScan.c:391`). Cloth kinds collapse Reforger's separate wear areas (Jacket/Pants/
Boots → one `gear_uniform`; Vest/ArmoredVest → one `gear_vest` via a `Contains("Vest")` match).
348 abstract `* Base` templates ship as normal rows. Weapon default-loadout edges were never
exported. No per-item mod provenance.

## Goal

1. **Schema v3** (`registry-items.schema.json`): kind enum += `gear_throwable`,
   `gear_explosive`, `gear_jacket`, `gear_pants`, `gear_boots`, `gear_armored_vest`,
   `gear_glasses`, `gear_gloves`, `gear_binoculars`, `gear_item` (old kinds stay valid;
   `gear_uniform` retires to 0 rows but remains accepted). Item fields += optional
   `abstract: boolean`, `arsenal_type: string`, `weight_kg: number`, `volume_cm3: number`
   (units per `ItemPhysicalAttributes` API: kg / cm³), `addon: string` (must match an
   `addons[].name`; vanilla-ness derived by joining `addons[].vanilla`).
2. **Compat schema**: edge_type += `character_default_weapon`
   (from `CharacterWeaponSlotComponent.WeaponTemplate`, ancestry-aware).
3. **Classifier rewrite** (`TBD_RegistryScan.c`) in census rule order R0/R2–R9 (see census
   §Rules) with inheritance-aware component matching (`ToType().IsInherited`), plus a
   Tier-B refinement pass reading `Configs/EntityCatalog/*` `SCR_ArsenalItem` entries →
   `arsenal_type` metadata (+ kind refinement ONLY for `weapon_unsplit`/`other` rows and the
   pre-authorized flare delta). Tier counters printed every export
   (`tierA + tierB + tierC + other == total`).
4. **DB/ingest**: migration adds nullable `abstract`, `arsenal_type`, `weight_kg`,
   `volume_cm3`, `addon` columns (+ addon index); `registry_import.rs` UNNEST + model + API +
   FE types expose them.
5. **Live spike (same Workbench session)**: resolve U1 (container capacity source), U2
   (ItemMode first flags), U3 (`SCR_ArsenalItem` shape + catalog coverage %), U4 (last 2
   LoadoutAreaType subclasses) — recorded in the verify log before classifier code lands.
6. Re-export (one Workbench restart), copy envelopes, `npm run validate`,
   `make registry-import`, gates G1–G7.

## Out of scope

Forge UI (T-068.10.3) · loadout doc v2 (T-068.10.4) · cargo editor (T-068.10.5, later) ·
T-068.11 compiler (PARKED) · icons.

## Locked decisions

| # | Decision |
|---|----------|
| 1 | **Census is the contract**: H_measured == H_pred per kind (G1); only census §Caveats deltas are pre-authorized (flares → gear_throwable via catalog, ≤ 8 named rows). |
| 2 | Kind = deterministic component/ancestor rules; catalog refines, never contradicts (keeps H_pred computable offline). |
| 3 | Statics (mortars/cannons/pods/non-carryable weapons) → `vehicle_weapon`; deployable weapon parts stay `gear_backpack` (engine wear area); `Weapon_Base` → `vehicle_weapon` abstract. |
| 4 | `abstract` = filename `*_base.et` OR display `* Base` — flagged, never dropped (bases carry classification signals for descendants). |
| 5 | Missing weight/volume = `null` (class defaults not serialized) — never guessed. |
| 6 | `addon` from the per-addon scan root; every `item.addon` ∈ `addons[].name` (validator-enforced). |
| 7 | All Rust/TS contract changes via `make schema-codegen` — no hand edits to generated files. |

## Verify (gates — all PASS with pasted evidence, `.ai/artifacts/t068_10_2_verify_log.md`)

| # | Assertion | Method |
|---|-----------|--------|
| A1 | `cd packages/tbd-schema && npm run validate` green (v3 goldens + strict addon check) | exit 0 |
| A2 | `make schema-codegen` → zero drift after re-run | `git status` clean |
| A3 | `cargo check` + registry integration tests green | exit 0 |
| G1 | H_measured == H_pred per kind (census §H_old → H_pred) | python diff, zero unexplained |
| G2 | `gear_primary` has 0 rows in Grenades/Flares/Explosives/Mortars/Cannons/AircraftWeapons categories, 0 `Pod*`/`* Base` names | jq assert, exit 1 on hit |
| G3 | `abstract == true` count == census `abstract_count` (352 on the frozen set) | jq == census |
| G4 | Σ kinds == items total; edge referential integrity | validate.mjs strict |
| G5 | `character_default_weapon` present for every census-armed character; 20-item random sample slot-exact vs `game_read` (sample listed) | script + paste |
| G6 | DB histogram == envelope histogram; re-import idempotent (0 rows affected on re-run) | SQL before/after |
| G7 | Two consecutive exports byte-identical modulo `generatedAt` | sha256 |

## Acceptance

- [ ] Census committed BEFORE classifier code (done @ this slice's first commit).
- [ ] Schema v3 + codegen + migration + ingest + API + FE types shipped.
- [ ] Classifier rewrite live; tier counters in export log.
- [ ] U1–U4 resolved in verify log.
- [ ] G1–G7 PASS; envelopes committed; `make registry-import` run.
- [ ] Tag **T-068.10.2**; PAUSE for operator review (census + histogram + picker sanity).
