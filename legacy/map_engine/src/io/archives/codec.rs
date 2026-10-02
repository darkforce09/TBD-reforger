//! Role: codec.
//! Position: `io/archives` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use rkyv::{
    Archive, Archived, Portable, Serialize,
    api::high::{HighSerializer, HighValidator},
    bytecheck::CheckBytes,
    rancor::Error as RkyvError,
    ser::allocator::ArenaHandle,
    util::AlignedVec,
};

/// Everything that can be wrong with a TBD binary buffer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BinaryError {
    /// Fewer bytes than the fixed part of the format needs.
    Truncated {
        what: &'static str,
        expected: usize,
        actual: usize,
    },

    /// The four magic bytes are not this format's.
    BadMagic {
        what: &'static str,
        expected: [u8; 4],
        actual: [u8; 4],
    },

    /// Right format, wrong version — the caller must dispatch, not guess.
    UnsupportedVersion {
        what: &'static str,
        expected: u16,
        actual: u16,
    },

    /// The buffer is well-formed but not aligned enough to reinterpret in place. Recoverable: copy into an aligned buffer and retry.
    Misaligned { what: &'static str, align: usize },

    /// The header's declared element count disagrees with the payload length.
    LengthMismatch {
        what: &'static str,
        expected: usize,
        actual: usize,
    },

    /// rkyv validation or serialisation failed. `cause` is rkyv's own rendered error, kept as a `String` so [`BinaryError`] stays `PartialEq` and free of an rkyv type in its public shape.
    Archive { what: &'static str, cause: String },
}

impl core::fmt::Display for BinaryError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Truncated {
                what,
                expected,
                actual,
            } => write!(f, "{what}: truncated — need {expected} bytes, got {actual}"),
            Self::BadMagic {
                what,
                expected,
                actual,
            } => write!(
                f,
                "{what}: bad magic — expected {:?}, got {:?}",
                DisplayMagic(*expected),
                DisplayMagic(*actual)
            ),
            Self::UnsupportedVersion {
                what,
                expected,
                actual,
            } => write!(
                f,
                "{what}: unsupported version {actual} (this build reads version {expected})"
            ),
            Self::Misaligned { what, align } => write!(
                f,
                "{what}: buffer is not {align}-byte aligned; copy it into an aligned buffer first"
            ),
            Self::LengthMismatch {
                what,
                expected,
                actual,
            } => write!(
                f,
                "{what}: payload length mismatch — header implies {expected} bytes, got {actual}"
            ),
            Self::Archive { what, cause } => write!(f, "{what}: rkyv archive error: {cause}"),
        }
    }
}

impl core::error::Error for BinaryError {}

struct DisplayMagic([u8; 4]);

impl core::fmt::Debug for DisplayMagic {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.0.iter().all(|b| b.is_ascii_graphic()) {
            write!(f, "b\"{}\"", self.0.escape_ascii())
        } else {
            write!(f, "{:?}", self.0)
        }
    }
}

/// **The only reader entry point for a Tier-2 rkyv archive**, and it is the *validating* one.
pub fn access_checked<T>(bytes: &[u8]) -> Result<&Archived<T>, BinaryError>
where
    T: Archive,
    Archived<T>: Portable + for<'a> CheckBytes<HighValidator<'a, RkyvError>>,
{
    rkyv::access::<Archived<T>, RkyvError>(bytes).map_err(|cause| BinaryError::Archive {
        what: core::any::type_name::<T>(),
        cause: cause.to_string(),
    })
}

/// Serialise a Tier-2 archive to its on-disk bytes.
pub fn to_bytes<T>(value: &T) -> Result<AlignedVec, BinaryError>
where
    T: for<'a> Serialize<HighSerializer<AlignedVec, ArenaHandle<'a>, RkyvError>>,
{
    rkyv::to_bytes::<RkyvError>(value).map_err(|cause| BinaryError::Archive {
        what: core::any::type_name::<T>(),
        cause: cause.to_string(),
    })
}

#[cfg(test)]
#[path = "tests/codec_tests.rs"]
mod tests;
