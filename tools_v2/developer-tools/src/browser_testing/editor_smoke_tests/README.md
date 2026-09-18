# Editor Smoke Tests

The parent module owns the browser harness and the fixed 21-entry `EDITOR_SUITE` order. Shared authentication, fixture serving, input, geometry, and runner helpers support scenario groups in this directory. Every production scenario file is below 450 lines. Standalone authentication, rendering, and performance checks remain available through `gate`.

Source modules: `arsenal.rs`, `cur.rs`, `editor_auth_seed.rs`, `fullmap.rs`, `marquee_drag.rs`, `mutations.rs`, `outliner_drag.rs`, `outliner_palette.rs`, `pan.rs`, `run_smoke.rs`, `save_dialog_rect.rs`, `serve_arsenal_golden.rs`, `virtual_outliner.rs`.
