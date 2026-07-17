//! T-165.8 — Enfusion `.pak` virtual filesystem (Rust port of enfusion-mcp's
//! `dist/pak/{reader,vfs}.js` — format-compatible, first-pak-wins merge, zlib inflate).
//!
//! Format: `FORM` (u32BE) + size + `PAC1`; then chunks `[magic u32BE][size u32BE][payload]`
//! (HEAD skipped, DATA = payload start recorded, FILE = recursive entry tree: u8 kind,
//! u8 nameLen, name; dir → u32LE childCount + children; file → u32LE offset/compressedLen/
//! decompressedLen, 6 B skip, u8 compressed, 5 B skip). Read = seek dataStart+offset,
//! inflate when compressed.

use std::collections::HashMap;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};

struct FileEntry {
    offset: u32,
    compressed_len: u32,
    decompressed_len: u32,
    compressed: bool,
}

struct FileRef {
    pak_path: PathBuf,
    data_start: u64,
    entry: FileEntry,
}

pub struct PakVfs {
    index: HashMap<String, FileRef>,
}

fn normalize_path(p: &str) -> String {
    let s = p.replace('\\', "/");
    let s = s.trim_matches('/');
    let mut out = String::with_capacity(s.len());
    let mut prev_slash = false;
    for c in s.chars() {
        if c == '/' {
            if !prev_slash {
                out.push(c);
            }
            prev_slash = true;
        } else {
            out.push(c);
            prev_slash = false;
        }
    }
    out
}

const MAGIC_FORM: u32 = 0x464f_524d;
const MAGIC_PAC1: u32 = 0x5041_4331;
const MAGIC_HEAD: u32 = 0x4845_4144;
const MAGIC_DATA: u32 = 0x4441_5441;
const MAGIC_FILE: u32 = 0x4649_4c45;

