//! TBDD density-grid decode — **Class R** (bit-identical to `decodeTBDD`, `forestMass.ts:38`).
//! Little-endian: 16 B header (u32 magic `TBDD`, u16 version, u16 cellM, u16 cols, u16 rows,
//! u8 channelCount, 3 B pad), then per channel `u16[cols·rows]` corner counts, row-major.
//!
//! T-935.5 — the decoder is a [`TbddHeader`] Pod read plus one [`bytemuck::cast_slice`] of the
//! payload; it no longer assembles each cell from its two bytes. The on-disk layout is unchanged
//! (the 625 committed `everon/objects/density/*.bin` tiles are the pin), so `encode_tbdd` and every
//! caller are untouched. `parity_reference` below keeps the pre-T-935.5 loop under `cfg(test)` and
//! the Class-R test decodes all 625 tiles both ways.

use std::borrow::Cow;

use bytemuck::{Pod, Zeroable};

pub const TBDD_HEADER_BYTES: usize = 16;
/// Channel order: index 0 = tree, 1 = rock (`DENSITY_CHANNEL_NAMES`).
pub const DENSITY_CHANNEL_NAMES: [&str; 2] = ["tree", "rock"];
/// File magic, first four bytes of every TBDD buffer.
pub const TBDD_MAGIC: [u8; 4] = *b"TBDD";

/// The 16-byte TBDD file header, exactly as it sits on disk.
///
/// Padding-free by construction (`4 + 2 + 2 + 2 + 2 + 1 + 3 = 16`), which is what lets
/// `#[derive(Pod)]` accept it — the derive refuses any type whose size exceeds the sum of its
/// fields, so a field added, widened or reordered is a compile error rather than a silent offset
/// shift in every emitter and loader.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod, Zeroable)]
pub struct TbddHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub cell_m: u16,
    pub cols: u16,
    pub rows: u16,
    pub channel_count: u8,
    /// Always written as zero. Present so the struct is exactly 16 B with no compiler padding a
    /// `Pod` cast would expose as uninitialised bytes.
    pub _pad: [u8; 3],
}

const _: () = assert!(
    core::mem::size_of::<TbddHeader>() == TBDD_HEADER_BYTES,
    "TbddHeader is the TBDD wire header and MUST be exactly 16 bytes: 625 committed everon density \
     tiles, `tbd_tools::density::TBDD_FILE_BYTES` and every payload offset are computed from it."
);
const _: () = assert!(
    core::mem::align_of::<TbddHeader>() == 2,
    "TbddHeader must stay 2-aligned: the header is 16 B, so a 2-aligned buffer puts the u16 cell \
     payload at a 2-aligned offset and `cast_slice` succeeds without a copy."
);
const _: () = assert!(
    cfg!(target_endian = "little"),
    "TBDD is little-endian on disk and bytemuck casts are NATIVE-endian, so a big-endian build \
     would read every cell byte-swapped while reporting success. Add explicit byte-swapping to \
     `decode_tbdd` before enabling such a target."
);

/// Decoded TBDD density grid (one export chunk).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TbddGrid {
    pub version: u16,
    pub cell_m: u16,
    pub cols: u16,
    pub rows: u16,
    /// Per-channel corner counts, `DENSITY_CHANNEL_NAMES` order.
    pub channels: Vec<Vec<u16>>,
}

/// Decode failure (the TS throws; the worker maps a throw to "no density for this chunk").
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TbddError {
    Short { len: usize },
    BadMagic,
    Truncated { len: usize, want: usize },
}

impl core::fmt::Display for TbddError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TbddError::Short { len } => write!(f, "TBDD: short buffer ({len} B)"),
            TbddError::BadMagic => write!(f, "TBDD: bad magic"),
            TbddError::Truncated { len, want } => {
                write!(f, "TBDD: truncated ({len} B, want {want})")
            }
        }
    }
}

impl std::error::Error for TbddError {}

