//! Bounded FORM/PAC1 chunk and directory parsing. File offsets are absolute.
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};

use super::{ReadPolicy, payload};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PakEntry {
    pub path: String,
    pub offset: u64,
    pub compressed_len: u32,
    pub decompressed_len: u32,
    pub compressed: bool,
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

#[derive(Debug)]
pub struct PakIndex {
    pub path: PathBuf,
    pub entries: Vec<PakEntry>,
    /// Metadata only; this value is never added to an entry offset.
    pub data_start: u64,
}

impl PakIndex {
    pub fn open(path: &Path) -> Result<Self> {
        Self::open_with_policy(path, ReadPolicy::Blueprint)
    }

    pub(crate) fn open_with_policy(path: &Path, policy: ReadPolicy) -> Result<Self> {
        let mut file = File::open(path).with_context(|| path.display().to_string())?;
        let (data_start, entries) =
            parse_archive(&mut file, policy).with_context(|| path.display().to_string())?;
        Ok(Self {
            path: path.to_path_buf(),
            entries,
            data_start,
        })
    }

    pub fn read(&self, entry: &PakEntry) -> Result<Vec<u8>> {
        self.read_with_policy(entry, ReadPolicy::Blueprint)
    }

    pub(crate) fn read_with_policy(&self, entry: &PakEntry, policy: ReadPolicy) -> Result<Vec<u8>> {
        payload::inflate(&self.read_raw(entry)?, entry, policy)
    }

    pub(crate) fn read_raw(&self, entry: &PakEntry) -> Result<Vec<u8>> {
        let mut file = File::open(&self.path).with_context(|| self.path.display().to_string())?;
        read_at(&mut file, entry.offset, u64::from(entry.stored_len()))
            .with_context(|| format!("{}: truncated read of {}", self.path.display(), entry.path))
    }
}

fn read_at(reader: &mut (impl Read + Seek), offset: u64, len: u64) -> Result<Vec<u8>> {
    let file_len = reader.seek(SeekFrom::End(0))?;
    ensure!(
        offset.checked_add(len).is_some_and(|end| end <= file_len),
        "EOF at {offset}+{len}"
    );
    reader.seek(SeekFrom::Start(offset))?;
    let mut bytes = vec![0; usize::try_from(len).context("pak read length exceeds address space")?];
    reader.read_exact(&mut bytes)?;
    Ok(bytes)
}

pub(crate) fn parse_archive(
    reader: &mut (impl Read + Seek),
    policy: ReadPolicy,
) -> Result<(u64, Vec<PakEntry>)> {
    let len = reader.seek(SeekFrom::End(0))?;
    let form = read_at(reader, 0, 12)?;
    ensure!(
        &form[..4] == b"FORM" && &form[8..] == b"PAC1",
        "not a FORM/PAC1 pak (magic mismatch)"
    );
    let mut pos = 12u64;
    let mut data_span = None;
    let mut tree = None;
    while pos.checked_add(8).is_some_and(|end| end <= len) {
        let header = read_at(reader, pos, 8)?;
        let size = u64::from(u32::from_be_bytes(header[4..8].try_into()?));
        let end = pos
            .checked_add(8)
            .and_then(|p| p.checked_add(size))
            .context("pak chunk size overflows")?;
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
    let (data_start, data_end) = data_span.context("pak has no DATA chunk")?;
    let tree = tree.context("pak has no FILE chunk")?;
    ensure!(
        tree.first() == Some(&0),
        "pak FILE chunk root entry is not a directory"
    );
    let entries = parse_directory(&tree)?;
    if policy == ReadPolicy::Blueprint {
        for entry in &entries {
            ensure!(
                entry.offset >= data_start
                    && entry
                        .offset
                        .checked_add(u64::from(entry.stored_len()))
                        .is_some_and(|end| end <= data_end),
                "pak entry {} at {} (+{}) lies outside the DATA chunk {data_start}..{data_end}",
                entry.path,
                entry.offset,
                entry.stored_len()
            );
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
        bail!("pak FILE tree truncated at {cursor} (+{len})");
    };
    let part = &bytes[*cursor..end];
    *cursor = end;
    Ok(part)
}
