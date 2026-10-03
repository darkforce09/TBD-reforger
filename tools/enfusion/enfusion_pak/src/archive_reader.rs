//! Bounded `FORM`/`PAC1` chunk and directory parsing, and payload reads.
//!
//! **Role:** opens one archive into a [`PakIndex`] of [`PakEntry`] records and reads an entry's
//! stored or inflated bytes.
//! **Position:** under [`crate::PakSet`] and [`crate::PakVfs`], which merge indexes; the inflation
//! itself is the crate's payload module.
//! **Signals & state:** none; each read opens the archive file again.
//! **Invariants:** file offsets are absolute and `data_start` is never added to them; every read
//! is bounds-checked against the file length; the directory tree is walked with an explicit stack,
//! so a malformed tree cannot exhaust the call stack; under the blueprint policy every entry lies
//! inside the `DATA` chunk.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::payload;
use crate::read_policy::ReadPolicy;
use crate::{Error, Result};

/// One file record of an archive's `FILE` directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PakEntry {
    /// The entry's `/`-separated path inside the archive.
    pub path: String,
    /// The absolute file offset of the stored bytes.
    pub offset: u64,
    /// The stored length when the entry is compressed.
    pub compressed_len: u32,
    /// The length after inflation (the stored length of an uncompressed entry).
    pub decompressed_len: u32,
    /// Whether the stored bytes are compressed.
    pub compressed: bool,
    /// The six-byte method tag of the record.
    pub method: [u8; 6],
}

impl PakEntry {
    pub(crate) fn stored_len(&self) -> u32 {
        if self.compressed {
            self.compressed_len
        } else {
            self.decompressed_len
        }
    }
}

/// The parsed directory of one archive file.
#[derive(Debug)]
pub struct PakIndex {
    /// The archive file.
    pub path: PathBuf,
    /// Every file record, in directory order.
    pub entries: Vec<PakEntry>,
    /// Metadata only; this value is never added to an entry offset.
    pub data_start: u64,
}

impl PakIndex {
    /// Open and parse `path` under the blueprint policy.
    pub fn open(path: &Path) -> Result<Self> {
        Self::open_with_policy(path, ReadPolicy::Blueprint)
    }

    pub(crate) fn open_with_policy(path: &Path, policy: ReadPolicy) -> Result<Self> {
        let mut file = File::open(path).map_err(|cause| Error::File {
            path: path.to_path_buf(),
            cause,
        })?;
        let (data_start, entries) =
            parse_archive(&mut file, policy).map_err(|cause| Error::Archive {
                archive: path.to_path_buf(),
                cause: Box::new(cause),
            })?;
        Ok(Self {
            path: path.to_path_buf(),
            entries,
            data_start,
        })
    }

    /// Read `entry` and inflate it under the blueprint policy.
    pub fn read(&self, entry: &PakEntry) -> Result<Vec<u8>> {
        self.read_with_policy(entry, ReadPolicy::Blueprint)
    }

    pub(crate) fn read_with_policy(&self, entry: &PakEntry, policy: ReadPolicy) -> Result<Vec<u8>> {
        payload::inflate(&self.read_raw(entry)?, entry, policy)
    }

    pub(crate) fn read_raw(&self, entry: &PakEntry) -> Result<Vec<u8>> {
        let mut file = File::open(&self.path).map_err(|cause| Error::File {
            path: self.path.clone(),
            cause,
        })?;
        read_at(&mut file, entry.offset, u64::from(entry.stored_len())).map_err(|cause| {
            Error::TruncatedRead {
                archive: self.path.clone(),
                entry: entry.path.clone(),
                cause: Box::new(cause),
            }
        })
    }
}

fn read_at(reader: &mut (impl Read + Seek), offset: u64, len: u64) -> Result<Vec<u8>> {
    let file_len = reader.seek(SeekFrom::End(0))?;
    if offset.checked_add(len).is_none_or(|end| end > file_len) {
        return Err(Error::EndOfFile { offset, len });
    }
    reader.seek(SeekFrom::Start(offset))?;
    let len = usize::try_from(len).map_err(|cause| Error::ReadLengthOverflow { cause })?;
    let mut bytes = vec![0; len];
    reader.read_exact(&mut bytes)?;
    Ok(bytes)
}

