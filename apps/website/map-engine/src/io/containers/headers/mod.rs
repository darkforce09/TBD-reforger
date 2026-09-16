//! Role: Module boundary for formats/containers/headers.
//! Position: `io/containers/headers` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Re-export `crate::io::containers::header::CONTAINER_VERSION`.
pub use crate::io::containers::header::CONTAINER_VERSION;

/// Re-export `crate::io::containers::header::ContainerHeader`.
pub use crate::io::containers::header::ContainerHeader;

/// Re-export `crate::io::containers::header::HEADER_BYTES`.
pub use crate::io::containers::header::HEADER_BYTES;

/// Re-export `crate::io::containers::header::peek_version`.
pub use crate::io::containers::header::peek_version;

/// Re-export `crate::io::containers::tbdc::TBDC_MAGIC`.
pub use crate::io::containers::tbdc::TBDC_MAGIC;

/// Re-export `crate::io::containers::tbdc::TbdcHeader`.
pub use crate::io::containers::tbdc::TbdcHeader;

/// Re-export `crate::io::containers::tbde::TBDE_MAGIC`.
pub use crate::io::containers::tbde::TBDE_MAGIC;

/// Re-export `crate::io::containers::tbde::TbdeHeader`.
pub use crate::io::containers::tbde::TbdeHeader;

/// Re-export `crate::io::containers::tbdb::LevelSpan`.
pub use crate::io::containers::tbdb::LevelSpan;

/// Re-export `crate::io::containers::tbdb::TBDB_MAGIC`.
pub use crate::io::containers::tbdb::TBDB_MAGIC;

/// Re-export `crate::io::containers::tbdb::TbdbHeader`.
pub use crate::io::containers::tbdb::TbdbHeader;

/// Re-export `crate::io::containers::tbds::TBDS_MAGIC`.
pub use crate::io::containers::tbds::TBDS_MAGIC;

/// Re-export `crate::io::containers::tbds::TBDS_VERSION_V2`.
pub use crate::io::containers::tbds::TBDS_VERSION_V2;

/// Re-export `crate::io::containers::tbds::TbdsHeader`.
pub use crate::io::containers::tbds::TbdsHeader;
#[cfg(test)]
mod tests;
