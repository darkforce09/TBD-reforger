# T-152.13 verify log — readable text atlas (font fidelity)

**Slice:** T-152.13 · **Branch:** `ticket/T-152` · **Worktree:** `.ai/artifacts/worktrees/TBD-T-152`
**Spec:** [`t152_13_text_atlas_fidelity.md`](../../docs/specs/Mission_Creator_Architecture/t152_13_text_atlas_fidelity.md) · **Audit basis:** [`t152_11_fidelity_audit_report.md`](t152_11_fidelity_audit_report.md) §7.3 (A11, S4)

## Summary

The procedural 8×8 px 5×7 stroke font is replaced by a real embedded pixel font: **Spleen 16×32**
(BSD-2-Clause, Frederic Cambus) baked into **32×32 px cells** on the same 16×6 grid → atlas
**512×192** (was 128×48). Full printable ASCII 32–126 with **distinct lowercase**, a documented
accent-fold map (é→e …, curly quotes, en/em dash), and a visually-obvious hollow-`□` **tofu** in
cell 95 replacing the silent O-blob. Grid dims now travel bake→shader through the
`TextUniforms.grid_cols/grid_rows` fields (written from `text_layout::TEXT_ATLAS_{COLS,ROWS}` at
atlas upload) — `vs_text` carries **no hardcoded 16/6**. Size policy retuned per spec: char cell
**16 px @ REF_ZOOM, min 14 px** (was 12/6 — the microscopic-text half of operator S4). Pen
advance is now `char_m × 0.5` (ink is half the square cell), so labels keep true monospace
proportions. 20 B instance format, `ensure_text_atlas()` entry, and all TS untouched.

Font provenance: extracted from `spleen-2.2.0.tar.gz`
(sha256 `ec42925c6b56d2138c862b2f97147c872e472f674bf03423417d827a08d69a89`) by the committed
generator `scripts/website/gen-text-font-table.mjs` → `crates/map-engine-render/src/text_font_table.rs`
(96 × 32 × u16 = 6,144 B of raster data; license + regen command in both headers).

## Gate table

| ID | Predicate | Result | Evidence |
|----|-----------|--------|----------|
| **G1** | Atlas cell ≥ 16×16 px; dims test updated | **PASS** — cell **32×32**, atlas 512×192 | `atlas_size` asserts dims from the shared constants |
| **G2** | ∀ ch ∈ ASCII 32–126 dedicated glyph ≠ fallback; lowercase ≠ uppercase raster | **PASS** | `g2_full_ascii_coverage_distinct_lowercase` (every printable has ink; a–z rasters ≠ A–Z; table tofu slot zeroed), `g2_tofu_cell_is_painted_box` |
| **G3** | 0 tofu hits over committed `locations.json` ∪ `road-names.json` names ∪ height numerals, after accent fold | **PASS — 0 fallback hits** | `g3_committed_label_data_no_tofu` parses both committed JSONs (66 names) + `"0123456789"`, asserts `glyph_index_for_char ≠ TOFU_GLYPH` per char, prints offenders on fail (none); `g3_fold_and_tofu_mapping` spot-checks é→e, Ü→U, 日→tofu |
| **G4** | T-152.12 gates still green (WGSL guard, UV corners, GPU upright readback) | **PASS** | `g1_text_uniforms_is_16_bytes_no_vec3` (still 4×f32), `g1_vs_text_has_v_flip`, `g2_glyph_cell_uv_corners_upright` (constants-derived), `g2_seven_is_top_heavy_in_atlas` rebuilt on the Spleen '7'; `text_self_check` **5/5 probes byte-exact** headless (was 4) — top bar (377,252), descender (392,312), **U-mirror trap** (407,312)=CLEAR, **V-flip trap** (377,347)=CLEAR, exterior (400,50)=CLEAR |
| **G5** | Wasm delta ≤ +256 KiB | **PASS — +3,637 B** (4,343,798 → **4,347,435 B**) | 6 KiB raster table − deleted 5×7 stroke synth ≈ +3.6 KiB net |
| **G6** | fmt/clippy/tests + `make wasm` + FE test/build/lint exit 0; no `apps/**` diffs | **PASS** | table below; `git status`: only slice files + known pre-existing drift (`map_export_everon.json`, `registry_import.rs` — not staged) |

## Pinned numbers

| Quantity | Value |
|----------|-------|
| Atlas | 16×6 cells @ **32×32 px** → **512×192** RGBA (was 8×8 → 128×48) |
| Charset | ASCII 32–126 full + fold map; tofu = cell 95 hollow □ |
| Advance | `TEXT_GLYPH_ADVANCE_RATIO = 0.5` (16 px ink / 32 px cell) |
| Size policy | 16 px @ REF_ZOOM 3, **min 14 px** (was 12/6) |
| Wasm `map_engine_wasm_bg.wasm` | 4,343,798 → **4,347,435 B** (**+3,637 B**; ceiling +262,144) |
| Render crate tests | 35 → **40** (+5: coverage, tofu box, no-blob, fold map, L2 grid guard; '7' + UV rebuilt) |
| Font table | `text_font_table.rs` — Spleen 16×32 v2.2.0, BSD-2-Clause, 96×32×u16 |

## Automated commands

