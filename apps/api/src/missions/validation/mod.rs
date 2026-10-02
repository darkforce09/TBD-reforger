//! Write-boundary predicates for the mission domain.
//!
//! Each module here answers one question about authored input before a row is written, and every
//! one of them REJECTS rather than repairs: normalising one side of a column that two sites write
//! is how the two sites come to disagree. The handlers own the HTTP shape; these own the accept
//! set, so CREATE and PATCH cannot drift apart.

pub mod access;
pub mod mission_fields;
pub mod semver;
pub mod version_payload;