pub(crate) fn parse_archive(
    reader: &mut (impl Read + Seek),
    policy: ReadPolicy,
) -> Result<(u64, Vec<PakEntry>)> {
    let len = reader.seek(SeekFrom::End(0))?;
    let form = read_at(reader, 0, 12)?;
    if !(&form[..4] == b"FORM" && &form[8..] == b"PAC1") {
        return Err(Error::MagicMismatch);
    }
    let mut pos = 12u64;
    let mut data_span = None;
    let mut tree = None;
    while pos.checked_add(8).is_some_and(|end| end <= len) {
        let header = read_at(reader, pos, 8)?;
        let size = u64::from(u32::from_be_bytes(header[4..8].try_into()?));
        let end = pos
            .checked_add(8)
            .and_then(|p| p.checked_add(size))
            .ok_or(Error::ChunkSizeOverflow)?;
        match &header[..4] {
            b"DATA" => data_span = Some((pos + 8, end)),
            b"FILE" => {
                tree = Some(read_at(reader, pos + 8, size)?);
                break;
            }
            _ => {}
        }
        pos = end;
    }
    let (data_start, data_end) = data_span.ok_or(Error::MissingDataChunk)?;
    let tree = tree.ok_or(Error::MissingFileChunk)?;
    if tree.first() != Some(&0) {
        return Err(Error::RootNotDirectory);
    }
    let entries = parse_directory(&tree)?;
    if policy == ReadPolicy::Blueprint {
        for entry in &entries {
            let inside = entry.offset >= data_start
                && entry
                    .offset
                    .checked_add(u64::from(entry.stored_len()))
                    .is_some_and(|end| end <= data_end);
            if !inside {
                return Err(Error::EntryOutsideData {
                    entry: entry.path.clone(),
                    offset: entry.offset,
                    stored_len: entry.stored_len(),
                    data_start,
                    data_end,
                });
            }
        }
    }
    Ok((data_start, entries))
}

/// Iterative directory traversal bounds stack usage even for malformed nested trees.
fn parse_directory(tree: &[u8]) -> Result<Vec<PakEntry>> {
    let mut cursor = 0;
    let mut stack = vec![(String::new(), 1u32)];
    let mut entries = Vec::new();
    while let Some((prefix, remaining)) = stack.last_mut() {
        if *remaining == 0 {
            stack.pop();
            continue;
        }
        *remaining -= 1;
        let kind = take(tree, &mut cursor, 1)?[0];
        let name_len = take(tree, &mut cursor, 1)?[0] as usize;
        let name = String::from_utf8_lossy(take(tree, &mut cursor, name_len)?);
        let path = if prefix.is_empty() {
            name.into_owned()
        } else if name.is_empty() {
            prefix.clone()
        } else {
            format!("{prefix}/{name}")
        };
        if kind == 0 {
            let count = u32::from_le_bytes(take(tree, &mut cursor, 4)?.try_into()?);
            stack.push((path, count));
        } else {
            let record = take(tree, &mut cursor, 24)?;
            entries.push(PakEntry {
                path,
                offset: u64::from(u32::from_le_bytes(record[..4].try_into()?)),
                compressed_len: u32::from_le_bytes(record[4..8].try_into()?),
                decompressed_len: u32::from_le_bytes(record[8..12].try_into()?),
                method: record[12..18].try_into()?,
                compressed: record[18] != 0,
            });
        }
    }
    Ok(entries)
}

fn take<'a>(bytes: &'a [u8], cursor: &mut usize, len: usize) -> Result<&'a [u8]> {
    let Some(end) = cursor.checked_add(len).filter(|end| *end <= bytes.len()) else {
        return Err(Error::DirectoryTruncated {
            cursor: *cursor,
            len,
        });
    };
    let part = &bytes[*cursor..end];
    *cursor = end;
    Ok(part)
}
