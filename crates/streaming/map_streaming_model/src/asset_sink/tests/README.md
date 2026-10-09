# Map asset sink tests

Unit tests of the shared sink handle over a sink that records every write: a booted slot and a
refused write.

## Contents

```text
crates/streaming/map_streaming_model/src/asset_sink/tests/
└── shared_sink_tests.rs  the recording sink and the two handle cases
```

## Boundaries

- Depends on: the parent module's surface through `use super::*`; `render_primitives` payloads.
- Used by: nothing outside the folder; `src/asset_sink/mod.rs` compiles it only in test builds.
- Rules:
  - a renderer cell coerces to the handle and every write lands in that cell, in call order
    (`writes_through_the_shared_handle_reach_the_renderer_cell_in_call_order`);
  - a refused write names the refused call and the renderer's reason
    (`a_refused_write_names_the_refused_call`).
