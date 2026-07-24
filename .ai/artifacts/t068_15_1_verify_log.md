# T-068.15.1 — cargo capacity export · verify log

**Date:** 2026-07-24 · **Executor:** Fable 5 / Claude Code · **Branch:** `main` ·
**Spec:** `docs/specs/Mission_Creator_Architecture/t068_15_1_cargo_capacity_export.md` ·
**Baseline:** `7c0078dd` (resumed the on-disk scanner/schema WIP — not rewritten)

## Result

**PASS.** One Workbench plugin run now auto-fills `max_weight_kg`, `cargo_grid_w/h`
and `character_default_cargo` edges; the export is landed, schema-validated, and
imported with qty preserved end-to-end. Class-R C1–C3 pass with measured values.

## What shipped

### Scanner / plugin (Enfusion, resumed WIP + one fix)
- Resumed WIP verified in place: `DeriveCargoGrid` (w=4, cells=ceil(vol/50), h=max(3,ceil(cells/4))),
  `CollectInitialCargo` (`InitialInventoryItems` → `TargetStorage` + `PrefabsToSpawn`),
  `EmitEdgeUnique` (dedup key includes seq → one edge per spawn entry = qty), plugin emit of
  `cargo_grid_w/h`.
- **New fix (this session): `ReadPhysAttrs` two-pass capacity sourcing** (`ReadPhysAttrsPass`).
  Measured defect: `Jacket_US_BDU` exported 1000 cm³ (→4×5) while its own
  `SCR_UniversalInventoryStorageComponent` serializes **800** (→4×4 = operator screenshot).
  Root cause: the jacket chain also carries a nested `SCR_EquipmentStorageComponent`
  (flashlight slot) whose resolved `MaxCumulativeVolume` (1000) won the class-map iteration
  order. Pass 1 now reads Universal storages only; pass 2 keeps the old any-`*StorageComponent`
  fallback (crates / vehicle trunks / pouches unchanged — spot-checked).

### Export (landed at `packages/tbd-schema/registry/*.workbench.json`)
- 1857 items; **1257 with `cargo_grid_*`** — exactly the set with readable `max_volume_cm3`;
  1033 with `max_weight_kg` (unchanged).
- **20,908 edges = 4,685 legacy (per-family histogram byte-identical to T-068.10.2) +
  16,223 `character_default_cargo`.**
- Garment parity (measured, post-fix):
  | Prefab | vol | grid | screenshot |
  |---|---|---|---|
  | `Jacket_US_BDU` | 800 | **4×4** | BDU Blouse 4×4 ✅ |
  | `Pants_Trousers_01_base` | 600 | **4×3** | BDU Trousers 4×3 ✅ |
  | `Pants_US_BDU` | 1600 | 4×8 | (no pin — serialized engine truth, see spike addendum) |
  | `Vest_SovietHarness_assembled` | 1000 | 4×5 | — |
  | `Vest_PASGT` | — | — | pure armor, no storage → absent per locked decision 3 |

### DB / API / ingest (Rust)
- Migration `0007_registry_cargo.sql`: `registry_items.cargo_grid_w/h`;
  `registry_compat.qty int NOT NULL DEFAULT 1`; unique index widened to
  `(modpack_id, from_node, to_node, edge_type, COALESCE(evidence, ''))` — same item in
  different storages stays distinct; NULL ≡ '' canonicalization preserved.
- `make schema-codegen` re-generated `registry_items.rs` (+`cargo_grid_*: Option<NonZeroU64>`)
  and `registry_compat.rs` (+`CharacterDefaultCargo`) — required before import (serde enum).
- `registry_import.rs`: items UNNEST +2 `int4[]` arrays; **compat aggregates duplicates per
  full identity `(from,to,type,evidence)` into `qty`** (was last-wins drop); ON CONFLICT targets
  the widened expression index; prune matches evidence too.
- `models/registry.rs` + both handler SELECTs + frontend `dto.rs` expose
  `cargo_grid_w/h` and `qty` (serde default 1). R-api items golden unaffected (absent
  optionals stay absent — proven by the golden suite).

## Class-R gates