/// The payload as `u16` cells, borrowed when the caller's buffer is already 2-aligned and copied
/// into an aligned `Vec` when it is not.
///
/// A `Vec<u8>` off `fetch_bytes` or `std::fs::read` frequently *is* 2-aligned, but nothing
/// guarantees it and a sub-slice of a larger buffer routinely is not — `try_cast_slice` answers
/// that with `Err` rather than UB, and this is the one copy that answers the `Err`.
fn aligned_cells(payload: &[u8]) -> Cow<'_, [u16]> {
    if payload.is_empty() {
        // An empty `&[u8]` carries a dangling 1-aligned pointer, so `try_cast_slice` rejects a
        // buffer that has no bytes to be misaligned. A zero-cell grid is a real grid.
        return Cow::Borrowed(&[]);
    }
    match bytemuck::try_cast_slice::<u8, u16>(payload) {
        Ok(cells) => Cow::Borrowed(cells),
        Err(_) => {
            let mut owned = vec![0u16; payload.len() / 2];
            bytemuck::cast_slice_mut::<u16, u8>(&mut owned).copy_from_slice(payload);
            Cow::Owned(owned)
        }
    }
}

/// Decode one TBDD buffer. Mirror of `decodeTBDD` (`forestMass.ts:38`).
///
/// # Errors
/// Returns [`TbddError`] on a short buffer, bad magic, or a truncated channel block. Never panics:
/// a hostile 16-byte header claiming 255 channels of 65535² cells is answered by
/// [`TbddError::Truncated`], not by an arithmetic overflow (`want` is computed in `u64`, which
/// matters on `wasm32` where `usize` is 32 bits).
pub fn decode_tbdd(bytes: &[u8]) -> Result<TbddGrid, TbddError> {
    let Some(head_bytes) = bytes.get(..TBDD_HEADER_BYTES) else {
        return Err(TbddError::Short { len: bytes.len() });
    };
    let head: TbddHeader = bytemuck::pod_read_unaligned(head_bytes);
    if head.magic != TBDD_MAGIC {
        return Err(TbddError::BadMagic);
    }
    // `cols * rows` cannot overflow a 32-bit usize (65535² = 4_294_836_225 < u32::MAX); the
    // channel product can, so it is taken in u64 and only narrowed once the length check has
    // proved it fits in the buffer we were handed.
    let plane = head.cols as usize * head.rows as usize;
    let want = TBDD_HEADER_BYTES as u64 + u64::from(head.channel_count) * plane as u64 * 2;
    if (bytes.len() as u64) < want {
        return Err(TbddError::Truncated {
            len: bytes.len(),
            want: usize::try_from(want).unwrap_or(usize::MAX),
        });
    }
    let cell_bytes = head.channel_count as usize * plane * 2;
    let payload = &bytes[TBDD_HEADER_BYTES..TBDD_HEADER_BYTES + cell_bytes];
    let cells = aligned_cells(payload);
    let channels = if plane == 0 {
        // `chunks_exact(0)` panics; a zero-cell grid still has `channel_count` (empty) channels.
        vec![Vec::new(); head.channel_count as usize]
    } else {
        cells.chunks_exact(plane).map(<[u16]>::to_vec).collect()
    };
    Ok(TbddGrid {
        version: head.version,
        cell_m: head.cell_m,
        cols: head.cols,
        rows: head.rows,
        channels,
    })
}

/// Encode a TBDD buffer (the exporter/tooling side of [`decode_tbdd`]; T-165.4 — promoted from
/// the test-private helper so the Rust world-export pipeline shares one codec with the engine).
/// Layout (locked, little-endian): 16-byte header (`TBDD`, u16 version=1, u16 cell_m, u16 cols,
/// u16 rows, u8 channel_count, 3B zero pad) then per-channel `u16[cols*rows]` row-major counts.
///
/// T-935.5 deliberately left this loop alone: 625 committed tiles are the acceptance, and the
/// cheapest guarantee that the emitter still writes exactly those bytes is not to touch it. The
/// guarantee is *proved*, not assumed — `tbd_tools::density` re-encodes all 625 and asserts byte
/// identity with the files on disk.
///
/// # Panics
/// Panics if a channel's length ≠ `cols * rows` (caller bug — mirrors the .mjs throw).
#[must_use]
pub fn encode_tbdd(cell_m: u16, cols: u16, rows: u16, channels: &[&[u16]]) -> Vec<u8> {
    let cells = cols as usize * rows as usize;
    let mut b = Vec::with_capacity(16 + channels.len() * cells * 2);
    b.extend_from_slice(b"TBDD");
    b.extend_from_slice(&1u16.to_le_bytes()); // version
    b.extend_from_slice(&cell_m.to_le_bytes());
    b.extend_from_slice(&cols.to_le_bytes());
    b.extend_from_slice(&rows.to_le_bytes());
    b.push(u8::try_from(channels.len()).expect("<=255 channels"));
    b.extend_from_slice(&[0, 0, 0]); // pad
    for (c, ch) in channels.iter().enumerate() {
        assert!(
            ch.len() == cells,
            "encode_tbdd: channel {c} has {} values, want {cells}",
            ch.len()
        );
        for &v in *ch {
            b.extend_from_slice(&v.to_le_bytes());
        }
    }
    b
}

