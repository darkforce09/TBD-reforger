//! Role: version.
//! Position: `io/archives` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Contract version written into every archive that carries one; bump when a field's meaning changes rather than reusing a name.
pub const ARCHIVE_SCHEMA_VERSION: u16 = 1;
