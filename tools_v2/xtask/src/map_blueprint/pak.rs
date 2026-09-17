//! Reforger `.pak` reader — the FORM/PAC1 IFF container the game ships its data in, read
//! natively so `map bvh-batch` can resolve a prefab closure (`.et` → `.xob`) straight from
//! `~/.cache/enfusion-mcp-root/addons/*.pak` with no Windows unpacker in the loop
//! (T-090.11.2). Layout knowledge follows the enfusion-mcp Node server's
//! `dist/pak/reader.js` + `vfs.js` (an independent Rust implementation; no code copied):
//!
//! - `FORM` · u32 BE form size · `PAC1`, then chunks `<id 4><size u32 BE><payload>`:
//!   `HEAD` (skipped), `DATA` (the file payloads), `FILE` (the directory tree, last chunk
//!   of interest).
//! - `FILE` tree entry: `kind u8` (0 = dir, else file) · `name_len u8` · name bytes; a dir
//!   continues with `u32 LE child_count` + children; a file with `u32 LE offset`,
//!   `u32 LE compressed_len`, `u32 LE decompressed_len`, 6 unknown bytes, `u8 compressed`,
//!   5 more (compression level + timestamp). Compressed payloads are zlib streams.
//! - **File offsets are absolute** (from the start of the `.pak`), NOT relative to the
//!   `DATA` payload: the farmhouse XOB's `FORM` header sits exactly at its entry offset,
//!   56 bytes before `DATA`-payload-relative would put it. (The MCP server adds the payload
//!   start and so returns every file 56 bytes late — which is why its `game_read` prefab
//!   dumps begin mid-path; the extracted files on disk are the oracle here.)
//! - Several paks are merged into one virtual tree; sorted pak order, first wins on a
//!   duplicate path (the MCP server's rule, so both tools see the same file).
//!
//! Extracted BI files are never committed: this module only READS the operator's local
//! game install; everything derived from it (`.bvh`, `.instances.json`) is what lands.

use std::collections::HashMap;
use std::fs;
use std::io::{Read as _, Seek as _, SeekFrom};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

const MAGIC_FORM: &[u8; 4] = b"FORM";
const MAGIC_PAC1: &[u8; 4] = b"PAC1";
const CHUNK_HEAD: &[u8; 4] = b"HEAD";
const CHUNK_DATA: &[u8; 4] = b"DATA";
const CHUNK_FILE: &[u8; 4] = b"FILE";

/// One file inside a pak. `offset` is absolute in the pak file (as stored), so a reader
/// seeks straight to it; `parse_pak_bytes` checks it lands inside the `DATA` chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PakEntry {
    /// Virtual path, forward slashes, no leading slash (`Prefabs/…/X.et`).
    pub path: String,
    pub offset: u64,
    pub compressed_len: u32,
    pub decompressed_len: u32,
    pub compressed: bool,
}

/// The parsed directory of one `.pak`.
#[derive(Debug)]
pub struct PakIndex {
    pub path: PathBuf,
    pub entries: Vec<PakEntry>,
}

