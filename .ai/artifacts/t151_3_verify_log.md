# T-151.3 (W3) verify log — chunk residency + world spatial index + first world instances

**Slice:** W3 — Rust chunk residency (LRU + budget), chunk-keyed world spatial index, building OBB
fill + outline GPU lanes on `WgpuTacticalMap`. wgpu path only; the Deck `chunkStore`/worker/rbush
path is untouched. Baseline: tag **T-151.2** (`a51e9dcb`), docs HEAD `88cde47b`.

**Verification philosophy:** every claim maps to a class — **R** (bit-exact), **T** (≤1 ULP),
**S** (result-set equality), **GPU-R** (byte-exact readback pixel), or a stated numeric bound. No
probabilistic arguments: the pick test asserts per-probe distance-uniqueness (checked, not assumed);
the residency test proves eviction via the completeness theorem over a cap-crossing script.

---

## Automated gates — ALL PASS

| Gate | Result |
|------|--------|
| `cargo fmt --check` | **PASS** |
| `cargo clippy --all-targets -- -D warnings` (workspace, native) | **PASS** |
| `cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings` | **PASS** |
| `cargo test -p map-engine-core --all-features` | **PASS** — 101 unit + 5 camera_props + 5 deckgl_ortho_parity |
| `cargo test -p map-engine-render` | **PASS** — 10 (scene/lanes, incl. `building_instance_layout_and_bytes_exact`) |
| `cargo build --workspace` | **PASS** |
| `make wasm` | **PASS** — merged `map_engine_wasm_bg.wasm` = **3,946,734 B** (T-151.2 baseline 3,858,591; **+88,143**) |
| `npm test` (vitest) | **PASS** — **371** (T-151.2 baseline 343; **+28** W3) |
| `npm run build` (tsc + vite) | **PASS** |
| `npm run lint` (eslint) | **PASS** |
| `! grep -l map_engine_wasm_bg dist/assets/index-*.js` | **PASS** — no wasm ref in the entry chunk (lazy-only) |

New native Rust tests: `chunk_math` (rect/margin/viewport/ring pins), `residency` (cap/pinned-immunity,
ascending-`last_used` eviction order, budget accounting, building-count/buffers, pick), `index`
(class-filter, circular rejection, idempotent insert/remove). New vitest: `chunkMathRust.parity`
(24 = 12 bboxes × 2 rings), `world.pick.parity` (2), `world.residency.parity` (2).

---

## Proof ledger status

| # | Claim | Class | Status |
|---|-------|-------|--------|
| P1 | `chunk_ids_for_viewport` (Rust) == `chunkIdsForViewport` (JS) | R | **PASS** — native pins + `chunkMathRust.parity.test.ts` ordered-array equality |
| P2 | Residency requests the same chunk ids as `chunkStore` every step | S | **PASS** — `world.residency.parity.test.ts` requested-id sequence, 22-step script |
| P3 | Eviction identical (incl. order) | S | **PASS** — completeness theorem via P2 over cap-crossing script + end-revisits; native ascending-`last_used` order test; `eviction_log().length > 0` |
| P4 | LRU cap `max(64, 3×pinned)`, pinned never evicted | R | **PASS** — native `residency.rs` |
| P5 | Budget accounting (`frames_over_budget`/`max_apply_ms`) | R | **PASS** — native fake-elapsed sequence (`[1,5,3,6,4] → over=2, max=6`) |
| P6 | Per-chunk building-row selection (u16 lookup) | R | **PASS** — residency parity `pinned_building_count == getWorldBuildings().length` every step |
| P7 | `pick_rect` == rbush `pickRect` (class mask) | S | **PASS** — 10k probes, sorted id-set equality |
| P8 | `pick_nearest` == rbush `pickNearest` (class mask) | S | **PASS** — 10k probes; **0 exact-distance ties** for the fixed seed (checked) → every id compared |
| P9 | Building OBB corners match Deck | T | **PASS (delegated)** — `world.parity.test.ts:128` `obb_corners` ≤1 ULP (reused, not re-derived) |
| P10 | Building fill center pixel = fill RGBA, byte-exact | GPU-R | **PASS** — headless readback below |
| P11 | Shader rotation sign matches obb.rs | GPU-R | **PASS** — orientation probe below |
| P12 | Instance/vertex upload bytes exact | R | **PASS** — `size_of` asserts (`BuildingInstance`==40, `QuadInstance`==32, `LineVertex`==24) + pinned-bytes test |
| P13 | W2 parser + spike self-check + basemap lanes unbroken | R/S/GPU-R | **PASS** — `world.parity.test.ts` green; calibration + texture self-checks below |
| P14 | Entry-chunk isolation; prior `stats()` fields intact | — | **PASS** — grep gate; `stats()` keys appended only |

---

## GPU-R — headless readback (SwiftShader WebGL2, `chrome-headless-shell`, raw CDP)

Driven via CDP against the built app on the `/_spike/wgpu?force=webgl` page (dev-only `window.__selfChecks`
hook), chromium `--use-angle=swiftshader --enable-unsafe-swiftshader`. Same mechanism T-151.1 used.
**All three byte-exact `pass:true`:**

