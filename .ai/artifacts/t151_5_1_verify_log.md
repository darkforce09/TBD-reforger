# T-151.5.1 verify log — forest mass / landcover fidelity

**Baseline:** tag **T-151.5** (`0b7621ed`); docs HEAD before code: `1d30532e`  
**Slice:** raise `DENSITY_ISO` 1→2 (Rust SoT); hide fill/outline/landcover in tree glyph band; fix sticky landcover LOD.

---

## Changes

| Layer | Change |
|-------|--------|
| `forest_mass.rs` | `DENSITY_ISO = 2.0` (Path B region floor); unit pins |
| `map-engine-wasm` | `density_iso()` export — wgpu must not feed TS iso |
| `lod_gates.rs` + `lodGates.ts` | `forestFill`: `zoom < 0`; `forestOutline`: `[-1.5, 0)`; glyphs unchanged |
| `useWgpuForestMass.ts` | `forest_mass(..., density_iso())` + wasm `class_visible` |
| `wgpuWorldLoader.ts` | `landcoverReady` + re-push on camera; clear when glyphs on |
| Deck TS | Thin mirror `DENSITY_ISO=2` + same LOD (Class R / dual-mount) |

**Not in this slice:** Path B / TBDD / `forest-regions.json.gz` rebuild; glyph atlas; T-151.6 slots; docs/registry.

---

## Automated gates — ALL PASS

| Gate | Result |
|------|--------|
| `cargo fmt --check` | **PASS** |
| `cargo clippy --all-targets -- -D warnings` | **PASS** |
| `cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings` | **PASS** |
| `cargo test -p map-engine-core --all-features` | **PASS** (127 lib + camera/ortho suites) |
| `cargo test -p map-engine-render` | **PASS** — 11 |
| `cargo build --workspace` | **PASS** |
| `make wasm` | **PASS** — **4,055,075 B** (T-151.5 was 4,054,850; **+225**) |
| `npm test` | **PASS** — **374** (≥ 372 baseline; +LOD/iso cases) |
| `npm run build` + `lint` | **PASS** |
| entry isolation (`! grep map_engine_wasm_bg` on main index chunk) | **PASS** |

---

## Manual acceptance (operator)

| ID | Check | Expected | Notes |
|----|-------|----------|-------|
| **S1** | `?engine=wgpu`, zoom ≥ 0 | Tree glyphs on; **no** bloated green fill/outline/landcover/grid over fields; thin stands not wrapped in huge blocks | Iso=2 drops sparse bleed; LOD clears green under glyphs |
| **S2** | zoom −2 (default) | Readable forest context (iso=2 mass); no tree glyphs | Outline still from −1.5 only |
| **S3** | Deck (`?engine=` off) | Matches wgpu forest LOD + iso=2 | Thin TS mirror of Rust policy |
| **S4** | Residual limits | Document only — no fix this slice | See below |

### S4 residuals → follow-up

1. **Mega-region** `forest-everon-001` (~479k trees) Path B hull can still look large at coarse zoom after iso=2 — **export / Path B split** (not runtime).
2. **32 m TBDD cell tessellation** can still show cell seams in the mass when fill is on (zoom &lt; 0) — finer grid + contour smoothing = **T-149** (idea).
3. Do **not** rebuild density bins or `forest-regions.json.gz` in this slice.

---

## LOD pin table (T-151.5.1)

| zoom | tree | forestFill | forestOutline | landcover (wgpu) |
|------|------|------------|---------------|------------------|
| −6 | off | on | off | on |
| −2 | off | on | off | on |
| −1.5 | off | on | on | on |
| −0.1 | off | on | on | on |
| 0 | **on** | **off** | **off** | **off** |
| +1 | on | off | off | off |

---

## Ready for Cursor doc sync

- Registry: mark **T-151.5.1** shipped; hub next → **T-151.6**
- Do not re-open iso/LOD policy in docs without code change