fn u32be(b: &[u8]) -> u32 {
    u32::from_be_bytes([b[0], b[1], b[2], b[3]])
}
fn u32le(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

/// Parse the chunk skeleton + FILE tree of an in-memory pak image. Pure so the synthetic
/// test image goes through the same code as the real files.
pub fn parse_pak_bytes(bytes: &[u8]) -> Result<Vec<PakEntry>> {
    if bytes.len() < 12 || &bytes[0..4] != MAGIC_FORM || &bytes[8..12] != MAGIC_PAC1 {
        bail!("not a FORM/PAC1 pak (magic mismatch)");
    }
    let mut pos = 12usize;
    let mut data_span: Option<(u64, u64)> = None;
    let mut file_chunk: Option<(usize, usize)> = None;
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = u32be(&bytes[pos + 4..pos + 8]) as usize;
        if id == CHUNK_DATA {
            data_span = Some(((pos + 8) as u64, (pos + 8 + size) as u64));
        } else if id == CHUNK_FILE {
            file_chunk = Some((pos + 8, size));
            break;
        } else if id != CHUNK_HEAD {
            // Unknown chunk: skipped, like the reference reader.
        }
        pos = pos
            .checked_add(8 + size)
            .context("pak chunk size overflows")?;
    }
    let (data_start, data_end) = data_span.context("pak has no DATA chunk")?;
    let (file_off, file_len) = file_chunk.context("pak has no FILE chunk")?;
    if file_off + file_len > bytes.len() {
        bail!("pak FILE chunk runs past the end of the file");
    }
    let tree = &bytes[file_off..file_off + file_len];
    let mut entries = Vec::new();
    let mut cursor = 0usize;
    let root_kind = *tree.first().context("empty FILE chunk")?;
    if root_kind != 0 {
        bail!("pak FILE chunk root entry is not a directory");
    }
    parse_entry(tree, &mut cursor, "", &mut entries)?;
    // Offsets are absolute; a DATA-relative writer would land below the payload start.
    // (A DATA chunk over 4 GB wraps its u32 size — only the lower bound is checked then.)
    for e in &entries {
        let stored = if e.compressed {
            u64::from(e.compressed_len)
        } else {
            u64::from(e.decompressed_len)
        };
        if e.offset < data_start || (data_end > data_start && e.offset + stored > data_end) {
            bail!(
                "pak entry {} at {} (+{stored}) lies outside the DATA chunk {data_start}..{data_end}",
                e.path,
                e.offset
            );
        }
    }
    Ok(entries)
}

fn parse_entry(
    tree: &[u8],
    cursor: &mut usize,
    prefix: &str,
    out: &mut Vec<PakEntry>,
) -> Result<()> {
    let need = |c: usize, n: usize| -> Result<()> {
        if c + n > tree.len() {
            bail!("pak FILE tree truncated at {c} (+{n})");
        }
        Ok(())
    };
    need(*cursor, 2)?;
    let kind = tree[*cursor];
    let name_len = tree[*cursor + 1] as usize;
    *cursor += 2;
    need(*cursor, name_len)?;
    let name = String::from_utf8_lossy(&tree[*cursor..*cursor + name_len]).into_owned();
    *cursor += name_len;
    let path = if prefix.is_empty() {
        name
    } else if name.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix}/{name}")
    };
    if kind == 0 {
        need(*cursor, 4)?;
        let count = u32le(&tree[*cursor..]) as usize;
        *cursor += 4;
        for _ in 0..count {
            parse_entry(tree, cursor, &path, out)?;
        }
    } else {
        need(*cursor, 24)?;
        let offset = u64::from(u32le(&tree[*cursor..]));
        let compressed_len = u32le(&tree[*cursor + 4..]);
        let decompressed_len = u32le(&tree[*cursor + 8..]);
        let compressed = tree[*cursor + 18] != 0;
        *cursor += 24;
        out.push(PakEntry {
            path,
            offset,
            compressed_len,
            decompressed_len,
            compressed,
        });
    }
    Ok(())
}

impl PakIndex {
    /// Parse one `.pak`'s directory (the whole file is read once; paks are ≤ a few hundred MB
    /// and the FILE chunk sits at the end, so a streaming reader would buy little).
    pub fn open(path: &Path) -> Result<Self> {
        let bytes = fs::read(path).with_context(|| path.display().to_string())?;
        let entries = parse_pak_bytes(&bytes).with_context(|| path.display().to_string())?;
        Ok(Self {
            path: path.to_path_buf(),
            entries,
        })
    }

