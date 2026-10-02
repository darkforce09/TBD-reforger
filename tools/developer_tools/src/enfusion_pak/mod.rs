//! FORM/PAC1 archive access with explicit blueprint and world-tooling policies.
mod archive_reader;
mod loose_source;
mod payload;
mod virtual_filesystem;
mod world_source;

pub use archive_reader::{PakEntry, PakIndex};
pub use loose_source::{DirSource, LayeredSource};
pub use virtual_filesystem::{AssetSource, PakSet, normalize_path};
pub use world_source::PakVfs;

/// Reader semantics required by each asset consumer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReadPolicy {
    /// Case-insensitive paths, validated DATA spans, zlib with exact output length.
    Blueprint,
    /// Case-sensitive paths, zlib or raw deflate, diagnostic metadata access.
    World,
}

#[cfg(test)]
fn parse_pak_bytes(bytes: &[u8]) -> anyhow::Result<Vec<PakEntry>> {
    Ok(archive_reader::parse_archive(&mut std::io::Cursor::new(bytes), ReadPolicy::Blueprint)?.1)
}

#[cfg(test)]
fn inflate_entry(bytes: &[u8], entry: &PakEntry) -> anyhow::Result<Vec<u8>> {
    payload::inflate(bytes, entry, ReadPolicy::Blueprint)
}

#[cfg(test)]
#[path = "tests/blueprint_source.rs"]
mod blueprint_source_tests;

#[cfg(test)]
#[path = "tests/policy_parity.rs"]
mod policy_parity_tests;
