//! Read-only access to Enfusion `.pak` archives and loose extracted folders.
//!
//! **Role:** parses the `FORM`/`PAC1` archives an Enfusion install ships under `addons/`
//! ([`PakIndex`], [`PakEntry`]), merges a folder of them into one virtual file system ([`PakSet`],
//! [`PakVfs`]), resolves loose folders and ordered fallbacks ([`DirSource`], [`LayeredSource`]),
//! and serves all of them through [`AssetSource`].
//! **Position:** tier 0 of `tools/enfusion`, with no workspace dependency. The building-blueprint
//! compiler reads through [`PakSet`] and the loose sources; the world export and map raster
//! pipelines and the `enf` command line of `enfusion_script_index` read through [`PakVfs`].
//! **Signals & state:** none held between calls; each read opens the archive file it needs.
//! **Invariants:** one parser and one decompressor serve both consumers, which differ only
//! through the crate-private read policy; offsets are absolute file offsets; a later archive never
//! shadows a path an earlier one holds; every failure is an [`Error`], never a panic.

mod archive_reader;
mod error;
mod loose_source;
mod payload;
pub mod prelude;
mod read_policy;
mod virtual_filesystem;
mod world_source;

pub use archive_reader::{PakEntry, PakIndex};
pub use error::{Error, Result};
pub use loose_source::{DirSource, LayeredSource};
pub use virtual_filesystem::{AssetSource, PakSet, normalize_path};
pub use world_source::PakVfs;

#[cfg(test)]
#[path = "tests/blueprint_source.rs"]
mod blueprint_source_tests;

#[cfg(test)]
#[path = "tests/policy_parity.rs"]
mod policy_parity_tests;
