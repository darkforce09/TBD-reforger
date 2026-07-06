//! DEM (digital elevation model) math. Phase 1 adds `downsample`, `hillshade`, `png`; Phase 0
//! seeds `sample` (the `uint16 → meters` core) to prove the JS↔wasm typed-array boundary.

pub mod sample;
