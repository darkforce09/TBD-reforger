# T-152.21 — Landmark early visibility (`importanceZoom` wired) — verify log

**Slice:** T-152.21 · **Worktree:** `.ai/artifacts/worktrees/TBD-T-152` · **Branch:** `ticket/T-152` · **Tag:** `T-152.21`
**Spec:** `docs/specs/Mission_Creator_Architecture/t152_21_landmark_early_visibility.md`

## Problem (audit A4/A5)
`render.importanceZoom` was built in four layers (schema, `prefab-classify.json`, LOD contract §N2,
Rust `PrefabRow.importance_zoom` parse) and **read by no render path**. At the Mission Creator's
default zoom (−2), landmarks — lighthouses, castles, military — drew only as OBB rectangles
(lighthouse fill `[235,235,235,220]` = the literal P1 white square). Badges appeared only at
`deckZoom ≥ +1` (`BUILDING_BADGE_MIN_ZOOM`).

## What shipped (Rust owns override policy — language gate)

### 1. Wire the override — badge path (`crates/map-engine-core/src/world/{obb.rs,residency.rs}`)
- `BuildingPrefabInfo` gains `importance_zoom: Option<f64>`, parsed from `render.importanceZoom` in
  `building_prefab_lookup` (`obb.rs`). This is the struct the badge loop already fetches per
  instance — no key round-trips.
- `WorldResidency.min_importance_zoom` cached in `load_prefabs_gz` (cheap O(1) outer guard).
- Badge gate (`rebuild_glyph_buffers`): outer `badge_want = toggle_buildings && (class gate OR any
  resident override active)`; per-instance `if !badge_gate && !importance_ok { continue }` where
  `importance_ok = deck_zoom ≥ binfo.importance_zoom`. Semantics = contract §N2 (`deckZoom ≥
  importanceZoom` overrides the `buildingBadge` class gate). `BUILDING_BADGE_MIN_ZOOM` (1.0)
  **unchanged** for non-landmarks. Size floor `BADGE_SIZE_MIN_PX` (8.0) unchanged.
- **Residency band fix (required by M2/G1):** the building footprint gate (`BUILDING_MIN_ZOOM =
  −2.5`) cleared pinned chunks below −2.5, which would have starved the badge lane at z=−4. Now
  `set_viewport` keeps chunks resident while `deck_zoom ≥ min_importance_zoom`; `rebuild_buffers`
  gates **fills/outlines** at −2.5 (no rectangles below it) while the badge lane still draws in the
  `[min_importance, −2.5)` band → "landmark badges persist, ordinary buildings gone."

### 2. Fill de-emphasis handoff — G4 (`residency.rs`)
- `early_landmark_glyph_active(cls, importance_zoom)` — true only below the badge band, when the
  override fires **and** the atlas carries the landmark glyph (state-conditioned; missing glyph ⇒
  bright fill kept → G4 "restored if empty").
- `rebuild_buffers` swaps the bright class tint for the neutral `FILL_DEFAULT` when active, so the
  glyph is the coarse-zoom face (no double-draw shout). `fill_band_changed(prev,next)` triggers a
  fill recompose on a pure zoom change that crosses the footprint gate, the badge band, or the
  importance boundary (fills are no longer static once de-emphasis is zoom-dependent).

### 3. Classify data-add (`packages/tbd-schema/rules/prefab-classify.json`) + offline fixture regen
Landmark-class rules that lacked `importanceZoom` were brought to parity at **−4** (the
already-committed landmark value; contract §N2 recommends −4). Recorded per spec §Out-of-scope
("landmark class verifiably lacks the field — record it"):

| rule # | class | match | had importanceZoom? | now |
|--------|-------|-------|---------------------|-----|
| #13 | lighthouse | `Lighthouse` | no | **−4** |
| #16 | castle | `Structures/Cultural/Castles`, `Castle_Ruins` | no | **−4** |
| #25 | tower | `GuardTower`/`ControlTower`/`Transmitter…`/`WaterTower`… | no | **−4** |
| #26 | military | `GuardHouse`/`GuardBox` | no | **−4** |
| #50 | military | `Structures/Military/` | −4 (unchanged) | −4 |
| #55/#59 | tower | `Structures/Industrial/Towers`, `Structures/Airport/ControlTower` | −4 (unchanged) | −4 |

Adding #25/#26 makes the `tower`/`military` classes **coherent** (both split rules now tagged;
previously one rule per class had the override and the other didn't). **Excluded (recorded):**
`bridge` (#12) and `bunker` (#49) — not contract §N2 landmarks, bespoke/incomplete render (bridge
has its own deck+casing path, bunker rule has no `iconKey`); not P1 "white square" symptoms.

**Reproducibility gate (safety):** regen with NO rule change → byte-identical artifact (sha256
`03b55ac6…` before == after, empty `git diff`) — proves the local staged-raw + `build-world-objects.mjs`
reproduce the committed `prefabs.json.gz` exactly. After the 4 rule edits:
`node scripts/map-assets/build-world-objects.mjs --terrain everon --phase P5_props --patch-manifest`
- Census **unchanged**: 1623 prefabs / 1,216,109 instances / 315 chunks.
- Diff = importanceZoom-only: `prefabs.json.gz` 120001→120203 B; fixture `importanceZoom:-4`
  entries 5 → **36** (lighthouse 3, castle 14, tower 9, military 10); `type-inventory.json` gains
  `importanceZoom:-4` on the castle/lighthouse/military/tower class rows (counts unchanged).

