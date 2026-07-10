# T-151.10 verify log — Fable 5 full-program audit (W0–W9)

**Audit HEAD:** `1cbe3a56` (= tag `T-151.9` tip `58c8fcc3` + 2 Cursor docs commits `a7a93368`, `1cbe3a56`) ·
**Worktree:** `tbd-reforger-wgpu-spike/` (`git rev-parse --show-toplevel` = `/var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike`) ·
**Date:** 2026-07-09/10 · **Executor:** Claude Code (Fable 5) ·
**Report:** [`t151_10_fable_audit_report.md`](t151_10_fable_audit_report.md)

**Toolchain:** rustc/cargo **1.95.0** · node **v26.4.0** · vitest **4.1.9** · wasm-pack via `make wasm`.

Preflight notes:

- `test "$(git rev-parse --show-toplevel)" = "$(pwd)"` fails **only** because the shell starts in
  `/home/Samuel/...` while git reports `/var/home/Samuel/...` — Fedora Silverblue `/home → /var/home`
  symlink, same directory. Recorded, not a defect.
- `git status --porcelain` at audit start: clean. During the audit the operator (Cursor lane) edited
  `t151_10_fable_program_audit.md` + `t151_10_claude_code_handoff.md` (agent-budget codification);
  those two diffs are **not** part of the audit commit (Cursor owns doc sync).
- `git lfs pull` + `make map-assets-link` + `make wasm` ran before any test.
- Process note: the first audit attempt (multi-agent workflow) hit the session token limit after
  3 of 13 lenses completed; the audit was re-planned single-thread. The completed lenses
  (claims extractor, A-integrity, F-honesty) were salvaged and every salvaged citation was
  re-verified on the main thread before use. One salvaged count was corrected
  (Deck-flip deletions: **35**, not 36 — see battery row 16).

---

## Class S battery (spec §Verify + the slice-spec commands the spec omitted)