| ID | Check | Evidence (measured) |
|----|-------|---------------------|
| C1 | trousers ≠ vest | `Pants_Trousers_01_base` 4×3 / 5 kg / 600 cm³ vs `Vest_SovietHarness_assembled` 4×5 / — / 1000 cm³ (SQL); bonus: jacket 4×4/5/800 |
| C2 | US rifleman ≥1 cargo edge | `{26A9756790131354}…/Character_US_Rifleman.et` present; US_Army riflemen: **98 rows, Σqty=331** (mags + medical + radio/compass/map) |
| C3 | no hand-authored rows | diff = scanner/plugin + re-export + ingest code only; capacity values traced to prefab dumps (`game_read`) |
| qty | multiplicity survives | max qty **40** (`Magazine_545x39_AK_30rnd_Last_5Tracer` ×40 in `Backpack_Kolobok` ammo carriers); Σqty = 20,908 = raw envelope count (conservation) |

Sample item (landed export):
```json
{ "resource_name": "{C7861F11D5334C0E}Prefabs/Characters/Uniforms/Jacket_US_BDU.et",
  "kind": "gear_jacket", "max_weight_kg": 5, "max_volume_cm3": 800,
  "cargo_grid_w": 4, "cargo_grid_h": 4 }
```
Sample cargo edges (DB, qty aggregated):
```text
FieldDressing_US_01.et  → Character_US_GL_Guard.et  TargetStorage=Pants/Pants_US_BDU.et   qty=2
Ammo_…_M433.et          → Campaign_US_Player_SF_GL_S.et  …/Vest_M79GrenadeCarrier.et      qty=33
```

## Automated gates (all exit 0)

```bash
make schema-codegen        # regenerated both registry contracts
make schema-validate       # xtask suite PASS
make registry-import       # items 1857/1857; compat total=20908 unique=10604 inserted=5919 updated=0
import-registry --prune    # all zeros — no stragglers under the widened index
make registry-import (2nd) # G2 live: inserted=0 updated=0 pruned=0 (items AND compat)
make test-it               # full IT suite PASS (fresh rust_it DB → migration 0007 exercised)
cargo test --test registry_compat  # G1–G5, G9, G10 + qty/grid pins PASS (3.55s)
make rust-fmt · make rust-clippy   # clean (-D warnings)
make ci-local-leptos       # fmt + clippy(wasm32) + 89 tests + trunk release PASS
```

Census re-pin (old → new, measured): compat rows 4,685 → **10,604**
(= 4,685 legacy + 5,919 aggregated cargo); raw envelope 4,685 → **20,908**;
histogram +`character_default_cargo: 16223`; `db_edge_snapshot` ORDER BY gained
`evidence` (+`qty` in the row) — required now that triples repeat; synthetic G9
grew a triplicate-edge case (raw 9 → unique 7, qty=3; prune drops it back to 1).

## Ops notes (evidence for the runbook)

- `wb_reload` "compilation triggered (`ExecuteAction=false`)" **proven insufficient**: a
  re-export after it still ran the old scanner (jacket stayed 1000). A Workbench restart
  (kill → Steam auto-relaunch → bootstrap `wb_connect`) compiled the disk fix (jacket 800).
- `wb_script_editor` `getLine`/`getLinesCount` are **0-based with a +1 phantom line**:
  disk line N = SE line N−1 (verified on 8 anchors incl. `DeriveCargoGrid` /
  `EmitEdgeUnique`). Stale-probe accordingly.
- `$profile` on this host =
  `~/.local/share/Steam/steamapps/compatdata/1874910/pfx/drive_c/users/steamuser/Documents/My Games/ArmaReforgerWorkbench/profile/`.
  Compat file written last = run-complete sentinel (poll mtime + size-stable).

## Known limitations (explicit, not silent)

- `Pants_US_BDU` 1600→4×8 has no operator screenshot pin; value is the serialized engine
  truth (spike addendum). If in-game paging shows otherwise, it's a formula-scope note for
  T-068.15.2 display, not an export defect.
- LBS/ALICE child-volume **sum** (spike §Cargo grid) was not needed for the Class-R pair
  (jacket + trousers pass via Universal preference; Soviet harness exports its own 1000).
  No garment lost capacity vs the pre-fix export except the corrected jacket family.

## Ready for Cursor

Registry/status/doc sync per this log. Tag: **T-068.15.1**.
