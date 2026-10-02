//! The contract version the rkyv archives carry.
//!
//! **Role:** declares [`ARCHIVE_SCHEMA_VERSION`], written into every archive that has a
//! `schema_version` field.
//! **Position:** read by the archive writers in the developer tools and compared by the readers
//! in the map engine.
//! **Signals & state:** none; one constant.
//! **Invariants:** a change to a field's meaning raises the version instead of reusing the
//! field, and every committed archive is then written again.

/// Contract version written into every archive that carries one; bump when a field's meaning changes rather than reusing a name.
pub const ARCHIVE_SCHEMA_VERSION: u16 = 1;
