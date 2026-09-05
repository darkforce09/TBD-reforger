# T-938.1 — Plan

## Context
engine.rs:4907 upload_slot_role_lane copies bytes (:4921) and calls create_buffer_init (:4924) on every call;
seven call sites (4565/4577/4610/4627/4642/4693/4870). Audit's :4024-4039 is cluster packing upstream.

## Approach
1. Verify on main: test-only creation counter over two equal uploads → paste the red.
2. `buffer_pool.rs` (new, in lib.rs): LanePool, write(device, queue, lane, bytes), doubling growth.
3. engine.rs: route the function through the pool; rebuild bind groups only on buffer change.
4. Perturbation: grow at old size → growth test red; restore, touch, green.

## Risks
- Usage flags and alignment must match the old buffers — copy them from the create_buffer_init call.
- Bind group caching keyed by buffer identity.

## Verification
- `cargo test -p map-engine-render`; `cargo xtask mk ci-local-leptos`
- `cargo xtask platform wave gate --slice T-938.1`
