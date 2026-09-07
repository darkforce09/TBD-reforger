# REPORT T-938.1 — GPU buffer pool for lanes

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-938.1
slice/T-938.1
```

First actions matched the brief. EnfusionMCP count was already 19. `export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target`. Did not create `/home/Samuel/.cache/tbd-target-T-938.1` — native `cargo test -p map-engine-render` used the shared cache; the wave gate ignored the inherited dir and picked its own.

## defect_verified_on_main

Live `upload_slot_role_lane` (then `engine.rs:4907`) still did `bytes.to_vec()` and `device.create_buffer_init` (VERTEX|COPY_DST, label `slot-icon-lane`) on every call. Seven call sites unchanged in signature: SlotPlacePreview / Slots / SlotDrag / Clusters / MissionVehicles×2 / MissionComments.

Forced `LanePool::write` to allocate on every call (the pre-pool policy). Equal-size reuse test went red:

```
thread 'buffer_pool::tests::equal_size_uploads_allocate_once' (3054146) panicked at crates/map-engine-render/src/buffer_pool.rs:196:9:
assertion `left == right` failed: two equal-size uploads must create one buffer
  left: 2
 right: 1
```

## changes

| path | why |
|---|---|
| `crates/map-engine-render/src/buffer_pool.rs` | NEW. `LanePool` keyed by `LaneRole as u32`; doubling `grow_capacity`; never shrinks; host shadow for tests; wasm32 `write_gpu` → `create_buffer` + `queue.write_buffer`. |
| `crates/map-engine-render/src/lib.rs` | Register `buffer_pool` as a native module so tests run under plain `cargo test`. |
| `crates/map-engine-render/src/engine.rs` | `upload_slot_role_lane` writes through the pool (no `to_vec` / `create_buffer_init`). Bind-group/batch buffer identity updates only when the GPU object is new. `clear_stress` does not `destroy()` pooled icon buffers; it `lane_pool.clear()`s. |

Call-site signatures unchanged. Usage flags remain `VERTEX \| COPY_DST`.

## perturbation

Broke `grow_capacity` to return the old size after the first alloc (no doubling).

**red_output VERBATIM:**

```
thread 'buffer_pool::tests::growth_keeps_content' (3107184) panicked at crates/map-engine-render/src/buffer_pool.rs:188:9:
assertion `left == right` failed
  left: 1
 right: 2
```

Restored doubling, `touch crates/map-engine-render/src/buffer_pool.rs`, re-ran: **restored_green** (`buffer_pool` 4 tests ok; `map-engine-render` 83 passed).

## gate_verdict_tail

```
  T-437 destroy inert      PASS
  T-586 route tags         PASS
  T-296 reporter identity  PASS
  T-452 player identity    PASS
  no-python (T-620)        PASS

  gate verdict PASS @ 56c49f1f2353 recorded: .ai/artifacts/verdicts/T-938.1.json
SLICE GATE: PASS
```

Lock wait ~450s (holder was another wave/slice gate) then acquired.

## mod_compile_verdict

N/A — no `.c` / `.layout` edits.

## files_outside_owns

[]

## found_not_fixed

Other `create_buffer_init` paths in `engine.rs` still allocate per upload: W4 polygon/line meshes, world glyph icon lanes, town/road/height labels, `MarkerComposite` captions, self-check probes. T-938.3 shares `engine.rs` and will pack later. `MissionMarkers` composite is not routed through `upload_slot_role_lane`.

## deviations

- Ticket verify named `cargo xtask mk ci-local-leptos`; brief forbids ci-local / leptos-gates. Ran `cargo test -p map-engine-render`, `cargo xtask schema validate`, wasm32 clippy `-D warnings`, and the slice gate.
- Ticket line numbers for `upload_slot_role_lane` were still live on this tree (not stale); call sites shifted after the insert.
- `cargo test -p map-engine-core --all-features`: 1011 passed, 1 failed `dem::peaks::tests::everon_peaks_max_above_350` (`Invalid PNG signature`) — worktree LFS pointer DEM, as briefed.
- `LanePool::write` on native is the host policy (`lane, bytes`); wasm32 GPU entry is `write_gpu(device, queue, lane, bytes, convert)` because wgpu is a wasm32-only dep.

## commits

- `56c49f1f23530ae32c7de12cceaab0b0603ef197` — T-938.1: pool GPU buffers for slot and cluster lanes.

## manual_checklist

1. Open the map editor with the GPU debug HUD.
2. Drag a slot for five seconds at constant instance count.
3. Confirm pooled slot/cluster buffer creations stay flat (one create per lane until a larger payload forces a doubling grow).
4. Place-preview, cluster discs, mission vehicles, and comment glyphs still draw; hiding a lane and showing it again reuses the pooled buffer.

## twins_confirmed

N/A — no Enfusion `.c` / `.layout` twins in this slice.
