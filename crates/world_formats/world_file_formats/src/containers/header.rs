//! The 32-byte header contract every binary container shares.
//!
//! **Role:** declares [`ContainerHeader`] (magic, version, name, the validating `parse` and the
//! copying `read`, and `to_header_bytes`), [`HEADER_BYTES`], [`CONTAINER_VERSION`] and
//! [`peek_version`].
//! **Position:** implemented by the four headers in [`crate::containers`]; called by every
//! container reader in the map engine and writer in the developer tools.
//! **Signals & state:** none; a trait and pure functions.
//! **Invariants:** magic is checked before version; `parse` borrows in place and refuses a
//! misaligned buffer, `read` copies and works on any buffer; a short buffer is an error, never
//! a panic.

use crate::archives::codec::BinaryError;
use bytemuck::Pod;

/// Every TBD container header is this long. Also the offset of the payload.
pub const HEADER_BYTES: usize = 32;

/// The version this build writes and reads for the three Tier-1/3 containers.
pub const CONTAINER_VERSION: u16 = 1;

/// The fixed 32-byte header a binary container starts with: its magic, its version, and the
/// readers and the writer every container shares.
pub trait ContainerHeader: Pod + Copy + Sized {
    /// The four bytes this container's files start with.
    const MAGIC: [u8; 4];

    /// The one version of this container the build reads and writes.
    const VERSION: u16;

    /// The container's name in error messages, such as `TBDC`.
    const NAME: &'static str;

    /// The magic this header holds.
    fn magic_bytes(&self) -> [u8; 4];

    /// The version this header holds.
    fn format_version(&self) -> u16;

    /// Check the magic, then the version: [`BinaryError::BadMagic`] or
    /// [`BinaryError::UnsupportedVersion`].
    fn validate(&self) -> Result<(), BinaryError> {
        if self.magic_bytes() != Self::MAGIC {
            return Err(BinaryError::BadMagic {
                what: Self::NAME,
                expected: Self::MAGIC,
                actual: self.magic_bytes(),
            });
        }
        if self.format_version() != Self::VERSION {
            return Err(BinaryError::UnsupportedVersion {
                what: Self::NAME,
                expected: Self::VERSION,
                actual: self.format_version(),
            });
        }
        Ok(())
    }

    /// Borrow the header in place and validate it, returning it with the payload after it;
    /// [`BinaryError::Misaligned`] when the buffer is not aligned for the header.
    fn parse(bytes: &[u8]) -> Result<(&Self, &[u8]), BinaryError> {
        if bytes.len() < HEADER_BYTES {
            return Err(BinaryError::Truncated {
                what: Self::NAME,
                expected: HEADER_BYTES,
                actual: bytes.len(),
            });
        }
        let (head, payload) = bytes.split_at(HEADER_BYTES);
        let header: &Self =
            bytemuck::try_from_bytes(head).map_err(|_| BinaryError::Misaligned {
                what: Self::NAME,
                align: align_of::<Self>(),
            })?;
        header.validate()?;
        Ok((header, payload))
    }

    /// Copy the header out of any buffer, aligned or not, and validate it, returning it with the
    /// payload after it; the reader every loader calls.
    fn read(bytes: &[u8]) -> Result<(Self, &[u8]), BinaryError> {
        if bytes.len() < HEADER_BYTES {
            return Err(BinaryError::Truncated {
                what: Self::NAME,
                expected: HEADER_BYTES,
                actual: bytes.len(),
            });
        }
        let (head, payload) = bytes.split_at(HEADER_BYTES);
        let header: Self = bytemuck::pod_read_unaligned(head);
        header.validate()?;
        Ok((header, payload))
    }

    /// The 32 bytes a writer puts first.
    fn to_header_bytes(&self) -> [u8; HEADER_BYTES] {
        let mut out = [0_u8; HEADER_BYTES];
        out.copy_from_slice(bytemuck::bytes_of(self));
        out
    }
}

/// Read the `version` field of any TBD container without committing to a header type.
pub fn peek_version(bytes: &[u8]) -> Result<([u8; 4], u16), BinaryError> {
    if bytes.len() < 6 {
        return Err(BinaryError::Truncated {
            what: "TBD container",
            expected: 6,
            actual: bytes.len(),
        });
    }
    let magic = [bytes[0], bytes[1], bytes[2], bytes[3]];
    Ok((magic, u16::from_le_bytes([bytes[4], bytes[5]])))
}
