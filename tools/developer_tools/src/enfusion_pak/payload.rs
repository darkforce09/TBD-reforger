//! Shared decompression with explicit per-consumer codec and length policies.
use super::{PakEntry, ReadPolicy};
use anyhow::{Context, Result, bail};
use std::io::Read;

pub(crate) fn inflate(raw: &[u8], entry: &PakEntry, policy: ReadPolicy) -> Result<Vec<u8>> {
    if !entry.compressed {
        return Ok(raw.to_vec());
    }
    let mut out = Vec::new();
    let zlib = flate2::read::ZlibDecoder::new(raw).read_to_end(&mut out);
    match policy {
        ReadPolicy::Blueprint => {
            zlib.with_context(|| format!("{}: zlib inflate failed", entry.path))?;
            if out.len() != entry.decompressed_len as usize {
                bail!(
                    "{}: inflated {} bytes, directory says {}",
                    entry.path,
                    out.len(),
                    entry.decompressed_len
                );
            }
        }
        ReadPolicy::World if zlib.is_err() => {
            out.clear();
            if flate2::read::DeflateDecoder::new(raw)
                .read_to_end(&mut out)
                .is_err()
            {
                let head: Vec<String> = raw.iter().take(12).map(|b| format!("{b:02x}")).collect();
                bail!(
                    "inflate failed as zlib and raw deflate: {} (clen={} dlen={} head={})",
                    entry.path,
                    raw.len(),
                    entry.decompressed_len,
                    head.join(" ")
                );
            }
        }
        ReadPolicy::World => {}
    }
    Ok(out)
}
