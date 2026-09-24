# T-938 — Engine and wasm performance

Owner: command center. Source: master audit S3 (2026-09-04), verified against main @ 072988d57
(README.md in this directory). Scope: TBD-Reforger only. Executor: claude-code.

## 1. Related existing tickets — referenced, never re-minted
T-935 map binary storage: **T-935.4 delivers the DEM decode peak** (raw `.dem`, no PNG inflate; png_decode.rs:58-79
and world_assets/mod.rs:620-626 go away) — no slice here. T-935.3/.11 own world_host.rs (T-938.2 packs after .11);
T-935.13 is the cutover T-938.6 measures against.

## 2. Verified anchors (2026-09-04)
| Finding | Anchor | Verdict | Slice |
|---|---|---|---|
| DEM decode peak ~400 MB | png_decode.rs:58/64/79 + world_assets/mod.rs:620,626 (sync on wasm thread) | TRUE | T-935.4 (existing) |
| satellite L0 655 MB RGBA | satellite.rs (TBDS via HTTP Range) | TRUE | T-938.6 |
| create_buffer_init per gesture tick | engine.rs:4907 fn, :4921 to_vec, :4924 create; 7 call sites 4565/4577/4610/4627/4642/4693/4870 | TRUE (audit's :4024-4039 is cluster packing upstream) | T-938.1 |
| 10+ buffers per chunk crossing | world_host.rs:454-525 | UNVERIFIED — measure first | T-938.2 |
| compute cull trees only | engine.rs:1812-1816 do_compute_trees | TRUE | T-938.3 |
| CPU count every frame | icon_cull_gpu.rs:226 before early-outs; compute_cull.rs:79 linear | TRUE | T-938.3 |
| single atomicAdd | shader.wgsl:315 | TRUE | T-938.3 |
| section cut brute force | building_section.rs:292; occl.bvh unused | TRUE | T-938.4 |
| HeightField 67 MB/level | building_section.rs:46 MAX_PLAN_DIM 2048, :81 vec![None; cols*rows] Option<f64> | TRUE | T-938.4 |
| 31k sync raycasts | building_viewshed.rs:251-268 via :156-195; defaults :37-41 | TRUE | T-938.5 |
| terrain viewshed O((R/C)²) no cap | dem/sample.rs:492-500 OVERSAMPLE 2.0, loop :500-535 | TRUE | T-938.5 |

## 3. Design
- **T-938.1** `map-engine-render/src/buffer_pool.rs`: `LanePool::write(device, queue, lane, bytes) -> &Buffer`,
  doubling growth, never shrinks, usage flags copied from the current create_buffer_init; bind groups rebuilt
  only when the buffer object changes. engine.rs shared with T-938.3 → .3 packs later.
- **T-938.2** measurement first (allocation counter behind a debug flag, three crossings on everon). Below 3
  per crossing ⇒ report + counter only; else a per-lane ring sized from the measured maximum.
- **T-938.3** per-lane compute gate generalising do_compute_trees; CPU count behind the debug HUD flag; shader
  workgroup-local reduce + one atomicAdd per workgroup; fixture test GPU count == CPU count.
- **T-938.4** `building_section_index.rs`: y-interval query over the loaded BVH → candidate triangles;
  HeightField as f32 + NaN sentinel in lazily allocated tiles behind the existing accessors; golden equality
  on the six committed buildings.
- **T-938.5** resumable batch iterators + cancel token in building_viewshed.rs and dem/sample.rs; caps r ≤ 400 m,
  cells ≤ 250k; `tools/viewshed_scheduler.rs` (≤ 4 ms per animation frame, cancel on new placement);
  los_tool.rs:904 is a call-site edit only (SIZE-3 allowlisted).
- **T-938.6** `world_assets/memory_budget.rs`: per-asset peaks, budget 1536 MB (query-param override),
  `reserve(bytes) -> Decision {Ok, Degrade, Refuse}`; satellite.rs raises its mip floor on Degrade; HUD row.

## 4. Rules every slice encodes
Defect verified on main first with a number (red or measurement pasted verbatim); perturbation with `touch`
after restore; `cargo test -p map-engine-core --all-features` only; edition-2024 rustfmt for map-engine-render;
no `git add -A`, `git stash`, `cargo xtask ci ci-local`; `skip:` = FAIL; no .py/.sh/.mjs; allowlists never grow;
agents never merge/push/change status. Report: pwd_branch · defect_verified_on_main · changes · perturbation ·
gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits.

## Claude Code prompt — T-938.1

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-938.1 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-938_1_plan.md; engine.rs:4550-4700,4860-4940; map-engine-render/src/lib.rs; scene.rs:32-80 (Pod structs).
═══ PROBLEM ═══
upload_slot_role_lane copies bytes and creates a new wgpu buffer on every call from seven sites; every gesture tick allocates.
═══ SHIPPED ═══
scene.rs Pod structs; bytemuck in map-engine-render. Do not add a second pool elsewhere.
═══ LANGUAGE GATE ═══
Rust (map-engine-render, edition-2024 rustfmt).
═══ LOCKED ═══
- One pool keyed by lane; doubling growth; never shrinks; usage flags identical to today.
- Call-site signatures unchanged; bind groups rebuilt only on buffer identity change.
- Rendered output identical across all seven lanes.
═══ DO ═══
1. Verify on main: test-only creation counter over two equal uploads shows two creates; paste the red.
2. buffer_pool.rs (tests: growth keeps content, reuse on smaller write, no shrink) registered in lib.rs.
3. engine.rs:4907-4924 writes through the pool; remove the :4921 to_vec.
4. Perturbation: grow at old size → growth test red; restore, touch, green.
═══ DO NOT ═══
No cull changes (T-938.3); no files outside owns.
═══ VERIFY ═══
cargo test -p map-engine-render ; cargo xtask mk ci-local-leptos ; cargo xtask platform wave gate --slice T-938.1
═══ MANUAL ═══
Drag a slot for five seconds with the GPU debug HUD open: buffer count stays flat.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-938.2

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-938.2 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §2-§3; docs/plans/t-938_2_plan.md; world_host.rs (chunk path after T-935.11); the debug HUD flag plumbing.
═══ PROBLEM ═══
The audit claims 10+ buffer allocations per chunk crossing at world_host.rs:454-525. Nobody verified it on main.
═══ SHIPPED ═══
T-935.3/.11 binary chunk loader + cutover in world_host.rs.
═══ LANGUAGE GATE ═══
Rust (Leptos, wasm).
═══ LOCKED ═══
- Measure before changing anything; the number goes in the report as defect_verified_on_main.
- Below 3 allocations per crossing ⇒ ship the counter only and state the claim is false.
- If confirmed: ring per lane sized from the measured maximum; re-measure.
═══ DO ═══
1. Allocation counter behind the debug flag on the crossing path.
2. Cross three chunk boundaries on everon; record per-crossing allocations.
3. Only if ≥ 3: implement the ring; re-measure; paste before/after.
4. Perturbation (only if implemented): ring of one → reuse test red; restore, touch, green.
═══ DO NOT ═══
No loader format changes; no files outside owns.
═══ VERIFY ═══
cargo xtask mk ci-local-leptos ; cargo xtask platform wave gate --slice T-938.2
═══ MANUAL ═══
Record the three crossing measurements in the report.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-938.3

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-938.3 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-938_3_plan.md; engine.rs:1790-1830; icon_cull_gpu.rs:200-260; compute_cull.rs:60-100; shader.wgsl:290-330; buffer_pool.rs (T-938.1).
═══ PROBLEM ═══
Only tree icons cull on the GPU; every frame scans all icons on the CPU; the shader serialises on one atomic.
═══ SHIPPED ═══
T-938.1 LanePool; existing compute path for trees; debug HUD flag.
═══ LANGUAGE GATE ═══
Rust (map-engine-render, edition-2024 rustfmt), WGSL.
═══ LOCKED ═══
- CPU cull survives only as the no-compute fallback; CPU count only with the debug flag on.
- Workgroup-local reduce, one atomicAdd per workgroup; readback count may lag one frame (documented).
- Fixture test: GPU visible count == CPU reference.
═══ DO ═══
1. Verify on main: CPU count runs with the flag off (test counter); paste the red.
2. engine.rs per-lane compute gate; lanes upload through the pool.
3. icon_cull_gpu.rs count behind the flag; shader.wgsl reduce rewrite.
4. Perturbation: drop the workgroup barrier → count-equality test red; restore, touch, green.
═══ DO NOT ═══
No pool changes; no files outside owns.
═══ VERIFY ═══
cargo test -p map-engine-render ; cargo xtask mk ci-local-leptos ; cargo xtask platform wave gate --slice T-938.3
═══ MANUAL ═══
Pan across everon with the HUD open: visible counts match between flag on and off.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-938.4

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-938.4 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-938_4_plan.md; building_section.rs:30-100,270-320; bvh_sidecar.rs:200-240; core lib.rs; packages/map-assets/everon/prefabs/buildings/ (6 golden).
═══ PROBLEM ═══
Every section cut iterates every triangle; every HeightField allocates the full 2048² Option<f64> plan (67 MB).
═══ SHIPPED ═══
occl.bvh sidecars (loaded, unused here); HeightField accessors; six golden buildings.
═══ LANGUAGE GATE ═══
Rust (map-engine-core).
═══ LOCKED ═══
- Cut polygon equals brute force on all six goldens (f32 tolerance documented in the test).
- MAX_PLAN_DIM cap stays; tiles allocate on first write; accessors unchanged.
- BVH bounds verified to enclose section geometry before use.
═══ DO ═══
1. Verify on main: triangles visited per cut + bytes per HeightField on a golden; paste the numbers.
2. building_section_index.rs (y-interval query + tests) registered in lib.rs.
3. section_at_owned uses the index; HeightField tiled f32 + NaN sentinel.
4. Perturbation: zero-height interval → golden equality red; restore, touch, green.
═══ DO NOT ═══
No viewshed changes (T-938.5); no files outside owns.
═══ VERIFY ═══
cargo test -p map-engine-core --all-features ; cargo xtask platform wave gate --slice T-938.4
═══ MANUAL ═══
None.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-938.5

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-938.5 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-938_5_plan.md; building_viewshed.rs:30-60,150-270; dem/sample.rs:480-540; canvas/gestures.rs:1290-1330 (read only); los_tool.rs:890-920; tools/mod.rs.
═══ PROBLEM ═══
Placing a viewshed fires ~31k synchronous raycasts per level and the terrain sweep is O((R/C)²) with no cap; the wasm thread freezes.
═══ SHIPPED ═══
level_wash/level_wash_compound; dem sampler; los_tool placement path.
═══ LANGUAGE GATE ═══
Rust (map-engine-core, Leptos).
═══ LOCKED ═══
- los_tool.rs: call-site edit only (SIZE-3 allowlisted); gestures.rs untouched.
- Caps r ≤ 400 m, cells ≤ 250k, refused with a message; batches ≤ 4 ms; new placement cancels the old.
- Sliced result bit-identical to the synchronous path on the fixture.
═══ DO ═══
1. Verify on main: time place_viewshed at r=25 on a golden building; paste the blocking ms.
2. Batch iterators + cancel token in building_viewshed.rs and dem/sample.rs (equality tests).
3. tools/viewshed_scheduler.rs in tools/mod.rs; los_tool.rs:904 submits to it.
4. Perturbation: iterator skips its last row → equality test red; restore, touch, green.
═══ DO NOT ═══
No new logic in los_tool.rs; no section-cut changes; no files outside owns.
═══ VERIFY ═══
cargo test -p map-engine-core --all-features ; cargo xtask mk ci-local-leptos ; cargo xtask platform wave gate --slice T-938.5
═══ MANUAL ═══
Place two viewsheds quickly: the first cancels, the UI never stalls.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-938.6

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-938.6 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §1-§3; docs/plans/t-938_6_plan.md; world_assets/mod.rs; satellite.rs (mip loading after T-935.13); the debug HUD.
═══ PROBLEM ═══
Loaders allocate independently (satellite L0 alone is 655 MB RGBA); the first OOM aborts the wasm instance with no warning.
═══ SHIPPED ═══
T-935.13 binary asset cutover; T-935.4 raw DEM (the DEM peak is theirs); debug HUD.
═══ LANGUAGE GATE ═══
Rust (Leptos, wasm).
═══ LOCKED ═══
- Budget default 1536 MB with a query-param override; reserve() decides Ok/Degrade/Refuse.
- Only satellite degrades here (mip floor +1 per Degrade); other assets register peaks for the HUD.
- Measured peaks per asset on everon go in the report.
═══ DO ═══
1. Verify on main: everon under a 512 MB heap cap aborts; paste it.
2. world_assets/memory_budget.rs (tests) registered in world_assets/mod.rs.
3. satellite.rs consults reserve() per mip; HUD row reserved/budget + floor.
4. Perturbation: reserve never degrades → floor test red; restore, touch, green.
═══ DO NOT ═══
No DEM changes; no loader format changes; no files outside owns.
═══ VERIFY ═══
cargo xtask mk ci-local-leptos ; cargo xtask mk leptos-gates ; cargo xtask platform wave gate --slice T-938.6
═══ MANUAL ═══
Load everon with ?memBudgetMb=512: satellite floor rises, no abort, HUD shows the numbers.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```