    /// Read + inflate one entry.
    pub fn read(&self, entry: &PakEntry) -> Result<Vec<u8>> {
        let mut f = fs::File::open(&self.path).with_context(|| self.path.display().to_string())?;
        f.seek(SeekFrom::Start(entry.offset))?;
        let len = if entry.compressed {
            entry.compressed_len
        } else {
            entry.decompressed_len
        } as usize;
        let mut raw = vec![0u8; len];
        f.read_exact(&mut raw).with_context(|| {
            format!("{}: truncated read of {}", self.path.display(), entry.path)
        })?;
        inflate_entry(&raw, entry)
    }
}

/// Inflate (or pass through) one entry's raw bytes; checks the decompressed length.
pub fn inflate_entry(raw: &[u8], entry: &PakEntry) -> Result<Vec<u8>> {
    if !entry.compressed {
        return Ok(raw.to_vec());
    }
    let mut out = Vec::with_capacity(entry.decompressed_len as usize);
    flate2::read::ZlibDecoder::new(raw)
        .read_to_end(&mut out)
        .with_context(|| format!("{}: zlib inflate failed", entry.path))?;
    if out.len() != entry.decompressed_len as usize {
        bail!(
            "{}: inflated {} bytes, directory says {}",
            entry.path,
            out.len(),
            entry.decompressed_len
        );
    }
    Ok(out)
}

/// Anything that can hand back a game file by virtual path — the pak set for the real
/// pipeline, a loose directory for tests and the operator's hand-extracted tree.
pub trait AssetSource {
    fn read(&self, rel_path: &str) -> Result<Vec<u8>>;
    fn exists(&self, rel_path: &str) -> bool;
    fn read_text(&self, rel_path: &str) -> Result<String> {
        Ok(String::from_utf8_lossy(&self.read(rel_path)?).into_owned())
    }
}

/// Every `.pak` under one directory, merged: sorted file order, first wins on a duplicate
/// path. Lookups are case-insensitive (prefab references are not always cased like the
/// directory entry).
#[derive(Debug)]
pub struct PakSet {
    paks: Vec<PakIndex>,
    lookup: HashMap<String, (usize, usize)>,
}

/// Case-fold + slash-normalize a virtual path for the lookup map.
pub fn normalize_path(p: &str) -> String {
    p.replace('\\', "/")
        .trim_start_matches('/')
        .to_ascii_lowercase()
}

impl PakSet {
    /// The enfusion-mcp symlink farm — the same paks the MCP server reads.
    pub fn default_dir() -> Option<PathBuf> {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache/enfusion-mcp-root/addons"))
    }

    pub fn from_dir(dir: &Path) -> Result<Self> {
        let mut paths: Vec<PathBuf> = fs::read_dir(dir)
            .with_context(|| dir.display().to_string())?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("pak")))
            .collect();
        paths.sort();
        if paths.is_empty() {
            bail!("no .pak files under {}", dir.display());
        }
        let mut set = Self {
            paks: Vec::new(),
            lookup: HashMap::new(),
        };
        for p in paths {
            let idx = PakIndex::open(&p)?;
            set.push(idx);
        }
        Ok(set)
    }

    /// Add one parsed pak (later paks never shadow earlier ones).
    pub fn push(&mut self, idx: PakIndex) {
        let pi = self.paks.len();
        for (ei, e) in idx.entries.iter().enumerate() {
            self.lookup
                .entry(normalize_path(&e.path))
                .or_insert((pi, ei));
        }
        self.paks.push(idx);
    }

    pub fn pak_count(&self) -> usize {
        self.paks.len()
    }

    pub fn file_count(&self) -> usize {
        self.lookup.len()
    }

    pub fn find(&self, rel_path: &str) -> Option<&PakEntry> {
        let (pi, ei) = *self.lookup.get(&normalize_path(rel_path))?;
        Some(&self.paks[pi].entries[ei])
    }

    /// Every virtual path with the given prefix (case-insensitive), sorted.
    pub fn paths_under(&self, prefix: &str) -> Vec<String> {
        let pre = normalize_path(prefix);
        let mut out: Vec<String> = self
            .lookup
            .iter()
            .filter(|(k, _)| k.starts_with(&pre))
            .map(|(_, &(pi, ei))| self.paks[pi].entries[ei].path.clone())
            .collect();
        out.sort();
        out
    }
}

