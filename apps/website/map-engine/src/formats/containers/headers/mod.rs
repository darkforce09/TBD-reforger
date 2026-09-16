//! Role: Module boundary for formats/containers/headers.
//! Position: `formats/containers/headers` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Re-export `crate::formats::containers::header::CONTAINER_VERSION`.
pub use crate::formats::containers::header::CONTAINER_VERSION;

/// Re-export `crate::formats::containers::header::ContainerHeader`.
pub use crate::formats::containers::header::ContainerHeader;

/// Re-export `crate::formats::containers::header::HEADER_BYTES`.
pub use crate::formats::containers::header::HEADER_BYTES;

/// Re-export `crate::formats::containers::header::peek_version`.
pub use crate::formats::containers::header::peek_version;

/// Re-export `crate::formats::containers::tbdc::TBDC_MAGIC`.
pub use crate::formats::containers::tbdc::TBDC_MAGIC;

/// Re-export `crate::formats::containers::tbdc::TbdcHeader`.
pub use crate::formats::containers::tbdc::TbdcHeader;

/// Re-export `crate::formats::containers::tbde::TBDE_MAGIC`.
pub use crate::formats::containers::tbde::TBDE_MAGIC;

/// Re-export `crate::formats::containers::tbde::TbdeHeader`.
pub use crate::formats::containers::tbde::TbdeHeader;

/// Re-export `crate::formats::containers::tbdb::LevelSpan`.
pub use crate::formats::containers::tbdb::LevelSpan;

/// Re-export `crate::formats::containers::tbdb::TBDB_MAGIC`.
pub use crate::formats::containers::tbdb::TBDB_MAGIC;

/// Re-export `crate::formats::containers::tbdb::TbdbHeader`.
pub use crate::formats::containers::tbdb::TbdbHeader;

/// Re-export `crate::formats::containers::tbds::TBDS_MAGIC`.
pub use crate::formats::containers::tbds::TBDS_MAGIC;

/// Re-export `crate::formats::containers::tbds::TBDS_VERSION_V2`.
pub use crate::formats::containers::tbds::TBDS_VERSION_V2;

/// Re-export `crate::formats::containers::tbds::TbdsHeader`.
pub use crate::formats::containers::tbds::TbdsHeader;
#[cfg(test)]
mod tests;
