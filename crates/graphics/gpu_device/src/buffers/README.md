# Pooled GPU buffers and readback fences

The bookkeeping behind two kinds of long-lived GPU buffer: per-lane instance buffers that grow
and never shrink, and mapped readback buffers that allow at most one mapping at a time.

## Contents

```text
crates/graphics/gpu_device/src/buffers/
├── mod.rs       the module tree
├── pool.rs      `LanePool`: one persistent vertex buffer per lane id; `grow_capacity`
├── readback.rs  `ReadbackLane`: the one-mapping-at-a-time guard and the fresh-sample flag
└── tests/       unit tests for the pool's growth and the readback lane's state changes
```

## How it works

`LanePool` keys a buffer on a caller's `u32` lane id and keeps a host copy of the bytes last
written. `grow_capacity` sets the size: the first write allocates exactly the needed bytes,
rounded up to 4; a larger write doubles the capacity until the bytes fit; a smaller write reuses
the buffer, which never shrinks. `write` is the host-only path the tests drive. `write_gpu`, in
the WebAssembly build only, runs the same arithmetic, applies the caller's in-place conversion to
the bytes, creates a new `VERTEX | COPY_DST` buffer when the capacity grew, and uploads the bytes
with `queue.write_buffer`. It returns the buffer and whether it is new, so the caller knows when
to rebuild its batch. `creates()` counts the buffers made, and `clear()` drops them all.

`ReadbackLane` tracks one buffer's `map_async` cycle with two cells. `begin` claims the lane and
refuses while a mapping is outstanding; `settle` releases the lane once the callback runs and
clears the sample flag when the mapping failed; `record_sample` marks that a value was actually
read. `has_sample` is therefore false after a failed readback, so a readout shows no reading
instead of an old number.

## Boundaries

- Depends on: `std` only on the native build; `wgpu` for `write_gpu` in the WebAssembly build.
- Used by: `symbology_layers_gpu`, whose slot symbology owns the `LanePool` its sprite-lane
  uploads write through (`crates/map_rendering/symbology_layers_gpu/src/slot_symbology/state.rs`);
  the frame timer of this crate (`crate::timing::gpu_timer`) keeps its readbacks in a
  `ReadbackLane`.
- Rules: capacity never shrinks and every size is a multiple of 4
  (`grow_capacity_doubles_and_never_shrinks`, `smaller_write_reuses_and_never_shrinks`); a second
  claim on a busy readback lane is refused and a failed map retires the old sample
  (`a_second_claim_while_one_is_outstanding_is_refused`,
  `a_failed_readback_retires_the_previous_sample`); only the crates the wgpu firewall admits
  depend on this crate (`cargo xtask verify crate-tiers`).