impl AssetSource for PakSet {
    fn read(&self, rel_path: &str) -> Result<Vec<u8>> {
        let (pi, ei) = *self
            .lookup
            .get(&normalize_path(rel_path))
            .with_context(|| format!("{rel_path}: not in any pak"))?;
        let pak = &self.paks[pi];
        pak.read(&pak.entries[ei])
    }
    fn exists(&self, rel_path: &str) -> bool {
        self.lookup.contains_key(&normalize_path(rel_path))
    }
}

/// A loose directory tree (the operator's hand-extracted files, or a test fixture dir).
/// Path lookup is exact first, then case-insensitive within the same directory.
#[derive(Debug)]
pub struct DirSource {
    pub root: PathBuf,
}

impl DirSource {
    fn resolve(&self, rel_path: &str) -> Option<PathBuf> {
        let rel = rel_path.replace('\\', "/");
        let direct = self.root.join(&rel);
        if direct.is_file() {
            return Some(direct);
        }
        // Case-insensitive walk, one component at a time.
        let mut cur = self.root.clone();
        for comp in rel.trim_start_matches('/').split('/') {
            let want = comp.to_ascii_lowercase();
            let next = fs::read_dir(&cur)
                .ok()?
                .filter_map(|e| e.ok())
                .find(|e| e.file_name().to_string_lossy().to_ascii_lowercase() == want)?;
            cur = next.path();
        }
        cur.is_file().then_some(cur)
    }
}

impl AssetSource for DirSource {
    fn read(&self, rel_path: &str) -> Result<Vec<u8>> {
        let p = self
            .resolve(rel_path)
            .with_context(|| format!("{rel_path}: not under {}", self.root.display()))?;
        fs::read(&p).with_context(|| p.display().to_string())
    }
    fn exists(&self, rel_path: &str) -> bool {
        self.resolve(rel_path).is_some()
    }
}

/// Paks first, loose files as a fallback (and vice versa when only a directory exists).
pub struct LayeredSource {
    pub layers: Vec<Box<dyn AssetSource>>,
}

