# T-151.5 verify log — glyph atlas + LOD gates (trees, props, badges)

**Baseline:** tag **T-151.4.1** (`552e68aa`) / worktree HEAD docs-sync after.  
**Slice:** W5 — world glyph atlas once; IconInstanced ≤20 B; tree/prop/badge streams with Class R size/LOD parity.

---

## Instance layout (L2)

| Field | Type | Bytes | Notes |
|-------|------|------:|-------|
| `pos` | `f32×2` | 8 | anchor-relative world meters (engine subtracts ANCHOR on upload) |
| `size` | `f32` | 4 | meters; min-px clamped on CPU |
| `yaw` | `i16` snorm | 2 | `angle_deg/180 × 32767` (Deck CCW after negate) |
| `glyph` | `u16` | 2 | index into 28-entry UV table |
| `tint` | `u32` | 4 | RGBA8 `r\|g<<8\|b<<16\|a<<24` |
| **Total** | | **20** | `assert_eq!(size_of::<IconInstance>(), 20)` |

Draw order: forest-outline → **trees** → **props** → **badges** → grid.

---

## What shipped

| Piece | Detail |
|-------|--------|
| Atlas | `world-glyphs.webp` + JSON → `upload_glyph_atlas` (28 UV rects asserted) |
| Pipeline | `PipelineKind::IconInstanced` + `vs_icon`/`fs_icon` (group 2 atlas) |
| Pure math | `lod_gates.rs` + `glyph_math.rs` Class R vs TS |
| Stream | `WorldResidency` compose + `INSTANCE_BUDGET`; prefs trees/props/buildings |
| Stats | `tree_glyphs`, `prop_glyphs`, `badge_glyphs`, `atlas_bytes` (additive) |
| GPU-R | `tree_glyph_self_check()` — solid white atlas × forest tint |

---

## Automated gates — ALL PASS

| Gate | Result |
|------|--------|
| `cargo fmt --check` | **PASS** |
| `cargo clippy --all-targets -- -D warnings` | **PASS** |
| `cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings` | **PASS** |
| `cargo test -p map-engine-core --all-features` | **PASS** (incl. glyph_math + lod_gates) |
| `cargo test -p map-engine-render` | **PASS** — 11 (incl. icon 20 B layout) |
| `cargo build --workspace` | **PASS** |
| `make wasm` | **PASS** — **4,054,850 B** (T-151.4.1 was 4,009,368; **+45,482**) |
| `npm test` | **PASS** — **372** (+1 exhaustive LOD parity; was 371) |
| `npm run build` + `lint` | **PASS** |
| entry isolation (`! grep map_engine_wasm_bg dist/assets/index-*.js`) | **PASS** |

### LOD scan (L6)

`glyphLod.parity.test.ts`: 16 classes × 121 zooms (−6.0…+6.0 @ 0.1) — Rust `class_visible` == TS `classVisible` exact.

---

## Manual (operator)

| ID | Check | Status |
|----|-------|--------|
| **S1** | `?engine=wgpu` zoom ≥ 0 — tree glyphs over forest mass | **operator** |
| **S2** | zoom &lt; 0 — glyphs hidden; forest mass remains | **operator** |
| **S3** | Deck path unchanged; density advisory compare | **operator** |
| **S4** | `tree_glyph_self_check` JSON (nonzero α + tint) | **operator** (API on `RenderEngine`) |

Dev surface: `window.__wgpuWorldStats` includes `tree_glyphs` / `prop_glyphs` / `badge_glyphs` / `atlas_ready`.

---

## Forest analysis note

Mass / Path B hulls **unchanged**. Glyphs are the instrument for overdraw judgment; retune is a follow-up.

---

## Ready for Cursor doc sync
