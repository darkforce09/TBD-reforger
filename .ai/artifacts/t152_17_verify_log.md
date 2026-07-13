# T-152.17 — Town label correctness — verify log

**Slice:** T-152.17 (remediation ladder #6) · **Executor:** claude-code
**Spec:** `docs/specs/Mission_Creator_Architecture/t152_17_town_label_correctness.md`
**Worktree:** `.ai/artifacts/worktrees/TBD-T-152` · branch `ticket/T-152` · tag `T-152.17`
**Depends on:** T-152.12 (text lane) · T-152.13 (font) · plays nicely with T-152.16 (heights lane)

## In one sentence

The wgpu town lane now draws **settlements only** (Rust kind filter), the polluted `kind` data is
fixed via the classifier + regen, the stale-lane bug (labels stuck on zoom-in) is fixed in the
engine, and the hard z>2.0 cut is replaced with an alpha fade over z∈[2.0, 3.0].

## Operator decisions (this slice)

| Topic | Decision | Rationale |
|-------|----------|-----------|
| **Zoom-in (M2)** | **Fade** over z∈[2.0, 3.0] (α 1→0), gone above 3.0 | Spec L4 default; smoother than a hard cut |
| **Wide band** | **min = −4.5**; below z=−3.0 only `importance ≥ 0.70` towns draw | Island-fit zoom (z≈−3.68) shows ~5 capital names |
| **Highstone** | **Keep `kind:"peak"`** on the T-152.16 heights lane; **remove from `REQUIRED_EVERON_TOWNS`** (8→7) | Avoids the `.16` heights-sidecar orphan; Highstone is a named peak there, not a town-lane settlement |
| **Raccoon Rock** | Reclassify `natural` → `village` (stays required) | Required settlement; not in the heights sidecar, so no `.16` collision |

## Root cause — stale labels at z=4.48 (operator req 2)

Both label lanes cap below z=4.48 (town max 2.0, heights max 3.0), so the label was **stale**, not
in-band. `Engine::upload_town_labels` (`engine.rs`) on an empty-but-`visible` upload only zeroed the
**write-only** `town_labels_drawn` scalar and left the `WorldTownLabels` Batch resident — but
`draw_batches` draws `IconInstanced` from the **Batch's `count`** (`engine.rs:921`/`943`), so the old
glyph buffer kept drawing when the draw set emptied on zoom-in. **Fix:** `remove_lane(WorldTownLabels)`
**unconditionally** on empty. The `useWgpuTownLabels` rAF loop already re-syncs on `eng.zoom` deltas,
so no TS change was needed (language gate: Rust owns lane policy; zero TS added).

## Changes

**Rust (lane policy — `world/importance_declutter.rs`)**
- Constants: `TOWN_LABEL_MIN_ZOOM −3.0→−4.5`; new `TOWN_LABEL_WIDE_ZOOM=−3.0`,
  `TOWN_LABEL_WIDE_MIN_IMPORTANCE=0.70`, `TOWN_LABEL_FADE_END=3.0`, `TOWN_LABEL_FADE_ENABLED=true`.
- `town_lane_kind_ok(kind, z)` — settlement filter (`town/village/airport`; `locality` z≥0; else out).
- `town_label_fade_alpha(z)` — 1.0 ≤2.0, linear to 0.0 at 3.0, 0.0 above.
- `should_draw_town_label` gains band-ceiling + kind filter + wide-band importance gate.
- `town_label_zoom_ceiling()` helper. Re-exported from `world/mod.rs`.

**Rust (pack fade — `text_layout.rs`)** `pack_town_label_bytes` scales the tint alpha
`234 · town_label_fade_alpha(z)` → alpha baked into the pack tint (no shader change).

**Rust (engine — `engine.rs`)** `upload_town_labels` drops the lane on empty (stale-lane fix).

**Rust (wasm — `lib.rs`)** exports `town_label_fade_alpha` (verify oracle).

**Node classifier (`lib/locations-export.mjs`)** sub-feature regex → `kind:"locality"`
`importance 0.40`; Raccoon Rock supplement `natural→village`; Highstone removed from
`REQUIRED_EVERON_TOWNS`; `verifyLocationsGates` G7 hygiene (no sub-feature `town`; locality ≤ 0.45).

**Schema/golden** `locality` added to the `kind` enum; one `locality` golden row.

**Node gate (`verify-town-labels.mjs`)** G1 kind hygiene + G4 fade/band-edge assertions.

## Census by kind (`locations.json`, 60 rows)

| kind | before | after |
|------|-------:|------:|
| town | 23 | **19** |
| locality | 0 | **4** (Le Moule Sawmill 01, Montignac Farm 01, Montignac Sawmil 01, North East Farm 01 — all @0.40) |
| village | 2 | **3** (Gorey, Kermovan, **Raccoon Rock**) |
| peak | 17 | 17 (incl. Highstone) |
| hill | 16 | 16 |
| natural | 1 | **0** |
| airport | 1 | 1 |
| **total** | 60 | 60 |

**Drawn set on the town lane @ z=−2** = **23** (19 town + 3 village + 1 airport; localities are z≥0
only) — **0** peak/hill/natural. Pack = 170 glyph instances. Required towns = **7** (all ⊆ drawn).

## Gate results (all green)

| Gate | Command | Result |
|------|---------|--------|
| G1 kind hygiene @ z=−2 | `verify-town-labels.mjs` | PASS — 23 drawn ⊆ {town,village,airport,locality}; 0 peak/hill/natural |
| G2 required-towns (7) ⊆ drawn @ z=−2 | `verify-town-labels.mjs` | PASS |
| G3 declutter invariant + wasm oracle | `verify-town-labels.mjs` | PASS |
| G4 fade α: 2.0→1.0, 2.5→0.5, 3.0→0.0 | `verify-town-labels.mjs` | PASS |
| G4 band edges: |drawn|=0 @ z=3.1 and z=−4.6 | `verify-town-labels.mjs` | PASS |
| G5 empty source → 0 drawn; pack len | `verify-town-labels.mjs` | PASS |
| Schema valid + census (7 required) | `verify-locations.mjs` | PASS |
| G7 kind hygiene (no sub-feature town; locality ≤ 0.45) | `verifyLocationsGates` | PASS |
| **.16 regression** — Highstone still peak, 23 named trace, no orphan | `verify-height-labels.mjs` | PASS |
| `make schema-validate` (incl. `locality` enum) | make | OK |
| Rust core (`--all-features`) | `cargo test -p map-engine-core --all-features` | 213 passed |
| Rust render | `cargo test -p map-engine-render` | 42 passed |
| clippy core+wasm `-D warnings` | `cargo clippy … --all-features` | clean |
| rustfmt (core/render/wasm) | `cargo fmt --check` | clean |
| wasm build | `make wasm` | OK |
| FE tests | `npm test` | 356 passed (48 files) |
| FE build / lint | `npm run build` / `npm run lint` | clean |

> **Ops note:** the `world` module is behind `#[cfg(feature = "world")]`; run core tests with
> `--all-features` (as CI's `wasm-ci` does) or the town-label tests are silently filtered out.
> The regen source `packages/map-assets/everon/staging/export/raw-entities.jsonl` is a symlink into
> the main checkout — it resolved, so the artifact was regenerated (not hand-edited).

## Manual acceptance (operator GPU pass — pending)

- **M1** island view: settlements only (~5 capitals at z≈−3.68); hills carry height labels.
- **M2** zoom past +2: names fade out by z=3.0 (recorded decision = **fade**). Automated coverage:
  `town_label_fade_alpha` endpoints + the engine lane-clear (GPU-observable via the headless WebGL2
  harness / operator pass).

## New Rust tests

`world::importance_declutter::tests` — `kind_filter_excludes_non_settlements`,
`locality_only_at_zoom_ge_0`, `wide_band_importance_gate`, `fade_alpha_endpoints`,
`zoom_band_hides_outside_range` (updated for the [−4.5, 3.0] band).