impl AssetSource for LayeredSource {
    fn read(&self, rel_path: &str) -> Result<Vec<u8>> {
        for l in &self.layers {
            if l.exists(rel_path) {
                return l.read(rel_path);
            }
        }
        bail!("{rel_path}: not found in any asset source")
    }
    fn exists(&self, rel_path: &str) -> bool {
        self.layers.iter().any(|l| l.exists(rel_path))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::Write as _;

    /// Build a PAC1 image: `files` = (path, bytes, compress?). Directory nesting follows the
    /// paths (one level of dirs is enough to exercise the tree walk).
    pub(crate) fn synth_pak(files: &[(&str, &[u8], bool)]) -> Vec<u8> {
        // DATA payload + per-file records. Offsets are absolute: FORM header (12) + the
        // HEAD chunk (8 + 3) + the DATA chunk header (8) precede the payload.
        const DATA_PAYLOAD_AT: u32 = 12 + 11 + 8;
        let mut data = Vec::new();
        let mut recs: Vec<(String, u32, u32, u32, bool)> = Vec::new();
        for (path, bytes, compress) in files {
            let off = DATA_PAYLOAD_AT + data.len() as u32;
            let stored: Vec<u8> = if *compress {
                let mut enc =
                    flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
                enc.write_all(bytes).unwrap();
                enc.finish().unwrap()
            } else {
                bytes.to_vec()
            };
            data.extend_from_slice(&stored);
            recs.push((
                path.to_string(),
                off,
                stored.len() as u32,
                bytes.len() as u32,
                *compress,
            ));
        }
        // FILE tree: root dir → { top-level files, one dir per distinct first component }.
        let mut tree = Vec::new();
        let mut dirs: Vec<(String, Vec<usize>)> = Vec::new();
        let mut top: Vec<usize> = Vec::new();
        for (i, r) in recs.iter().enumerate() {
            match r.0.split_once('/') {
                Some((d, _)) => match dirs.iter_mut().find(|x| x.0 == d) {
                    Some(x) => x.1.push(i),
                    None => dirs.push((d.to_string(), vec![i])),
                },
                None => top.push(i),
            }
        }
        let file_rec = |t: &mut Vec<u8>, name: &str, r: &(String, u32, u32, u32, bool)| {
            t.push(1);
            t.push(name.len() as u8);
            t.extend_from_slice(name.as_bytes());
            t.extend_from_slice(&r.1.to_le_bytes());
            t.extend_from_slice(&r.2.to_le_bytes());
            t.extend_from_slice(&r.3.to_le_bytes());
            t.extend_from_slice(&[0u8; 6]);
            t.push(u8::from(r.4));
            t.extend_from_slice(&[0u8; 5]);
        };
        tree.push(0);
        tree.push(0); // root: empty name
        tree.extend_from_slice(&((top.len() + dirs.len()) as u32).to_le_bytes());
        for i in &top {
            file_rec(&mut tree, &recs[*i].0, &recs[*i]);
        }
        for (d, members) in &dirs {
            tree.push(0);
            tree.push(d.len() as u8);
            tree.extend_from_slice(d.as_bytes());
            tree.extend_from_slice(&(members.len() as u32).to_le_bytes());
            for i in members {
                let leaf = recs[*i].0.split_once('/').unwrap().1;
                file_rec(&mut tree, leaf, &recs[*i]);
            }
        }
        let mut img = Vec::new();
        img.extend_from_slice(b"FORM");
        img.extend_from_slice(&0u32.to_be_bytes()); // patched below
        img.extend_from_slice(b"PAC1");
        let head = [7u8, 7, 7]; // an opaque HEAD chunk the reader must skip
        img.extend_from_slice(b"HEAD");
        img.extend_from_slice(&(head.len() as u32).to_be_bytes());
        img.extend_from_slice(&head);
        img.extend_from_slice(b"DATA");
        img.extend_from_slice(&(data.len() as u32).to_be_bytes());
        img.extend_from_slice(&data);
        img.extend_from_slice(b"FILE");
        img.extend_from_slice(&(tree.len() as u32).to_be_bytes());
        img.extend_from_slice(&tree);
        let total = (img.len() - 8) as u32;
        img[4..8].copy_from_slice(&total.to_be_bytes());
        img
    }

    #[test]
    fn synthetic_pak_lists_and_reads_stored_and_zlib_files() {
        let big: Vec<u8> = (0..5000u32).map(|i| (i % 7) as u8).collect();
        let img = synth_pak(&[
            ("readme.txt", b"hello", false),
            ("Prefabs/Door.et", b"GenericEntity {\n}\n", true),
            ("Prefabs/Table.et", &big, true),
        ]);
        let entries = parse_pak_bytes(&img).expect("parse");
        let paths: Vec<&str> = entries.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(paths, ["readme.txt", "Prefabs/Door.et", "Prefabs/Table.et"]);
        assert!(!entries[0].compressed && entries[1].compressed);
        assert_eq!(entries[2].decompressed_len, 5000);
        assert!(entries[2].compressed_len < 5000, "zlib shrank the pattern");
        // Offsets are absolute: the DATA payload starts after FORM(12) + HEAD chunk(8+3).
        assert_eq!(entries[0].offset, 12 + 11 + 8);
        // A DATA-relative offset (below the payload start) is rejected by name.
        let mut bad = img.clone();
        // FILE chunk is last; find its entry record by searching from the end.
        let needle = (12u32 + 11 + 8).to_le_bytes();
        let at = bad
            .windows(4)
            .rposition(|w| w == needle)
            .expect("first entry offset in the FILE tree");
        bad[at..at + 4].copy_from_slice(&0u32.to_le_bytes());
        let err = parse_pak_bytes(&bad).unwrap_err().to_string();
        assert!(err.contains("outside the DATA chunk"), "{err}");
        let dir = std::env::temp_dir().join(format!("tbd-pak-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join("b_second.pak");
        fs::write(&p, &img).unwrap();
        let idx = PakIndex::open(&p).expect("open");
        assert_eq!(idx.read(&idx.entries[0]).unwrap(), b"hello");
        assert_eq!(idx.read(&idx.entries[1]).unwrap(), b"GenericEntity {\n}\n");
        assert_eq!(idx.read(&idx.entries[2]).unwrap(), big);
        // A second pak (sorted first) shadows the duplicate path.
        let first = synth_pak(&[("Prefabs/Door.et", b"FIRST WINS", false)]);
        fs::write(dir.join("a_first.pak"), &first).unwrap();
        let set = PakSet::from_dir(&dir).expect("set");
        assert_eq!(set.pak_count(), 2);
        assert_eq!(set.file_count(), 3);
        assert_eq!(set.read("prefabs/door.et").unwrap(), b"FIRST WINS");
        assert_eq!(set.read("Prefabs/Table.et").unwrap(), big);
        assert!(set.exists("README.TXT") && !set.exists("nope.et"));
        assert_eq!(
            set.paths_under("Prefabs/"),
            ["Prefabs/Door.et", "Prefabs/Table.et"]
        );
        assert!(set.read("missing.et").is_err());
        // Loose-directory source with case-insensitive lookup, layered under the paks.
        fs::create_dir_all(dir.join("loose/Assets")).unwrap();
        fs::write(dir.join("loose/Assets/Thing.xob"), b"xob").unwrap();
        let loose = DirSource {
            root: dir.join("loose"),
        };
        assert_eq!(loose.read("assets/thing.XOB").unwrap(), b"xob");
        let layered = LayeredSource {
            layers: vec![Box::new(set), Box::new(loose)],
        };
        assert_eq!(layered.read("Prefabs/Door.et").unwrap(), b"FIRST WINS");
        assert_eq!(layered.read("Assets/Thing.xob").unwrap(), b"xob");
        assert!(layered.read("Assets/Other.xob").is_err());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn malformed_pak_images_are_rejected() {
        assert!(parse_pak_bytes(b"FORM\0\0\0\0XOB9").is_err());
        let img = synth_pak(&[("a.txt", b"x", false)]);
        // Drop the FILE chunk.
        let cut = img[..img.len() - 40].to_vec();
        assert!(parse_pak_bytes(&cut).is_err());
        // Wrong inflated length is caught.
        let e = PakEntry {
            path: "z".into(),
            offset: 0,
            compressed_len: 0,
            decompressed_len: 99,
            compressed: true,
        };
        let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
        enc.write_all(b"short").unwrap();
        assert!(inflate_entry(&enc.finish().unwrap(), &e).is_err());
    }

    /// Census of the real install: entries per pak (the MCP log reports 7319 for data002
    /// and 5372 for data004 — a lower count here means a truncated tree walk) and every
    /// holder of the farmhouse XOB with the first byte that differs from the extract.
    #[test]
    #[ignore = "needs ~/.cache/enfusion-mcp-root/addons"]
    fn real_pak_census() {
        let Some(dir) = PakSet::default_dir().filter(|d| d.is_dir()) else {
            return;
        };
        let home = std::env::var("HOME").unwrap();
        let want = fs::read(PathBuf::from(home).join(
            "ReforgerExtract/unpacked/Assets/Structures/Houses/Farm/FarmHouse_E_1L01/FarmHouse_E_1L01.xob",
        ))
        .unwrap_or_default();
        let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "pak"))
            .collect();
        paths.sort();
        for p in paths {
            let idx = PakIndex::open(&p).expect("open pak");
            let farm: Vec<&PakEntry> = idx
                .entries
                .iter()
                .filter(|e| e.path.to_ascii_lowercase().contains("farmhouse_e_1l01.xob"))
                .collect();
            eprintln!(
                "{}: {} entries; farmhouse xob entries: {:?}",
                p.file_name().unwrap().to_string_lossy(),
                idx.entries.len(),
                farm.iter()
                    .map(|e| (
                        e.path.as_str(),
                        e.compressed,
                        e.compressed_len,
                        e.decompressed_len
                    ))
                    .collect::<Vec<_>>()
            );
            for e in farm {
                let got = idx.read(e).expect("read");
                let first_diff = got.iter().zip(want.iter()).position(|(a, b)| a != b);
                eprintln!(
                    "  {} → {} bytes, first diff vs extract at {:?} (extract {} bytes); pak {} bytes, entry.offset {}",
                    e.path,
                    got.len(),
                    first_diff,
                    want.len(),
                    fs::metadata(&p).map(|m| m.len()).unwrap_or(0),
                    e.offset
                );
                eprintln!("  got[..16]  = {:02x?}", &got[..16.min(got.len())]);
                eprintln!("  want[..16] = {:02x?}", &want[..16.min(want.len())]);
                // Where does the extract's first 16 bytes actually sit in this pak?
                let pak_bytes = fs::read(&p).unwrap();
                if want.len() >= 16 {
                    let needle = &want[..16];
                    let hits: Vec<usize> = pak_bytes
                        .windows(16)
                        .enumerate()
                        .filter(|(_, w)| *w == needle)
                        .map(|(i, _)| i)
                        .take(4)
                        .collect();
                    eprintln!("  extract head found in pak at {hits:?}");
                }
            }
        }
    }

    /// The real install: the farmhouse XOB read through the pak set must byte-equal the
    /// operator's hand-extracted copy. Needs the MCP symlink farm + the extract — skipped
    /// (not failed) elsewhere.
    #[test]
    #[ignore = "needs ~/.cache/enfusion-mcp-root/addons + ~/ReforgerExtract"]
    fn real_pak_farmhouse_xob_matches_extract() {
        let Some(dir) = PakSet::default_dir().filter(|d| d.is_dir()) else {
            return;
        };
        let home = std::env::var("HOME").unwrap();
        let extract = PathBuf::from(home).join(
            "ReforgerExtract/unpacked/Assets/Structures/Houses/Farm/FarmHouse_E_1L01/FarmHouse_E_1L01.xob",
        );
        let Ok(want) = fs::read(&extract) else {
            return;
        };
        let set = PakSet::from_dir(&dir).expect("pak set");
        let rel = "Assets/Structures/Houses/Farm/FarmHouse_E_1L01/FarmHouse_E_1L01.xob";
        // Diagnostic: which paks carry the path, and which copy equals the extract.
        let mut holders = Vec::new();
        for pak in &set.paks {
            if let Some(e) = pak
                .entries
                .iter()
                .find(|e| normalize_path(&e.path) == normalize_path(rel))
            {
                let bytes = pak.read(e).expect("read");
                let name = pak.path.file_name().unwrap().to_string_lossy().into_owned();
                holders.push(format!(
                    "{name}: {} bytes, {}",
                    bytes.len(),
                    if bytes == want {
                        "== extract"
                    } else {
                        "differs"
                    }
                ));
            }
        }
        eprintln!("farmhouse xob holders: {holders:?}");
        let got = set.read(rel).expect("farmhouse xob in a pak");
        assert_eq!(got.len(), want.len());
        assert!(
            got == want,
            "pak read differs from the extracted file ({holders:?})"
        );
    }
}