/// The pre-T-935.5 decoder, kept verbatim as the Class-R oracle.
///
/// It lives here, in the test half of the file it grades, because a parity test that calls the
/// production function twice — or that compares against a copy sharing the production constants —
/// passes forever. `REF_HEADER_BYTES` is therefore an INDEPENDENT literal and not
/// `super::TBDD_HEADER_BYTES`: spelling the production constant here would make the oracle move
/// with the thing it is grading, and the off-by-one perturbation this module exists to catch would
/// shift both sides equally and stay green.
#[cfg(test)]
mod parity_reference {
    use super::{TbddError, TbddGrid};

    const REF_HEADER_BYTES: usize = 16;

    #[inline]
    fn u16_le(bytes: &[u8], at: usize) -> u16 {
        u16::from_le_bytes([bytes[at], bytes[at + 1]])
    }

    pub(super) fn decode_tbdd(bytes: &[u8]) -> Result<TbddGrid, TbddError> {
        if bytes.len() < REF_HEADER_BYTES {
            return Err(TbddError::Short { len: bytes.len() });
        }
        if &bytes[0..4] != b"TBDD" {
            return Err(TbddError::BadMagic);
        }
        let version = u16_le(bytes, 4);
        let cell_m = u16_le(bytes, 6);
        let cols = u16_le(bytes, 8);
        let rows = u16_le(bytes, 10);
        let channel_count = bytes[12] as usize;
        let plane = cols as usize * rows as usize;
        let need = REF_HEADER_BYTES + channel_count * plane * 2;
        if bytes.len() < need {
            return Err(TbddError::Truncated {
                len: bytes.len(),
                want: need,
            });
        }
        let mut channels = Vec::with_capacity(channel_count);
        for c in 0..channel_count {
            let base = REF_HEADER_BYTES + c * plane * 2;
            let mut ch = vec![0u16; plane];
            for (k, slot) in ch.iter_mut().enumerate() {
                *slot = u16_le(bytes, base + 2 * k);
            }
            channels.push(ch);
        }
        Ok(TbddGrid {
            version,
            cell_m,
            cols,
            rows,
            channels,
        })
    }
}

/// The shipped half of this file, with `#[cfg(test)]` items cut out.
///
/// Same job as the SPA's `editor::arsenal::class_r_scrub::live_source` (T-601), reimplemented here
/// because that one is `pub(crate)` inside `website-frontend` and this crate cannot reach it. A
/// source pin that greps the whole file would grade [`super::parity_reference`] — a verbatim copy
/// of the very loop the pin exists to forbid — and could never go green, and the mirror-image
/// mistake (grepping a haystack that still holds a pristine copy) is how a Class-R pin passes
/// forever. The scrubber's own non-vacuity is asserted by its caller.
#[cfg(test)]
mod class_r_scrub {
    /// Same-length copy of `src` with comments and string/char literals blanked to spaces
    /// (newlines kept). Every structural decision below is taken on this copy, so a `{` inside a
    /// string or a `#[cfg(test)]` inside a doc comment can never steer brace balancing.
    fn mask(src: &[char]) -> Vec<char> {
        let mut out: Vec<char> = Vec::with_capacity(src.len());
        let blank = |c: char| if c == '\n' { '\n' } else { ' ' };
        let mut i = 0usize;
        while i < src.len() {
            if src[i] == '/' && src.get(i + 1) == Some(&'/') {
                while i < src.len() && src[i] != '\n' {
                    out.push(blank(src[i]));
                    i += 1;
                }
                continue;
            }
            if src[i] == '/' && src.get(i + 1) == Some(&'*') {
                let mut depth = 0usize;
                while i < src.len() {
                    if src[i] == '/' && src.get(i + 1) == Some(&'*') {
                        depth += 1;
                        out.extend_from_slice(&[' ', ' ']);
                        i += 2;
                    } else if src[i] == '*' && src.get(i + 1) == Some(&'/') {
                        depth -= 1;
                        out.extend_from_slice(&[' ', ' ']);
                        i += 2;
                        if depth == 0 {
                            break;
                        }
                    } else {
                        out.push(blank(src[i]));
                        i += 1;
                    }
                }
                continue;
            }
            if let Some(end) = literal_end(src, i) {
                for c in &src[i..end] {
                    out.push(blank(*c));
                }
                i = end;
                continue;
            }
            out.push(src[i]);
            i += 1;
        }
        assert_eq!(
            out.len(),
            src.len(),
            "T-935.5: the scrubber mask lost alignment with the source — nothing built on it can \
             be trusted, so this is a hard failure rather than a silent skip"
        );
        out
    }

