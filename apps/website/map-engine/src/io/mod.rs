//! Role: Module boundary for io.
//! Position: `io` in the map engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: **the byte format is frozen.** rkyv stays `little_endian` + `bytecheck`; a
//! committed `.bvh` sidecar or map-asset fixture written before this rename must still load
//! after it. T-0xx Phase 2B renamed the directory `formats/` → `io/` and changed no byte.

/// Archives.
#[cfg(feature = "io")]
pub mod archives;

/// Containers.
#[cfg(feature = "io")]
pub mod containers;

/// Density.
pub mod density;

/// Pod.
#[cfg(feature = "io")]
pub mod pod;
