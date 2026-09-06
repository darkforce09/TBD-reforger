//! T-935.1 — the one authoritative definition of every TBD map binary format.
//!
//! Emitters (`tbd-tools`, `xtask`) and loaders (the Leptos SPA) both link *these* types, so a
//! format can only ever change in one place. Nothing in this module reads or writes a file: it is
//! layout, validation and casts, which is why it carries no `flate2` and no `serde_json` — binary
//! containers are the thing that replaces gzip-JSON on the chunk hot path (audit.md Finding 1.4).
//!
//! # Three tiers (spec §1)
//!
//! | Tier | Encoding | Assets | Module |
//! |---|---|---|---|
//! | 1 | raw `#[repr(C)]` POD + `bytemuck` | chunk instances, DEM grid, density | [`pod`], [`chunk_container`] |
//! | 2 | rkyv 0.8 archives, bytecheck on | roads, labels, water, catalogue, regions, buildings, sat index | [`archives`] |
//! | 3 | mipmapped containers | satellite `.tbd-sat`, bathymetry `.tbd-bath` | [`chunk_container`] |
//!
//! # The reader contract
//!
//! **[`access_checked`] is the only public way to read a Tier-2 archive, and it validates.**
//! `rkyv::access_unchecked` appears nowhere in this crate: an archive is a *file*, files get
//! truncated, half-written and bit-rotted, and an unchecked relative-pointer chase over a corrupt
//! buffer is a wild read, not a wrong answer. Every entry point in this module — [`access_checked`],
//! [`ContainerHeader::parse`](chunk_container::ContainerHeader::parse),
//! [`instances_from_bytes`](pod::instances_from_bytes) — answers a malformed buffer with
//! [`BinaryError`], never a panic, so a bad asset degrades one layer instead of killing the frame.

pub mod archives;
pub mod chunk_container;
pub mod pod;

use core::fmt;

use rkyv::{
    Archive, Archived, Portable, Serialize,
    api::high::{HighSerializer, HighValidator},
    bytecheck::CheckBytes,
    rancor::Error as RkyvError,
    ser::allocator::ArenaHandle,
    util::AlignedVec,
};

/// Everything that can be wrong with a TBD binary buffer.
///
/// `what` is the format or type the failure is about (`"TBDC"`, `"ObjectInstancePod"`, an archive's
/// type name) — a loader juggling seven assets needs the message to say *which* file is bad.
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
    /// The buffer is well-formed but not aligned enough to reinterpret in place. Recoverable: copy
    /// into an aligned buffer and retry.
    Misaligned { what: &'static str, align: usize },
    /// The header's declared element count disagrees with the payload length.
    LengthMismatch {
        what: &'static str,
        expected: usize,
        actual: usize,
    },
    /// rkyv validation or serialisation failed. `cause` is rkyv's own rendered error, kept as a
    /// `String` so [`BinaryError`] stays `PartialEq` and free of an rkyv type in its public shape.
    Archive { what: &'static str, cause: String },
}

impl fmt::Display for BinaryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

/// Renders a magic as its ASCII when it is printable, else as raw bytes — a corrupt header is
/// usually recognisable at a glance (`"vers"` is a git-LFS pointer, for instance).
struct DisplayMagic([u8; 4]);

impl fmt::Debug for DisplayMagic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.iter().all(|b| b.is_ascii_graphic()) {
            write!(f, "b\"{}\"", self.0.escape_ascii())
        } else {
            write!(f, "{:?}", self.0)
        }
    }
}

/// **The only reader entry point for a Tier-2 rkyv archive**, and it is the *validating* one.
///
/// Wraps [`rkyv::access`], which walks the buffer with bytecheck before handing back a reference:
/// a truncated file, a flipped bit in a relative pointer, a length that would run off the end all
/// come back as [`BinaryError::Archive`]. The unchecked twin (`rkyv::access_unchecked`) is
/// deliberately not re-exported anywhere in this crate.
///
/// Zero-copy: the returned `&Archived<T>` borrows `bytes`, nothing is deserialised or allocated.
/// `bytes` must be aligned for the archive (an [`AlignedVec`] from [`to_bytes`], or the aligned
/// buffer a loader reads a file into); misalignment is reported, not assumed away.
///
/// # Errors
/// [`BinaryError::Archive`] when the buffer fails rkyv validation for any reason.
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
///
/// Returns rkyv's [`AlignedVec`] rather than a `Vec<u8>` on purpose: the archive's root sits at the
/// *end* of the buffer at an alignment [`access_checked`] then requires, so handing back a
/// `Vec<u8>` would invite a round-trip that fails to re-read what it just wrote.
///
/// # Errors
/// [`BinaryError::Archive`] when rkyv cannot serialise the value.
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
mod tests {
    use super::*;

    #[test]
    fn magic_renders_as_ascii_when_printable() {
        let e = BinaryError::BadMagic {
            what: "TBDC",
            expected: *b"TBDC",
            actual: *b"vers",
        };
        let s = e.to_string();
        assert!(s.contains("b\"TBDC\""), "{s}");
        assert!(s.contains("b\"vers\""), "{s}");
    }

    #[test]
    fn magic_renders_as_bytes_when_not_printable() {
        let e = BinaryError::BadMagic {
            what: "TBDE",
            expected: *b"TBDE",
            actual: [0, 1, 2, 3],
        };
        assert!(e.to_string().contains("[0, 1, 2, 3]"), "{e}");
    }

    #[test]
    fn every_variant_renders_and_is_an_error() {
        let cases = [
            BinaryError::Truncated {
                what: "TBDC",
                expected: 32,
                actual: 4,
            },
            BinaryError::UnsupportedVersion {
                what: "TBDC",
                expected: 1,
                actual: 9,
            },
            BinaryError::Misaligned {
                what: "ObjectInstancePod",
                align: 4,
            },
            BinaryError::LengthMismatch {
                what: "TBDC",
                expected: 64,
                actual: 63,
            },
            BinaryError::Archive {
                what: "RoadNetworkArchive",
                cause: "boom".to_string(),
            },
        ];
        for c in &cases {
            let s = c.to_string();
            assert!(!s.is_empty(), "{c:?} rendered empty");
            let _: &dyn core::error::Error = c;
        }
    }
}
