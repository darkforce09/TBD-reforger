# Paper doll renderer tests

The native unit tests of the `paper_doll_renderer` crate: the instance stream's byte layout, its
cube-before-cylinder order, and the colour rewrites a state or hover change makes.

## Contents

```text
crates/paper_doll/paper_doll_renderer/src/tests/
└── instance_packing_tests.rs  the 80-byte layout, the launcher as the one cylinder, state and hover colour flips
```

## Boundaries

- Depends on: the crate's `instance_packing` module, `paper_doll_scene::soldier_parts` and
  `bytemuck`.
- Used by: `cargo test -p paper_doll_renderer`, through the `tests` module `instance_packing.rs`
  mounts.
- Rules: the cases run natively; the GPU half of the crate is checked in the browser by the
  readback self-check, never here.
