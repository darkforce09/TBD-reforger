//! The reader rules each asset consumer needs from one shared parser and decompressor.
//!
//! **Role:** names the two policies the archive reader, the decompressor and the path lookup
//! branch on.
//! **Position:** private to the crate; [`crate::PakSet::from_dir`] and [`crate::PakIndex`] read
//! under [`ReadPolicy::Blueprint`], [`crate::PakVfs`] under [`ReadPolicy::World`].
//! **Signals & state:** none; a plain enum.
//! **Invariants:** the two consumers differ only through this value; every policy-specific
//! branch matches on it rather than on the caller.

/// Reader semantics required by each asset consumer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReadPolicy {
    /// Case-insensitive paths, validated DATA spans, zlib with exact output length.
    Blueprint,
    /// Case-sensitive paths, zlib or raw deflate, diagnostic metadata access.
    World,
}
