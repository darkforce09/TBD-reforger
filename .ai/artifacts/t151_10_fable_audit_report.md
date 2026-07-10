# T-151.10 — Fable 5 T-151 Program Audit [2026-07-10]

**Living tracker** — update statuses in place as remediations ship; do not rewrite history.
**Program hub:** [`t151_wgpu_engine_program.md`](../../docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md) ·
**Spec:** [`t151_10_fable_program_audit.md`](../../docs/specs/Mission_Creator_Architecture/t151_10_fable_program_audit.md) ·
**Evidence:** [`t151_10_verify_log.md`](t151_10_verify_log.md)
**Audit baseline:** worktree `tbd-reforger-wgpu-spike` @ **`1cbe3a56`** (tag `T-151.9` tip `58c8fcc3` + 2 docs commits) · audited 2026-07-09/10.
**Severity:** **S** security / **R** reliability / **T** tech-debt / **M** maintainability / **D** docs ·
**Status:** **PASS** (claim re-verified with evidence) / **PARTIAL** (holds with a named gap) / **OPEN** (defect, silent deferral, or unverifiable claim).
**Rigor rule:** no finding without a proof artifact (command output or file:line quote). A PASS without proof is not written as PASS. Claims one pass could not re-verify are labeled NOT RE-VERIFIED in the register — never silently dropped.

---

## Verdict summary

| Status | Count | Meaning |
|--------|-------|---------|
| PASS | 30 | Re-verified with fresh evidence at HEAD |
| PARTIAL | 18 | Substance holds; named gap recorded |
| OPEN | 13 | Defect / never-closed gate / D5 leak / stale doc |

**Program verdict:** the engine flip is real and sound — every automated Class R/S/T gate re-ran
green at HEAD (13/13 commands, 281/281 vitest incl. 99 parity tests, 176 cargo tests, 17/17 GPU
probes byte-exact on WebGL2), the Deck runtime is gone from the app and from `dist`, and the
claimed numbers (wasm bytes, vitest chains, census, bundle size) reconcile to the byte almost
everywhere. The two systemic weaknesses are: **(1)** a cluster of live TypeScript policy twins
that violate D5 letter (though every twin is parity-gated against its Rust counterpart), and
**(2)** a program-wide honesty gap on perceptual gates — **no operator S-gate was ever recorded
closed in any verify log**, and none of the T-151 gates run in CI.

**Ledger closure:** 250 claims extracted → 250 dispositions (§Claims register): 205 re-verified or
covered by a finding/battery row (179 PASS · 12 PARTIAL · 14 OPEN), 45 NOT RE-VERIFIED (each labeled with why — operator-hardware measurements, WebGPU-env items, and prose rows one pass did not individually re-derive).

---

## Program thesis verdict — "Rust owns engine policy; TS is dumb UI"

**QUALIFIED PASS.** Quantified:

- **Rust:** 19,133 LOC across `map-engine-core` (11,143) / `map-engine-render` (5,964) /
  `map-engine-wasm` (2,026). Owns outright, with no live TS twin: camera/unproject (ULP-0
  `OrthoCameraJs` / `RenderEngine.unproject_xy`), slot pick engine (`SlotIndex.pick_rect`),
  SoA→GPU slot packing (`slots_gpu.rs`; `wgpuSlots.ts` = 56 LOC bridge), world parsing + residency
  + LRU (`residency.rs`, golden-locked), building/vector/glyph geometry (`geometry/*`,
  marching squares, triangulation, polyline strips), density ladder + heatmap
  (`density_ladder.rs`, `DENSITY_ISO` SoT with `density_iso()` export), draw-set + compute cull
  (`compute_cull.rs`, Class R 1k-frusta oracle), hillshade, DEM decode/sample, all shaders.
- **TypeScript (live, non-test):** 7,505 LOC under `tactical-map/` (of which `wgpu/` 1,702).
  Oracle/test code excluded (`_wasm/` 2,116 LOC is test-only).
- **D5 leaks found:** one live-twin cluster of **~778 LOC across 5 files** (B-01…B-06): the
  DEM-vectors lane calls TS `classVisible`/`contourIntervalForZoom`/`seaFillAlpha` instead of the
  existing Rust exports; the forest lane calls TS `forestFillAlpha` + TS `chunkIdsForViewport`;
  `worldSpatialIndex.ts` is dead. Every live twin is pinned byte/behavior-equal to Rust by a
  parity test that runs in `npm test` — so drift is machine-caught — but the D5 rule
  ("TS may only call wasm") is violated in letter on those call sites.
- Spec-sanctioned TS exceptions (not leaks): TBDS container parse + `pickBaseLevel` + `computeLod`
  (T-151.1 spec L2 keeps these in JS, Class R/S tested), gesture state machine (`useSelectTool`,
  pointer/DOM domain), supercluster index (named out-of-scope in the T-151.7.3 verify log).

---

## Finding index

| ID | § | Slice | Sev | Finding (short) | Status |
|----|---|-------|-----|-----------------|--------|
| A-01 | A | 151.0 | T | D1 one wasm module / one memory | PASS |
| A-02 | A | 151.3 | D | D2 worker retired; hub's "DecompressionStream" clause ≠ shipped mechanism (gunzip in Rust) | PARTIAL |
| A-03 | A | 151.9 | T | D3 superseded: sole WgpuTacticalMap mount, no engine flag | PASS |
| A-04 | A | 151.4 | T | D4 current asset wire only | PASS |
| A-05 | A | 151.7.3 | T | D5 LOC gate: `wgpuSlots.ts` 56 ≤ 60 | PASS |
| A-06 | A | 151.9 | R | Flip + 35-file Deck deletion + dep demotion atomic @ `c4831451` | PASS |
| A-07 | A | 151.9 | M | Stale engine-flag comments survive the flip | OPEN |
| A-08 | A | hub | D | Decision numbering drift: heading "D1–D4" over 5 decisions; spec invokes "D1–D9" | OPEN |
| A-09 | A | 151.9 | T | deck/luma = 6 devDependencies; dist deck-free (fresh grep) | PASS |
| A-10 | A | program | M | `/_spike/wgpu` + `__selfChecks` reachable in production bundle | OPEN |
| B-01 | B | 151.4 | T | DEM-vectors lane runs live TS twins (`classVisible`, `contourIntervalForZoom`, `seaFillAlpha`) of existing Rust fns | OPEN |
| B-02 | B | 151.4/5.1 | T | Forest lane runs live TS `forestFillAlpha` twin + per-vertex alpha recolor in TS | OPEN |
| B-03 | B | 151.4 | T | Density-lane chunk ids via TS `chunkIdsForViewport`; hub still cites `chunkMath.ts` as SoT | PARTIAL |
| B-04 | B | 151.3 | T | Apply-budget policy (4 ms) enforced JS-side; Rust only records | PARTIAL |
| B-05 | B | 151.6/7.3 | T | Supercluster + drill policy in TS (240 LOC) — named deferral, not silent | PARTIAL |
| B-06 | B | 151.9 | M | `worldSpatialIndex.ts` dead module (125 LOC, zero live importers, stale worker comment) | OPEN |
| B-07 | B | 151.7 | T | Slot pick engine in wasm; TS facade only | PASS |
| B-08 | B | 151.7 | T | Camera math in wasm (ULP-0); 2 mirrored zoom consts noted | PASS |
| B-09 | B | 151.1 | T | Basemap TS policy is spec-sanctioned + tested; pack-offset mirror noted | PASS |
| B-10 | B | 151.5–8 | T | Geometry/iso/pack/cull/draw-set/heatmap all Rust-owned | PASS |
| C-sp-01 | C | spike | — | Spike gates V1–V8 closed; honest plan-vs-reality corrections; no git tag | PASS |
| C-0-01 | C | 151.0 | R | Automated + byte chain verified; S1 closed later headless; **S2 (shared-memory HUD 2000/2000) never recorded closed**; S3 mooted unrecorded | PARTIAL |
| C-1-01 | C | 151.1 | R | Automated + GPU executed (fresh 3/3 texture); S1–S4 perceptual never recorded closed | PARTIAL |
| C-2-01 | C | 151.2 | — | All gates closed incl. S1 runtime (1.13 s); census re-verified fresh at HEAD | PASS |
| C-3-01 | C | 151.3 | R | P1–P14 + GPU-R re-proven fresh (3/3); S1–S3 operator never closed | PARTIAL |
| C-3-02 | C | 151.3 | D | Outline color `[30,30,34,255]` vs Deck stroke `[150,150,158,204]` — "flagged for Cursor", never reconciled | OPEN |
| C-4-01 | C | 151.4 | R | Sea/road GPU-R "expected JSON, not executed" at ship — **now EXECUTED PASS by this audit** (2/2 + 2/2 byte-exact) | PARTIAL |
| C-4-02 | C | 151.4 | T | Optional `vector.layers.parity.test.ts` never created; log silent about it | OPEN |
| C-4.1-01 | C | 151.4.1 | — | Building-wipe race + inflight fix verified in current loader; honest 0-buildings disclosure | PASS |
| C-5-01 | C | 151.5 | — | Atlas + IconInstanced 20 B + LOD scan gates hold (glyphLod parity in fresh run) | PASS |
| C-5-02 | C | 151.5 | R | S4 `tree_glyph_self_check`: API exists (`engine.rs:4346`) but never wired into `__selfChecks`, never executed anywhere | OPEN |
| C-5.1-01 | C | 151.5.1 | — | `DENSITY_ISO=2` Rust SoT verified live (`density_iso()` call); residuals correctly named → T-149 (`idea` in registry) | PASS |
| C-6-01 | C | 151.6 | R | Automated gates hold; S1–S5 operator never closed | PARTIAL |
| C-7-01 | C | 151.7 | R | Parity suite (12 tests) green fresh; S1–S5 operator never closed; 7.1 hotfix proves live-use bugs the unrecorded gates missed | PARTIAL |
| C-7.1-01 | C | 151.7.1 | — | B1–B3 hotfix verified; no spec file for 7.2 follow-on noted | PASS |
| C-7.2-01 | C | 151.7.2 | M | Shipped from verify log only (no spec, no docs tag) — pattern break | PARTIAL |
| C-7.3-01 | C | 151.7.3 | — | Rust collapse proven: 521→56 LOC then, 56 now; no TS pack twin | PASS |
| C-8-01 | C | 151.8/8.1 | R | Class S/R cargo gates re-ran green (draw-set, density R-rows, 1k-frusta cull); **S4 band table still the empty operator template**; exact wasm bytes never recorded (audit records 4,123,261 B) | PARTIAL |
| C-9-01 | C | 151.9 | — | Flip fully re-verified fresh: 281/281, dist deck-free, bundle 6,273,423 B exact, 56 LOC | PASS |
| C-9-02 | C | 151.9 | D | Log's tag line self-invalidated (`c87d74fa` vs tag tip `58c8fcc3`) — honesty-fixed this audit | PARTIAL |
| C-ALL-01 | C | program | R | **Zero operator S-gates recorded closed across all 16 verify logs** — every perceptual gate program-wide is formally open | OPEN |
| D-01 | D | — | R | 16 parity files / 99 tests all enumerate + run (list == run == 281); nothing skipped | PASS |
| D-02 | D | 151.9 | T | Residency golden: 22 steps, baseline `ec59d10e`, regen path exists; post-Deck regen self-certifies (regression-lock by design) | PASS |
| D-03 | D | 151.8.1 | — | Compute-cull Class R oracle present + ran (`class_r_1k_random_frusta_count_stable`) | PASS |
| D-04 | D | 151.9 | — | Deck oracle residue = exactly the 2 sanctioned camera parity tests | PASS |
| D-05 | D | hub | D | Pinned inventory re-derived: 10 exact PASS; road-classes row PARTIAL (5 in data vs "6"); 3 rows cite deleted files | PARTIAL |
| D-06 | D | program | T | GPU-R harness is ad-hoc (no committed runner); this audit rebuilt it from scratch to execute the gates | OPEN |
| E-01 | E | — | R | wasm shim: 0 unwrap/expect/panic; core unwraps (50) all inside `#[cfg(test)]` | PASS |
| E-02 | E | — | R | Init/render failures surface to the error banner (I6); WebGL2 fallback + spike Force-WebGL2 path | PASS |
| E-03 | E | — | S | No secrets in map surface (grep clean) | PASS |
| E-04 | E | 151.4.1 | R | Loader abort/inflight discipline + anti-wipe sticky lanes verified in current code | PASS |
| E-05 | E | 151.4 | R | Forest-mass session cache unbounded (no LRU, by admitted design) — long-session memory growth | OPEN |
| F-01 | F | 151.0–7.3 | D | wasm byte-size chain exact across all ten documented steps | PASS |
| F-02 | F | 151.0–9 | D | vitest count chain exact across all fifteen steps incl. 281 = 393 − 112 | PASS |
| F-03 | F | all | D | Every claimed SHA exists + is an ancestor; baselines match tag map (31/35 hex = commits, 4 = checksums) | PASS |
| F-04 | F | 151.8 | D | "~4.09 MB" stale through the 8.1 amendment; no exact bytes recorded for W8 — honesty-fixed + audit records exact | PARTIAL |
| F-05 | F | tags | D | Dual-role docs tags legitimate; convention breaks after T-151.8 (no 7.2/8.1/9 docs tags; `T-151.7.3-docs` sits on `fa7a4b1d` whose message says "T-151-docs") | PARTIAL |
| F-06 | F | hub | D | Hub links 33/33 resolve; t151_5_1 verify log never linked from hub | PARTIAL |
| F-07 | F | 151.9 | D | Bundle ledger arithmetic exact (−877,776 B = ~12.27%); CLAUDE/hub mirrors correct | PASS |
| F-08 | F | 151.7.3 | D | Log baseline line omits parent `fa7a4b1d` (docs-only; code state unaffected) | PARTIAL |
| F-09 | F | program | R | **T-151 gates run zero times in CI**: rust job pinned to `apps/website`; frontend job builds without `make wasm` against a gitignored pkg; branch has no remote — spike log finding 7 named this for merge time, still open | OPEN |

---

## A — Program integrity (locked decisions, one verdict per decision)

- **A-01 PASS — D1 (one wasm module, one memory).** `Makefile:65-67` holds the single wasm-pack
  invocation in the repo (`wasm-pack build crates/map-engine-wasm --release --target bundler
  --out-dir ../../apps/website/frontend/src/wasm/pkg`); `crates/map-engine-wasm/Cargo.toml:15,19`
  depends on both `map-engine-core` (features png/mission/doc/world) and `map-engine-render`; no
  `--target web` pkg remains; render crate ships no cdylib. Entry-chunk isolation re-proven on the
  fresh build (`grep -l map_engine_wasm_bg dist/assets/index-*.js` → no match). **Proof:** battery
  rows 6/8 + fresh greps.
- **A-02 PARTIAL — D2 (worker retired, not ported).** Substance holds: no `*.worker.*` files and
  no Comlink under `tactical-map/` (the only Comlink is the sanctioned mission-compiler pair);
  worker trio deleted at `c4831451`; ingest is budgeted (`wgpuWorldLoader.ts:30,386`). Named gap:
  the hub sentence "chunks are fetched + gunzipped by a thin JS async loader
  (`DecompressionStream`)" (hub:137-138) does not describe the shipped mechanism — the loader
  passes raw `.json.gz` bytes and **Rust gunzips** via `flate2`
  (`map-engine-core/Cargo.toml:30,38`; `WorldResidency.ingest_chunk_gz`,
  `wgpuWorldLoader.ts:391`); `DecompressionStream` appears nowhere in the loader. Better than
  promised (fewer JS copies), but the hub clause is wrong as written. Severity D.
- **A-03 PASS — D3 (dual-mount superseded at the flip).** `MissionCreatorPage.tsx:19` lazy-imports
  `WgpuTacticalMap` under the comment "T-151.9: wgpu is the sole Mission Creator map engine";
  rendered unconditionally at `:234`. `rg "VITE_MC_ENGINE|engine=wgpu|\?engine"` over
  `apps/website/frontend/src` returns only two stale comments (A-07). No Deck mount path exists —
  `TacticalMap.tsx` deleted at `c4831451`.
- **A-04 PASS — D4 (current asset wire only).** Every wgpu-path fetch is an existing format:
  chunks `objects/chunks/{id}.json.gz` (`wgpuWorldLoader.ts:525`), roads/regions `.json.gz`
  defaults (`:189-190`), `prefabs.json.gz` (`:287-293`), TBDD `objects/density/{id}.bin`
  (`useWgpuForestMass.ts:109`), TBDS container (`wgpuBasemap.ts:14,155-208`), DEM PNG via
  `DemController`. No binary chunk wire shipped anywhere.
- **A-05 PASS — D5 LOC gate (scoped; full census in §B).** `wc -l wgpuSlots.ts` = **56** ≤ 60,
  matching the T-151.7.3/9 claims. Battery row 11.
- **A-06 PASS — flip + delete + demotion atomic.** `git show c4831451 --name-status --format= |
  grep -c '^D'` = **35** deleted files (TacticalMap.tsx; `useBaseMapLayer`/`useClusterIconLayer`/
  `useDemLayer`/`useIconLayer`/`useSelectionLayer`/`useTerrainBasemapLayer`;
  `workers/worldObjects.worker.ts` + client + core; worldmap chunk/road/building stores/layers;
  hybrids; DocCoreSpike) in the same commit as the mount flip and the package.json deck/luma
  demotion. (Corrects the salvaged lens's count of 36.)
- **A-07 OPEN (M) — stale engine-flag comments.** `WgpuTacticalMap.tsx:3` still says the component
  is "selected by `MissionCreatorPage` behind the engine flag (`VITE_MC_ENGINE=wgpu` or
  `?engine=wgpu`)"; `wgpuWorldLoader.ts:10-11` still says "Deck's worker/chunkStore/rbush path is
  untouched — this only drives the `?engine=wgpu` mount". Both describe deleted machinery.
  Remediation: two comment rewrites (Cursor ticket; code-comment change, trivial).
- **A-08 OPEN (D) — decision-numbering drift.** Hub heading `## Locked architecture decisions
  (D1–D4)` (hub:129) sits over a list defining **five** decisions D1…D5 (:131,:137,:144,:150,:153);
  the audit spec instructs "Hub D1–D9 still true in code" (spec:64) and "decisions D1–D9" (:31);
  D6–D9 are defined nowhere in the repo. Remediation: Cursor fixes the heading and the spec's
  range.
