# World layers tests

The native tests of `world_layers_gpu`: the error messages' stable call tags and reason texts,
the single-bitmap raster rule, the basemap mode's report spellings and codes, and the textured
quad's anchor-relative rectangle and pipeline choice. The GPU layers compile for `wasm32` only.

## Contents

```text
crates/map_rendering/world_layers_gpu/src/tests/
├── basemap_mode_tests.rs   the report spellings and the mode codes of `BasemapMode`
├── error_tests.rs          the call tags and reason texts of `Error`'s messages
├── raster_layout_tests.rs  the order and the refusals of the single-bitmap raster check
└── textured_quad_tests.rs  the anchor-relative rectangle and the textured lane's pipeline
```
