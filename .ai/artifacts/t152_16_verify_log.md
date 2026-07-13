# T-152.16 — Height markers visible + credible — verify log

Slice **T-152.16** · branch `ticket/T-152` · tag `T-152.16` · depends on T-152.12 (text lane) + T-152.13 (font).
Authority: audit `t152_11_fidelity_audit_report.md` §6.3 (S8, D9, A6) · spec `docs/specs/Mission_Creator_Architecture/t152_16_height_markers_visible.md`.

## Decisions

- **Band** `[−2.0, +3.0]`; **`PEAK_MIN_VALUE_M = 80`**; prominence `≥ 15 m` (kept). LANGUAGE GATE honoured: band/floor/pack/sampling in Rust; Node exporter = data plumbing; TS gained only an optional `name?` type field.
- **Q1 (contour) → FRESH WAIVER.** No contour-index-label code this slice; see §Contour decision. The T-152.7 waiver is **not** reused.
- **Q2 (sub-80 named) → ONE FLOOR RULE (operator re-confirmed after data).** The named merge revealed that **10 of 33** `locations.json` peak/hill rows are coastal features mis-tagged `peak`/`hill` (A8 pollution) that DEM-sample below the floor: `Coast=1 m`, `headland=13 m`, `Peninsula=20 m`, `Lighthouse=24 m`, `beach=49/61 m`, plus low hills `48/55/67/71 m`. The initial answer ("keep named <80") assumed all named ≥80; the data falsified it, so it was re-surfaced. Resolution: the **80 m floor applies to named and unnamed alike** — coastal mis-tags are dropped, not shipped as "height markers". Kind fixes remain **T-152.17/.19**.

## Changes

| File | Change |
|------|--------|
| `crates/map-engine-core/src/dem/peaks.rs` | `HEIGHT_LABEL_MIN/MAX_ZOOM` + `PEAK_MIN_VALUE_M` consts; `name: Option<String>` on `HeightLabel`; 80 m floor in `find_peaks`; `should_draw_height_label` + band gate in `declutter_height_labels`; name-aware `height_labels_to_specs` (`"{name} · {value} m"`); global-max injection floor-gated (see §Deviation); +3 tests (G1/G2/G3). |
| `crates/map-engine-wasm/src/lib.rs` | `HeightLabelJson` optional `name` (serde default + skip); `label_from/to_json` pass name; new `sample_dem_elevations` (batch DEM sampler, reuses `sample_elevation_from_meters_cache`); new `peak_min_value_m` (single-source the floor). |
| `scripts/map-assets/export-height-labels.mjs` | `--terrain` arg; read `locations.json` peak+hill; DEM-sample via wasm; **floor named rows** at `peak_min_value_m()`; dedupe DEM peaks within 200 m of a named row; write post-floor set; census + dropped report. |
| `scripts/map-assets/verify-height-labels.mjs` | New G2 floor (all rows), G3 named-merge + DEM completeness, G4 200 m dedupe; skip-not-fail when wasm/DEM absent; ASL oracle over all rows. |
| `packages/tbd-schema/schema/height-labels.schema.json` (new) + `scripts/validate.mjs` | Additive schema (optional `name`); live everon sidecar validated in `make schema-validate`. |
| `Makefile` | `schema-validate` now runs `verify-height-labels.mjs`. |
| `apps/website/frontend/src/features/tactical-map/wgpu/wgpuHeightLabels.ts` | `HeightLabelRow.name?` (type only — name round-trips through wasm declutter/pack unchanged; `useWgpuHeightLabels` needs no logic change). |
| `packages/map-assets/everon/height-labels.json` | Regenerated: 10 unnamed knolls → 26 credible rows. |

## Gate results — automated, all PASS