**`world_building_self_check` (T-151.3 S4 gate):**
```json
{"backend":"webgl2","probes":[
  {"px":400,"py":300,"expect":[38,38,44,255],"got":[38,38,44,255],"pass":true,"label":"center fill byte-exact (FILL_DEFAULT rgb)"},
  {"px":460,"py":300,"expect":[51,68,85,255],"got":[51,68,85,255],"pass":true,"label":"exterior = CLEAR_COLOR"},
  {"px":425,"py":310,"expect":[38,38,44,255],"got":[38,38,44,255],"pass":true,"label":"orientation +37 (inside +37, outside -37)"}
],"pass":true}
```
Confirms P10 (center = FILL_DEFAULT `[38,38,44,255]` byte-exact; α=1 collapses `ALPHA_BLENDING` to
`src`, `k/255` round-trips unorm8) + P11 (orientation texel `rel(25,−10)` inside the +37° OBB but
outside −37°, ≥2 px margins → proves the shader rotation handedness matches `obb::obb_corners`).

**`self_check` (T-151.0 calibration regression):** `pass:true`, all **7** probes (G center + 2px-inside
corners + clear margins + R north-up + mirror).
**`texture_self_check` (T-151.1 basemap regression):** `pass:true`, 3 probes (NW red / NE green / SW blue).

→ my `draw_batches` signature + new lane roles + shared-pipeline changes did **not** disturb the
T-151.0/1 GPU paths (P13).

---

## Colour reconciliation (operator decision + spec-vs-oracle divergence)

- **Fill:** `FILL_BY_CLASS[class] ?? FILL_DEFAULT` from `buildingLayer.ts:126-139`, verbatim
  (`FILL_DEFAULT = [38,38,44,184]`, 11 per-class entries). These *are* the Deck values — no conflict.
- **Outline:** near-black **`[30,30,34,255]`** (operator decision this session, spec L8 literal). This
  **diverges** from the Deck oracle's grey `STROKE = [150,150,158,204]` (`buildingLayer.ts:140`). It
  affects no automated gate (the readback samples the fill). **Flagged for Cursor** to reconcile the
  spec-vs-oracle text; the outline color is a one-line constant in `residency.rs::OUTLINE_COLOR`.

---

## Manual acceptance S1–S4

- **S1** (buildings visible @ deckZoom ≥ −2.5, pan smooth) — **operator/perceptual** (editor
  `/missions/:id/edit?engine=wgpu`). The correctness pieces are proven: residency composes the fill
  buffer (P6 count parity), `upload_world_buildings` is byte-exact (S4 readback), draw order is
  `basemap→hillshade→buildings→outline→grid` (`lane_order`). Integration wiring typechecks + builds.
- **S2** (Deck path unchanged with `?engine=` off) — **operator/perceptual**. No Deck files changed;
  `world.parity.test.ts` + all Deck unit tests green (part of the 371).
- **S3** (HUD `world_building_instances > 0` in view; `frames_over_budget = 0` on scripted pan) —
  **operator** (editor HUD). Budget accounting is R-proven (P5); the additive `stats()` keys
  (`world_building_instances`, `world_building_outline_vertices`, `world_chunks_drawn`) are wired.
- **S4** (readback JSON byte-exact) — **EXECUTED PASS** (headless, JSON above).

---

## Files

**Rust core (new):** `world/chunk_math.rs`, `world/index.rs`, `world/residency.rs`; **edit**
`world/mod.rs` (register + re-export), `world/store.rs` (`bytes_to_json` → `pub(super)`).
**Wasm:** `map-engine-wasm/src/lib.rs` — `WorldResidency` + `WorldSpatialIndex` handles +
`world_chunk_ids_for_viewport` free fn (`WorldStore`/`SlotIndex` untouched).
**Render:** `scene.rs` (`BuildingInstance` 40 B + bytes test), `shader.wgsl` (`vs_building`/
`fs_building`), `engine.rs` (2 lane roles, `create_building_pipeline`, `BuildingInstanced` payload +
draw arm, `upload_world_buildings`/`upload_world_building_outlines`/`clear_world_buildings`, `stats()`
additive keys, `world_building_self_check`).
**Frontend (new):** `wgpu/wgpuWorldLoader.ts` (`WgpuWorldController`: 12-way fetch + budgeted ingest +
engine upload), `wgpu/useWgpuWorldResidency.ts`; **edit** `WgpuTacticalMap.tsx` (world controller +
hook + `onCameraMoved` wiring), `_spike/wgpu/WgpuCanvas.tsx` (dev-only `window.__selfChecks` + operator
button).
**Tests (new):** `_wasm/chunkMathRust.parity.test.ts`, `_wasm/world.pick.parity.test.ts`,
`_wasm/world.residency.parity.test.ts`.

**Out of scope (untouched):** Deck `worldmap/*`, `state/worldSpatialIndex.ts`, worker; trees/roads/
sea/forest/slots (W4–W6); editor world picks (W7).
