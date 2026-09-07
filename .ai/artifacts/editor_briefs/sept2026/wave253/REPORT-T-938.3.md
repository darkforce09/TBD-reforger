# REPORT T-938.3 — GPU cull for all icon lanes

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-938.3
slice/T-938.3
```

First actions matched the brief. `export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target`. Did not create `/home/Samuel/.cache/tbd-target-T-938.3` — native `cargo test -p map-engine-render` used the shared cache; the wave gate ignored the inherited dir and picked its own.

## defect_verified

`encode_cull` (`icon_cull_gpu.rs:226`) called `count_icons_in_frustum` unconditionally, including with the debug HUD flag off. `cpu_count_for_encode(..., debug_hud=false)` on the live policy returned `Some(3)` on the fixture.

**red_output VERBATIM (debug HUD off still scans):**

```
thread 'compute_cull::tests::t938_3_debug_hud_off_skips_cpu_count' (4140597) panicked at crates/map-engine-render/src/compute_cull.rs:319:9:
debug HUD off must skip the CPU frustum scan; got Some(3)
```

Compute cull was trees-only (`do_compute_trees` gated on `!tree_icons_20.is_empty()`). `cs_icon_cull` did one `atomicAdd` per visible icon and had no workgroup reduce-barrier; the fixture GPU-sim count was 1 vs CPU 3.

**red_output VERBATIM (no workgroup reduce-barrier):**

```
thread 'compute_cull::tests::t938_3_gpu_visible_count_equals_cpu_on_fixture' (4140599) panicked at crates/map-engine-render/src/compute_cull.rs:336:9:
assertion `left == right` failed: GPU workgroup reduce must match the CPU oracle on the fixture frustum
  left: 1
 right: 3
```

## changes

| path | why |
|---|---|
| `crates/map-engine-render/src/shader.wgsl` | `cs_icon_cull`: workgroup-local vis bits, exclusive prefix on thread 0, one `atomicAdd` per workgroup. First `workgroupBarrier` before the reduce. |
| `crates/map-engine-render/src/icon_cull_gpu.rs` | Per-lane src/dst/counter/indirect. `encode_cull` uses `cpu_count_for_encode` (CPU scan only when `debug_hud`). GPU readback may lag one frame. |
| `crates/map-engine-render/src/engine.rs` | Same compute gate for WorldTrees / WorldProps / WorldBadges and the T-938.1 slot-role lanes. Indirect draws emit in draw order. `set_compute_cull_debug_hud`. T-938.1 `lane_pool.write_gpu` still runs. |
| `crates/map-engine-render/src/compute_cull.rs` | Native `cpu_count_for_encode` + fixture GPU==CPU test (icon_cull_gpu is wasm32-only). |

## perturbation

Removed the first `workgroupBarrier()` in `cs_icon_cull` (the reduce barrier; the scatter barrier stayed).

**red_output VERBATIM:**

```
thread 'compute_cull::tests::t938_3_gpu_visible_count_equals_cpu_on_fixture' (67508) panicked at crates/map-engine-render/src/compute_cull.rs:321:9:
assertion `left == right` failed: GPU workgroup reduce must match the CPU oracle on the fixture frustum
  left: 1
 right: 3
```

Restored the barrier, `touch crates/map-engine-render/src/shader.wgsl`, re-ran: **restored_green** (`t938_3_gpu_visible_count_equals_cpu_on_fixture` ok; `map-engine-render` 86 passed).

## gate_verdict_tail

```
  T-437 destroy inert      PASS
  T-586 route tags         PASS
  T-296 reporter identity  PASS
  T-452 player identity    PASS
  no-python (T-620)        PASS

  gate verdict PASS @ 8796f3e02b9d recorded: .ai/artifacts/verdicts/T-938.3.json
SLICE GATE: PASS
```

Lock wait ~30s (holder was slice T-938.4).

## mod_compile_verdict

N/A — no `.c` / `.layout` edits.

## files_outside_owns

`crates/map-engine-render/src/compute_cull.rs` — native Class-R fixture (`cpu_count_for_encode`, GPU workgroup-sim == CPU count). `icon_cull_gpu.rs` is wasm32-gated and `lib.rs` is not owned, so a new native module could not be registered. Siblings in this pack do not own this file.

## found_not_fixed

- `MissionMarkers` `MarkerComposite` (icon + caption) is not routed through `upload_slot_role_lane`; the icon half is not compute-culled.
- WorldLabels / WorldRoadLabels / WorldTownLabels use `vs_text`, not `cs_icon_cull`.
- SPA `debug_hud_shown` is not in owns; `RenderEngine::set_compute_cull_debug_hud` exists but is unwired. Default off ⇒ production skips the CPU scan (the ticket). HUD on/off comparison needs a later FE call.
- Slot/cluster lanes still write T-938.1 pooled VERTEX\|COPY_DST buffers, then also feed cull src (pool has no STORAGE; `buffer_pool.rs` not owned). Not a revert.
- Glyph icon lanes (`upload_icon_lane`) still `create_buffer_init` on the WebGL2 fallback.

## deviations

- Ticket verify named `cargo xtask mk ci-local-leptos`; brief forbids ci-local / leptos-gates. Ran `cargo test -p map-engine-render`, `cargo xtask schema validate`, wasm32 clippy `-D warnings`, and the slice gate.
- `compute_cull.rs` is outside the brief owns list (see files_outside_owns).

## commits

- `8796f3e02b9dc652836ffa2d3cf1b548ef703732` — T-938.3: GPU cull every icon lane with workgroup reduce.

## manual_checklist

1. Pan across Everon with the GPU debug HUD open.
2. Visible glyph counts with the HUD flag off come from GPU readback (may lag one frame).
3. Toggle `set_compute_cull_debug_hud(true)` (or wire Ctrl/Cmd+Alt+D later): CPU oracle matches the GPU count on the same frustum.
4. Trees, props, badges, slots, drag, clusters, vehicles, and comments still draw in order; WebGL2 still uses the direct IconInstanced / pooled path.

## twins_confirmed

N/A — no Enfusion `.c` / `.layout` twins in this slice.