| Gate | Result | Evidence |
|------|--------|----------|
| **G1** band | PASS | `cargo … peaks::zoom_band_gates_height_labels`: draw @ −2.0/+3.0, hidden @ −2.01/+3.01; `declutter` empty out-of-band. |
| **G2** floor | PASS | `peaks::value_floor_drops_sub_80_knolls` (90 m kept, 55 m floored); `verify-height-labels`: 0 rows < 80 m (23 named + 3 DEM). |
| **G3** named merge | PASS | `verify-height-labels`: 23 named rows all trace to `locations.json`; **completeness** — sidecar named set == 23 locations peak/hill ≥ 80 m (DEM oracle). |
| **G3** pack text | PASS | `peaks::named_label_packs_name_and_value`: `"Highstone · 372 m"` / bare `"210"`. |
| **G4** dedupe | PASS | `verify-height-labels`: no DEM peak within 200 m of a named row. |
| **G5** declutter cap | PASS | `verify-height-labels`: 26 ≤ 48 drawn @ z=0 (sep 80 m). |
| **G6** max ≥ 350 | PASS | `peaks::everon_peaks_max_above_350` (`--features png`); `verify-height-labels`: max = 375. |
| **ASL oracle** | PASS | `verify_height_labels_json`: value_m within ±0.5 m of DEM sample for all 26 rows incl. named. |
| **G7** regression | PASS | `cargo test -p map-engine-core` 78 + peaks 8 (png) + `-p map-engine-render` 41; clippy core + wasm clean; `make wasm`; `make schema-validate`; FE `npm test` 356 / `build` / `lint` — all exit 0. `apps/**` = one type-only field. |

**G5 GPU render note:** the text lane itself (alive + upright + readable) is the shipped deliverable of T-152.12/.13; this slice feeds it credible data and gates the pack path via unit tests + the FE vitest suite. On-screen upright render is confirmed by operator **M1** below.

## Sidecar census (RETURN)

- **Count:** 26 (was 10). **Named:** 23 (88 %). **Unnamed DEM:** 3.
- **min `value_m`:** 113 (was 15). **max:** 375. **All ≥ 80.**
- **Named summit:** `Moutains West Hill 01 · 375 m` @ (8087, 2752) — the plateau global-max, deduped from the anonymous DEM injection into its toponym.
- **Unnamed DEM peaks kept:** 236, 159, 150 m (genuine prominence peaks ≥ 80, none within 200 m of a named row).
- **Dropped (10 sub-80 named, coastal mis-tags):** Mountains East Coast 11=1, Moutains West headland 01=13, Mountains East Peninsula 01=20, Moutains West Lighthouse 01=24, St Philippe Hill 01=48, Moutains West beach 01=49, Center North Hill 02=55, Moutains East beach 01=61, Center West Hill 06=67, Tyrone Ridge 01=71.
- **Legacy knolls (29/15/23/21/55)** — gone.

## Deviation from spec (recorded)

- The spec said "drop the injected-summit hack (real max already qualifies)." **Empirically false on Everon:** the ~375 m summit sits on a plateau and fails the 9×9 local-prominence gate, so `find_peaks` with the injection removed dropped the max below 350 → `everon_peaks_max_above_350` **FAILED**. Per the plan's stated risk mitigation, the global-max injection is **restored but floor-gated** (injected only when `≥ PEAK_MIN_VALUE_M`). The exporter's 200 m named-dedupe then replaces the anonymous summit with `Moutains West Hill 01` in the shipped sidecar, so the summit renders **named** and `find_peaks` alone stays G6-honest.
- Q2 flipped from "keep named <80" to "floor named too" after the merge surfaced the coastal mis-tags; operator re-confirmed. One floor rule for named + unnamed.

## Contour decision (G6 / M3) — FRESH WAIVER, operator-pending

- Q1 = **waiver**. No contour index labels shipped in T-152.16; `height_contour_labels_waived()` stays `true`. The T-152.7 waiver is **void** ("no inherited waiver").
- **FRESH operator waiver quote (required by G6):** `‹OPERATOR TO RECORD AT M3›` — a new sentence against the new visible baseline (26 credible named/DEM peaks @ z ∈ [−2, +3]). Do not reuse `t152_7_verify_log.md:22`.
- **Status: G6 contour decision is OPERATOR-PENDING.** All automated peak gates are green; only the fresh waiver sentence remains for the operator to sign.

## Manual acceptance — operator-pending

- **M1** z=0 ridge pan: named summits read `"{Name} · {N} m"`, upright + legible.
- **M2** z=−4 island: no height-label clutter (band hides labels at z=−4).
- **M3** contour decision signed: paste the fresh waiver quote above.