    /// End index (exclusive) of the string/char literal starting at `i`, if one does. A lifetime
    /// (`'_`, `'a`) is deliberately not a literal.
    fn literal_end(src: &[char], i: usize) -> Option<usize> {
        let ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
        if src[i] == 'r' && (i == 0 || !ident(src[i - 1])) {
            let mut j = i + 1;
            let mut hashes = 0usize;
            while src.get(j) == Some(&'#') {
                hashes += 1;
                j += 1;
            }
            if src.get(j) != Some(&'"') {
                return None;
            }
            j += 1;
            while j < src.len() {
                if src[j] == '"' && src[j + 1..].iter().take(hashes).all(|c| *c == '#') {
                    return Some(j + 1 + hashes);
                }
                j += 1;
            }
            return Some(src.len());
        }
        if src[i] == '"' {
            let mut j = i + 1;
            while j < src.len() {
                match src[j] {
                    '\\' => j += 2,
                    '"' => return Some(j + 1),
                    _ => j += 1,
                }
            }
            return Some(src.len());
        }
        if src[i] == '\'' {
            // `'x'` / `'\n'` are literals; `'a` and `'_` are lifetimes.
            let is_char = src.get(i + 1) == Some(&'\\') || src.get(i + 2) == Some(&'\'');
            if !is_char {
                return None;
            }
            let mut j = i + 1;
            while j < src.len() {
                match src[j] {
                    '\\' => j += 2,
                    '\'' => return Some(j + 1),
                    _ => j += 1,
                }
            }
            return Some(src.len());
        }
        None
    }

