# T-152.12 verify log — text lane resurrection (16 B uniform) + upright orientation

**Slice:** T-152.12 · **Branch:** `ticket/T-152` · **Worktree:** `.ai/artifacts/worktrees/TBD-T-152`
**Spec:** [`t152_12_text_lane_orientation.md`](../../docs/specs/Mission_Creator_Architecture/t152_12_text_lane_orientation.md) · **Audit basis:** [`t152_11_fidelity_audit_report.md`](t152_11_fidelity_audit_report.md) §7

## Summary

The wgpu text lane is alive and upright. Landed the previously-uncommitted `TextUniforms` 16 B pad fix (3×f32 — a `vec3` pad align-16s the WGSL struct to 32 B against the 16 B `min_binding_size`, failing pipeline creation), added the missing V-flip in `vs_text` (world-top now samples the y-down atlas cell top, matching `vs_textured`), and locked both bug classes behind gates: WGSL source guards, a CPU UV oracle (`text_layout::glyph_cell_uv`), and a new `text_self_check` GPU probe that renders glyph `'7'` through the real text pipeline and asserts upright orientation by readback. Operator confirmed upright text on real WebGPU hardware (screenshots archived). Legibility remains poor by design of the current 8×8 font/size policy — routed to T-152.13/.16/.17, **not** reopened here.

## Gate table

| ID | Predicate | Result | Evidence |
|----|-----------|--------|----------|
| **G1** | `TextUniforms` block: no `vec3`, exactly 4×f32; `vs_text` contains the V-flip | **PASS** | `text_layout.rs` tests `g1_text_uniforms_is_16_bytes_no_vec3`, `g1_vs_text_has_v_flip` (source guards over `include_str!("shader.wgsl")`) |
| **G2** | UV corner proof: unit(0,1)→(u0,v0), (0,0)→(u0,v1), (1,1)→(u1,v0); U not mirrored | **PASS** | `g2_glyph_cell_uv_corners_upright` + `g2_seven_is_top_heavy_in_atlas` (probe-geometry cross-check) |
| **G3** | Text pipeline creates without validation error, headless | **PASS** | `text_self_check` runs inside `verify-wgpu-gpu.mjs` — pipeline is created from the same WGSL + 16 B layout; check `pass:true` |
| **G4** | Upright glyph readback on both backends | **PASS** | WebGL2 (SwiftShader, committed harness): 4/4 probes byte-exact — top bar (400,250)=[240,240,230,255], bottom stroke (370,350)=[240,240,230,255], V-flip trap (400,350)=CLEAR [51,68,85,255], exterior CLEAR. **WebGPU: operator visual PASS on real hardware** (`t152_12_operator/upright_z4.48_font_quality.png` — label reads left-to-right, right-side-up). Note: the committed harness pins `?force=webgl` (WebGL2-only by design); a scratchpad default-backend run was attempted and superseded by the operator evidence |
| **G5** | Label lanes draw through the shared pipeline | **PASS** | `text_self_check` exercises the exact `vs_text`/`fs_text` pipeline + 20 B instance layout shared by `WorldLabels`/`WorldTownLabels`/`WorldRoadLabels` (`engine.rs` lane dispatch); operator screenshots show live lane output end-to-end |
| **G6** | fmt, clippy (native + wasm32), cargo tests, `make wasm` | **PASS** | fmt clean; clippy `-D warnings` clean both targets; core **85** (75+5+5) / render **35** (29→35, +6 this slice); wasm builds |
| **G7** | FE diff = exactly the one-line spike `text:` registration; FE suites green | **PASS** | `git diff --stat apps/` = `WgpuCanvas.tsx | 1 +`; vitest **355/355**, `npm run build`, `npm run lint` clean |

## Pinned numbers

| Quantity | Value |
|----------|-------|
| `TEXT_UNIFORM_BYTES` | 16 (unchanged; WGSL struct now actually 16 B) |
| Wasm `map_engine_wasm_bg.wasm` | **4,343,798 B** |
| Render crate tests | 29 → **35** |
| GPU self-checks registered | 8 → **9** (`text`) |
| Operator screenshots | 5 (`.ai/artifacts/t152_12_operator/`) |

## Automated commands

```bash
cargo fmt --check                                                    # → 0
cargo clippy --all-targets -- -D warnings                            # → 0
cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings  # → 0
cargo test -p map-engine-core                                        # → 0 (85)
cargo test -p map-engine-render                                      # → 0 (35)
make wasm                                                            # → 0 (4,343,798 B)
node scripts/website/verify-wgpu-gpu.mjs                             # → 0 (allPass:true incl. text 4/4 probes)
cd apps/website/frontend && npm test && npm run build && npm run lint # → 0 (355/355)
```

## Notes / decisions

1. **Committed tags .7–.10 could not have rendered at all.** The text pipeline is created in the `RenderEngine` constructor; with the committed 32 B struct vs 16 B `min_binding_size`, wgpu validation fails engine construction — so every successful browser/harness run during .7–.10 must have used the then-uncommitted hotfix in the working tree. Relevant to audit A14 (gates ran against a dirty tree); the hotfix is now a real commit with source guards.
2. **rust-1.95 lint drift fixed in passing** (pre-existing at `e9319ceb`, blocked this slice's G6): `dem/peaks.rs` collapsible-if + `sort_by_key(Reverse)`, `importance_declutter.rs` range-contains, `road_labels.rs` test `slice::from_ref`, two `#[allow(clippy::too_many_arguments)]` on wasm-bindgen API exports in `map-engine-wasm`. Also wired `text_atlas.bytes` into the GPU-memory accounting sum (was a dead field → wasm-target dead_code error).
3. **Spec amendment (pre-ship, same slice):** G7/L6 reworded to permit exactly the one-line spike-page self-check registration (thin wasm call, same pattern as the 8 existing checks). No other TS.
4. **Operator evidence routed forward** (quotes verbatim): "it is unreadable zoomed out (you have to be really zoomed in to read the text, same for webgl2)" · "the font also looks like shit" · "Pretty sure this town is missing a name" · "more missing names". → size floor + font = **T-152.13** (addendum added); town band misses island view (no labels at z=−3.68) + a label visible at z=4.48 despite band max +2 (suspect: labels not re-decluttered on zoom change) = **T-152.17** (addendum added); harbor/coverage gaps = **T-152.16/.17/.19**; Reforger in-game map name layer (HORNBEAM VALLEY / Ramtop Meadows / Raccoon Rock in `workbench_ingame_map_names_reference.png`) = extraction target proof for **T-152.19** (addendum added).

## Manual (operator)

| ID | Check | Result |
|----|-------|--------|
| M1 | Labels visible + **upright** at z ∈ [−2, +1] | **PARTIAL** — upright **PASS** (WebGPU screenshots); visibility/legibility **FAIL** at those zooms (~6 px 5×7 font) → in-scope for T-152.13/.16/.17, not this slice |
| M2 | No mirrored/flipped glyphs across pan/zoom | **PASS** (operator, real WebGPU) |

## Prior slices

T-152.0–.10 shipped (see logs) · T-152.11 audit @ `a8a7a22c` · T-152.11.1 scaffold @ `e9319ceb`.

## Ready for

Registry `T-152.12 → shipped` + advance → **T-152.13** (readable text atlas — size-floor addendum included). Session stops here per operator directive.