- **A-09 PASS — dependency demotion + dist.** All six packages (`deck.gl`, `@deck.gl/core`,
  `@deck.gl/extensions`, `@deck.gl/layers`, `@deck.gl/react`, `@luma.gl/core`) sit in
  `devDependencies` (package.json:45+); `dependencies` (lines 17–44) contains none. Runtime
  imports: only the two sanctioned oracle tests (D-04). Fresh `dist/assets` grep: no matches.
- **A-10 OPEN (M) — debug surface ships to production.** `/_spike/wgpu` is registered
  unconditionally in the prod router (`router.tsx:52-55`) and `WgpuCanvas.tsx:86-94` registers
  `window.__selfChecks` unconditionally on mount (the comment says "Dev-only headless hooks" but
  nothing gates it). Impact is low — lazy chunk, no secrets, requires visiting the route — but it
  is a live debug/perf-stress surface (20M-instance stress buttons) in production. Remediation:
  gate route registration on `import.meta.env.DEV` or strip in prod build.

---

## B — LANGUAGE GATE (D5): subsystem ownership + TS policy census

### B1 — Subsystem ownership matrix

| Subsystem | Rust owner (evidence) | Live TS twin? | Verdict |
|-----------|----------------------|---------------|---------|
| Camera / unproject | `OrthoCameraJs`, `RenderEngine.unproject_xy` (`mapCamera.ts:5,26-49` wraps wasm) | mirrored consts `MAP_MIN/MAX_ZOOM` only (`mapCamera.ts:14-15`) | **PASS** (B-08) |
| Zoom clamp/band | Rust `OrthoCamera` + engine-side clamp | `clampMapZoom` pre-clamp + literals `-6/6` (`mapCamera.ts:17-21`, `wgpuBasemap.ts:385-386`) | PASS w/ note |
| LOD gates | `lod_gates.rs:5-73` (`class_visible`, `contour_interval_for_zoom`, consts) exported via wasm (`lib.rs:27`) | **YES — live**: `useWgpuDemVectors.ts:18` imports TS `classVisible`+`contourIntervalForZoom`; other lanes correctly use wasm `class_visible` (`wgpuWorldLoader.ts:16,236`; `useWgpuForestMass.ts:6,168-169`) | **OPEN** (B-01) |
| Fade ladders | `forest_mass.rs:241 forest_fill_alpha`, `sea_band.rs:28 sea_fill_alpha` | **YES — live**: TS `forestFillAlpha` (`useWgpuForestMass.ts:8,170`), TS `seaFillAlpha` (`useWgpuDemVectors.ts:19,83`) | **OPEN** (B-01/B-02) |
| Chunk math (512 m grid) | `world/chunk_math.rs` (+ `WorldResidency.set_viewport` for the building/glyph lane) | **YES — live** for the density lane: `chunkIdsForViewport` (`useWgpuForestMass.ts:7,80-84`) | **PARTIAL** (B-03) |
| Residency / LRU | `world/residency.rs` (golden-locked: 22-step parity ×2 tests) | none (loader only fetches/queues) | PASS |
| Ingest budget | — (Rust records via `end_apply_frame`) | **policy in TS**: `APPLY_BUDGET_MS = 4` + drain loop (`wgpuWorldLoader.ts:30,386-404`) | **PARTIAL** (B-04) |
| SoA→GPU slot pack | `slots_gpu.rs` + `SlotGpuBridge` (T-151.7.3) | none — `wgpuSlots.ts` 56 LOC bridge | PASS |
| Selection/drag GPU policy | engine tint columns + delta uniform (`uniform_bytes_last_frame` 80 during drag) | none | PASS |
| Cluster policy | disc rendering + gates in engine | **index + drill in TS**: `slotClusterIndex.ts` (240 LOC, supercluster) — named out-of-scope in t151_7_3 log §out-of-scope ("Supercluster in Rust (FE still feeds set_cluster_markers)") | **PARTIAL** (B-05) |
| Spatial indexes | `spatial/point_index.rs` — `SlotIndex.pick_rect` does the query (`slotSpatialIndex.ts:7,112-115,131,154` facade) | facade only; nearest-selection post-loop in TS | **PASS** (B-07) |
| Density ladder / heatmap | `density_ladder.rs` (R1–R3 tests fresh); `DENSITY_ISO` SoT + `density_iso()` (`useWgpuForestMass.ts:136,144` — "Rust owns DENSITY_ISO — never pass a TS iso") | none | PASS |
| Cull (draw-set + compute) | `residency.rs` draw-set (Class S test), `compute_cull.rs` (Class R 1k frusta) | none | PASS |
| Vector geometry | `geometry/*` (sea_band, contours, forest_mass, triangulate, polyline_strip) — all composes via wasm | mesh **concatenation** + alpha recolor loop in TS (`useWgpuForestMass.ts:203-224`) | PASS w/ note (B-02 covers the alpha) |
| Hillshade | `dem/hillshade.rs` via wasm `hillshade` (`wgpuBasemap.ts:30,331`) | none | PASS |
| TBDS mip pick / basemap LOD | — | TS `pickBaseLevel` + `computeLod` — **spec-sanctioned** (t151_1 spec L2: engine never parses TBDS; both Class R/S tested) | **PASS** (B-09) |
| Shaders | all WGSL inside `map-engine-render` (no `wgsl` in TS — grep clean) | none | PASS |

### B2 — TS policy census (live, non-test; leak cluster detailed)

Live non-test TS under `tactical-map/`: **7,505 LOC** (wgpu/ 1,702). Classifications with findings:

- **B-01 OPEN (T) — DEM-vectors lane live twins.** `useWgpuDemVectors.ts:18-19` imports
  `classVisible, contourIntervalForZoom` from `worldmap/lodGates` and `seaFillAlpha` from
  `worldmap/seaBand`; used at `:82-83` (sea) and `:112-113` (contours). Rust equivalents exist and
  are already wasm-exported (`lod_gates.rs:50,73`; `sea_band.rs:28`; wasm `lib.rs:24-28`). The
  same file also holds the contour grid-reduction ladder (`:123 reductions = interval >= 100 ? 2 :
  interval >= 50 ? 1 : 0`). Mitigation in place: `glyphLod.parity.test.ts` runs the 121-zoom
  `class_visible` scan Rust-vs-TS, and the sea/forest parity suites pin the alpha ladders — drift
  is machine-caught. Remediation: swap the three imports to the wasm exports; port the reduction
  ladder; then `worldmap/lodGates.ts`/`seaBand.ts` demote to oracle-only under `_wasm/oracles/`.
- **B-02 OPEN (T) — forest fade ladder in TS.** `useWgpuForestMass.ts:8` imports
  `forestFillAlpha`; applied per-vertex at `:217` (`colors[d+3] = c.fillColors[s+3] * alpha`)
  while Rust `forest_fill_alpha` (`forest_mass.rs:241`, unit-tested at `:291-294`) goes unused by
  the live path. Same remediation shape as B-01 (the alpha could also move into
  `mass.compose(alpha)` which already accepts an alpha — the TS path deliberately composes at 1.0
  and recolors, `:147`).
- **B-03 PARTIAL (T) — chunk-id math split-brain.** The density lane derives ids in TS
  (`useWgpuForestMass.ts:80-84` via `chunkIdsForViewport`) while the building/glyph lane derives
  them in Rust (`WorldResidency.set_viewport`, `wgpuWorldLoader.ts:331`). `chunkMathRust.parity.test.ts`
  (24 tests) pins TS == Rust. The hub's own inventory row still cites `chunkMath.ts` as the
  source of truth (hub:107) — the doc and D5 disagree about where this policy lives.
- **B-04 PARTIAL (T) — ingest budget policy JS-side.** `APPLY_BUDGET_MS = 4` with the comment
  "enforced JS-side" (`wgpuWorldLoader.ts:29-30`) and the enforcement loop at `:386-404`; Rust
  receives `end_apply_frame(elapsed)` for stats only. Defensible under D2 (drain scheduling is
  IO-adjacent, rAF/visibility-aware `:407-424`), but the hub pins the 4 ms budget as engine
  policy — either move the loop behind a wasm `ingest_until_budget` or amend the hub wording.
- **B-05 PARTIAL (T, named deferral) — supercluster in TS.** `slotClusterIndex.ts` (240 LOC)
  owns the cluster index + drill/expansion zoom policy; engine renders discs from
  `set_cluster_markers`. The T-151.7.3 verify log names "Supercluster in Rust (FE still feeds
  set_cluster_markers)" under out-of-scope — documented, not silent; stays a D5 debt row until a
  slice ports it.
- **B-06 OPEN (M) — dead module.** `state/worldSpatialIndex.ts` (125 LOC) has **zero** live
  importers (only its own file and a comment in `slotSpatialIndex.ts`; the worker that used it was
  deleted at `c4831451`) and its header still says "inside the world-objects worker (W2)". Delete,
  or move under `_wasm/oracles/` if wanted as a reference.
- **PASS rows:** `wgpuSlots.ts` 56 (bridge, no pack math); `slotSpatialIndex.ts` facade over wasm
  (`:112-115` builds `wasm.SlotIndex`, `:131,:154` query it); `mapCamera.ts` thin wasm camera
  facade; `useSelectTool.ts` gesture machine (pointer/DOM domain, picks delegated);
  `wgpuBasemap.ts` sanctioned TBDS/LOD policy + upload plumbing (mirror-comment `:281-283`
  documents the `lanes::pack_offset` twin); `WgpuTacticalMap.tsx` mount/UI; `slotIconCache.ts`
  index maintenance.

---

## C — Slice-by-slice (17 slices)

Format: every spec §Verify command re-ran green at HEAD via the battery (rows 1–13) unless noted;
the per-slice rows below cover the gates beyond that shared surface.

- **C-sp-01 PASS — spike.** V1–V8 recorded closed in `t151_wgpu_spike_verify_log.md` (incl. the
  operator-completed WebGPU half; the log's findings 1–8 record plan-vs-reality corrections
  honestly — e.g. "60 cases" → 50). Its calibration self-check re-ran byte-exact today (7/7,
  verify log §GPU). Only slice with no git tag (commits `152b3a12…94261dd6`) — acceptable
  (pre-program spike), noted for the record.
- **C-0-01 PARTIAL (R) — T-151.0.** Byte chain (931,424 → 3,658,383 = +2,726,959) and vitest 317
  verified from the log and consistent with the fresh chain end. Fresh evidence: merged-pkg
  self-check byte-exact (WebGL2 7/7), entry isolation re-proven. Gaps: **S2** (editor HUD
  "shared-memory: PASS 2000/2000" proof) appears in no later log — never recorded closed; **S1**'s
  WebGPU half rests on T-151.1's lavapipe run (not reproducible here — verify log §GPU); **S3**
  (Deck smoke) was mooted by the T-151.9 deletion but never recorded before it.
- **C-1-01 PARTIAL (R) — T-151.1.** New tests exist and run fresh (`pickBaseLevel.test.ts`,
  `basemapLod.test.ts`, `hillshade.parity.test.ts` — in the vitest listing); GPU gates were
  EXECUTED then and re-executed now (`texture` 3/3 byte-exact). Gaps: S1 (unified satellite
  visual), S2 (map-style pyramid visual), S3 (hillshade slider visual), S4 (dual-mount screenshot
  diff, advisory) — all perceptual, none ever recorded closed (C-ALL-01).
- **C-2-01 PASS — T-151.2.** The strongest slice: all gates closed at ship (S1 runtime recorded:
  parity sweep cold 1.13 s; S2 spot-check present), and the census re-derived **fresh** today:
  391 / 508,291 / 275 / 888 / 36 / 625 all exact (verify log §Pinned inventory).
  `world.parity.test.ts` (9 tests) in the fresh run.
- **C-3-01 PARTIAL (R) — T-151.3.** P1–P14 ledger internally consistent; residency behavior now
  golden-locked (22-step, 2 tests, fresh PASS); 10k pick Class S via `world.pick.parity.test.ts`
  (fresh PASS); `world_building_self_check` re-executed byte-exact today (3/3). Gaps: S1–S3
  operator rows never closed; P9 was a "delegated PASS" (reused W2 evidence) — acceptable but
  noted.