Every command captured to its own output file with exit code; nothing aborted on red.
Rows 1–11 ran as one sequential background script; 12–13 ran after
(they appear in every slice spec's §Verify but not in the T-151.10 spec §Verify).

| # | Command | Exit | Key output |
|---|---------|------|-----------|
| 1 | `cargo fmt --check` | **0** | no diffs |
| 2 | `cargo clippy --all-targets -- -D warnings` | **0** | clean (workspace root: backend + 3 map-engine crates) |
| 3 | `make wasm-ci` | **0** | fmt + clippy + tests of core/wasm/render crates |
| 4 | `cargo test -p map-engine-core --all-features` | **0** | `145 passed` + `5 passed` + `5 passed`; 0 failed/ignored |
| 5 | `cargo test -p map-engine-render` | **0** | `21 passed`; 0 failed/ignored — incl. `compute_cull::tests::class_r_1k_random_frusta_count_stable`, `class_r_inside_outside`, `class_r_compact_preserves_order_and_count`, `storage32_roundtrip` |
| 6 | `make wasm` | **0** | `map_engine_wasm_bg.wasm` = **4,123,261 B** (fresh, `ls -la` 2026-07-09 21:14) |
| 7 | `npm test` (`vitest run`) | **0** | **Test Files 39 passed (39) · Tests 281 passed (281)** |
| 8 | `npm run build` | **0** | vite build clean |
| 9 | `npm run lint` | **0** | clean |
| 10 | deck/luma grep over `dist/assets` | **0** | `grep -rn -E "deck\.gl|@deck\.gl|@luma\.gl" dist/assets` → **no matches** (grep exit 1 = clean). Note: `rg` is not installed in this environment; POSIX grep used. `du -sb dist/assets` = **6,273,423 B** |
| 11 | `wc -l src/features/tactical-map/wgpu/wgpuSlots.ts` | **0** | **56** (≤ 60 budget) |
| 12 | `cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings` | **0** | clean |
| 13 | `cargo build --workspace` | **0** | clean |

Supplementary (evidence for §D of the report):

- `npx vitest list` → **281** tests enumerated == 281 executed → **nothing skipped or excluded**.
- Parity census from the same listing: **16** `*.parity.test.ts` files, **99** parity tests
  (chunkMathRust 24, interaction 12, dem-png 12, world 9, forest 9, slotGpu 7, dem 7,
  slotIndex 3, cluster 3, world.residency 2, world.pick 2, orthoCamera 2, mission 2, meters 2,
  hillshade 2, glyphLod 1).
- Entry-chunk isolation re-proven on the fresh build: `grep -l map_engine_wasm_bg dist/assets/index-*.js`
  → no match (exit 1).

### Reconciliations

- **vitest N:** fresh **281/281** == T-151.9 claim (`281 = 393 − 112 + 0`). 39 files.
- **wasm bytes:** fresh **4,123,261 B** == hub "~4.12 MB". The t151_8 verify log's "~4.09 MB"
  was stale (pre-8.1 figure retained through the 8.1 amendment) — honesty-fixed below.
- **bundle:** fresh `du -sb dist/assets` = **6,273,423 B** == T-151.9 log's post-flip figure exactly.

---

## GPU-R gates — re-executed headless (independent of prior logs)

Harness reconstructed from scratch (no committed runner exists — see report finding D-06):
`chrome-headless-shell` (Chrome for Testing **149.0.7827.55**, playwright cache
`chromium_headless_shell-1228`) + raw CDP over the Node built-in WebSocket + Vite dev server
(port 5199), against `/_spike/wgpu` and its `window.__selfChecks` hook (`WgpuCanvas.tsx:86-94`).

### WebGL2 (SwiftShader: `--use-angle=swiftshader --enable-unsafe-swiftshader`) — EXECUTED PASS

`/_spike/wgpu?force=webgl`, backend HUD `"backend":"webgl2"`. All five registered self-checks
returned `pass: true` with **every probe byte-exact**:

| Check | Probes | Result |
|-------|--------|--------|
| `calibration` (= `self_check`, T-151.0 gate) | 7 | all pass — G center + 2px-inside corners + off-quad |
| `texture` (= `texture_self_check`, T-151.1 gate) | 3 | all pass — NW red / NE green / SW blue |
| `worldBuilding` (= `world_building_self_check`, T-151.3 S4 gate) | 3 | all pass |
| `seaBand` (= `sea_band_self_check`, T-151.4 L11 gate) | 2 | all pass |
| `roadCenterline` (= `road_centerline_self_check`, T-151.4 L11 gate) | 2 | all pass |

Raw JSON: scratchpad `gpu/webgl2-run.json` (17/17 probes `pass:true`; expect==got byte-equal on
every RGBA quadruple). **This closes the T-151.4 verify log's "operator paste pending" GPU-R gap
with fresh independent evidence**, and re-proves the T-151.0/1/3 readback gates at HEAD.

### WebGPU (lavapipe) — NOT RE-RUN (environment)

Attempted: `VK_ICD_FILENAMES=lvp_icd.x86_64.json` + `--enable-unsafe-webgpu --enable-features=Vulkan`.
The engine fell back and failed surface creation; page error (quoted verbatim):

```
Error: create-surface: Failed to create surface for any enabled backend: {Gl: InstanceError { message: "canvas.getContext() returned null; webgl2 not available or c…
```

`chrome-headless-shell` in this cache does not expose WebGPU with these flags. The T-151.1 log's
lavapipe WebGPU execution is therefore **not reproducible in this environment today** — recorded
as NOT RE-RUN (environment difference), not as a falsified claim. WebGL2 evidence above stands on
its own; the WebGPU-specific upload path (`copy_external_image_to_texture`) retains only the
T-151.1-era evidence.

- `tree_glyph_self_check` (T-151.5 S4): **could not be executed** — the engine API exists
  (`crates/map-engine-render/src/engine.rs:4346`) but is not registered in `__selfChecks`
  (`WgpuCanvas.tsx:87-93` registers five checks). OPEN finding C-5-02 in the report.

---

## Tag ↔ SHA reconciliation (F evidence)

`git tag -l 'T-151*'` → **37 tags**; every one resolved with `git rev-parse '<tag>^{commit}'`
(full map in report §F). Headlines re-verified on the main thread:

- `T-151.9` tag → `58c8fcc3`; ship SHA `c4831451`; `git log c4831451..58c8fcc3 --oneline` =
  2 docs-only commits (`c87d74fa` verify-log record, `58c8fcc3` verify-log clarify) — benign.
- The t151_9 log's line 3 `(c87d74fa)` was self-invalidated by the tag moving to the clarify
  commit — honesty-fixed below.
- Deck-flip commit `c4831451`: `git show c4831451 --name-status --format= | grep -c '^D'` = **35**
  deleted files (TacticalMap.tsx, 6 layer hooks, worker trio, worldmap stores/layers, hybrids);
  mount flip + deck/luma dependency demotion in the same commit.
- `804f779a` (T-151.7.3) parent is `fa7a4b1d`, not `5457dd4e` — the t151_7_3 log's baseline line
  omits one docs commit (report F-09; both intermediate commits are docs-only, code state unaffected).

---

## Pinned-inventory re-derivation (live commands, packages/map-assets/everon)

| Quantity | Pinned | Re-derived | Verdict |
|----------|--------|-----------|---------|
| `objects.prefabCount` | 391 | **391** (`node -e` over manifest.json) | PASS |
| `objects.instanceCount` | 508,291 | **508,291** | PASS |
| Chunk files | 275 | **275** (`ls objects/chunks/*.json.gz | wc -l`; index manifest present) | PASS |
| Road segments | 888 | **888** (`gunzip -c objects/roads.json.gz` → `roadSegments.length`) | PASS |
| Road classes | "6 classes incl. runway" | **5 in data** (runway 5, highway_paved 12, road_paved 110, road_dirt 367, track 394); the 6-class closed style enum incl. `path` lives at `roads.rs:14-23` with 0 `path` segments on Everon | PARTIAL |
| Forest regions | 36 | **36** | PASS |
| TBDD density grids | 625 × 1,172 B | **625** files, `stat -c%s | sort -u` = **1172** (single value) | PASS |
| Glyph atlas | 28 | **28** (`world-glyphs.json` `.icons` keys) | PASS |
| TBDS | 152,713,114 B, 14 mips, 12800² | **152,713,114** (`stat`), `mipCount: 14`, `baseWidthPx: 12800` | PASS |
| DEM | 6400², −204.78…375.53 m, no flip | `widthPx: 6400`, `heightRangeMinM: -204.78`, `heightRangeMaxM: 375.53`, `axisFlip {x:false,z:false}` | PASS |
| Zoom band −6…+6 default −2 | cited `useOrthographicView.ts:12-13,33` | values live (`mapCamera.ts:14-15` + Rust `OrthoCamera`; default −2 in `WgpuTacticalMap`), **but the cited file was deleted at T-151.9** | PARTIAL (stale citation) |
| Slot pick radius 4 px | `slotSpatialIndex.ts:123` | `pickNearest(..., radiusPx = 4)` at **:123** | PASS |
| Drag threshold 4 px | `useSelectTool.ts:21` | `DRAG_THRESHOLD = 4` at **:25** (comment at :24) | PASS (4-line drift) |
| Cluster gates | >500 ∧ zoom ≤ −4; 48 px | `CLUSTER_SLOT_THRESHOLD = 500` (constants.ts:17), `ZOOM_CLUSTER_MAX = -4` (:12), `radiusPx = 48` (slotClusterIndex.ts:218) | PASS |
| Instance budget 150,000 | cited `worldObjectsCore.ts` (deleted) | value alive in **Rust**: `INSTANCE_BUDGET: usize = 150_000` (`lod_gates.rs:26`) | PARTIAL (stale citation) |
| Chunk apply budget ≤ 4 ms | cited `chunkStore.ts` (deleted) | `APPLY_BUDGET_MS = 4` now at `wgpuWorldLoader.ts:30` (JS-side enforcement — report B-04) | PARTIAL (stale citation) |
| Engine chunk pool 2,097,152 × 32 B | `scene.rs` | `CHUNK_CAPACITY: usize = 2_097_152` (`scene.rs:21`) | PASS |
| World pick radius 12 px | LOD contract §N2 | not re-verified this pass (no live world-pick call traced) | NOT RE-VERIFIED |
| LRU `max(64, 3 × pinned)` | `chunkStore.ts` (deleted) → `residency.rs` | behavior covered by the 22-step residency golden (2 parity tests PASS); constant line not re-cited | NOT RE-VERIFIED (line cite) |

---

## Honesty fixes applied to prior verify logs (spec-sanctioned, one line each)

1. `.ai/artifacts/t151_9_verify_log.md:3` — was `**Tag:** \`T-151.9\` (\`c87d74fa\`)`;
   now records the actual tag tip `58c8fcc3` (the clarify commit itself became the tip) with the
   original SHA retained in parentheses.
2. `.ai/artifacts/t151_8_verify_log.md:86` — wasm row said `~4.09 MB` (stale pre-8.1 figure);
   now records `~4.12 MB` with the exact fresh byte count **4,123,261 B** measured at this audit.

No other file outside `.ai/artifacts/t151_10_*` was modified by this audit.

---

## Verdict

Automated Class S surface at HEAD: **13/13 commands exit 0**. Class R/S test surface: **281/281**
vitest (99 parity) + **176** cargo tests (155 core + 21 render), nothing ignored. GPU-R: **17/17**
probes byte-exact on WebGL2/SwiftShader; WebGPU not reproducible in this environment; one GPU gate
(`tree_glyph_self_check`) unexecutable because it was never wired. Findings, per-slice verdicts,
LANGUAGE GATE census, and the claims register live in
[`t151_10_fable_audit_report.md`](t151_10_fable_audit_report.md).