### 3b. Test expectations updated to the new (correct) behavior — recorded
The fix intentionally changes two behaviors that pre-existing tests pinned to the OLD state.
Updating the expected values keeps the tests honest (not TS policy — Rust owns policy):
- **Rust** `g3_zoom_gate_below_one_only_importance_landmarks` (was `..._badges_off_below_one`) +
  the badge oracles (`oracle_badge_count` gains the importance term): below the badge band, badges =
  importanceZoom-tagged landmarks (was 0). g4/g5 unchanged at z ≥ 1 (importance is a no-op there).
- **FE** `world.landmark-glyphs.parity.test.ts`: z=0.9 now asserts landmarks-on (`> 0`, `< z=2 count`)
  instead of `== 0`.
- **FE** `world.residency.parity.test.ts` golden `residency_everon_v1.json`: regenerated via
  `T151_CAPTURE_RESIDENCY=1`. Diff is **exactly one field** — step 14 (`zoom -3`, below the −2.5
  footprint gate) `pinnedBuildingCount 0 → 139`: chunks now stay resident there so landmark badges
  persist (M2). `missingIds`/`residentIds` and all other steps unchanged (surgical).

### 4. Oracle decision (L5 / G5): **DELETE**
The TS mirror is `visibleWithImportance` (`lodGates.ts` — the spec/audit's stale `landmarkVisible`
name). Zero render callers; the whole file is oracle-only; no wasm importance export exists to
parity-check against. Per the language gate (Rust owns override policy), the dead mirror was
**deleted** (function + its `describe` block/import in `lodGates.test.ts`; `classVisible` doc updated
to point at the Rust engine). Grep gate: `rg visibleWithImportance apps/website/frontend/src` and
`rg landmarkVisible apps/website/frontend/src` → **empty**.

## Acceptance gates (spec §Mathematical acceptance matrix)
| Gate | Result |
|------|--------|
| **G1** synthetic override boundary: badge at z=−4.0, absent at z=−4.1; non-landmark unchanged | PASS — `t152_21_landmark_early_visibility` (real lighthouse @ importanceZoom −4) + `obb::lookup_parses_importance_zoom` |
| **G2** Everon lighthouse chunk: `badge_glyph_count > 0` at z=−2 | PASS — same test; buffer contains `building-lighthouse` glyph |
| **G3** early badge size ≥ `BADGE_SIZE_MIN_PX` at z=−4 | PASS — `size_with_min_px(.., 8.0, z)` unchanged; verified in code path |
| **G4** lighthouse bright fill suppressed at z<1 when glyph drew; restored if empty | PASS — `t152_21_fill_deemphasis_handoff` |
| **G5** `visibleWithImportance` wired-or-deleted (no un-called export) | PASS — deleted; grep empty |
| **G6** cargo/wasm/FE suites exit 0 | see below |

## Verify results
```
cargo fmt --check -p map-engine-core                    → clean (exit 0)
cargo clippy -p map-engine-core --all-targets --all-features -D warnings → clean
cargo test -p map-engine-core --all-features            → 216 passed / 0 failed (+ camera 5, ortho 5)
make wasm                                               → Done (exit 0). pkg is gitignored (build-verify only; no binary to commit)
cd apps/website/frontend && npm test                    → 363 passed / 0 failed (49 files) [after golden + smoke-test update]
                              npm run build              → built in 935ms (exit 0)
                              npm run lint               → clean (exit 0)
rg -n "landmarkVisible|visibleWithImportance" apps/website/frontend/src → empty (G5)
```
Note: the T-152.3 badge tests live behind the `world` feature; run with `--all-features` (as
Makefile `test-core` / CI do). The bare `cargo test -p map-engine-core` from the spec §Verify runs
only the 78 non-world lib tests — the G1–G4 gates need `--all-features`.

## Early-landmark census at z=−2 (badge count)
Pre-fix: **0** landmark badges at the default editor zoom (all rectangles — the P1 symptom).
Post-fix, on the fixture lighthouse chunk `2_12` (replicated across the strict draw set): **44**
landmark badges at z=−2 (was 0; measured via a temporary instrument, reverted). Island-wide, the
newly early-visible building instances are the importanceZoom-tagged classes: lighthouse 21 + castle
104 + military 48 + tower 124 (tower badges airfield-gated) = 297 instances now carry the −4 override.

## Manual (M1/M2)
The badge/fill buffers are the data the wgpu `IconInstanced`/fill lanes draw (unchanged since
T-151.5/.6). M1 (z=−2 landmarks read as icon badges, not white squares) and M2 (z=−4 landmark
badges persist, ordinary buildings gone) are proven at the **buffer** level by G2 (badge buffer
contains `building-lighthouse` at z=−2), G4 (white fill de-emphasized at z=−2), and
`t152_21_landmark_early_visibility` (badge count 0 past the −4 boundary). On-GPU visual confirmation
is the operator pass (existing wgpu glyph pipeline; no render-lane change this slice).

## Oracle decision (RETURN)
**Delete** `visibleWithImportance` — Rust owns the override policy (language gate); the TS helper had
zero render callers and no parity export, so no dead mirror remains.
