//! Entry decompression under each consumer's codec and length rules.
//!
//! **Role:** turns an entry's stored bytes into its content.
//! **Position:** called by [`crate::PakIndex`] for every inflated read.
//! **Signals & state:** none; pure functions over the stored bytes.
//! **Invariants:** an uncompressed entry is returned as stored; the blueprint policy accepts only
//! zlib whose output length equals the directory's; the world policy falls back to raw deflate
//! and checks no length.

use std::io::Read;

use crate::read_policy::ReadPolicy;
use crate::{Error, PakEntry, Result};

pub(crate) fn inflate(raw: &[u8], entry: &PakEntry, policy: ReadPolicy) -> Result<Vec<u8>> {
    if !entry.compressed {
        return Ok(raw.to_vec());
    }
    let mut out = Vec::new();
    let zlib = flate2::read::ZlibDecoder::new(raw).read_to_end(&mut out);
    match policy {
        ReadPolicy::Blueprint => {
            zlib.map_err(|cause| Error::ZlibInflate {
                entry: entry.path.clone(),
                cause,
            })?;
            if out.len() != entry.decompressed_len as usize {
                return Err(Error::InflatedLength {
                    entry: entry.path.clone(),
                    inflated: out.len(),
                    declared: entry.decompressed_len,
                });
            }
        }
        ReadPolicy::World if zlib.is_err() => {
            out.clear();
            if flate2::read::DeflateDecoder::new(raw)
                .read_to_end(&mut out)
                .is_err()
            {
                let head: Vec<String> = raw.iter().take(12).map(|b| format!("{b:02x}")).collect();
                return Err(Error::InflateFailed {
                    entry: entry.path.clone(),
                    stored: raw.len(),
                    declared: entry.decompressed_len,
                    head: head.join(" "),
                });
            }
        }
        ReadPolicy::World => {}
    }
    Ok(out)
}
