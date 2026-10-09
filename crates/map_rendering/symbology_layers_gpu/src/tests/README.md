# Symbology layers tests

The native tests of `symbology_layers_gpu`: the icon uniform block layout, the anchor conversion
and the sprite atlas table, and the error messages' stable tags. The GPU layers compile for
`wasm32` only; their bind bodies are pinned by the map engine's lane-bind source pins.

## Contents

```text
crates/map_rendering/symbology_layers_gpu/src/tests/
└── icon_uniforms_tests.rs  the uniform block offsets and packing, the anchor conversion, `sprite_atlas_for`
```