    /// The shipped half of `src`: comments and literals blanked, every `#[cfg(test)]` item cut.
    /// Length and line numbers are preserved throughout (everything removed becomes spaces).
    ///
    /// Comments are cut for the same reason the test half is: this doc block names
    /// `REF_HEADER_BYTES` and `u16_le`, and a ban pin that reads prose finds the words it forbids
    /// in the explanation of why they are forbidden. Literals go too — a needle sitting inside a
    /// string is a mention, not a call (the SPA's `live_code`, T-601).
    pub(super) fn live_source(src: &str) -> String {
        let chars: Vec<char> = src.chars().collect();
        let masked = mask(&chars);
        let mut out = masked.clone();
        let needle: Vec<char> = "#[cfg(test)]".chars().collect();
        let mut i = 0usize;
        while i + needle.len() <= masked.len() {
            if masked[i..i + needle.len()] != needle[..] {
                i += 1;
                continue;
            }
            let Some(open) = (i..masked.len()).find(|k| masked[*k] == '{') else {
                break;
            };
            let mut depth = 0usize;
            let mut close = masked.len();
            for (k, ch) in masked.iter().enumerate().skip(open) {
                if *ch == '{' {
                    depth += 1;
                } else if *ch == '}' {
                    depth -= 1;
                    if depth == 0 {
                        close = k;
                        break;
                    }
                }
            }
            let last = close.min(out.len() - 1);
            for slot in &mut out[i..=last] {
                if *slot != '\n' {
                    *slot = ' ';
                }
            }
            i = close + 1;
        }
        out.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn encode(cell_m: u16, cols: u16, rows: u16, channels: &[&[u16]]) -> Vec<u8> {
        encode_tbdd(cell_m, cols, rows, channels)
    }

    /// The 625 committed everon density tiles (`objects/density/*.bin`), sorted.
    fn everon_density_tiles() -> Vec<PathBuf> {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../packages/map-assets/everon/objects/density");
        let rd = std::fs::read_dir(&dir).unwrap_or_else(|e| {
            panic!(
                "T-935.5: {} could not be read ({e}). The Class-R acceptance is all 625 committed \
                 tiles; a missing corpus is a FAILURE, never a skip.",
                dir.display()
            )
        });
        let mut files: Vec<PathBuf> = rd
            .map(|e| e.expect("density dir entry").path())
            .filter(|p| p.extension().is_some_and(|x| x == "bin"))
            .collect();
        files.sort();
        // A flat literal, not `files.len()`: the acceptance is "all 625 everon density tiles", and
        // a corpus that silently shrank to one file must be RED, not a one-file pass.
        assert_eq!(
            files.len(),
            625,
            "expected 625 everon density tiles in {}, found {}",
            dir.display(),
            files.len()
        );
        files
    }

    fn read_tile(path: &PathBuf) -> Vec<u8> {
        let bytes = std::fs::read(path).expect("read density tile");
        assert_eq!(
            bytes.get(..4),
            Some(&TBDD_MAGIC[..]),
            "{} does not start with the TBDD magic — if it starts with `vers` this checkout has \
             an LFS POINTER instead of the payload, which is an environment fault, not a decode \
             fault",
            path.display()
        );
        bytes
    }

    /// T-935.5 acceptance — **all 625** committed everon tiles decode bit-identically through the
    /// `cast_slice` decoder and through the pre-T-935.5 byte loop.
    #[test]
    fn everon_tiles_decode_bit_identically_to_the_old_loop() {
        let files = everon_density_tiles();
        let mut nonzero_cells = 0u64;
        let mut compared_cells = 0u64;
        for path in &files {
            let bytes = read_tile(path);
            let got = decode_tbdd(&bytes);
            let want = parity_reference::decode_tbdd(&bytes);
            let (a, b) = match (&got, &want) {
                (Ok(a), Ok(b)) => (a, b),
                _ => panic!(
                    "T-935.5 Class-R: {} — cast decode {got:?} vs old loop {want:?}",
                    path.display()
                ),
            };
            assert_eq!(
                (a.version, a.cell_m, a.cols, a.rows),
                (b.version, b.cell_m, b.cols, b.rows),
                "T-935.5 Class-R: {} header mismatch",
                path.display()
            );
            assert_eq!(
                a.channels.len(),
                b.channels.len(),
                "T-935.5 Class-R: {} channel count mismatch",
                path.display()
            );
            for (ci, (ac, bc)) in a.channels.iter().zip(&b.channels).enumerate() {
                assert_eq!(
                    ac.len(),
                    bc.len(),
                    "T-935.5 Class-R: {} channel {ci} length mismatch",
                    path.display()
                );
                if let Some(at) = ac.iter().zip(bc).position(|(x, y)| x != y) {
                    panic!(
                        "T-935.5 Class-R: {} channel {ci} cell {at}: cast decode {} != old loop {}",
                        path.display(),
                        ac[at],
                        bc[at]
                    );
                }
                compared_cells += ac.len() as u64;
                nonzero_cells += ac.iter().filter(|v| **v != 0).count() as u64;
            }
            // Shape, so "both decoders returned nothing" cannot read as agreement.
            assert_eq!(
                (a.channels.len(), a.cols, a.rows),
                (DENSITY_CHANNEL_NAMES.len(), 65, 65),
                "T-935.5: {} is not a 2-channel 65×65 tile",
                path.display()
            );
        }
        assert_eq!(
            compared_cells,
            625 * 2 * 65 * 65,
            "the parity loop compared {compared_cells} cells, not the whole corpus"
        );
        assert!(
            nonzero_cells > 0,
            "every cell in all 625 tiles decoded to zero — the corpus carries no signal, so the \
             parity above compared nothing that could differ"
        );
    }

    /// The two decoders agree on shapes the corpus does not contain, errors included.
    #[test]
    fn decode_matches_the_old_loop_on_synthetic_shapes() {
        let mut cases: Vec<Vec<u8>> = vec![
            Vec::new(),
            vec![1, 2, 3],
            encode(32, 2, 2, &[&[1, 2, 3, 4]]),
            encode(8, 1, 1, &[&[7]]),
            encode(8, 0, 0, &[]),
            encode(8, 65, 65, &[&vec![9u16; 4225], &vec![3u16; 4225]]),
        ];
        // zero cells but a declared channel; bad magic; truncated tail; header-only.
        let mut zero_plane = encode(8, 0, 0, &[]);
        zero_plane[12] = 2;
        cases.push(zero_plane);
        let mut bad = encode(32, 2, 2, &[&[0, 0, 0, 0]]);
        bad[0] = b'X';
        cases.push(bad);
        let full = encode(32, 2, 2, &[&[1, 2, 3, 4]]);
        cases.push(full[..full.len() - 2].to_vec());
        cases.push(full[..TBDD_HEADER_BYTES].to_vec());
        // A header claiming more than any buffer can hold: `Truncated`, never an overflow panic.
        let mut hostile = encode(8, 1, 1, &[&[0]]);
        hostile[8] = 0xFF;
        hostile[9] = 0xFF;
        hostile[10] = 0xFF;
        hostile[11] = 0xFF;
        hostile[12] = 0xFF;
        cases.push(hostile);

        for (i, buf) in cases.iter().enumerate() {
            assert_eq!(
                decode_tbdd(buf),
                parity_reference::decode_tbdd(buf),
                "T-935.5 Class-R: synthetic case {i} ({} B) disagrees",
                buf.len()
            );
        }
        // Non-vacuity: the case list must actually exercise both arms of the Result.
        assert!(cases.iter().any(|b| decode_tbdd(b).is_ok()));
        assert!(cases.iter().any(|b| decode_tbdd(b).is_err()));
    }

    /// An odd-addressed buffer decodes to the same grid — the `cast_slice` fast path cannot take
    /// it, so this is the aligned-copy branch and nothing else.
    #[test]
    fn unaligned_payload_decodes_identically() {
        let bytes = read_tile(&everon_density_tiles()[0]);
        let mut shifted = Vec::with_capacity(bytes.len() + 1);
        shifted.push(0u8);
        shifted.extend_from_slice(&bytes);
        let unaligned = &shifted[1..];
        assert_eq!(
            unaligned.as_ptr() as usize % 2,
            1,
            "the buffer under test is 2-aligned, so this test never reached the aligned-copy \
             branch it exists to cover"
        );
        assert!(
            bytemuck::try_cast_slice::<u8, u16>(&unaligned[TBDD_HEADER_BYTES..]).is_err(),
            "the payload cast succeeded on an odd address — the copy branch was not exercised"
        );
        assert_eq!(decode_tbdd(unaligned), decode_tbdd(&bytes));
        assert_eq!(
            decode_tbdd(unaligned),
            parity_reference::decode_tbdd(&bytes)
        );
    }

    /// Every truncation of a real tile is an `Err`, never a panic.
    #[test]
    fn short_payloads_are_err_never_panic() {
        let bytes = read_tile(&everon_density_tiles()[0]);
        for cut in [0usize, 1, 4, 12, 15, 16, 17, 100, 16_915] {
            let short = &bytes[..cut];
            let got = decode_tbdd(short);
            assert!(got.is_err(), "{cut} B decoded as Ok: {got:?}");
            assert_eq!(got, parity_reference::decode_tbdd(short), "{cut} B");
        }
        assert!(
            decode_tbdd(&bytes).is_ok(),
            "the full tile must still decode"
        );
    }

    #[test]
    fn header_pod_is_the_on_disk_header() {
        let bytes = read_tile(&everon_density_tiles()[0]);
        let head: TbddHeader = bytemuck::pod_read_unaligned(&bytes[..TBDD_HEADER_BYTES]);
        assert_eq!(size_of::<TbddHeader>(), TBDD_HEADER_BYTES);
        assert_eq!(align_of::<TbddHeader>(), 2);
        assert_eq!(head.magic, TBDD_MAGIC);
        assert_eq!((head.version, head.cell_m), (1, 8));
        assert_eq!((head.cols, head.rows), (65, 65));
        assert_eq!(head.channel_count, 2);
        assert_eq!(head._pad, [0, 0, 0]);
        // Field offsets are the format, so read them off the wire rather than off the struct.
        assert_eq!(bytes[4..6], head.version.to_le_bytes());
        assert_eq!(bytes[8..10], head.cols.to_le_bytes());
    }

    /// Acceptance — `decode_tbdd` contains no per-byte assembly loop **in production code**.
    #[test]
    fn production_decode_has_no_per_byte_assembly_loop() {
        const SRC: &str = include_str!("tbdd.rs");
        let live = class_r_scrub::live_source(SRC);

        // The scrubber's own non-vacuity FIRST: an over-eager scrub that returns whitespace would
        // satisfy every absence assertion below without reading a line of production code.
        assert!(
            live.contains("pub fn decode_tbdd(bytes: &[u8]) -> Result<TbddGrid, TbddError> {"),
            "the scrub ate the production decoder"
        );
        assert!(
            live.contains("bytemuck::try_cast_slice::<u8, u16>(payload)"),
            "the production decoder no longer casts the payload"
        );
        assert!(
            live.contains("pub fn encode_tbdd("),
            "the scrub ate the production encoder"
        );
        // …and that it really did cut the test half.
        assert!(
            !live.contains("REF_HEADER_BYTES"),
            "parity_reference survived"
        );
        assert!(
            !live.contains("mod parity_reference"),
            "the cut missed its item"
        );
        assert!(
            !live.contains("fn production_decode_has_no_per_byte_assembly_loop"),
            "the pin is grading itself"
        );
        // Blanking preserves length, so `live.len()` proves nothing; count what is left instead.
        // The production half is ~170 lines of the file, and every one of them is code the ban
        // below has to be reading.
        let kept = live.lines().filter(|l| !l.trim().is_empty()).count();
        assert!(
            kept > 100,
            "the scrub left {kept} non-blank lines — that is not the production half of this file"
        );

        for needle in ["u16_le(", "from_le_bytes(["] {
            assert!(
                !live.contains(needle),
                "T-935.5 acceptance: `{needle}` is back in production TBDD code — the decode is a \
                 per-byte assembly loop again"
            );
        }
    }

    /// The scrubber the pin above stands on, graded directly — both failure modes.
    ///
    /// Cutting too little makes the ban unfixable (`parity_reference` is a verbatim copy of the
    /// forbidden loop); cutting too much makes it unfailable. Neither is visible from the pin's
    /// own verdict, so the scrubber gets its own oracle.
    #[test]
    fn the_scrubber_keeps_production_and_cuts_the_test_half() {
        let src = concat!(
            "fn live() { let a = 1; }\n",
            "/// doc naming u16_le( and #[cfg(test)]\n",
            "const S: &str = \"u16_le( in a literal\";\n",
            "#[cfg(test)]\n",
            "mod t {\n    fn dead() { u16_le(b, 0); if x { y } }\n}\n",
            "fn live2() { let c = '{'; }\n",
        );
        let out = class_r_scrub::live_source(src);
        assert_eq!(out.lines().count(), src.lines().count(), "line count moved");
        assert!(out.contains("fn live()"), "production item cut");
        assert!(
            out.contains("fn live2()"),
            "the cut ran past its item's closing brace"
        );
        assert!(
            out.contains("const S: &str ="),
            "the literal's declaration was cut"
        );
        assert!(!out.contains("mod t"), "the test module survived");
        assert!(!out.contains("fn dead"), "the test module's body survived");
        assert!(
            !out.contains("u16_le("),
            "a banned needle survived in a comment or a literal"
        );
    }

    #[test]
    fn round_trip() {
        let tree: Vec<u16> = (0..4).collect();
        let rock: Vec<u16> = vec![9, 8, 7, 6];
        let buf = encode(32, 2, 2, &[&tree, &rock]);
        let g = decode_tbdd(&buf).unwrap();
        assert_eq!(g.cell_m, 32);
        assert_eq!((g.cols, g.rows), (2, 2));
        assert_eq!(g.channels.len(), 2);
        assert_eq!(g.channels[0], tree);
        assert_eq!(g.channels[1], rock);
    }

    #[test]
    fn bad_magic() {
        let mut buf = encode(32, 2, 2, &[&[0, 0, 0, 0]]);
        buf[0] = b'X';
        assert_eq!(decode_tbdd(&buf), Err(TbddError::BadMagic));
    }

    #[test]
    fn truncated() {
        let buf = encode(32, 2, 2, &[&[1, 2, 3, 4]]);
        let short = &buf[..buf.len() - 2];
        assert!(matches!(
            decode_tbdd(short),
            Err(TbddError::Truncated { .. })
        ));
    }

    #[test]
    fn short_header() {
        assert!(matches!(
            decode_tbdd(&[1, 2, 3]),
            Err(TbddError::Short { .. })
        ));
    }
}
