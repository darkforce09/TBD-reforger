use super::*;
use std::fs;
use std::io::Write as _;

const PLAIN: &str = "plain.txt";
const DEFLATED: &str = "deflated.bin";
/// HEAD payload length chosen so the DATA chunk header lands at 48 and `data_start` == 56.
const HEAD_LEN: usize = 28;
const DATA_START: u64 = 56;

/// Deterministic filler that zlib cannot shrink, so the compressed entry has a stable
/// length well past the 56-byte shift (an LCG — no `rand` dependency).
fn lcg_bytes(seed: u32, len: usize) -> Vec<u8> {
    let mut s = seed;
    (0..len)
        .map(|_| {
            s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            s.to_be_bytes()[0]
        })
        .collect()
}

fn zlib(raw: &[u8]) -> Vec<u8> {
    let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(raw).unwrap();
    enc.finish().unwrap()
}

/// One 24-byte FILE-chunk file record, laid out exactly as `parse_entry` reads it:
/// offset / compressed_len / decompressed_len (LE), method at +12, compressed flag at +18.
fn file_record(offset: u32, clen: u32, dlen: u32, compressed: bool) -> Vec<u8> {
    let mut r = Vec::with_capacity(24);
    r.extend_from_slice(&offset.to_le_bytes());
    r.extend_from_slice(&clen.to_le_bytes());
    r.extend_from_slice(&dlen.to_le_bytes());
    r.extend_from_slice(&[0u8; 6]); // method
    r.push(u8::from(compressed)); // +18
    r.extend_from_slice(&[0u8; 5]); // compression level + timestamp
    r
}

struct SynthPak {
    bytes: Vec<u8>,
    /// Stored verbatim at absolute offset 56.
    plain: Vec<u8>,
    /// Inflated form of the entry stored at absolute offset 312.
    deflated_raw: Vec<u8>,
    /// Deflated bytes as they sit in the pak.
    deflated_z: Vec<u8>,
}

fn synth_pak() -> SynthPak {
    let plain: Vec<u8> = (0..=255u8).collect();
    let deflated_raw = lcg_bytes(0x5041_4331, 512);
    let deflated_z = zlib(&deflated_raw);

    let plain_off = DATA_START as u32;
    let deflated_off = plain_off + plain.len() as u32;

    // FILE chunk: nameless root dir + two file children.
    let mut file_chunk = vec![0u8, 0u8]; // kind = dir, name_len = 0
    file_chunk.extend_from_slice(&2u32.to_le_bytes()); // child count
    for (name, rec) in [
        (
            PLAIN,
            file_record(plain_off, plain.len() as u32, plain.len() as u32, false),
        ),
        (
            DEFLATED,
            file_record(
                deflated_off,
                deflated_z.len() as u32,
                deflated_raw.len() as u32,
                true,
            ),
        ),
    ] {
        file_chunk.push(1); // kind = file
        file_chunk.push(name.len() as u8);
        file_chunk.extend_from_slice(name.as_bytes());
        file_chunk.extend_from_slice(&rec);
    }

    let data_len = plain.len() + deflated_z.len();
    let total = 12 + (8 + HEAD_LEN) + (8 + data_len) + (8 + file_chunk.len());

    let mut b = Vec::with_capacity(total);
    b.extend_from_slice(b"FORM");
    b.extend_from_slice(&((total - 8) as u32).to_be_bytes());
    b.extend_from_slice(b"PAC1");
    b.extend_from_slice(b"HEAD");
    b.extend_from_slice(&(HEAD_LEN as u32).to_be_bytes());
    b.extend_from_slice(&[0u8; HEAD_LEN]);
    b.extend_from_slice(b"DATA");
    b.extend_from_slice(&(data_len as u32).to_be_bytes());
    assert_eq!(
        b.len() as u64,
        DATA_START,
        "fixture must put DATA payload at 56"
    );
    b.extend_from_slice(&plain);
    b.extend_from_slice(&deflated_z);
    b.extend_from_slice(b"FILE");
    b.extend_from_slice(&(file_chunk.len() as u32).to_be_bytes());
    b.extend_from_slice(&file_chunk);
    assert_eq!(b.len(), total);

    // The whole point of the fixture: a `data_start + offset` seek stays inside the file,
    // so the buggy read returns rotated bytes rather than an EOF that could mask the bug.
    assert!(
        DATA_START + u64::from(deflated_off) + deflated_z.len() as u64 <= b.len() as u64,
        "fixture too short — a shifted read would EOF instead of returning wrong bytes"
    );

    SynthPak {
        bytes: b,
        plain,
        deflated_raw,
        deflated_z,
    }
}

