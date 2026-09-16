//! Role: header.
//! Position: `io/containers` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::io::archives::codec::BinaryError;
use bytemuck::Pod;

/// Every TBD container header is this long. Also the offset of the payload.
pub const HEADER_BYTES: usize = 32;

/// The version this build writes and reads for the three Tier-1/3 containers.
pub const CONTAINER_VERSION: u16 = 1;

pub trait ContainerHeader: Pod + Copy + Sized {
    const MAGIC: [u8; 4];

    const VERSION: u16;

    const NAME: &'static str;

    fn magic_bytes(&self) -> [u8; 4];

    fn format_version(&self) -> u16;

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