- **C-3-02 OPEN (D) — outline color divergence.** wgpu casing `[30,30,34,255]` vs Deck stroke
  `[150,150,158,204]`, logged in t151_3 (§divergence, "Flagged for Cursor to reconcile the
  spec-vs-oracle text") and repeated in t151_4 ("Left unchanged… No gate impact"). No later log,
  spec, or hub note reconciles it. The Deck oracle is gone; the divergence is now permanent
  unless a decision is written. Remediation: Cursor one-liner in the hub or a 1-line constant
  change (`residency.rs::OUTLINE_COLOR`).
- **C-4-01 PARTIAL (R) — T-151.4.** Area-conservation + width gates are native tests (fresh
  cargo PASS); the slice's GPU-R gates (`sea_band_self_check`, `road_centerline_self_check`)
  shipped as "**API ready** — operator headless JSON below … pending local CDP run" — i.e. the
  gate did **not** execute at ship. **This audit executed both**: 2/2 + 2/2 probes byte-exact on
  WebGL2 (verify log §GPU). Residual gap: they never ran between 2026-07-09 ship and this audit,
  and S1/S3 visual rows remain unclosed.
- **C-4-02 OPEN (T) — optional parity test dropped silently.** Spec lists "Optional:
  `features/_wasm/vector.layers.parity.test.ts` — road buffer sample Class R"; the file does not
  exist and the t151_4 log never mentions the choice. Under the house no-silent-deferrals rule,
  soft "Optional" still requires an explicit disposition line. Minor (the native L8/L9 gates cover
  the math).
- **C-4.1-01 PASS — T-151.4.1.** The race fixes are verifiably in the current loader:
  abort → `clear_inflight` → `mark_inflight` (`wgpuWorldLoader.ts:342-347`), abort-path clears
  (`:370-376`), anti-wipe sticky lanes (`:426-447,460-465`). Buildings restored (0 → correct
  counts; today's `worldBuilding` readback passes). The log's honest disclosure that T-151.4
  shipped 0 buildings is the program's best honesty moment.
- **C-5-01 PASS — T-151.5.** Atlas 28 glyphs (fresh count), IconInstanced 20 B layout claims
  consistent, `glyphLod.parity.test.ts` (121-zoom scan) in the fresh run; size-math Class R rows
  covered by cargo `glyph_math` tests.
- **C-5-02 OPEN (R) — the never-executable GPU gate.** Spec S4: "GPU-R tree probe JSON pasted
  (nonzero α)". The engine API exists — `pub fn tree_glyph_self_check` at
  `crates/map-engine-render/src/engine.rs:4346` — but `WgpuCanvas.tsx:87-93` registers only
  `calibration`/`texture`/`worldBuilding`/`seaBand`/`roadCenterline`. The check is not callable
  from the harness, was never executed in any log, and could not be executed by this audit.
  Remediation: one-line registration + a headless run.
- **C-5.1-01 PASS — T-151.5.1.** `DENSITY_ISO` Rust SoT verified live (`density_iso()` passed
  into `forest_mass`, `useWgpuForestMass.ts:136-145` with the comment "Rust owns DENSITY_ISO —
  never pass a TS iso"); landcover LOD re-evaluation non-sticky (`wgpuWorldLoader.ts:229-241`);
  residual limits correctly routed to **T-149**, which exists in the registry (`idea`,
  "Forest Mass Polygon Smoothing").
- **C-6-01 PARTIAL (R) — T-151.6.** Automated gates (instance count == `slot_len`, drag delta
  uniform 80 B, undo SoA sampling) live in `slotGpu.parity.test.ts` (7 tests, fresh PASS) +
  engine tests. S1–S5 were operator rows and remain unclosed; the slice log itself notes "W7
  pick/gesture not wired — selection/drag tint via Zustand (console)" — an honest limitation of
  what could be observed at ship time.
- **C-7-01 PARTIAL (R) — T-151.7.** The interaction parity suite is real and green fresh
  (`interaction.parity.test.ts`, 12 tests — synthetic pointer scripts asserting identical
  selection sets and `encode_state` bytes vs the Deck oracle). S1–S5 operator rows never closed;
  the immediate 7.1 hotfix (tint, drag FPS, zoom-at-cursor — all operator-reported) demonstrates
  exactly the class of defect the unclosed visual gates existed to catch. Process finding, not a
  code defect today.
- **C-7.1-01 PASS — T-151.7.1.** B1–B3 fixes verified in current code (dual-lane tint
  rematerialize; delta-uniform drag; zoom anchor). vitest +1 chain consistent.
- **C-7.2-01 PARTIAL (M) — T-151.7.2.** Shipped from a verify log only: no spec file
  (`t151_7_2_*.md` absent from docs/specs), no `-docs` tag. The wheel-restore commit `69ca1c08`
  landing after the tag is documented in CLAUDE/hub and verified in git. Fixes themselves verified
  (engine camera SoT; full selection rematerialize).
- **C-7.3-01 PASS — T-151.7.3.** LOC collapse proven then and re-proven now (`wgpuSlots.ts` 56;
  no TS `pack_slot_instances`/`cluster_mode` reimplementation — grep clean). Named out-of-scope
  rows (supercluster, gesture SM) documented → B-05.
- **C-8-01 PARTIAL (R) — T-151.8 + 8.1.** All Class S/R gates are cargo-native and re-ran green
  today: `class_s_draw_set_equals_strict_reference`, `class_r_chunks_draw_matches_draw_ids_len`,
  `density_ladder::r1/r2/r3`, compute-cull Class R (incl. 1k frusta). The 8/8.1 deferral history
  is honest in git (the pre-8.1 log revision names compute cull DEFERRED; 8.1 amends it SHIPPED).
  Gaps: **S4 band table** (`fps` + `gpu_frame_ms` per LOD band) is still the empty
  `*operator*` template in the log — never filled; exact wasm bytes for W8 were never recorded
  (only "~4.09/~4.12 MB") — this audit pins **4,123,261 B** at HEAD (F-04 honesty fix applied).
- **C-9-01 PASS — T-151.9.** Every automated claim re-verified fresh at HEAD: vitest **281/281**
  (= 393 − 112 exactly; 39 files), `dist/assets` deck/luma-free, bundle **6,273,423 B** equals the
  log's post-flip byte count exactly, `wgpuSlots.ts` 56, six packages demoted, 35-file deletion
  atomic, oracles relocated under `_wasm/oracles/` with the residency golden. S1–S3 operator
  confirmations "recommended" — folded into C-ALL-01.
- **C-9-02 PARTIAL (D) — self-invalidated tag line.** The log recorded tag tip `c87d74fa`;
  the very commit that added the clarification (`58c8fcc3`) became the new tag tip. One-line
  honesty fix applied by this audit (verify log §Honesty fixes).
- **C-ALL-01 OPEN (R) — program-wide operator-gate honesty gap.** `grep -in "operator.*(PASS|
  confirm|good enough|verified)"` across all 16 T-151 verify logs returns **zero** recorded
  operator closures (the two hits are a code-gate note and a legend line). Every S1–S5 perceptual
  gate in every slice — ~45 rows program-wide — remains formally open. The 7.x hotfix chain proves
  the operator did use the map (and found bugs the gates enumerate), but no gate row was ever
  flipped to PASS. Either record a single consolidated operator sign-off against the current
  build, or accept and document that perceptual gates are advisory. This is the audit's largest
  process finding.

---

## D — Class R oracles / goldens

- **D-01 PASS.** `npx vitest list` enumerates **281** tests == 281 executed (nothing skipped,
  no `.only`/`.skip` in effect). **16** parity files / **99** parity tests (census in verify log).
  Oracle taxonomy: pure-JS ports (`jsWorldChunkOracle.ts`), frozen goldens (residency), Deck
  devDependency (2 camera/interaction tests — sanctioned "camera oracle forever" per hub), native
  Rust twins (cargo).
- **D-02 PASS (T note).** Residency golden `residency_everon_v1.json`: `steps: 22`, `baseline:
  "ec59d10e"`, asserted by `world.residency.parity.test.ts` (`SCRIPT.length === 22`, missing/
  resident/pinned per step). Regen path committed (`T151_CAPTURE_RESIDENCY=1 npx vitest run …`).
  Note: post-Deck a regeneration snapshots the wasm itself — the golden is now a regression lock,
  not an independent oracle; the independent evidence is the frozen `ec59d10e`-era capture. This
  mirrors the F1→F4 Yjs discipline and the test header says so — by design, recorded here for
  the record.
- **D-03 PASS.** Compute-cull Class R oracle present and fresh-run:
  `compute_cull::tests::class_r_1k_random_frusta_count_stable` + `class_r_inside_outside` +
  `class_r_compact_preserves_order_and_count` + `storage32_roundtrip` (battery row 5). The 8.1
  claim "Class R CPU AABB oracle (1k frusta)" is reproduced exactly.
- **D-04 PASS.** Deck residue: `@deck.gl` imports exist in exactly 2 files, both under
  `_wasm/` (orthoCamera + interaction parity tests). Zero Deck imports in live code; the 3
  worldmap "deck" mentions are comments.
- **D-05 PARTIAL (D).** Pinned-inventory re-derivation: full table in the verify log. 10 rows
  exact PASS (391 / 508,291 / 275 / 888 / 36 / 625×1,172 / 28 / TBDS bytes+mips / DEM range /
  cluster+pick constants / CHUNK_CAPACITY). PARTIAL rows: road classes ("6 classes" = the closed
  style enum `roads.rs:14-23`; live Everon data has **5**, `path` = 0 segments); three rows cite
  files deleted at T-151.9 (`useOrthographicView.ts`, `worldObjectsCore.ts`, `chunkStore.ts`) whose
  values now live at `mapCamera.ts:14-15`, `lod_gates.rs:26` (`INSTANCE_BUDGET` — now Rust-owned,
  which is the direction D5 wants), and `wgpuWorldLoader.ts:30`. Remediation: Cursor refreshes the
  hub table citations.
- **D-06 OPEN (T).** The GPU-R harness (headless chromium + CDP + `__selfChecks`) exists only as
  procedure descriptions inside old verify logs — no committed runner script. This audit had to
  rebuild it from scratch (driver in scratchpad; procedure documented in the verify log §GPU).
  Every future GPU-R re-verification pays that cost again, and the WebGPU/lavapipe half did not
  reproduce in this environment at all. Remediation: commit a `scripts/` runner (or a vitest-driven
  CDP harness) so `sea_band`/`road_centerline`/`tree_glyph`/`calibration` checks are one command.

---

## E — Reliability / security (map surface)

- **E-01 PASS (R).** FE-reachable panic surface: `crates/map-engine-wasm/src/lib.rs` contains
  **zero** `unwrap()`/`expect(`/`panic!`. The 50 `unwrap()` in map-engine-core all sit inside
  `#[cfg(test)]` modules (awk pre-test-marker scan of the four worst files — residency 13,
  store 8, slots_gpu 8, glyph_math 5 — returns zero hits before the test marker).
- **E-02 PASS (R).** Init/render failure UX: engine/init rejections and render throws surface to
  a visible error banner (`WgpuTacticalMap.tsx:104,466,485,594-596` — "I6 errors surface into the
  banner, never swallowed"); the engine's own backend fallback covers WebGPU-absent browsers
  (create-surface retry → WebGL2; observed live in the audit's lavapipe attempt), and the spike
  page offers Force WebGL2.
- **E-03 PASS (S).** Secrets grep over `tactical-map/` + all three crates: no tokens/keys;
  the only hit is a CSS color comment. The map surface never touches `X-Service-Token`.
- **E-04 PASS (R).** Loader failure discipline verified in current code: HTML-fallback detection
  treats SPA-fallback 200s as missing (`httpFetchBytes`, `wgpuWorldLoader.ts:34-41` — an LFS
  pointer served as text would fail Rust gunzip and be marked `note_undelivered`, `:392-396`);
  abort → `clear_inflight`/`mark_inflight` re-mark discipline (`:342-347,370-376`); empty-mid-
  hydration uploads suppressed so lanes never wipe (`:426-447,460-465`); glyph-atlas failures
  degrade to glyphs-off with a console warning (`:128-167`).
- **E-05 OPEN (R).** Forest-mass session cache is unbounded by design: "Session cache of density
  chunks; composite grows with exploration" / "No eviction — composite covers every hydrated
  chunk (Deck forestMassStore policy)" (`useWgpuForestMass.ts:2,36`) — `cache` (`:44`) only ever
  grows, and each entry holds composed Float32/Uint32 meshes. A long editing session panning the
  whole island accumulates all 625 chunks' meshes in JS memory (bounded by island size, unbounded
  by working set). Deck-parity inherited debt, admitted in comments, never ticketed. Remediation
  ticket: small LRU or residency-driven eviction.

---

## F — Docs honesty (claims ↔ git ↔ live tree)

Tag map (37 tags) and full reconciliation commands in the verify log §Tag reconciliation.

- **F-01 PASS.** wasm byte-size chain exact at every documented step:
  931,424 → **3,658,383** (+2,726,959) → **3,723,192** (+64,809) → **3,858,591** (+135,399) →
  **3,946,734** (+88,143) → **4,005,415** (+58,681) → **4,009,368** (+3,953) → **4,054,850**
  (+45,482) → **4,055,075** (+225) → **4,063,618** (+8,543) → **4,063,911** (+293) →
  **4,071,877**. Every delta re-computed; every log agrees with its neighbors.
- **F-02 PASS.** vitest chain exact at every step: 317 → 334 (+17) → 343 (+9) → 371 (+28) → 371 →
  371 → 372 (+1) → 374 (+2) → 379 (+5) → 391 (+12) → 392 (+1) → 393 (+1) → 393 → 393 →
  **281** (= 393 − 112 + 0, verified by the deleted-test accounting in the T-151.9 log and by the
  fresh run).
- **F-03 PASS.** All 35 8-hex strings extracted from the 16 logs: 31 are commits — every one
  exists (`git cat-file -e`) and is an ancestor of HEAD; baselines match the tag map (t151_0's
  parent claim `16d19d28` = `git rev-parse f019512d^` exactly). The 4 non-resolving strings are
  checksums/screenshot names, not commit claims.
- **F-04 PARTIAL (D) — fixed.** t151_8's "~4.09 MB" was the pre-8.1 figure and survived the 8.1
  amendment of the same log; the hub's "~4.12 MB" is correct (fresh build **4,123,261 B**). No
  exact byte count was ever recorded for W8/W8.1 — the exact-byte discipline every other slice
  held broke here. One-line honesty fix applied (verify log §Honesty fixes #2).
- **F-05 PARTIAL (D).** Docs-tag topology: the five dual-role tags (`cd71736b`, `00cc9b8a`,
  `fb21b9dd`, `d1ed26e2`, `4042e686`) are legitimate one-commit "prev-sync + next-spec" pairs
  (message + `--stat` verified). Convention breaks after T-151.8: no docs tags for 7.2 / 8.1 / 9
  (commits `c52d1fc8`, `a7a93368` carry tag-role messages but no tags), and tag `T-151.7.3-docs`
  sits on `fa7a4b1d` ("T-151-docs: mandatory LANGUAGE GATE…") while the commit whose message says
  "T-151.7.3-docs:" is `5457dd4e`. Cosmetic; Cursor may retag or drop the convention explicitly.
- **F-06 PARTIAL (D).** Hub link integrity: all 33 link targets resolve. Gap: the T-151.5.1
  section links spec + corrective note but never its verify log (`t151_5_1_verify_log.md` is the
  only slice log unlinked from the hub).
- **F-07 PASS.** T-151.9 bundle ledger: 7,151,199 → 6,273,423 = **−877,776 B** = 12.27% ("~12%");
  CLAUDE "~7.15 → 6.27 MB" and hub mirror exactly; fresh build reproduces 6,273,423 B to the byte.
- **F-08 PARTIAL (D).** t151_7_3 baseline line reads "`69ca1c08` + docs `5457dd4e`" but
  `git rev-parse 804f779a^` = `fa7a4b1d` — one more docs-only commit (the LANGUAGE GATE doc)
  between the stated baseline and the code commit. Code state unaffected; precision miss only.
- **F-09 OPEN (R) — T-151 gates have never run in CI.** `ci.yml`'s rust job runs
  `cargo fmt/clippy/build/test` with `working-directory: apps/website` — member-scoped, so the
  three map-engine crates are never checked; `make wasm-ci` appears in no workflow. The frontend
  job runs `npm run build`/`npm test` with **no `make wasm` step** while
  `apps/website/frontend/src/wasm/pkg/` is gitignored (`.gitignore:28`) — on any push of this
  branch the job fails at import resolution. The branch has no remote (`git branch -r` shows only
  `origin/main`), so CI has never exercised any T-151 commit. The spike log's finding 7 named
  exactly this as a "merge-time follow-up"; it remains unimplemented. Remediation (blocking
  before merge to main): add `make wasm-ci` + a wasm build step to `ci.yml`.

---

## Class S battery summary

13/13 commands exit 0 at `1cbe3a56` — full table with outputs in
[`t151_10_verify_log.md`](t151_10_verify_log.md) §Class S battery. Headline: cargo **176** tests
(155 core + 21 render), vitest **281/281** (39 files; 99 parity), fresh wasm **4,123,261 B**,
`dist/assets` **6,273,423 B** deck/luma-free, `wgpuSlots.ts` **56** LOC, wasm32 clippy clean,
workspace build clean. GPU-R: **17/17** probes byte-exact (WebGL2/SwiftShader); WebGPU
NOT RE-RUN (environment; error quoted in verify log).

---

## Remediation candidates for Cursor (OPEN first, then PARTIAL worth ticketing)

| Priority | Finding | One-line remediation |
|----------|---------|----------------------|
| 1 | **F-09** CI never runs T-151 gates | `ci.yml`: add `make wasm-ci` job + `make wasm` before frontend build (named blocking-before-merge) |
| 2 | **C-ALL-01** zero operator gates ever closed | One consolidated operator sign-off pass against the current build, recorded in a log; or declare perceptual gates advisory in the hub |
| 3 | **B-01/B-02** live TS policy twins (DEM-vectors + forest alpha) | Swap 4 imports to existing wasm exports (`class_visible`, `contour_interval_for_zoom` needs export, `sea_fill_alpha`, `forest_fill_alpha`); demote `lodGates.ts`/`seaBand.ts`/`forestMass.ts` alpha fns to oracles |
| 4 | **C-5-02** `tree_glyph_self_check` never executable | Register in `WgpuCanvas.tsx` `__selfChecks` + run headless once |
| 5 | **D-06** GPU harness uncommitted | Commit the CDP runner (this audit's driver is a working reference) |
| 6 | **E-05** forest cache unbounded | LRU or residency-driven eviction ticket |
| 7 | **A-10** `/_spike/wgpu` + `__selfChecks` in prod | Gate on `import.meta.env.DEV` |
| 8 | **B-06** dead `worldSpatialIndex.ts` | Delete (125 LOC) |
| 9 | **A-07** stale engine-flag comments | Rewrite 2 comments (`WgpuTacticalMap.tsx:3`, `wgpuWorldLoader.ts:10-11`) |
| 10 | **A-08 / D-05 / F-05 / F-06** hub/doc drift | Cursor doc pass: D1–D5 heading, spec "D1–D9" range, 3 stale inventory citations, road-classes wording, link t151_5_1 log, tag convention note |
| 11 | **C-3-02** outline color divergence | Decide + write it down (hub note or `residency.rs::OUTLINE_COLOR` change) |
| 12 | **B-03/B-04/B-05** remaining D5 debts | Port or explicitly sanction: TS chunk ids (density lane), JS-side 4 ms budget, supercluster |
| 13 | **C-8-01** S4 band table empty | Fill during the operator sign-off pass (same session as #2) |

---


## Claims register (appendix) — 250 extracted claims, 250 dispositions

Extracted by the salvaged claims-extractor lens from hub + slice specs + verify logs;
dispositions assigned by rule against the findings above (proofs live at the referenced finding/verify-log section).

| # | Slice | Kind | Claim | Disposition |
|---|-------|------|-------|-------------|
| 1 | hub | docs | Program status: W0–W9 shipped, Deck retired @ c4831451 (tag T-151.9); next is T-151.10 audit. (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:3) | **PASS** — F-03/C-9-02 — SHAs reconciled; tag line honesty-fixed |
| 2 | hub | docs | Spike shipped as commits 152b3a12…94261dd6 (camera parity + render spine + 20M stress + byte-exact self-check). (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:6-8) | **PARTIAL** — F-03 commits verified + calibration self-check re-executed (7/7); 20M stress = operator-hardware, not reproducible headless |
| 3 | hub | behavior | After W9 the Deck runtime is retired and deck.gl remains a devDependency camera oracle only. (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:16-17) | **PASS** — A-03/A-09/D-04 — verified: sole mount, devDeps, 2 oracle tests |
| 4 | hub | behavior | Verification philosophy: JS/Deck implementation stays in tree as oracle for every ported system until T-151.9; deck.gl remains a devDependency forever (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:94-96) | **PASS** — A-03/A-09/D-04 — verified: sole mount, devDeps, 2 oracle tests |
| 5 | hub | docs | Anything irreducibly perceptual (pan feel) is recorded as an explicit operator statement, never claimed as verified. (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:98-99) | **PASS** — F-03 — SHA/link/status claims reconciled against git (see §F) |
| 6 | hub | count | Pinned inventory: prefab count = 391 (manifest.json objects.prefabCount). (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:105) | **PASS** — D-05 + verify log §Pinned inventory — re-derived exact |
| 7 | hub | count | Pinned inventory: world object instances = 508,291 (objects.instanceCount). (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:106) | **PASS** — D-05 + verify log §Pinned inventory — re-derived exact |
| 8 | hub | count | Pinned inventory: chunk files = 275 on a 512 m grid using floor(x/512). (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:107) | **PASS** — D-05 + verify log §Pinned inventory — re-derived exact |
| 9 | hub | count | Pinned inventory: road segments = 888 across 6 classes including runway. (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:108) | **PARTIAL** — D-05 — 888 exact; "6 classes" = style enum, 5 in data |
| 10 | hub | count | Pinned inventory: forest regions = 36. (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:109) | **PASS** — D-05 + verify log §Pinned inventory — re-derived exact |
| 11 | hub | count | Pinned inventory: TBDD density grids = 625 files × 1,172 B each (17×17 u16 corners, 32 m cells, 2 channels). (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:110) | **PASS** — D-05 + verify log §Pinned inventory — re-derived exact |
| 12 | hub | count | Pinned inventory: world glyph atlas = 28 glyphs (glyphs/atlas/world-glyphs.json). (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:111) | **PASS** — D-05 + verify log §Pinned inventory — re-derived exact |
| 13 | hub | bytes | Pinned inventory: TBDS satellite = 12800² base, 14 mips, 152,713,114 B total. (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:112) | **PASS** — D-05 + verify log §Pinned inventory — re-derived exact |
| 14 | hub | count | Pinned inventory: DEM = 6400² u16 spanning −204.78…375.53 m with no axis flip. (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:113) | **PASS** — D-05 + verify log §Pinned inventory — re-derived exact |
| 15 | hub | behavior | Pinned inventory: zoom band = −6…+6 with default −2 (useOrthographicView.ts:12–13,33). (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:114) | **PARTIAL** — D-05 — values live; cited file deleted at T-151.9 |
| 16 | hub | behavior | Pinned inventory: slot pick radius = 4 px and drag threshold = 4 px (slotSpatialIndex.ts:123, useSelectTool.ts:21). (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:115) | **PASS** — D-05 + verify log §Pinned inventory — re-derived exact |
| 17 | hub | behavior | Pinned inventory: cluster gates = >500 slots AND zoom ≤ −4; cluster pick 48 px; super-zoom round(z+8) clamped 0–16. (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:116) | **PASS** — D-05 + verify log §Pinned inventory — re-derived exact |
| 18 | hub | behavior | Pinned inventory: world pick radius = 12 px (t090_render_lod_contract.md §N2). (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:117) | **NOT RE-VERIFIED** — no live world-pick call traced this pass |
| 19 | hub | count | Pinned inventory: legacy Deck instance budget = 150,000 (worldObjectsCore.ts), described as 'to be lifted'. (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:118) | **PARTIAL** — D-05 — value now Rust lod_gates.rs:26; hub cites deleted file |
| 20 | hub | behavior | Pinned inventory: chunk apply budget ≤ 4 ms/frame (chunkStore.ts APPLY_BUDGET_MS). (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:119) | **PARTIAL** — B-04/D-05 — enforced JS-side at wgpuWorldLoader.ts:30; hub cites deleted file |
| 21 | hub | behavior | Pinned inventory: chunk LRU cap = max(64, 3 × pinned). (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:120) | **PASS** — D-02 — behavior locked by 22-step residency golden (fresh PASS); line cite not re-derived |
| 22 | hub | bytes | Spike constants: engine chunk pool = 2,097,152 × 32 B = 64 MiB; scene anchor (6400, 6400); navigation invariant = 64 B/frame. (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:121) | **PASS** — D-05 + verify log §Pinned inventory — re-derived exact |
| 23 | hub | gpu-probe | Measured GPU constant ≈ 0.69 ms per 1M instances (32 B layout, operator hardware; gpu_frame_ms 13.9–14.4 at 20M). (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:122) | **NOT RE-VERIFIED** — operator-hardware measurement; not reproducible headless |
| 24 | hub | behavior | LOD gate constants: TREE_GLYPH_MIN_ZOOM=0, BUILDING_FOOTPRINT_MIN_ZOOM=−2.5, BUILDING_BADGE_MIN_ZOOM=+1, VEGETATION_MIN_ZOOM=+1.5, PROP_MIN_ZOOM=+3, f (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:124-127) | **PASS** — D-05 + verify log §Pinned inventory — re-derived exact |
| 25 | hub | behavior | D1: one wasm module with one linear memory — MissionDoc, WorldStore columns, and RenderEngine share memory; --target web spike pkg retired at T-151.0; (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:131-136) | **PASS** — A-01 |
| 26 | hub | behavior | D2: world-object worker retired not ported — chunks fetched+gunzipped by thin JS loader, parsed once in Rust under ≤4 ms/frame amortized budget, uploa (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:137-143) | **PARTIAL** — A-02 — substance holds; gunzip is Rust flate2, not DecompressionStream |
| 27 | hub | behavior | D3: dual-mount migration — Deck TacticalMap or WgpuTacticalMap behind VITE_MC_ENGINE=wgpu / ?engine=wgpu with the same TacticalMapProps contract; Deck (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:144-149) | **PASS** — A-03 — flag gone; sole wgpu mount |
| 28 | hub | behavior | D5: Rust owns engine policy (geometry, LOD, residency, SoA→GPU sync, selection/drag/cluster GPU policy, camera math, spatial indexes, pack helpers, sh (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:153-159) | **PARTIAL** — §B thesis — QUALIFIED PASS; leak cluster B-01…B-06 |
| 29 | T-151.0 | gate | T-151.0 hub gates: all shipped spike gates re-run green on the merged pkg; vitest baseline 317 + moved tests green; entry-chunk isolation via '! grep  (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:175-179) | **PASS** — F-02 — vitest chain exact; fresh run 281/281 |
| 30 | T-151.1 | gate | T-151.1 hub gates: texture corner probes byte-exact at projected worldBounds NW/NE/SW vs source mip corner texels; mip-selection golden test; hillshad (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:199-202) | **PASS** — A-03 — flag gone; sole wgpu mount |
| 31 | T-151.1 | behavior | T-151.1 hub: grid is procedural 1 km lines (~40) + border per useBaseMapLayer.ts; pyramid fallback ≤64 tile quads with south-first Y per tileUrl.ts; p (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:194-198) | **PASS** — C-1-01 — covered by slice verdict (automated surface green) |
| 32 | T-151.2 | gate | T-151.2 hub gates: world.parity.test.ts covers all 275 real chunk files with SoA columns byte-equal (Class R), per-class row sets equal (Class S), tot (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:224-227) | **PASS** — F-02 — vitest chain exact; fresh run 281/281 |
| 33 | T-151.3 | gate | T-151.3 hub gates: scripted pan/zoom path → identical chunk-id sets and identical LRU eviction order (Class S); per-chunk instance counts exact; apply (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:247-251) | **PASS** — verify log §GPU — re-executed WebGL2, byte-exact (17/17 probes) |
| 34 | T-151.4 | gate | T-151.4 hub gates: triangulation area conservation (Σ triangle areas == ring polygon area within ULP-scaled tolerance per polygon); polyline width at  (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:269-274) | **PASS** — verify log §GPU — re-executed WebGL2, byte-exact (17/17 probes) |
| 35 | T-151.5 | gate | T-151.5 hub gates: exhaustive LOD equality scan Rust class_visible == lodGates.ts for every class × 121 zooms {−6.0…+6.0}; glyph UV rect golden test;  (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:295-299) | **OPEN** — C-5-02 — gate never wired/executed |
| 36 | T-151.5 | bytes | T-151.5 hub: production icon instance layout pinned ≤ 20 B (pos 2×f32=8, size 4, rotation snorm16=2, glyph u16=2, tint u32=4); per-instance UV via 28- (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:288-294) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 37 | T-151.6 | gate | T-151.6 hub gates: rendered instance count == slot_len after scripted mutations (add / paste 10k / delete / undo / redo); selection flag population == (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:317-321) | **PASS** — C-6-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 38 | T-151.6 | gate | T-151.6 hub gate: criterion-6 re-run at 500k seeded slots with fps + gpu_frame_ms recorded. (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:321-322) | **PASS** — C-6-01 — covered by slice verdict (automated surface green) |
| 39 | T-151.7 | class-R | T-151.7 hub gate: interaction parity suite — scripted pointer/keyboard sequences on Deck vs wgpu produce identical selection sets and identical doc mu (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:340-344) | **PASS** — C-7-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 40 | T-151.8 | bytes | T-151.8 hub summary: strict draw-set cull (visible ∩ pinned, DRAW_CULL_MARGIN_M = 0); exact-count density ladder + heatmap; damage-driven render; WebG (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:373-375) | **PARTIAL** — F-04 — W8 exact bytes never recorded; audit pins 4,123,261 B |
| 41 | T-151.9 | docs | T-151.9 hub summary: always WgpuTacticalMap with no engine flag or Deck escape hatch; Deck runtime deleted; six deck/luma pkgs → devDependencies; vite (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:383-388) | **PASS** — F-02 — vitest chain exact; fresh run 281/281 |
| 42 | spike | count | Spike baseline: npm test measured 37 files / 312 tests PASS; CLAUDE.md's stated '223' baseline was stale. (.ai/artifacts/t151_wgpu_spike_verify_log.md:11-12) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 43 | spike | count | Spike V3: cargo test map-engine-core = 66 PASS (56+5+5) including T1–T4 parity, closed-form, and 13k property cases. (.ai/artifacts/t151_wgpu_spike_verify_log.md:21) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 44 | spike | count | Spike V3: cargo test map-engine-render = 4 PASS (Class-R instance-byte memcmp including JS-cross-oracle LCG pins). (.ai/artifacts/t151_wgpu_spike_verify_log.md:22) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 45 | spike | gate | Spike V4: regenerated goldens stable — fixture sha256 9b213a6c…392b across runs, 300 cases, 509,708 B. (.ai/artifacts/t151_wgpu_spike_verify_log.md:23) | **PASS** — C-sp-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 46 | spike | count | Spike V5: npm test = 39 files / 317 tests PASS (baseline 312 + orthoCamera.parity 2 + deviceSize 3). (.ai/artifacts/t151_wgpu_spike_verify_log.md:24) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 47 | spike | bytes | Spike V6: map_engine_render_bg.wasm = 2,828,906 B (wgpu webgpu+webgl payload). (.ai/artifacts/t151_wgpu_spike_verify_log.md:25) | **PASS** — F-01 — byte chain exact, every delta re-computed |
| 48 | spike | gpu-probe | Spike V8a: self-check on detected webgl2 backend pass:true with all 7 probes byte-exact (JSON pasted), including the R north-up proof and its green mi (.ai/artifacts/t151_wgpu_spike_verify_log.md:33-36) | **PASS** — verify log §GPU — re-executed WebGL2, byte-exact (17/17 probes) |
| 49 | spike | gpu-probe | Spike V8a: self-check with ?force=webgl (fresh page) produced an identical report, pass:true. (.ai/artifacts/t151_wgpu_spike_verify_log.md:39) | **PASS** — verify log §GPU — re-executed WebGL2, byte-exact (17/17 probes) |
| 50 | spike | gpu-probe | Spike V8a stress @1M (Electron/webgl2): 165 fps display-capped, gen 12.0 ms, upload 9.0 ms, gpu_bytes 32,000,160, staging_peak 32,000,000, instances e (.ai/artifacts/t151_wgpu_spike_verify_log.md:45) | **NOT RE-VERIFIED** — operator-hardware measurement; not reproducible headless |
| 51 | spike | gpu-probe | Spike V8a stress @20M (Electron/webgl2): 65–70 fps, gen 92.0 ms, upload 196.0 ms, gpu_bytes 640,000,160, staging_peak 67,108,864 (= one 64 MiB chunk), (.ai/artifacts/t151_wgpu_spike_verify_log.md:46) | **NOT RE-VERIFIED** — operator-hardware measurement; not reproducible headless |
| 52 | spike | gate | Spike navigation invariant: uniform_bytes_last_frame read 64 in every sampled frame, including during wheel-zoom and after seeding 20M; seeding 20M en (.ai/artifacts/t151_wgpu_spike_verify_log.md:48-51) | **PASS** — C-sp-01 — covered by slice verdict (automated surface green) |
| 53 | spike | gpu-probe | Spike V8b operator Firefox/webgl2: 20M instances at 58 fps; seed 384 ms (gen 153.0 ms, upload ≈221–231 ms); staging_peak 67,108,864; uniform 64 after  (.ai/artifacts/t151_wgpu_spike_verify_log.md:66-74) | **NOT RE-VERIFIED** — operator-hardware measurement; not reproducible headless |
| 54 | spike | gpu-probe | Spike V8b operator Chrome/webgpu: 20M instances at 67 fps; gpu_frame_ms 13.894 ms via live TIMESTAMP_QUERY; seed 488 ms (gen 164.0, upload 323.0); sta (.ai/artifacts/t151_wgpu_spike_verify_log.md:82-89) | **NOT RE-VERIFIED** — verify log §GPU — WebGPU not reproducible in this env; T-151.1-era evidence only |
| 55 | spike | gpu-probe | Spike calibrated §20M ladder constant: 13.894 ms / 20M ≈ 0.69 ms GPU per million instances; L0 icon budget B=2M costs ≈1.4 ms of a 16.67 ms frame. (.ai/artifacts/t151_wgpu_spike_verify_log.md:91-94) | **NOT RE-VERIFIED** — operator-hardware measurement; not reproducible headless |
| 56 | spike | gpu-probe | Spike V8b WebGPU self-check (Chrome): all 7 probes pass:true byte-identical to the webgl2 reports; same session gpu_frame_ms 14.428 ms with 20M re-see (.ai/artifacts/t151_wgpu_spike_verify_log.md:103-112) | **NOT RE-VERIFIED** — verify log §GPU — WebGPU not reproducible in this env; T-151.1-era evidence only |
| 57 | spike | behavior | Spike operator perceptual trio recorded as operator statement 'yes ×3' (wheel-zoom at cursor, drag-right moves content right, smooth ≤1M) — not claime (.ai/artifacts/t151_wgpu_spike_verify_log.md:126-131) | **OPEN** — C-ALL-01 — operator gate never recorded closed (see C-sp-01) |
| 58 | spike | behavior | Spike deviation: serde_json default float parse measured 1 ULP off; float_roundtrip feature enabled for the parity dev-dependency. (.ai/artifacts/t151_wgpu_spike_verify_log.md:150-152) | **PASS** — C-sp-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 59 | spike | behavior | Spike deviation: matrix-path center cancellation inexact (project(target).y = 300.0000000000009); 'center exact ==' property corrected to 1e-9 toleran (.ai/artifacts/t151_wgpu_spike_verify_log.md:153-155) | **PASS** — C-sp-01 — covered by slice verdict (automated surface green) |
| 60 | spike | docs | Spike deviation: plan text said 60 800×600 cases; actual fixture is 50 (10 zooms × 5 targets). (.ai/artifacts/t151_wgpu_spike_verify_log.md:156-157) | **PASS** — F-03 — SHA/link/status claims reconciled against git (see §F) |
| 61 | T-151.0 | bytes | T-151.0 wasm sizes: baseline 931,424 B → post-merge 3,657,508 B → post-batch-refactor 3,658,383 B. (.ai/artifacts/t151_0_verify_log.md:29-33) | **PASS** — F-01 — byte chain exact, every delta re-computed |
| 62 | T-151.0 | bytes | T-151.0 delta baseline→merged = +2,726,959 B (≈2.6 MB engine payload); batch-list refactor adds +875 B. (.ai/artifacts/t151_0_verify_log.md:35-36) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 63 | T-151.0 | gate | T-151.0: merged .d.ts exports classes RenderEngine, MissionDoc, OrthoCameraJs with full RenderEngine surface (create, render, self_check, seed_stress, (.ai/artifacts/t151_0_verify_log.md:38-46) | **PASS** — verify log §GPU — re-executed WebGL2, byte-exact (17/17 probes) |
| 64 | T-151.0 | behavior | T-151.0 L3 decision: no wasm-bindgen duplicate-start error occurred; map-engine-render keeps its #[wasm_bindgen(start)] panic hook (engine.rs:42). (.ai/artifacts/t151_0_verify_log.md:22-26) | **PASS** — C-0-01 — covered by slice verdict (automated surface green) |
| 65 | T-151.0 | count | T-151.0: cargo test map-engine-core = 66 (56+5+5) all passed, 0 failed. (.ai/artifacts/t151_0_verify_log.md:68-79) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 66 | T-151.0 | count | T-151.0: cargo test map-engine-render = 4 passed, 0 failed. (.ai/artifacts/t151_0_verify_log.md:81-87) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 67 | T-151.0 | count | T-151.0: npm test = 39 files / 317 tests passed (count unchanged; deviceSize.test.ts move kept it). (.ai/artifacts/t151_0_verify_log.md:103-107) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 68 | T-151.0 | gate | T-151.0 entry-chunk isolation PASS: grep exit 1 on index-*.js; wasm referenced only by map_engine_wasm-dxJ23z94.js, worldObjects.worker-zWRK5Wzp.js, a (.ai/artifacts/t151_0_verify_log.md:122-131) | **PASS** — C-0-01 — covered by slice verdict (automated surface green) |
| 69 | T-151.0 | behavior | T-151.0: WgpuTacticalMap is lazy-loaded via React.lazy (static import broke Vite esbuild dep-scan on the raw @/ wasm import); build/lint/test (317) +  (.ai/artifacts/t151_0_verify_log.md:133-144) | **PASS** — C-0-01 — covered by slice verdict (automated surface green) |
| 70 | T-151.0 | behavior | T-151.0 batch refactor behavior-identical: same draw order (stress then calibration), same LoadOp::Clear(CLEAR_COLOR), one TIMESTAMP_QUERY set, unifor (.ai/artifacts/t151_0_verify_log.md:149-153) | **PASS** — C-0-01 — covered by slice verdict (automated surface green) |
| 71 | T-151.0 | gate | T-151.0 manual S1–S3 (spike self-check JSONs, 20M stress re-record, editor shared-memory HUD, Deck flag-off smoke) are recorded as operator-pending —  (.ai/artifacts/t151_0_verify_log.md:176-198) | **NOT RE-VERIFIED** — operator-hardware measurement; not reproducible headless |
| 72 | T-151.0 | gate | T-151.0 spec L10: shared-memory proof is numeric — seed_random(1000, 12800, 12800, 0x12345678), refresh(), Float32Array(memory.buffer, slot_xy_ptr, 20 (docs/specs/Mission_Creator_Architecture/t151_0_wasm_merge_dual_mount.md:63) | **PASS** — C-0-01 — covered by slice verdict (automated surface green) |
| 73 | T-151.1 | count | T-151.1: cargo test map-engine-core = 56 lib + 5 camera_props + 5 deckgl_ortho_parity, all passed. (.ai/artifacts/t151_1_verify_log.md:23-27) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 74 | T-151.1 | count | T-151.1: cargo test map-engine-render = 9 passed (4 scene + 5 lanes, all Class R). (.ai/artifacts/t151_1_verify_log.md:29-30) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 75 | T-151.1 | bytes | T-151.1: merged map_engine_wasm_bg.wasm = 3,723,192 B (baseline 3,658,383 → +64,809 B). (.ai/artifacts/t151_1_verify_log.md:35-36) | **PASS** — F-01 — byte chain exact, every delta re-computed |
| 76 | T-151.1 | count | T-151.1: vitest = 41 files / 334 tests passed (baseline 317 → +17 W1 tests). (.ai/artifacts/t151_1_verify_log.md:38-40) | **PASS** — F-02 — vitest chain exact; fresh run 281/281 |
| 77 | T-151.1 | gate | T-151.1: WgpuTacticalMap builds into its own lazy chunk (WgpuTacticalMap-*.js, 10.7 kB); entry chunk has no raw-wasm reference. (.ai/artifacts/t151_1_verify_log.md:42-50) | **PASS** — C-1-01 — covered by slice verdict (automated surface green) |
| 78 | T-151.1 | class-R | T-151.1 Class R test grid_lines_everon_pinned: 52 vertices, BORDER@x=0, MAJOR@5000, MINOR@1000, exact color [173,198,255,α]; hs palette boosted alphas (.ai/artifacts/t151_1_verify_log.md:59-60) | **PASS** — C-1-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 79 | T-151.1 | class-R | T-151.1 Class R pickBaseLevel matrix: Everon index × limit {16384→0, 8192→1, 4096→2} plus {256→6, 1→13} (4 tests). (.ai/artifacts/t151_1_verify_log.md:61) | **PASS** — C-1-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 80 | T-151.1 | class-S | T-151.1 Class S basemapLod: 13 tests over ≥12 (viewState, viewBounds, mode) tuples → golden Lod + tileUrl south-first Y inversion. (.ai/artifacts/t151_1_verify_log.md:62) | **PASS** — C-1-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 81 | T-151.1 | class-T | T-151.1 Class T: Rust build_hillshade_image ≤1 gray level vs JS buildHillshadeImage (hillshade.parity.test.ts unchanged). (.ai/artifacts/t151_1_verify_log.md:63) | **PASS** — C-1-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 82 | T-151.1 | class-R | T-151.1 basemapResolve.ts extraction proven byte-identical to the Deck path: satelliteUnified/tileUrl/basemapView/styleModes tests all stay green and  (.ai/artifacts/t151_1_verify_log.md:65-67) | **PASS** — C-1-01 — covered by slice verdict (automated surface green) |
| 83 | T-151.1 | behavior | T-151.1 L7: grid drawn with PrimitiveTopology::LineList device-native 1 px screen-space lines matching Deck widthUnits:'pixels' getWidth:1 — no world- (.ai/artifacts/t151_1_verify_log.md:88-91) | **PASS** — C-1-01 — covered by slice verdict (automated surface green) |
| 84 | T-151.1 | gpu-probe | T-151.1 GPU-R texture_self_check EXECUTED PASS (headless SwiftShader WebGL2): NW(100,100)=[255,0,0,255] (north-up kill-shot), NE(700,100)=[0,255,0,255 (.ai/artifacts/t151_1_verify_log.md:114-118) | **PASS** — verify log §GPU — re-executed WebGL2, byte-exact (17/17 probes) |
| 85 | T-151.1 | gpu-probe | T-151.1 GPU-R: T-151.0 self_check calibration regression EXECUTED PASS — all 7 probes byte-exact incl. clear [51,68,85,255]; the render-loop refactor  (.ai/artifacts/t151_1_verify_log.md:118-121) | **PASS** — verify log §GPU — re-executed WebGL2, byte-exact (17/17 probes) |
| 86 | T-151.1 | gpu-probe | T-151.1 stress accounting EXECUTED PASS (WebGL2): seed_stress(1,000,000) → instances 1000000, chunks 1, gpu_bytes 32000160 (= 1,000,000·32 + 64+32+64) (.ai/artifacts/t151_1_verify_log.md:122-125) | **PASS** — verify log §GPU — re-executed WebGL2, byte-exact (17/17 probes) |
| 87 | T-151.1 | gpu-probe | T-151.1 live W1 draw_batches EXECUTED PASS (WebGL2): basemap_mode:'pyramid', basemap_tiles:1, basemap_bytes:16, uniform_bytes_last_frame:64, chunks:0. (.ai/artifacts/t151_1_verify_log.md:126-128) | **PASS** — verify log §GPU — re-executed WebGL2, byte-exact (17/17 probes) |
| 88 | T-151.1 | gpu-probe | T-151.1 hillshade end-to-end EXECUTED PASS on the real 71 MB DEM: basemap_bytes 3,341,584 = 914²·4 (Class-T MAX_EDGE-1024 downsample, 6400/scale7 = 91 (.ai/artifacts/t151_1_verify_log.md:129-134) | **PASS** — verify log §GPU — re-executed WebGL2, byte-exact (17/17 probes) |
| 89 | T-151.1 | gpu-probe | T-151.1 WebGPU copy_external_image_to_texture fast path EXECUTED PASS (lavapipe): backend webgpu, tex_layer_write_bitmap + commit + render succeed, ba (.ai/artifacts/t151_1_verify_log.md:135-137) | **NOT RE-VERIFIED** — verify log §GPU — WebGPU not reproducible in this env; T-151.1-era evidence only |
| 90 | T-151.1 | gate | T-151.1 remaining operator gates: S1 unified-satellite visual (OP), S2 map-style pyramid (asset-gated, tiles local-gitignored), S3 hillshade toggle (O (.ai/artifacts/t151_1_verify_log.md:139-153) | **PASS** — verify log §GPU — re-executed WebGL2, byte-exact (17/17 probes) |
| 91 | T-151.1 | gate | T-151.1 summary: automated verify PASS 11/11 gates exit 0; 334 vitest; 75 cargo tests; 0 clippy/lint warnings. (.ai/artifacts/t151_1_verify_log.md:157-158) | **PASS** — F-02 — vitest chain exact; fresh run 281/281 |
| 92 | T-151.1 | count | T-151.1 spec pinned: TBDS magic 0x53444254; Everon unified base 12800² / 14 mips / 152,713,114 B; Everon bounds [0,0,12800,12800]; vitest baseline 317 (docs/specs/Mission_Creator_Architecture/t151_1_basemap_lane.md:71-82) | **PASS** — F-02 — vitest chain exact; fresh run 281/281 |
| 93 | T-151.1 | behavior | T-151.1 spec pinned hillshade constants: MAX_EDGE 1024; Horn light azimuth 315°, altitude 45°; opacity default 0.4 with 0–100% slider at 0.1% steps. (docs/specs/Mission_Creator_Architecture/t151_1_basemap_lane.md:76-78) | **PASS** — C-1-01 — covered by slice verdict (automated surface green) |
| 94 | T-151.1 | behavior | T-151.1 spec pinned tile/grid constants: pyramid zoom 0–6, TILE_PX 256, MAX_VISIBLE_BASEMAP_TILES 64; grid step 1000 m with major every 5000 m; paper  (docs/specs/Mission_Creator_Architecture/t151_1_basemap_lane.md:74-80) | **PASS** — C-1-01 — covered by slice verdict (automated surface green) |
| 95 | T-151.2 | count | T-151.2: cargo test map-engine-core = 85 lib + 5 camera_props + 5 deckgl_ortho_parity, 0 failed. (.ai/artifacts/t151_2_verify_log.md:19) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 96 | T-151.2 | count | T-151.2: cargo test map-engine-render = 9 passed. (.ai/artifacts/t151_2_verify_log.md:20) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 97 | T-151.2 | count | T-151.2: full vitest = 42 files / 343 passed (334 baseline + 9 new world.parity), 0 failed. (.ai/artifacts/t151_2_verify_log.md:23) | **PASS** — F-02 — vitest chain exact; fresh run 281/281 |
| 98 | T-151.2 | count | T-151.2 census assert: prefab table = 391 (WorldStore.load_prefabs_gz count == JS buildPrefabMaps().byId.size == 391). (.ai/artifacts/t151_2_verify_log.md:39) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 99 | T-151.2 | count | T-151.2 census assert: Σ per-chunk parsed count over all 275 chunks == 508,291 (declared strong cross-check: no rows lost or double-counted). (.ai/artifacts/t151_2_verify_log.md:40 and 54-56) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 100 | T-151.2 | count | T-151.2 census asserts: chunk files 275 (readdir), road segments kept 888, forest regions kept 36, TBDD grids 625 (readdir + decode smoke on 3). (.ai/artifacts/t151_2_verify_log.md:41-44) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 101 | T-151.2 | gate | T-151.2: WorldStore.stats() returns exactly {prefab_count:391, instance_count_total:508291, chunk_count_loaded:275, road_segment_count:888, forest_reg (.ai/artifacts/t151_2_verify_log.md:45) | **PASS** — C-2-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 102 | T-151.2 | class-R | T-151.2 Class R proven on all 275 chunks: positions/rotations/z via f32BytesEqual; prefab_idx/cls_codes via intArrayEqual, oracle arrays sliced to cou (.ai/artifacts/t151_2_verify_log.md:48-50) | **PASS** — C-2-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 103 | T-151.2 | class-S | T-151.2 Class S proven on all 275 chunks: each render-class rowsByClass[code] == chunk_rows_for_class(code). (.ai/artifacts/t151_2_verify_log.md:51) | **PASS** — C-2-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 104 | T-151.2 | class-T | T-151.2 Class T ≤1 ULP: wasm.obb_corners vs TS obbCorners on 5 pinned cases; wasm.road_centerline vs TS extractRoadCenterline (width + every vertex) o (.ai/artifacts/t151_2_verify_log.md:51-53) | **PASS** — C-2-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 105 | T-151.2 | count | T-151.2 micro-decision: pid as u16 valid because Everon pids ∈ [0, 390] < 65536 (identical to JS Uint16Array ToUint16). (.ai/artifacts/t151_2_verify_log.md:63) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 106 | T-151.2 | behavior | T-151.2 micro-decision: serde_json/float_roundtrip is load-bearing — default float parse (~1 ULP off) would diverge the f64→as f32 positions store and (.ai/artifacts/t151_2_verify_log.md:64-66) | **PASS** — C-2-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 107 | T-151.2 | bytes | T-151.2 wasm size: 3,858,591 B (baseline 3,723,192; Δ +135,399 = flate2 gunzip + float_roundtrip parser + world module + WorldStore bindings + 2 ULP f (.ai/artifacts/t151_2_verify_log.md:78-83) | **PASS** — F-01 — byte chain exact, every delta re-computed |
| 108 | T-151.2 | behavior | T-151.2: RenderEngine::stats() 12 fields untouched; WorldStore.stats() is a separate additive handle; no prior wasm export renamed/removed. (.ai/artifacts/t151_2_verify_log.md:85-87) | **PASS** — C-2-01 — covered by slice verdict (automated surface green) |
| 109 | T-151.2 | gate | T-151.2 S1: world.parity 275-chunk sweep cold wall-clock 1.13 s (/usr/bin/time); vitest-reported 362 ms for the 9 tests — far under the <120 s target. (.ai/artifacts/t151_2_verify_log.md:93-95) | **PASS** — F-02 — vitest chain exact; fresh run 281/281 |
| 110 | T-151.2 | count | T-151.2 S2 spot-check chunk 10_10: oracleCount 499 == wasmCount 499; positions len 998 == 2·499 both sides; class distribution 449 tree + 50 building  (.ai/artifacts/t151_2_verify_log.md:96-99) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 111 | T-151.2 | behavior | T-151.2 S3: parse-only slice — worker parseChunk closure delegates to exported parseChunkOracle behavior-identically; Deck path untouched; GPU headles (.ai/artifacts/t151_2_verify_log.md:100-110) | **PASS** — verify log §GPU — re-executed WebGL2, byte-exact (17/17 probes) |
| 112 | T-151.2 | behavior | T-151.2 spec pinned: chunk size 512 m; 5 instance render classes; NO_CLASS sentinel 255; oversized half-extent gate 64 m; CENTERLINE_DEDUPE_M 0.05; ro (docs/specs/Mission_Creator_Architecture/t151_2_world_parser.md:62 and 80-83) | **PASS** — C-2-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 113 | T-151.3 | count | T-151.3: cargo test map-engine-core = 101 unit + 5 camera_props + 5 deckgl_ortho_parity PASS. (.ai/artifacts/t151_3_verify_log.md:21) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 114 | T-151.3 | count | T-151.3: cargo test map-engine-render = 10 PASS incl. building_instance_layout_and_bytes_exact. (.ai/artifacts/t151_3_verify_log.md:22) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 115 | T-151.3 | bytes | T-151.3: merged wasm = 3,946,734 B (baseline 3,858,591; +88,143). (.ai/artifacts/t151_3_verify_log.md:24) | **PASS** — F-01 — byte chain exact, every delta re-computed |
| 116 | T-151.3 | count | T-151.3: vitest = 371 PASS (baseline 343; +28 W3). (.ai/artifacts/t151_3_verify_log.md:25) | **PASS** — F-02 — vitest chain exact; fresh run 281/281 |
| 117 | T-151.3 | count | T-151.3 new vitest counts: chunkMathRust.parity = 24 (12 bboxes × 2 rings); world.pick.parity = 2; world.residency.parity = 2. (.ai/artifacts/t151_3_verify_log.md:31-33) | **PASS** — F-02 — vitest chain exact; fresh run 281/281 |
| 118 | T-151.3 | class-R | T-151.3 P1 PASS (Class R): Rust chunk_ids_for_viewport == JS chunkIdsForViewport via native pins + ordered-array equality. (.ai/artifacts/t151_3_verify_log.md:41) | **PASS** — C-3-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 119 | T-151.3 | class-S | T-151.3 P2 PASS (Class S): residency requests the same chunk ids as chunkStore at every step of a 22-step script. (.ai/artifacts/t151_3_verify_log.md:42) | **PASS** — C-3-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 120 | T-151.3 | class-S | T-151.3 P3 PASS (Class S): eviction identical including order — completeness theorem over cap-crossing script + end-revisits; native ascending-last_us (.ai/artifacts/t151_3_verify_log.md:43) | **PASS** — C-3-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 121 | T-151.3 | class-R | T-151.3 P4 PASS (Class R): LRU cap max(64, 3×pinned) with pinned never evicted (native residency.rs test). (.ai/artifacts/t151_3_verify_log.md:44) | **PASS** — C-3-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 122 | T-151.3 | class-R | T-151.3 P5 PASS (Class R): budget accounting via native fake-elapsed sequence [1,5,3,6,4] → frames_over_budget=2, max_apply_ms=6. (.ai/artifacts/t151_3_verify_log.md:45) | **PASS** — C-3-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 123 | T-151.3 | class-R | T-151.3 P6 PASS (Class R): pinned_building_count == getWorldBuildings().length at every residency-parity step. (.ai/artifacts/t151_3_verify_log.md:46) | **PASS** — C-3-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 124 | T-151.3 | class-S | T-151.3 P7 PASS (Class S): pick_rect == rbush pickRect over 10k probes, sorted id-set equality. (.ai/artifacts/t151_3_verify_log.md:47) | **PASS** — C-3-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 125 | T-151.3 | class-S | T-151.3 P8 PASS (Class S): pick_nearest == rbush pickNearest over 10k probes with 0 exact-distance ties for the fixed seed (checked, not assumed) → ev (.ai/artifacts/t151_3_verify_log.md:48) | **PASS** — C-3-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 126 | T-151.3 | class-T | T-151.3 P9 PASS (Class T, delegated): building OBB corners ≤1 ULP via world.parity.test.ts:128 (reused, not re-derived). (.ai/artifacts/t151_3_verify_log.md:49) | **PASS** — C-3-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 127 | T-151.3 | bytes | T-151.3 P12 PASS (Class R): size_of asserts BuildingInstance==40, QuadInstance==32, LineVertex==24 plus a pinned-bytes test. (.ai/artifacts/t151_3_verify_log.md:52) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 128 | T-151.3 | gpu-probe | T-151.3 GPU-R world_building_self_check EXECUTED PASS headless: center (400,300)=[38,38,44,255] FILL_DEFAULT byte-exact; exterior (460,300)=[51,68,85, (.ai/artifacts/t151_3_verify_log.md:64-74) | **PASS** — verify log §GPU — re-executed WebGL2, byte-exact (17/17 probes) |
| 129 | T-151.3 | gpu-probe | T-151.3 GPU-R regressions PASS: T-151.0 self_check all 7 probes and T-151.1 texture_self_check 3 probes byte-exact after the draw_batches/lane changes (.ai/artifacts/t151_3_verify_log.md:76-80) | **PASS** — verify log §GPU — re-executed WebGL2, byte-exact (17/17 probes) |
| 130 | T-151.3 | behavior | T-151.3 colour divergence logged: wgpu building outline [30,30,34,255] (operator decision) vs Deck oracle STROKE [150,150,158,204] (buildingLayer.ts:1 (.ai/artifacts/t151_3_verify_log.md:87-92) | **OPEN** — C-ALL-01 — operator gate never recorded closed (see C-3-01) |
| 131 | T-151.3 | gate | T-151.3 manual: S1 (buildings visible @ deckZoom ≥ −2.5), S2 (Deck path unchanged), S3 (HUD world_building_instances > 0, frames_over_budget = 0 on sc (.ai/artifacts/t151_3_verify_log.md:96-107) | **PASS** — verify log §GPU — re-executed WebGL2, byte-exact (17/17 probes) |
| 132 | T-151.3 | behavior | T-151.3 spec pinned: LRU floor 64; formula max(64, 3×pinned); APPLY_BUDGET_MS 4.0; fetch concurrency 12; preload margin max(5% viewport span, 512 m);  (docs/specs/Mission_Creator_Architecture/t151_3_world_residency.md:80-89) | **PASS** — C-3-01 — covered by slice verdict (automated surface green) |
| 133 | T-151.4 | count | T-151.4: cargo test map-engine-core = 114 lib PASS (+ geometry triangulate/polyline_strip/vector_compose). (.ai/artifacts/t151_4_verify_log.md:21) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 134 | T-151.4 | count | T-151.4: cargo test map-engine-render = 10 PASS. (.ai/artifacts/t151_4_verify_log.md:22) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 135 | T-151.4 | bytes | T-151.4: merged wasm = 4,005,415 B (baseline 3,946,734; +58,681). (.ai/artifacts/t151_4_verify_log.md:24) | **PASS** — F-01 — byte chain exact, every delta re-computed |
| 136 | T-151.4 | count | T-151.4: vitest = 371 PASS (parity suites unchanged green; no new vitest beyond L10). (.ai/artifacts/t151_4_verify_log.md:25) | **PASS** — F-02 — vitest chain exact; fresh run 281/281 |
| 137 | T-151.4 | gate | T-151.4 native gates: triangulation area conservation (unit square, closed ring, triangle, hole — earcutr); polyline_strip midpoint projection ±1e-6,  (.ai/artifacts/t151_4_verify_log.md:30-34) | **PASS** — C-4-01 — covered by slice verdict (automated surface green) |
| 138 | T-151.4 | gate | T-151.4 L8 triangulation area conservation gate PASS and L9 polyline width midpoint widthM·2^zoom ±1e-6 gate PASS. (.ai/artifacts/t151_4_verify_log.md:56-57) | **PASS** — C-4-01 — covered by slice verdict (automated surface green) |
| 139 | T-151.4 | behavior | T-151.4 L3 draw order shipped via lane_order: basemap → sea → hillshade → landcover → contours → roads → buildings → forest → grid → marquee. (.ai/artifacts/t151_4_verify_log.md:51) | **PASS** — C-4-01 — covered by slice verdict (automated surface green) |
| 140 | T-151.4 | behavior | T-151.4 stats() additive keys appended after T-151.3 fields: sea_polygons, landcover_polygons, contour_segments, road_segments, forest_polygons, fores (.ai/artifacts/t151_4_verify_log.md:66-73) | **PASS** — C-4-01 — covered by slice verdict (automated surface green) |
| 141 | T-151.4 | gpu-probe | T-151.4 GPU-R sea_band_self_check + road_centerline_self_check: expected byte-exact JSON documented (sea [72,118,160,255]; road [200,200,200,255]) but (.ai/artifacts/t151_4_verify_log.md:82-110) | **PASS** — verify log §GPU — re-executed WebGL2, byte-exact (17/17 probes) |
| 142 | T-151.4 | gate | T-151.4 manual S1 (full stack @ zoom −2), S2 (Deck unchanged), S3 (per-layer screenshot diff @ 3 cameras, advisory ±3/ch), S4 (sea+road readback JSON) (.ai/artifacts/t151_4_verify_log.md:114-122) | **PASS** — verify log §GPU — re-executed WebGL2, byte-exact (17/17 probes) |
| 143 | T-151.4 | behavior | T-151.4 spec pinned: contour ladder 100/50/20/10 m; 6 road classes; DEM vector downsample factor 4; sea fill max zoom +3; forest fill max zoom +1. (docs/specs/Mission_Creator_Architecture/t151_4_vector_layers.md:79-89) | **PASS** — C-4-01 — covered by slice verdict (automated surface green) |
| 144 | T-151.4.1 | behavior | T-151.4.1 root cause A: empty pushToEngine wiped the GPU building lane — upload_world_buildings([]) → remove_lane(WorldBuildings) after a debounced se (.ai/artifacts/t151_4_1_verify_log.md:14-26) | **PASS** — C-4.1-01 — covered by slice verdict (automated surface green) |
| 145 | T-151.4.1 | behavior | T-151.4.1 root cause B: aborted fetch left ids stuck in inflight (never re-requested due to !inflight filter) → permanent empty fill for the pin set. (.ai/artifacts/t151_4_1_verify_log.md:27-30) | **PASS** — C-4.1-01 — covered by slice verdict (automated surface green) |
| 146 | T-151.4.1 | gate | T-151.4.1 fix: residency.rs gains clear_inflight/mark_inflight/pin_settled/inflight_count; loader skips empty upload while inflight/pending/unsettled; (.ai/artifacts/t151_4_1_verify_log.md:32-37) | **PASS** — C-4.1-01 — covered by slice verdict (automated surface green) |
| 147 | T-151.4.1 | behavior | T-151.4.1 roads: expand_polyline_strip builds miter joins (bevel at miter limit 4× half-width) and round end caps (8-seg semicircle), matching Deck ca (.ai/artifacts/t151_4_1_verify_log.md:41-47) | **PASS** — C-4.1-01 — covered by slice verdict (automated surface green) |
| 148 | T-151.4.1 | bytes | T-151.4.1: wasm = 4,009,368 B (T-151.4 was 4,005,415; +3,953). (.ai/artifacts/t151_4_1_verify_log.md:67) | **PASS** — F-01 — byte chain exact, every delta re-computed |
| 149 | T-151.4.1 | count | T-151.4.1: vitest = 371 PASS (world.parity / residency.parity / pick green). (.ai/artifacts/t151_4_1_verify_log.md:68) | **PASS** — F-02 — vitest chain exact; fresh run 281/281 |
| 150 | T-151.4.1 | gate | T-151.4.1 operator paste for S1/S4 (__wgpuWorldStats with building_instances ≥ 8, pin_settled true) recorded as 'pending browser settle' — not filled  (.ai/artifacts/t151_4_1_verify_log.md:74-97) | **OPEN** — C-ALL-01 — operator gate never recorded closed (see C-4.1-01) |
| 151 | T-151.4.1 | behavior | T-151.4.1 before/after: T-151.4 broken state drew 0 town-cluster buildings (lane wiped); T-151.4.1 restores ~8–9 with sticky empty-mid-flight semantic (.ai/artifacts/t151_4_1_verify_log.md:101-107) | **PASS** — C-4.1-01 — covered by slice verdict (automated surface green) |
| 152 | T-151.4.1 | docs | T-151.4.1 P2 forest explicitly unchanged: TBDD iso=1 + Path B mega hulls kept (Deck parity); overdraw declared a shared T-090.8.1/N11 policy issue def (.ai/artifacts/t151_4_1_verify_log.md:50-53) | **PASS** — F-03 — SHA/link/status claims reconciled against git (see §F) |
| 153 | T-151.5 | bytes | T-151.5 IconInstance layout exact 20 B: pos f32×2=8, size f32=4, yaw i16 snorm=2 (angle_deg/180 × 32767), glyph u16=2 (index into 28-entry UV table),  (.ai/artifacts/t151_5_verify_log.md:9-18) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 154 | T-151.5 | behavior | T-151.5 draw order: forest-outline → trees → props → badges → grid. (.ai/artifacts/t151_5_verify_log.md:19) | **PASS** — C-5-01 — covered by slice verdict (automated surface green) |
| 155 | T-151.5 | count | T-151.5: atlas world-glyphs.webp + JSON uploaded via upload_glyph_atlas with 28 UV rects asserted; GPU-R tree_glyph_self_check API (solid white atlas  (.ai/artifacts/t151_5_verify_log.md:27-32) | **OPEN** — C-5-02 — gate never wired/executed |
| 156 | T-151.5 | count | T-151.5: cargo test map-engine-render = 11 PASS (incl. icon 20 B layout). (.ai/artifacts/t151_5_verify_log.md:44) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 157 | T-151.5 | bytes | T-151.5: wasm = 4,054,850 B (T-151.4.1 was 4,009,368; +45,482). (.ai/artifacts/t151_5_verify_log.md:46) | **PASS** — F-01 — byte chain exact, every delta re-computed |
| 158 | T-151.5 | count | T-151.5: vitest = 372 PASS (+1 exhaustive LOD parity; was 371). (.ai/artifacts/t151_5_verify_log.md:47) | **PASS** — F-02 — vitest chain exact; fresh run 281/281 |
| 159 | T-151.5 | class-R | T-151.5 LOD scan (glyphLod.parity.test.ts): 16 classes × 121 zooms (−6.0…+6.0 @ 0.1) — Rust class_visible == TS classVisible exact. (.ai/artifacts/t151_5_verify_log.md:51-53) | **PASS** — C-5-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 160 | T-151.5 | gate | T-151.5 manual S1–S4 (glyphs over forest @ zoom ≥ 0; hidden < 0; Deck advisory compare; tree_glyph_self_check JSON) all marked operator — including th (.ai/artifacts/t151_5_verify_log.md:57-64) | **OPEN** — C-5-02 — gate never wired/executed |
| 161 | T-151.5 | docs | T-151.5 forest note: mass / Path B hulls unchanged this slice; glyphs are the instrument for overdraw judgment; retune is a follow-up. (.ai/artifacts/t151_5_verify_log.md:70-72) | **PASS** — F-03 — SHA/link/status claims reconciled against git (see §F) |
| 162 | T-151.5 | behavior | T-151.5 spec pinned: 28 atlas glyphs; 51 tree types; ~501,861 Everon tree instances; REF_ZOOM 3; TREE_GLYPH_MIN_ZOOM 0; INSTANCE_BUDGET 150,000; LOD g (docs/specs/Mission_Creator_Architecture/t151_5_glyph_atlas.md:36-37 and 59 and 71-80) | **PASS** — C-5-01 — covered by slice verdict (automated surface green) |
| 163 | T-151.5.1 | behavior | T-151.5.1: DENSITY_ISO raised 1→2 in Rust forest_mass.rs as source of truth; density_iso() wasm export added so wgpu never feeds a TS iso. (.ai/artifacts/t151_5_1_verify_log.md:4 and 12-14) | **PASS** — C-5.1-01 — covered by slice verdict (automated surface green) |
| 164 | T-151.5.1 | behavior | T-151.5.1 LOD policy: forestFill visible only at zoom < 0; forestOutline only in [−1.5, 0); landcover off (refreshed, not sticky) at zoom ≥ 0; tree gl (.ai/artifacts/t151_5_1_verify_log.md:14 and 57-66) | **PASS** — C-5.1-01 — covered by slice verdict (automated surface green) |
| 165 | T-151.5.1 | count | T-151.5.1: cargo test map-engine-core = 127 lib (+ camera/ortho suites); map-engine-render = 11. (.ai/artifacts/t151_5_1_verify_log.md:30-31) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 166 | T-151.5.1 | bytes | T-151.5.1: wasm = 4,055,075 B (T-151.5 was 4,054,850; +225). (.ai/artifacts/t151_5_1_verify_log.md:33) | **PASS** — F-01 — byte chain exact, every delta re-computed |
| 167 | T-151.5.1 | count | T-151.5.1: vitest = 374 PASS (≥ 372 baseline; +LOD/iso cases). (.ai/artifacts/t151_5_1_verify_log.md:34) | **PASS** — F-02 — vitest chain exact; fresh run 281/281 |
| 168 | T-151.5.1 | docs | T-151.5.1 residuals documented as deferrals: mega-region forest-everon-001 (~479k trees) Path B hull still large at coarse zoom (export split); 32 m T (.ai/artifacts/t151_5_1_verify_log.md:49-54) | **PASS** — F-03 — SHA/link/status claims reconciled against git (see §F) |
| 169 | T-151.6 | bytes | T-151.6: slot lane keeps the 20 B IconInstance layout; slot size is CSS pixels multiplied by px_to_m in shader; world glyphs keep meters (px_to_m=1). (.ai/artifacts/t151_6_verify_log.md:8-20) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 170 | T-151.6 | bytes | T-151.6: IconUniforms = 464 B (UV[28] + drag_delta vec2 + px_to_m f32 + pad); slots use px_to_m = 2^(−zoom). (.ai/artifacts/t151_6_verify_log.md:19-20) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 171 | T-151.6 | behavior | T-151.6 draw order: badges → slots → slot-drag → clusters → grid → marquee. (.ai/artifacts/t151_6_verify_log.md:22) | **PASS** — C-6-01 — covered by slice verdict (automated surface green) |
| 172 | T-151.6 | behavior | T-151.6: slot/cluster art is a dedicated procedural ring+disc atlas (slotAtlas.ts → upload_slot_atlas), not the 28-key world-glyphs atlas. (.ai/artifacts/t151_6_verify_log.md:30) | **PASS** — C-6-01 — covered by slice verdict (automated surface green) |
| 173 | T-151.6 | behavior | T-151.6 SoT: GPU slot positions come from MissionDoc.refresh() → slot_xy_ptr/slot_len — never from Zustand slotsById. (.ai/artifacts/t151_6_verify_log.md:36) | **PASS** — C-6-01 — covered by slice verdict (automated surface green) |
| 174 | T-151.6 | gate | T-151.6 drag contract: T-061 hide base (α=0) + SlotDrag overlay + 16 B delta uniform; uniform_bytes_last_frame = 80 while a drag is live. (.ai/artifacts/t151_6_verify_log.md:38) | **PASS** — C-6-01 — covered by slice verdict (automated surface green) |
| 175 | T-151.6 | behavior | T-151.6 cluster gate: slot_len > 500 && zoom ≤ −4; discs from getClusterMarkers / Rust ClusterIndex. (.ai/artifacts/t151_6_verify_log.md:39) | **PASS** — C-6-01 — covered by slice verdict (automated surface green) |
| 176 | T-151.6 | count | T-151.6: cargo core PASS incl. slots_gpu ×7; render = 11. (.ai/artifacts/t151_6_verify_log.md:51-53) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 177 | T-151.6 | bytes | T-151.6: wasm = 4,063,618 B (T-151.5.1 was 4,055,075; +8,543). (.ai/artifacts/t151_6_verify_log.md:54) | **PASS** — F-01 — byte chain exact, every delta re-computed |
| 178 | T-151.6 | count | T-151.6: vitest = 379 PASS (+5 slot pack/gate tests; was 374). (.ai/artifacts/t151_6_verify_log.md:55) | **PASS** — F-02 — vitest chain exact; fresh run 281/281 |
| 179 | T-151.6 | gate | T-151.6 unit gates: instance pack count == xy.len()/2; selection size 28 + yellow tint bytes; drag math base+(dx,dy); cluster_mode truth table vs cons (.ai/artifacts/t151_6_verify_log.md:59-65) | **PASS** — C-6-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 180 | T-151.6 | gate | T-151.6 manual S1–S5 (rings visible/count, scripted selection tint, scripted drag with uniform 80, cluster discs, undo slot_instances == slot_len) all (.ai/artifacts/t151_6_verify_log.md:69-80) | **OPEN** — C-ALL-01 — operator gate never recorded closed (see C-6-01) |
| 181 | T-151.6 | behavior | T-151.6 spec pinned: slot ring 20 px base / 28 px selected; colors primary [173,198,255] / selected [250,204,21]; ZOOM_CLUSTER_MAX −4; CLUSTER_SLOT_TH (docs/specs/Mission_Creator_Architecture/t151_6_mission_entities.md:71-77) | **PASS** — C-6-01 — covered by slice verdict (automated surface green) |
| 182 | T-151.7 | behavior | T-151.7: RenderEngine.unproject_xy (screen CSS px → world m) exposed via OrthoCamera; wgpu path removes raw LMB pan and hosts useSelectTool + wheel zo (.ai/artifacts/t151_7_verify_log.md:10-18) | **PASS** — C-7-01 — covered by slice verdict (automated surface green) |
| 183 | T-151.7 | bytes | T-151.7: wasm = 4,063,911 B (T-151.6 was 4,063,618; +293). (.ai/artifacts/t151_7_verify_log.md:34) | **PASS** — F-01 — byte chain exact, every delta re-computed |
| 184 | T-151.7 | count | T-151.7: vitest = 391 PASS (+12 interaction parity tests; was 379). (.ai/artifacts/t151_7_verify_log.md:35) | **PASS** — F-02 — vitest chain exact; fresh run 281/281 |
| 185 | T-151.7 | class-R | T-151.7 parity suite: camera unproject Class R vs Deck/OrthoCameraJs at integer zooms across sizes/targets. (.ai/artifacts/t151_7_verify_log.md:41) | **PASS** — C-7-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 186 | T-151.7 | class-R | T-151.7 parity suite: pick radius 4 px world scale Class R — r_world = 16 m @ zoom −2. (.ai/artifacts/t151_7_verify_log.md:42) | **PASS** — C-7-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 187 | T-151.7 | class-S | T-151.7 parity suite: selection scripts (click / Ctrl toggle / empty clear, T-053) and marquee pickRect Class S. (.ai/artifacts/t151_7_verify_log.md:43-44) | **PASS** — C-7-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 188 | T-151.7 | class-R | T-151.7 parity suite: encode_state stable re-encode Class R; cross-doc move positions Class R; cluster 48 px world radius @ zoom −4; CUR z == DEM samp (.ai/artifacts/t151_7_verify_log.md:45-47) | **PASS** — C-7-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 189 | T-151.7 | gate | T-151.7 manual: S1–S5 (click/toggle, marquee+delete+undo, drag no-pan-steal, Space/drop/dbl-click, CUR XYZ + drill) operator; S6 (vitest interaction p (.ai/artifacts/t151_7_verify_log.md:53-60) | **PASS** — F-02 — vitest chain exact; fresh run 281/281 |
| 190 | T-151.7 | behavior | T-151.7 spec pinned: slot pick radius 4 px; cluster pick 48 px; gesture threshold 4 px; ZOOM_CLUSTER_MAX −4 unchanged. (docs/specs/Mission_Creator_Architecture/t151_7_interaction_rewire.md:74-79) | **PASS** — C-7-01 — covered by slice verdict (automated surface green) |
| 191 | T-151.7.1 | behavior | T-151.7.1 root causes locked: B1 cluster short-lane patched by full-doc index → silent OOB no-op in patch_slot_lane; B2 every drag delta ran full sync (.ai/artifacts/t151_7_1_verify_log.md:8-14) | **PASS** — C-7.1-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 192 | T-151.7.1 | bytes | T-151.7.1: make wasm not required — no engine/wasm surface change; baseline stays 4,063,911 B. (.ai/artifacts/t151_7_1_verify_log.md:40) | **PASS** — F-01 — byte chain exact, every delta re-computed |
| 193 | T-151.7.1 | count | T-151.7.1: vitest = 392 PASS (+1 classifyDragTransition phase test; was 391). (.ai/artifacts/t151_7_1_verify_log.md:41) | **PASS** — F-02 — vitest chain exact; fresh run 281/281 |
| 194 | T-151.7.1 | gate | T-151.7.1 manual S1–S3 (tint always matches, drag ~1000 FPS drop ≪ 40 with uniform ~80, RMB+wheel anchored) all operator; dev surface adds slots_lane_ (.ai/artifacts/t151_7_1_verify_log.md:52-59) | **OPEN** — C-ALL-01 — operator gate never recorded closed (see C-7.1-01) |
| 195 | T-151.7.1 | bytes | T-151.7.1 spec pinned: drag delta uniform 16 B; uniform_bytes_last_frame 80 (= 64+16) while dragging. (docs/specs/Mission_Creator_Architecture/t151_7_1_interaction_hotfix.md:60-64) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 196 | T-151.7.2 | behavior | T-151.7.2 fixes: selection = full re-pack from SoA + selectedMask on every change (no per-row patch); camera SoT = engine (wheel syncs viewStateRef, p (.ai/artifacts/t151_7_2_verify_log.md:8-25) | **PASS** — C-7.2-01 — covered by slice verdict (automated surface green) |
| 197 | T-151.7.2 | count | T-151.7.2: vitest = 393 PASS (+1 pan zoom merge contract test); wasm unchanged (no engine surface). (.ai/artifacts/t151_7_2_verify_log.md:33-37) | **PASS** — F-02 — vitest chain exact; fresh run 281/281 |
| 198 | T-151.7.2 | gate | T-151.7.2 manual S1–S3 (yellow+SEL without zoom, rapid toggle, wheel anchored) all operator. (.ai/artifacts/t151_7_2_verify_log.md:42-47) | **OPEN** — C-ALL-01 — operator gate never recorded closed (see C-7.2-01) |
| 199 | T-151.7.3 | loc | T-151.7.3 LOC proof: wgpuSlots.ts 521 → 56 LOC (≤ 60), with verbatim 'wc -l … 56' output; slotAtlas.ts reduced from 154 (pack+atlas) to canvas atlas o (.ai/artifacts/t151_7_3_verify_log.md:8-18) | **PASS** — C-7.3-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 200 | T-151.7.3 | behavior | T-151.7.3: no TS reimplementation of pack_slot_instances / classifyDragTransition / cluster_mode remains in wgpuSlots.ts. (.ai/artifacts/t151_7_3_verify_log.md:20) | **PASS** — C-7.3-01 — covered by slice verdict (automated surface green) |
| 201 | T-151.7.3 | behavior | T-151.7.3 public wasm slot surface: ensure_slot_atlas, set_selection, set_drag, on_camera_changed, set_cluster_markers, cluster_mode, slot_stats_json, (.ai/artifacts/t151_7_3_verify_log.md:27-39) | **PASS** — C-7.3-01 — covered by slice verdict (automated surface green) |
| 202 | T-151.7.3 | count | T-151.7.3: cargo core PASS incl. slots_gpu 11; render = 11. (.ai/artifacts/t151_7_3_verify_log.md:56-57) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 203 | T-151.7.3 | bytes | T-151.7.3: wasm = 4,071,877 B (was ~4,063,911). (.ai/artifacts/t151_7_3_verify_log.md:59) | **PASS** — F-01 — byte chain exact, every delta re-computed |
| 204 | T-151.7.3 | count | T-151.7.3: vitest = 393 PASS. (.ai/artifacts/t151_7_3_verify_log.md:60) | **PASS** — F-02 — vitest chain exact; fresh run 281/281 |
| 205 | T-151.7.3 | behavior | T-151.7.3 behaviors preserved from 7.1/7.2: selection full-rematerialize (no OOB patch into short lanes), drag start/restart one overlay upload with p (.ai/artifacts/t151_7_3_verify_log.md:41-46) | **PASS** — C-7.3-01 — covered by slice verdict (automated surface green) |
| 206 | T-151.7.3 | gate | T-151.7.3 manual: S4 (wgpuSlots ≤ 60, no TS pack policy) automated PASS; S1–S3 operator. (.ai/artifacts/t151_7_3_verify_log.md:66-75) | **OPEN** — C-ALL-01 — operator gate never recorded closed (see C-7.3-01) |
| 207 | T-151.7.3 | docs | T-151.7.3 out of scope locked: supercluster stays FE-fed via set_cluster_markers; useSelectTool gesture SM stays TS. (.ai/artifacts/t151_7_3_verify_log.md:79-84) | **PASS** — F-03 — SHA/link/status claims reconciled against git (see §F) |
| 208 | T-151.8 | behavior | T-151.8 scope: CPU draw-set cull (Class S), exact-count density ladder + heatmap (Class R), damage-driven render skip (Class R), WebGPU compute cull S (.ai/artifacts/t151_8_verify_log.md:9-14) | **PASS** — C-8-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 209 | T-151.8 | behavior | T-151.8: DRAW_CULL_MARGIN_M = 0 — draw cull is strict visible rect; hub's '+ margin' is satisfied by residency preload for fetch; glyph/heatmap compos (.ai/artifacts/t151_8_verify_log.md:16) | **PASS** — C-8-01 — covered by slice verdict (automated surface green) |
| 210 | T-151.8 | docs | T-151.8 declared deferrals: props/badges get no heatmap ladder this slice; TBDD visual heatmap deferred (count-grid satisfies hub Class R since TBDD c (.ai/artifacts/t151_8_verify_log.md:19-20) | **PASS** — F-03 — SHA/link/status claims reconciled against git (see §F) |
| 211 | T-151.8.1 | class-R | T-151.8.1 compute cull: CPU AABB oracle compute_cull::{count,compact}_icons_* Class R on both backends; cs_icon_cull WGSL with VERTEX\|STORAGE compact (.ai/artifacts/t151_8_verify_log.md:25-38) | **PASS** — C-8-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 212 | T-151.8.1 | class-R | T-151.8.1 Class R gate: 1k random frusta — GPU compact count == count-only oracle with order preserved; stats gains compute_cull, compute_cull_cpu_cou (.ai/artifacts/t151_8_verify_log.md:34-35) | **PASS** — C-8-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 213 | T-151.8 | class-S | T-151.8 Class S draw-set gates PASS: draw_chunk_ids(strict) == chunk_ids_for_rect(chunk_rect_for_bbox(strict)) ∩ pinned ∩ cells sorted; every draw id  (.ai/artifacts/t151_8_verify_log.md:44-49) | **PASS** — C-8-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 214 | T-151.8 | class-R | T-151.8 Class R density ladder PASS: R1 exact_tree_count == hand-sum of row lens; R2 heatmap false @ 150000 and true @ 150001; R3 Σ density texels ove (.ai/artifacts/t151_8_verify_log.md:53-61) | **PASS** — C-8-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 215 | T-151.8 | class-R | T-151.8 Class R damage-driven render PASS: dirty→submit; clean second frame skips submit; pan marks dirty; continuous always submits; engine early-out (.ai/artifacts/t151_8_verify_log.md:65-74) | **PASS** — C-8-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 216 | T-151.8 | gate | T-151.8 LANGUAGE GATE: wgpuSlots.ts = 56 LOC (≤ 60); new cull/ladder math in TS = 0 (wasm getters + upload_density_grid only); vitest 393/393; entry i (.ai/artifacts/t151_8_verify_log.md:78-86) | **PARTIAL** — F-04 — W8 exact bytes never recorded; audit pins 4,123,261 B |
| 217 | T-151.8 | gate | T-151.8 band table S4 (chunks_draw / tree_glyph_count / heatmap / gpu_frame_ms / submitted_idle at zooms −6/−2/0/+2) is left as 'operator fill after h (.ai/artifacts/t151_8_verify_log.md:90-101) | **OPEN** — C-ALL-01 — operator gate never recorded closed (see C-8-01) |
| 218 | T-151.8 | gate | T-151.8 manual S1–S4 (draw-set tracks viewport; over-budget heatmap swap; idle submit=false; band table) recorded as post-ship operator items. (.ai/artifacts/t151_8_verify_log.md:105-110) | **OPEN** — C-ALL-01 — operator gate never recorded closed (see C-8-01) |
| 219 | T-151.8 | behavior | T-151.8 spec pinned: INSTANCE_BUDGET 150,000 (lod_gates.rs); vitest baseline 393; wasm baseline 4,071,877 B; Everon chunks 275; any new TS file ≤ 80 L (docs/specs/Mission_Creator_Architecture/t151_8_culling_density.md:60 and 66-71) | **PARTIAL** — F-04 — W8 exact bytes never recorded; audit pins 4,123,261 B |
| 220 | T-151.9 | docs | T-151.9 identity: tag T-151.9 recorded at c87d74fa with ship SHA c4831451, baseline ec59d10e (T-151.8.1); hub/spec state tag T-151.9 with tip 58c8fcc3 (.ai/artifacts/t151_9_verify_log.md:3-5) | **PASS** — F-03/C-9-02 — SHAs reconciled; tag line honesty-fixed |
| 221 | T-151.9 | behavior | T-151.9: Mission Creator always mounts WgpuTacticalMap — no ?engine= parameter and no VITE_MC_ENGINE branch remain. (.ai/artifacts/t151_9_verify_log.md:11) | **PASS** — A-03 — flag gone; sole wgpu mount |
| 222 | T-151.9 | behavior | T-151.9: Deck runtime deleted — TacticalMap, layer hooks, worldmap *Layer/stores, worker trio, hybrids, viewportBbox, DocCoreSpike. (.ai/artifacts/t151_9_verify_log.md:12) | **PASS** — C-9-01 — covered by slice verdict (automated surface green) |
| 223 | T-151.9 | gate | T-151.9: Deck-free JS oracle _wasm/oracles/jsWorldChunkOracle.ts + residency goldens (22 steps, residency_everon_v1.json, baseline ec59d10e) replace t (.ai/artifacts/t151_9_verify_log.md:13 and 66-67) | **PASS** — C-9-01 + battery/D-01 — automated surface re-ran green at HEAD |
| 224 | T-151.9 | behavior | T-151.9: satelliteUnified reduced to parseTbdSat + pickBaseLevel only (luma loadUnifiedSatTexture removed); FpsCounter off the Deck glyph stream (rAF  (.ai/artifacts/t151_9_verify_log.md:14-16) | **PASS** — C-9-01 — covered by slice verdict (automated surface green) |
| 225 | T-151.9 | count | T-151.9: six deck/luma packages moved dependencies → devDependencies, with a node assert gate PASS. (.ai/artifacts/t151_9_verify_log.md:15 and 31) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 226 | T-151.9 | loc | T-151.9 LANGUAGE GATE: wgpuSlots.ts = 56 LOC (≤ 60); no new cull/LOD policy in TS. (.ai/artifacts/t151_9_verify_log.md:18) | **PARTIAL** — §B thesis — QUALIFIED PASS; leak cluster B-01…B-06 |
| 227 | T-151.9 | count | T-151.9: vitest = 281 = 393 − 112 + 0 (M=0) PASS — 112 Deck-only tests deleted, 0 added. (.ai/artifacts/t151_9_verify_log.md:26) | **PASS** — F-02 — vitest chain exact; fresh run 281/281 |
| 228 | T-151.9 | gate | T-151.9 dist gates PASS: dist/assets free of @deck.gl / DeckGL / @luma.gl; map_engine_wasm present in dist; no map_engine_wasm_bg in index-*.js; DELET (.ai/artifacts/t151_9_verify_log.md:32-37) | **PASS** — C-9-01 — covered by slice verdict (automated surface green) |
| 229 | T-151.9 | bytes | T-151.9 bundle ledger: du -sb dist/assets 7,151,199 → 6,273,423 B (delta −877,776 B, ~12%); MissionCreatorPage-*.js 103,945 B; WgpuTacticalMap-*.js 40 (.ai/artifacts/t151_9_verify_log.md:39-48) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 230 | T-151.9 | gate | T-151.9 manual: S1 empty-query edit mounts wgpu only — PASS as a code gate with operator pan/zoom confirm 'recommended'; S2 (place/select/drag + Save  (.ai/artifacts/t151_9_verify_log.md:52-61) | **OPEN** — C-ALL-01 — operator gate never recorded closed (see C-9-01) |
| 231 | T-151.9 | gate | T-151.9 spec L7: vitest floor ≥ 393 after retarget, but Deck-only tests that are deleted may drop the count if recorded — the shipped 281 relies on th (docs/specs/Mission_Creator_Architecture/t151_9_deck_retirement.md:58) | **PASS** — F-02 — vitest chain exact; fresh run 281/281 |
| 232 | T-151.9 | behavior | T-151.9 spec L9: no silent deferrals — do not ship 'flip only, delete later' unless the user explicitly defers. (docs/specs/Mission_Creator_Architecture/t151_9_deck_retirement.md:60) | **PASS** — C-9-01 — covered by slice verdict (automated surface green) |
| 233 | T-151.9 | gate | T-151.9 spec E2E gate list: editor load ~367k from IDB + server, edit, Save Version 201, Export download, conflict path — all on wgpu default. (docs/specs/Mission_Creator_Architecture/t151_9_deck_retirement.md:35-37) | **PASS** — C-9-01 — covered by slice verdict (automated surface green) |
| 234 | T-151.7.3 | loc | T-151.7.3 spec pinned targets: wgpuSlots.ts ~521 → ≤ 60 LOC; vitest ≥ 393; wasm delta to be recorded; wasm surface ≤ ~10 methods. (docs/specs/Mission_Creator_Architecture/t151_7_3_rust_collapse.md:58 and 77-83) | **PASS** — F-02 — vitest chain exact; fresh run 281/281 |
| 235 | T-151.5.1 | behavior | T-151.5.1 spec pinned before/after: DENSITY_ISO 1 → 2; forest fill / outline / landcover each on → off at zoom ≥ 0; vitest baseline 372 → ≥ 372. (docs/specs/Mission_Creator_Architecture/t151_5_1_forest_fidelity.md:75-81) | **PASS** — F-02 — vitest chain exact; fresh run 281/281 |
| 236 | T-151.4.1 | behavior | T-151.4.1 spec shipped note: operator confirmed ~8–9 buildings restored at town clusters (the only operator-confirmed item in the 4.1 chain). (docs/specs/Mission_Creator_Architecture/t151_4_1_building_road_hotfix.md:15-17) | **OPEN** — C-ALL-01 — operator gate never recorded closed (see C-4.1-01) |
| 237 | hub | docs | Hub risk register: 6 named tripwire→response pairs (bundler-target regression → web-target fallback; WebGL2 copyExternalImageToTexture → RGBA write_te (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:417-425) | **PASS** — F-03 — SHA/link/status claims reconciled against git (see §F) |
| 238 | hub | behavior | Hub execution model: all T-151.x slices run in the standing worktree with linear commits — no per-slice branches, no ./scripts/ticket run, never touch (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:19-51) | **NOT RE-VERIFIED** — hub prose row not individually re-derived |
| 239 | hub | docs | Hub named deferred items after audit: binary chunk wire (Workbench re-export), T-110 terrain deltas, T-111 lazy doc residency, T-143 water, per-chunk  (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:404-406) | **PASS** — F-03 — SHA/link/status claims reconciled against git (see §F) |
| 240 | T-151.0 | gate | T-151.0 spec: 20M stress re-record expectation instances==20000000, staging_peak_bytes==67108864, uniform_bytes_last_frame==64, fps within shipped fam (docs/specs/Mission_Creator_Architecture/t151_0_wasm_merge_dual_mount.md:105-108) | **PASS** — C-0-01 — covered by slice verdict (automated surface green) |
| 241 | T-151.6 | docs | T-151.6 shipped-note (spec): vitest 379 and wasm 4,063,618 B restated; interaction deferred to T-151.7 by design (not a silent deferral). (docs/specs/Mission_Creator_Architecture/t151_6_mission_entities.md:11-12) | **PASS** — F-01 — byte chain exact, every delta re-computed |
| 242 | T-151.7.2 | docs | T-151.7.2 shipped without its own spec file — hub records it only as 'residual tint + zoom SoT (+ wheel restore)' @ 64c64d98 with wheel restore @ 69ca (docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md:352-356) | **PASS** — F-03 — SHA/link/status claims reconciled against git (see §F) |
| 243 | T-151.1 | class-R | T-151.1 L12: stats() gains basemap_mode/basemap_tiles/basemap_bytes appended after the 9 T-151.0 keys with order/names unchanged; spike chunks/gpu_byt (.ai/artifacts/t151_1_verify_log.md:101-104) | **PASS** — C-1-01 — covered by slice verdict (automated surface green) |
| 244 | T-151.3 | gate | T-151.3 P14 PASS: entry-chunk isolation grep gate holds and prior stats() fields intact (keys appended only). (.ai/artifacts/t151_3_verify_log.md:54) | **PASS** — C-3-01 — covered by slice verdict (automated surface green) |
| 245 | T-151.4 | behavior | T-151.4 L7: WgpuForestMassController streams TBDD with a session cache and no LRU (N11 P2b pinned policy). (.ai/artifacts/t151_4_verify_log.md:55) | **PASS** — C-4-01 — covered by slice verdict (automated surface green) |
| 246 | T-151.5 | behavior | T-151.5 stats() gains additive keys tree_glyphs, prop_glyphs, badge_glyphs, atlas_bytes; dev surface window.__wgpuWorldStats includes them plus atlas_ (.ai/artifacts/t151_5_verify_log.md:31 and 66) | **PASS** — C-5-01 — covered by slice verdict (automated surface green) |
| 247 | T-151.6 | behavior | T-151.6 stats() gains slot_instances, slot_drag_instances, cluster_instances; dev surface window.__wgpuSlotStats added. (.ai/artifacts/t151_6_verify_log.md:40-41) | **PASS** — C-6-01 — covered by slice verdict (automated surface green) |
| 248 | program | gate | All twelve slices T-151.0–T-151.9 (incl. hotfixes) claim the identical automated gate battery green: cargo fmt --check, clippy (native + wasm32 render (.ai/artifacts/t151_0_verify_log.md:48-131 (and the same table in each of t151_1..t151_9 verify logs)) | **NOT RE-VERIFIED** — not reached by this single-pass audit |
| 249 | program | count | Vitest count trajectory claimed across the program: 312 (pre-spike baseline) → 317 (spike) → 317 (T-151.0) → 334 (.1) → 343 (.2) → 371 (.3) → 371 (.4/ (.ai/artifacts/t151_wgpu_spike_verify_log.md:11,24 + per-slice verify logs (t151_1:40, t151_2:23, t151_3:25, t151_4:25, t151_5:47, t151_5_1:34, t151_6:55, t151_7:35, t151_7_1:41, t151_7_2:33, t151_7_3:60, t151_8:84, t151_9:26)) | **PASS** — F-02 — vitest chain exact; fresh run 281/281 |
| 250 | program | bytes | Merged wasm byte trajectory claimed: 931,424 (pre-merge) → 3,658,383 (.0) → 3,723,192 (.1) → 3,858,591 (.2) → 3,946,734 (.3) → 4,005,415 (.4) → 4,009, (.ai/artifacts/t151_0_verify_log.md:29-33 + per-slice verify logs (t151_1:36, t151_2:78-83, t151_3:24, t151_4:24, t151_4_1:67, t151_5:46, t151_5_1:33, t151_6:54, t151_7:34, t151_7_3:59, t151_8:85)) | **PARTIAL** — F-04 — W8 exact bytes never recorded; audit pins 4,123,261 B |

**Tally:** PASS 179 · PARTIAL 12 · NOT RE-VERIFIED 45 · OPEN 14 — 250/250 dispositioned.

---

# Round 2 — code-level completeness audit (T-151.10.1) [2026-07-10]

**Trigger:** operator challenge — "did you actually analyse the code, or just the documentation?" —
plus the report that the product is missing things. The challenge was correct in substance:

**Honest characterization of Round 1 depth.** Round 1 re-ran every automated gate, re-derived
every documented number, reconciled docs against git, and line-read ~7 FE hot files plus surface
greps of Rust. It did **not** read the engine or the mount line-by-line, and it audited recorded
promises — so divergences that no test asserts and no document mentions were invisible to it.
Round 2 closes that: a feature-parity matrix against the deleted Deck implementation
(`git show c4831451^:…` as ground truth for "meant to have") and a line-by-line read of the
render/interaction code. Coverage of this round is itemized in the verify log §Round 2.

**Round-2 verdict in one paragraph.** The port is substantially faithful: 12/12 props consumed,
the gesture machine is behavior-exact, every Deck color/width/alpha/gate table re-appears in Rust
with pinned tests (grid palettes, road styles + dash + class gates, building class fills, sea
hypsometric bands, forest ladders, slot ring/cluster constants), and the wasm lifecycle
(I2–I7) is sound. Against that baseline the audit found **2 real rendering defects**
(draw-order: grid over unit markers everywhere; compute-culled trees over everything on WebGPU),
**3 dropped/changed behaviors** (basemap preview mode gone, marquee border gone, buildings-toggle
semantics narrowed), **1 misleading stats field** (GPU cull count never read back), and a small
set of dead code and prod-hygiene issues. Verdicts below; matrix after.

## New findings index (Round 2)

| ID | Sev | Finding (short) | Status |
|----|-----|-----------------|--------|
| X-01 | R | Compute-culled trees draw after the ENTIRE batch list on WebGPU — on top of slots, clusters, grid, marquee; comment claims "after forest, before props"; WebGL2 draws them at the correct order-13 slot → per-backend visual divergence | OPEN |
| P-01 | R | Grid z-order diverges from Deck: `lane_order` Grid=19 above Slots=16/SlotDrag=17/Clusters=18 — grid lines overprint unit markers; Deck drew grid below icons | OPEN |
| P-02 | M | Marquee border dropped: engine builds a fill-only quad (α 60/255); Deck drew fill α 40/255 + 1 px `[173,198,255,200]` outline | OPEN |
| P-03 | M | Basemap `preview` render mode dropped: Deck showed a capped `full.webp` under the unified-satellite load; wgpu shows nothing until the 153 MB texture commits (toast only) | OPEN |
| P-04 | M | `buildings` layer toggle narrowed: gates badge glyphs only (OBB fills always on); Deck's toggle hid the whole building lane | OPEN |
| P-05 | M | Un-gated debug HUD ships in the production editor, labeled with a stale slice id ("T-151.8 · cull + density") | OPEN |
| P-06 | T | Damage-driven render (T-151.8 headline) is disabled in the editor: mount forces `set_continuous_render(true)`; idle-skip benefits only the spike page. Deliberate (code comment; spec S3 "HUD continuous OK") but the product never gets the feature | PARTIAL |
| X-02 | T | Engine camera bounds hard-coded to Everon 12,800² at create; `set_bounds` not wasm-exported; the REAL terrain clamp lives in TS (`clampViewState` + wheel corrective) — functional on all terrains, but the engine-side clamp is wrong for Arland and camera-clamp policy resides in TS (D5 tension) | PARTIAL |
| X-03 | T | `compute_cull_gpu_count` never actually read back: `readback_buf` is copied every frame and mapped nowhere; `last_gpu_count = last_cpu_count` — the stats field is a CPU mirror presented as GPU proof; the f64→f32 frustum truncation on the GPU path is therefore unverified at runtime | OPEN |
| X-04 | M | Marquee geometry exists twice: engine-inline (`upload_marquee`) and `compose_marquee_mesh` in core — the core fn has zero callers outside its own test (dead twin) | OPEN |
| X-05 | M | Dead engine/FE surface: `pan`, `unproject_xy` (+ `viewportFromEngine` never called), `mark_dirty`, `clear_world_buildings`, `clear_icon_lanes`, `tree_glyph_self_check` (also C-5-02) | OPEN |
| X-06 | T | `patch_slot_lane` full-row world→anchor conversion rests on a comment-documented heuristic ("if it looks like a full instance"); only sub-row patches exist today, so the fragile branch is currently unreachable | PARTIAL |
| X-07 | — | Verified-sound list: wasm lifecycle I2–I7 (incl. StrictMode create-race serialization), surface Lost/Outdated self-heal, GpuTimer in-flight guard, loader abort/inflight discipline, drag phase machine truth table, deviceSize↔resize bit-parity, anchor-relative f32 precision argument | PASS |

### Evidence (file:line, both sides where parity)

- **X-01:** `crates/map-engine-render/src/engine.rs:1552-1568` — indirect tree draw encoded after
  `draw_batches` returns (inside the same pass, after Marquee order-20 has drawn); comment at
  `:1552-1553` states the opposite intent. Trees enter this path whenever
  `compute_cull_trees && icon_cull && atlas && !tree_icons_20.is_empty()` — i.e. **always on
  WebGPU** (`compute_cull_trees = !is_gl`, `:1294`); `upload_icon_lane` removes the ordered
  WorldTrees batch on this path (`:2645-2652`), so there is no double-draw, only wrong order.
  WebGL2 keeps the ordered batch (`:2655-2675`). Fix shape: draw the indirect batch when the
  iteration reaches the WorldTrees order slot (or give the compacted buffer its own ordered batch).
- **P-01:** `engine.rs:192-215` (`Grid => 19` vs `Slots => 16`) vs Deck order
  `c4831451^:apps/website/frontend/src/features/tactical-map/TacticalMap.tsx:382-395`
  (`…worldMapLayers → baseMap(grid) → clusterLayers → iconLayer → dragIconLayer → selectionLayer`).
  Deck's marquee was topmost; wgpu keeps marquee topmost (20) correctly — only the grid moved
  above the mission lanes.
- **P-02:** `engine.rs:3516-3564` (fill quad, `[173/255,198/255,1.0,60/255]`, indices 2 tris, no
  line lane) vs `c4831451^:…/layers/useSelectionLayer.ts:11-39` (`FILL=[173,198,255,40]`,
  `LINE=[173,198,255,200]`, `getLineWidth:1`).
- **P-03:** `wgpuBasemap.ts` mode surface = unified/pyramid/single/none (`:118-129,215-237`; no
  preview branch) vs `c4831451^:…/layers/useTerrainBasemapLayer.ts:201-212` (`preview` BitmapLayer
  under the unified load).
- **P-04:** `WgpuTacticalMap.tsx:150-153` routes the toggles to `syncGlyphToggles` only;
  `residency.rs:661-671` consumes `toggle_buildings` for `badge_want` only; `rebuild_buffers`
  (`:549-616`) has no toggle input. Deck: `c4831451^:…/worldmap/useWorldMapLayers.ts:186-244`
  gated every building layer.
- **P-05 / P-06:** `WgpuTacticalMap.tsx:586-602` (always-rendered PANEL) and `:449-450`
  (`set_continuous_render(true)` + comment).
- **X-02:** `engine.rs:58-60,1214-1226` (Everon-hard-coded `set_bounds` at create; only call
  site) + `camera/ortho.rs:156` (the setter; not exported in `map-engine-wasm/src/lib.rs`);
  functional clamp: `WgpuTacticalMap.tsx:191-206` (`clampViewState` to `terrainDef.width/height`)
  + `:536-543` (wheel corrective `set_view` when the engine's own clamp disagrees).
- **X-03:** `icon_cull_gpu.rs:244-248` (`readback_buf` copy + `last_gpu_count = last_cpu_count`);
  `rg readback_buf` → no map_async anywhere; `engine.rs:1366-1374` exposes the mirrored value as
  `compute_cull_gpu_count`. Frustum f32 truncation at `:196-204`.
- **X-04:** `vector_compose.rs:242-275` (`compose_marquee_mesh` + its only caller = own test) vs
  `engine.rs:3508-3564`.
- **X-05:** callers verified absent by grep over `apps/website/frontend/src` (non-test):
  `pan` (`engine.rs:1322`), `unproject_xy` (`:1429`) + `viewportFromEngine` (`mapCamera.ts:52`),
  `mark_dirty` (`:1334`), `clear_world_buildings` (`:2492`), `clear_icon_lanes` (`:2679`),
  `tree_glyph_self_check` (`:4346`).

## Feature-parity matrix (Deck-era ground truth @ `c4831451^` vs HEAD)

Verdicts: **MATCH** (behavior + constants equal, evidence both sides) · **DIVERGED** ·
**MISSING** · **IMPROVED** · **DORMANT** (present, no live consumer — was also unused in Deck).

| # | Feature | Deck evidence (`c4831451^:`) | wgpu evidence (HEAD) | Verdict |
|---|---------|------------------------------|----------------------|---------|
| 1 | Props contract (12 props) | `types.ts:33-72` (byte-identical file at HEAD) | all 12 destructured + consumed `WgpuTacticalMap.tsx:79-93` | MATCH |
| 2 | `TacticalMapApi.flyTo` + Space centering | `TacticalMap.tsx:308-312,349` | `:369-374` + page Space handler | MATCH |
| 3 | Satellite unified basemap (one mip-chained texture) | `useTerrainBasemapLayer.ts:190-199` | `wgpuBasemap.ts:155-208` + engine tex lanes | MATCH |
| 4 | Basemap **preview** mode during unified load | `useTerrainBasemapLayer.ts:201-212` | absent (`wgpuBasemap.ts:118-129`) | **MISSING** (P-03) |
| 5 | Pyramid fallback ≤64 tiles, south-first Y | `:227-247` + `tileUrl.ts` | `wgpuBasemap.ts:240-289` (pack mirror `lanes::pack_offset`, tested) | MATCH |
| 6 | Single-bitmap + none degrade + degraded toast | `:216-225`, page toasts | `wgpuBasemap.ts:210-237` + `onDegraded` refs | MATCH |
| 7 | Unified load progress (0→0.8 fetch, →1 decode) | `:130-166` | `wgpuBasemap.ts:172-207,395-427` | MATCH |
| 8 | Hillshade overlay + 0.1 % opacity re-tint w/o rebuild | `useDemLayer.ts:90-120` | `wgpuBasemap.ts:317-354` + `set_lane_opacity` | MATCH |
| 9 | Procedural 1 km grid, 6-palette (normal/_HS) | `useBaseMapLayer.ts:11-21,44-69` | `lanes.rs:27-110` (pinned tests `:170-205`) | MATCH (colors) |
| 10 | Grid **z-position** (below mission icons) | `TacticalMap.tsx:382-395` | `engine.rs:213` Grid=19 above Slots/Clusters | **DIVERGED** (P-01) |
| 11 | Sea band hypsometric colors + fade ladder | `seaBand.ts:31-44` | `sea_band.rs` (tests pin 1.0/0.6 @ 0/1.5) | MATCH |
| 12 | Contours color/1 px + interval ladder 100/50/20/10 | `contourLayer.ts:12-40`, `lodGates.ts:97-102` | `CONTOUR_RGBA` + `contour_interval_for_zoom` (`lod_gates.rs:73`) | MATCH (values) — live TS twin = B-01 |
| 13 | Land-cover per-kind tints | `landCoverRegions.ts:39-43` | `vector_compose.rs landcover_fill` | MATCH |
| 14 | Road casing (×1.4, near-black) + per-class color/width/dash + class min-zoom gates | `roadLayer.ts:39-52,127-166`, `lodGates.ts:68-73` | `polyline_strip.rs:317-380` + `expand_dashed_polyline_strip:264-310` + `compose_roads_mesh` | MATCH |
| 15 | Building OBB fills per-class + default | `buildingLayer.ts:126-139` | `residency.rs:72-87` (verbatim table) | MATCH |
| 16 | Building outline color | `buildingLayer.ts:140` `[150,150,158,204]` | `residency.rs:68` `[30,30,34,255]` | DIVERGED (known C-3-02; also lighthouse red stroke dropped) |
| 17 | Building badges (military/tower/bunker, 10 px base, min 8) | `buildingLayer.ts:170-198` | `glyph_math.rs:6-10,127-136` + residency badge pack | MATCH |
| 18 | Forest fill α ladder + outline hairline | `forestMass.ts:227-231`, layer `:44-82` | `forest_mass.rs:241` (tests 0.45/0.35/0.12/0) + compose | MATCH (values) — live TS twin = B-02 |
| 19 | Tree/veg/prop/rock glyphs: size mult [1,1.5], REF_ZOOM 3, min-px 4, tint hex parse | `treePropLayer.ts:45-138` | `glyph_math.rs` (full test suite) | MATCH |
| 20 | LOD gate table (all classes) | `lodGates.ts:11-36` | `lod_gates.rs:5-73` + 121-zoom parity scan | MATCH |
| 21 | Slot ring 20/28 px, primary/yellow | `useIconLayer.ts:15-17,96-119` | `slots_gpu.rs:17-24` | MATCH |
| 22 | Cluster gate >500 ∧ ≤−4, disc `22+min(26,log10·12)`, α 235 | `useClusterIconLayer.ts:23-95`, `constants.ts` | `slots_gpu.rs:26-54` (truth-table tests) | MATCH |
| 23 | Cluster count display (size-encoded disc, no numerals) | `useClusterIconLayer.ts:60-62` (no TextLayer) | disc size only | MATCH (Deck had no numerals either) |
| 24 | Drag preview dual-lane + delta uniform | `useIconLayer.ts:126-152` | engine SlotDrag lane + 16 B delta uniform | MATCH (IMPROVED: no per-frame re-upload) |
| 25 | Marquee visual | `useSelectionLayer.ts:11-39` fill+line | `engine.rs:3508-3564` fill only, α 60 vs 40 | **DIVERGED** (P-02) |
| 26 | Marquee topmost | selectionLayer last | Marquee=20 top | MATCH |
| 27 | Click select / Ctrl-toggle / empty-click rules (T-053) | old `useSelectTool.ts:266-296` | `useSelectTool.ts:300-333` | MATCH |
| 28 | Marquee select ≥1×1 guard, frozen-vp unproject | `:239-255` | `:281-296` | MATCH |
| 29 | Drag-move: threshold 4 px, exclude/patch/restore, zero-delta skip, one txn | `:187-232` | `:191-233,258-278` | MATCH |
| 30 | Pan middle/right, rAF-coalesced, frozen vp | `:120-169` | `:140-152,177-187,245-253` | MATCH |
| 31 | Wheel zoom-at-cursor clamped | Deck controller + `useOrthographicView` | `zoom_at` (ULP-0) + corrective clamp `WgpuTacticalMap.tsx:519-549` | MATCH |
| 32 | Dbl-click → Attributes (≤1 sel guard page-side); cluster dbl-click drills | `TacticalMap.tsx:216-234` | `WgpuTacticalMap.tsx:351-367` | MATCH |
| 33 | Cluster click drill-in +1 zoom; press never grabs hidden slot | old tool `:150-156,275-282` | `useSelectTool.ts:159-164,307-315` + `drillIntoCluster` | MATCH |
| 34 | Asset drag-drop (MIME gate, copy effect, unproject) | `TacticalMap.tsx:314-347` | `WgpuTacticalMap.tsx:376-400` | MATCH |
| 35 | Cursor channel rAF X/Y/Z + DEM z + demVersion re-emit + leave→null | `:245-296` | `:294-349` | MATCH |
| 36 | Keyboard: Space/Delete/Ctrl+C/V (cap 500)/undo-redo (page-owned) | `MissionCreatorPage` | unchanged page handlers | MATCH |
| 37 | Viewport clamp to terrain + zoom band ±6 | `useOrthographicView.ts:12-13,37-50` | TS `clampViewState` + Rust MIN/MAX consts | MATCH (policy split noted X-02) |
| 38 | Terrain-switch remount (`key={terrainId}`) | page `:270` | page `:235` + full engine rebuild | MATCH |
| 39 | worldLayerPrefs toggles | `useWorldMapLayers.ts:186-244` (whole lanes) | trees/props MATCH; buildings→badges only | **DIVERGED** (P-04) |
| 40 | Style modes satellite/hybrid/map, satOpacity 1/0.55/0, paper tint | `styleModes.ts:30-45` (file survives) | `setSatOpacity` + `set_clear_color(PAPER_TINT)` | MATCH |
| 41 | FpsCounter DEV-gated + Ctrl+Alt+D stream debug | page `:308`, `FpsCounter.tsx` | page DEV FpsCounter kept; **plus** un-gated in-map panel | DIVERGED (P-05) |
| 42 | Idle GPU behavior | Deck: rAF renders continuously (deck default) | damage-skip exists but continuous forced in editor | PARTIAL (P-06; parity with Deck is technically MATCH — Deck also drew every frame — the regression is vs T-151.8's own claim) |
| 43 | Detail-band viewport cull (`CHUNK_CULL_THRESHOLD` 50k) | `useIconLayer` chunk cull | slot lane draws full SoA (no threshold cull); W8 draw-set culls WORLD lanes; slot counts ≤ ~10k per mission today | DIVERGED-minor (T; note for T-069+ scale work) |
| 44 | World-object pick (12 px, worker rbush) | worker-only, no editor UI consumer | Rust `pick_nearest/pick_rect` exposed, no editor consumer | DORMANT (both eras) |
| 45 | Load/conflict/restore overlays, invalid-id gate | page-owned | page unchanged | MATCH |
| 46 | Basemap unified texture freed on unmount/terrain switch | `useTerrainBasemapLayer.ts:153-162` | engine free on remount (`tex_layer_clear` + engine.free) | MATCH |
| 47 | `onBasemapProgress` sonner contract (`sat-unified` toast id) | page `:93-98` | unchanged page + controller emits same fractions | MATCH |
| 48 | Slot pick radius 4 px nearest-in-box; pick engine | `slotSpatialIndex` rbush→ | same facade, wasm `SlotIndex.pick_rect` | MATCH (IMPROVED: wasm grid) |
| 49 | Icon min sizes (`sizeMinPixels` 4/8) | layers | `size_with_min_px` at pack time | MATCH |
| 50 | Background color behind map | page dark field | `#0b0f14` container + engine clear | MATCH |

Rows 1–50 + the Round-1 §B ownership matrix rows (16 subsystems) constitute the completeness
proof; every DIVERGED/MISSING row has a finding ID above. **No Deck-era feature beyond rows
4/10/16/25/39/41 lost function or fidelity**, and rows 24/48 are strictly better than Deck.

## Round-2 remediation additions (ranked into the Cursor queue)

| Priority (merged) | Finding | One-line remediation |
|---|---------|----------------------|
| 2a (product visual) | X-01 | Draw the compute-culled tree batch at the WorldTrees order slot (encode indirect draw inside the ordered loop), or fall back to ordered batch when mission lanes active |
| 2b (product visual) | P-01 | `Grid` order 19 → between DensityHeat (12) and WorldTrees (13) — one-line `lane_order` change + probe |
| 3a | P-02 | Add a 4-segment hairline ring to `upload_marquee` (engine-side, colors from Deck oracle) |
| 3b | P-03 | Reintroduce preview: `tex_layer_begin/commit` a capped `full.webp` before the unified fetch |
| 3c | P-04 | Either gate OBB fills on `toggle_buildings` in `rebuild_buffers` or document the narrowed toggle in the hub |
| 4 | P-05 | Gate the panel on `import.meta.env.DEV` (and drop the stale label) — pairs with Round-1 A-10 |
| 5 | X-03 | Map `readback_buf` on a cadence (or in the verify API) and surface a real `compute_cull_gpu_count`; assert CPU==GPU in the headless gate |
| 6 | X-02 | Export `set_bounds`; call with terrain dims at mount; keep TS clamp as backstop |
| 7 | X-04/X-05 | Delete dead twins/fns (compose_marquee_mesh or switch engine to it; pan/unproject_xy/viewportFromEngine/mark_dirty/clear_* / register tree_glyph_self_check per C-5-02) |
| 8 | P-06 | Decide: damage-driven render in editor (drop continuous; drive HUD fps from a timer) or write the trade-off into the hub |
| 9 | 43 | Note slot-lane cull threshold as a T-069+ scale prerequisite |