/// Temp `<game>/addons/synthetic.pak`, removed on drop (including on a panicking assert).
struct GameDir(PathBuf);
impl Drop for GameDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn install(tag: &str, pak: &[u8]) -> GameDir {
    let dir = std::env::temp_dir().join(format!("t305-pak-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("addons")).unwrap();
    fs::write(dir.join("addons").join("synthetic.pak"), pak).unwrap();
    GameDir(dir)
}

fn open(tag: &str, s: &SynthPak) -> (GameDir, PakVfs) {
    let g = install(tag, &s.bytes);
    let vfs = PakVfs::open(&g.0).expect("synthetic pak must open");
    (g, vfs)
}

#[test]
fn synthetic_pak_parses_with_data_start_56() {
    let s = synth_pak();
    let (_g, vfs) = open("hdr", &s);
    assert!(vfs.exists(PLAIN), "entry table did not yield {PLAIN}");
    assert!(vfs.exists(DEFLATED), "entry table did not yield {DEFLATED}");
    assert_eq!(
        vfs.entry_data_start(PLAIN),
        Some(DATA_START),
        "fixture pins data_start = 56 — the term the seeks must NOT add"
    );
}

#[test]
fn read_file_uncompressed_returns_stored_bytes() {
    let s = synth_pak();
    let (_g, vfs) = open("plain", &s);
    let got = vfs.read_file(PLAIN).expect("read_file(plain.txt)");
    // Head first: a rotated read shows up immediately, without dumping 256 bytes twice.
    assert_eq!(
        &got[..8],
        &s.plain[..8],
        "read_file must seek entry.offset; adding data_start rotates the read by 56 bytes"
    );
    assert_eq!(got, s.plain, "read_file(plain.txt) whole-entry mismatch");
}

#[test]
fn read_file_compressed_returns_inflated_bytes() {
    let s = synth_pak();
    let (_g, vfs) = open("deflated", &s);
    let got = vfs.read_file(DEFLATED).expect("read_file(deflated.bin)");
    assert_eq!(
        &got[..8],
        &s.deflated_raw[..8],
        "read_file must inflate the bytes at entry.offset"
    );
    assert_eq!(
        got, s.deflated_raw,
        "read_file(deflated.bin) whole-entry mismatch"
    );
}

#[test]
fn read_raw_returns_stored_bytes() {
    let s = synth_pak();
    let (_g, vfs) = open("raw", &s);

    let (raw_z, dlen) = vfs.read_raw(DEFLATED).expect("read_raw(deflated.bin)");
    assert_eq!(dlen, s.deflated_raw.len() as u32);
    assert_eq!(
        &raw_z[..8],
        &s.deflated_z[..8],
        "read_raw must seek entry.offset; adding data_start rotates the read by 56 bytes"
    );
    assert_eq!(raw_z, s.deflated_z, "read_raw(deflated.bin) mismatch");

    let (raw_plain, dlen) = vfs.read_raw(PLAIN).expect("read_raw(plain.txt)");
    assert_eq!(dlen, s.plain.len() as u32);
    assert_eq!(
        &raw_plain[..8],
        &s.plain[..8],
        "read_raw(plain.txt) rotated"
    );
    assert_eq!(raw_plain, s.plain, "read_raw(plain.txt) mismatch");
}
