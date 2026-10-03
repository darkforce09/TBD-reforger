# Pooled buffer and readback tests

The native unit tests of the pooled lane buffers and the readback guard: growth, reuse and the
creation count of `LanePool`, and the claims, settles and samples of `ReadbackLane`.

## Contents

```text
crates/graphics/gpu_device/src/buffers/tests/
├── pool_tests.rs      `LanePool` growth, reuse, clearing and `grow_capacity`
└── readback_tests.rs  `ReadbackLane` claims, settles and fresh samples
```

## Boundaries

- Depends on: `crate::buffers::pool` and `crate::buffers::readback`, through the
  `#[path = "tests/…"]` modules of `pool.rs` and `readback.rs`.
- Used by: `cargo test -p gpu_device`.
- Rules: native only; no GPU handle is created.