```bash
cargo fmt --check                                                     # → 0
cargo clippy --all-targets -- -D warnings                             # → 0
cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings  # → 0
cargo test -p map-engine-core                                         # → 0 (85 = 75+5+5)
cargo test -p map-engine-render                                       # → 0 (40)
make wasm                                                             # → 0 (4,347,435 B)
node scripts/website/verify-wgpu-gpu.mjs                              # → 0 (allPass:true incl. text 5/5)
cd apps/website/frontend && npm test && npm run build && npm run lint # → 0 (vitest 355/355)
```

## Notes / decisions

1. **Font choice:** system console fonts rejected on license (kbd Terminus 4.20 README = GPL-2.0+;
   sun12x22 = kernel-sourced). Spleen 16×32 is BSD-2-Clause, purpose-built for legibility, native
   16×32 → the spec-preferred 32×32 cell with correct 1:2 monospace aspect. Redistribution notice
   kept in `text_font_table.rs`; regen is one documented command (BDF not committed — URL + sha256
   pinned in the generator).
2. **Grid threading via uniform, not WGSL literals:** the 3 dead pad f32s in `TextUniforms` now
   carry `grid_cols`/`grid_rows` (struct stays exactly 4×f32 = 16 B — the .12 guard is untouched).
   New `l2_vs_text_grid_from_uniform` source-guard fails on any `/ 16.0`, `/ 6.0`, or `% 16u`
   remnant in `vs_text`.
3. **Advance ≠ cell:** old bake advanced one full cell per char (5 px ink in 8 px cell); Spleen ink
   is exactly half the 32 px cell, so pen advance is `char_m × 0.5` while quads stay square
   (`char_m`) — transparent cell padding overlaps harmlessly under alpha blend. Label footprints
   shrink ~50 % horizontally; declutter policy files untouched (they estimate conservatively —
   band/declutter work stays T-152.16/.17).
4. **Test-extraction bug found in .12 guards (fixed here):** `SHADER_SRC.find("fn vs_text")`
   prefix-matched **`fn vs_textured`**, so the .12 `vs_text` guards were actually scanning the
   basemap shader (it happens to contain the same V-flip idiom, so the gate was green by luck).
   Now paren-anchored (`"fn vs_text("` / `"fn fs_text("`).
5. **GPU probe rebuild:** '7' (glyph 23) probes re-derived for 5 px/texel (160 px quad / 32-texel
   cell); added a **U-mirror trap** probe (the .12 set had lost it when the bottom-stroke pixel
   moved). CPU oracle `g2_seven_is_top_heavy_in_atlas` checks the same four cell texels against
   the baked RGBA, so the screen-space probe and the atlas can't drift apart silently.
6. `serde_json` added as **dev-dependency only** (G3 test parses committed JSON) — zero wasm/size
   impact; `Cargo.lock` delta is that one entry.

## Manual acceptance (operator)

- **M1** (town labels readable at z=−2, "Saint-Philippe" hyphen intact): **operator-pending** — browser pass on real hardware.
- **M2** (height numerals crisp at z=0; road names legible along curves): **operator-pending**.

Automated proxies shipped: hyphen + full lowercase mapped and gate-locked (G2/G3); upright + real-pipeline draw proven by `text_self_check` 5/5 on GPU readback.

---

## T-152.13.1 — readability hotfix (baked halo + floor raise)

**Operator (2026-07-13, post-.13 screenshot @ z=−2.91):** "better, but the font is still very hard
to read." Diagnosis: (a) at the 14 px cell floor, Spleen's ~20/32 cap height leaves ~8.75 px
capitals whose 2 px strokes land sub-pixel under the production **Linear** sampler
(`icon_sampler`, `engine.rs:1302` — confirmed Linear, so washout not dropout); (b) zero contrast
treatment — off-white hairlines directly over busy terrain/roads.

**Fix (Rust-only, atlas + one constant):**
1. **Baked cartographic halo** — every glyph (tofu included) gets a 2 px Chebyshev-dilated
   outline `TEXT_HALO_RGBA = [16,21,29,255]` behind `TEXT_INK_RGBA` ink, baked in
   `bake_ascii_atlas_rgba` via cell-local bit-row masks. Lanes inherit it with zero
   shader/instance changes (`fs_text` tint multiply keeps the halo dark under any lane tint).
2. **Floor raise** — `text_char_meters`: base 16→**24 px**, min 14→**20 px** cell ⇒ rendered
   capitals ≥ 12.5 px with ~1.25 px strokes + ~1.25 px halo at the floor. Town band (≤ +2)
   renders at a constant 20 px cell; base takes over above z≈2.74.

**Gate deltas:**
- `g2_seven_is_top_heavy_in_atlas` now asserts exact classes: ink / **halo** (U-mirror trap texel
  (17,18) — mirrored ink would land there) / true-clear (V-flip trap (11,25), ≥3 px from ink).
- `text_self_check` probe (407,312) expects **halo bytes [16,21,29,255]**; V-flip + exterior
  probes stay `CLEAR_COLOR`. GPU readback headless: **allPass:true, 5/5**.
- Full suite re-run green: fmt ✓ · clippy native+wasm32 `-D warnings` ✓ · render **40/40** ·
  core 85 · `make wasm` → **4,347,812 B** (cumulative **+4,014 B** over the .12 baseline
  4,343,798 — ceiling +262,144) · vitest 355/355 · FE build/lint ✓.

**M1/M2: operator-pending** (re-check with halo + 20 px floor).