fn u32be(b: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
fn u32le(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

/// Parse one pak's FILE tree into (virtual path → entry) pairs + the DATA payload offset.
fn parse_pak_index(pak_path: &Path) -> Result<(Vec<(String, FileEntry)>, u64)> {
    use std::io::{Seek, SeekFrom};
    let mut f = std::fs::File::open(pak_path)?;
    let file_size = f.metadata()?.len();
    let mut read_at = |pos: u64, len: usize| -> Result<Vec<u8>> {
        f.seek(SeekFrom::Start(pos))?;
        let mut buf = vec![0u8; len];
        f.read_exact(&mut buf)
            .with_context(|| format!("EOF at {pos}+{len}"))?;
        Ok(buf)
    };

    let form = read_at(0, 12)?;
    if u32be(&form, 0) != MAGIC_FORM {
        bail!("not a PAK: missing FORM");
    }
    if u32be(&form, 8) != MAGIC_PAC1 {
        bail!("not a PAK: expected PAC1");
    }

    let mut pos = 12u64;
    let mut data_start: i64 = -1;
    let mut file_chunk: Option<(u64, u32)> = None;
    while pos + 8 <= file_size {
        let hdr = read_at(pos, 8)?;
        let magic = u32be(&hdr, 0);
        let chunk_len = u32be(&hdr, 4) as u64;
        match magic {
            MAGIC_HEAD => pos += 8 + chunk_len,
            MAGIC_DATA => {
                data_start = (pos + 8) as i64;
                pos += 8 + chunk_len;
            }
            MAGIC_FILE => {
                file_chunk = Some((pos + 8, chunk_len as u32));
                break;
            }
            _ => pos += 8 + chunk_len,
        }
    }
    if data_start < 0 {
        bail!("PAK missing DATA chunk");
    }
    let Some((file_off, file_len)) = file_chunk else {
        bail!("PAK missing FILE chunk");
    };
    let buf = read_at(file_off, file_len as usize)?;

    // Iterative walk of the recursive entry tree (root must be a dir).
    let mut files = Vec::new();
    let mut offset = 0usize;
    // stack of (path_prefix, remaining_children)
    fn parse_entry(
        buf: &[u8],
        offset: &mut usize,
        prefix: &str,
        files: &mut Vec<(String, FileEntry)>,
    ) -> Result<()> {
        let kind = buf[*offset];
        *offset += 1;
        let name_len = buf[*offset] as usize;
        *offset += 1;
        let name = String::from_utf8_lossy(&buf[*offset..*offset + name_len]).into_owned();
        *offset += name_len;
        if kind == 0 {
            let child_count = u32le(buf, *offset);
            *offset += 4;
            let child_prefix = if prefix.is_empty() {
                name
            } else if name.is_empty() {
                prefix.to_string()
            } else {
                format!("{prefix}/{name}")
            };
            for _ in 0..child_count {
                parse_entry(buf, offset, &child_prefix, files)?;
            }
        } else {
            let off = u32le(buf, *offset);
            let compressed_len = u32le(buf, *offset + 4);
            let decompressed_len = u32le(buf, *offset + 8);
            // skip unknown u32 + unk2 u16
            let compressed = buf[*offset + 18] != 0;
            // skip compression_level u8 + timestamp u32
            *offset += 24;
            let path = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            files.push((
                path,
                FileEntry {
                    offset: off,
                    compressed_len,
                    decompressed_len,
                    compressed,
                },
            ));
        }
        Ok(())
    }
    // The root entry is always a directory (often nameless).
    let root_kind = buf[0];
    if root_kind != 0 {
        bail!("PAK FILE chunk root entry is not a directory");
    }
    parse_entry(&buf, &mut offset, "", &mut files)?;
    Ok((files, data_start as u64))
}

impl PakVfs {
    /// Build the merged VFS over `<game_path>/addons/*.pak` (sorted; first pak wins).
    pub fn open(game_path: &Path) -> Result<PakVfs> {
        let addons = game_path.join("addons");
        if !addons.exists() {
            bail!("no addons/ under {}", game_path.display());
        }
        let mut paks: Vec<PathBuf> = std::fs::read_dir(&addons)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case("pak"))
            })
            .collect();
        paks.sort();
        if paks.is_empty() {
            bail!("no .pak files under {}", addons.display());
        }
        let mut index = HashMap::new();
        for pak in &paks {
            match parse_pak_index(pak) {
                Ok((files, data_start)) => {
                    for (path, entry) in files {
                        let norm = normalize_path(&path);
                        index.entry(norm).or_insert_with(|| FileRef {
                            pak_path: pak.clone(),
                            data_start,
                            entry,
                        });
                    }
                }
                Err(e) => eprintln!("pak: failed to parse {}: {e}", pak.display()),
            }
        }
        Ok(PakVfs { index })
    }

    /// The Node lane's game-path resolution: `ENFUSION_GAME_PATH` → `~/.cache/enfusion-mcp-root`.
    pub fn open_default() -> Result<PakVfs> {
        let game = std::env::var("ENFUSION_GAME_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                PathBuf::from(std::env::var("HOME").unwrap_or_default())
                    .join(".cache/enfusion-mcp-root")
            });
        PakVfs::open(&game).map_err(|e| anyhow!("No pak VFS ({e}) — set ENFUSION_GAME_PATH"))
    }

    pub fn exists(&self, virtual_path: &str) -> bool {
        self.index.contains_key(&normalize_path(virtual_path))
    }

    pub fn read_file(&self, virtual_path: &str) -> Result<Vec<u8>> {
        use std::io::{Seek, SeekFrom};
        let norm = normalize_path(virtual_path);
        let r = self
            .index
            .get(&norm)
            .ok_or_else(|| anyhow!("File not found in pak: {virtual_path}"))?;
        let read_len = if r.entry.compressed {
            r.entry.compressed_len
        } else {
            r.entry.decompressed_len
        } as usize;
        let mut f = std::fs::File::open(&r.pak_path)?;
        f.seek(SeekFrom::Start(r.data_start + u64::from(r.entry.offset)))?;
        let mut buf = vec![0u8; read_len];
        f.read_exact(&mut buf).context("truncated pak read")?;
        if r.entry.compressed {
            let mut out = Vec::with_capacity(r.entry.decompressed_len as usize);
            flate2::read::ZlibDecoder::new(&buf[..]).read_to_end(&mut out)?;
            Ok(out)
        } else {
            Ok(buf)
        }
    }

    pub fn all_file_paths(&self) -> Vec<&str> {
        self.index.keys().map(String::as_str).collect()
    }
}
