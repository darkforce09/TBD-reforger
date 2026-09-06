//! T-935.4 — `dem/elevation.dem` (`TBDE`, spec §3.2): the raw `u16` elevation grid that replaces
//! the 16-bit DEM PNG on the boot path.
//!
//! The PNG is 71.9 MB and reaches the DEM store through four full-grid buffers — the fetched
//! `Vec<u8>`, the `png` crate's output buffer, the `u16` raster [`png_decode`](super::png_decode)
//! builds from it, and the `f32` metres cache. The raw grid is 81.9 MB and reaches it through
//! **one**: the bytes are decoded little-endian into the final `Vec<u16>` as they arrive.
//!
//! # Two entry points, one decoder
//!
//! * [`RawDem::parse`] — the whole file is already in memory (tests, tools, a non-streamed fetch).
//! * [`RawDemSink`] — the streaming one. [`push`](RawDemSink::push) takes network chunks in
//!   whatever sizes the socket produced and never reallocates: the sample vector is allocated once,
//!   at the size the header declares, and every chunk writes into its tail. A chunk boundary that
//!   splits a `u16` is handled by carrying the odd byte (`pending`) to the next chunk — the socket
//!   has no reason to hand back even-sized buffers and a 6400² grid gives it 40,960,000 chances to
//!   split one.
//!
//! # Why the sink demands the file length up front
//!
//! `total_bytes` is not a convenience: it is the guard that makes the header safe to trust. The
//! header is the *first thing off an untrusted wire*, and it declares `width * height` — a
//! `65535 x 65535` header asks for an 8.6 GB allocation before a single sample has been read.
//! Checking the header's implied file length against the length the transport already knows
//! (`content-length`, or the buffer length for [`RawDem::parse`]) makes that allocation impossible
//! to request. It also means [`TbdeHeader::payload_bytes`]'s `None` — the 32-bit overflow arm,
//! which is reachable on wasm32, this crate's shipping target — is answered with
//! [`BinaryError::LengthMismatch`] rather than an `unwrap`.
//!
//! # Metres are lazy
//!
//! [`RawDem`] holds `u16` and nothing else. Metres come from [`TbdeHeader::metres`] per sample
//! ([`RawDem::metres`]), so the 163.8 MB `f32` cache the PNG path materialises eagerly is never
//! built unless a caller asks for it — see [`RawDem::metres_grid`], which exists for the one
//! shipped consumer (hillshade) that still wants a whole grid.

use crate::world::binary::BinaryError;
use crate::world::binary::chunk_container::{ContainerHeader, HEADER_BYTES, TbdeHeader};

/// A decoded `dem/elevation.dem`: the header verbatim plus its `width * height` row-major samples.
///
/// Row 0 is the north edge — the same orientation as the PNG this replaces, so every consumer's
/// existing `(x, y)` indexing keeps its meaning.
#[derive(Clone, Debug, PartialEq)]
pub struct RawDem {
    pub header: TbdeHeader,
    pub samples: Vec<u16>,
}

impl RawDem {
    /// Decode a complete `TBDE` buffer.
    ///
    /// One allocation: `bytes.len()` bounds the header, so the sample vector is sized once and
    /// filled in place. The payload is `cast_slice`d when the buffer happens to be `u16`-aligned
    /// (a `memcpy`) and decoded byte-pair-wise when it is not — a `Vec<u8>` off the network is
    /// 1-aligned about half the time, and both branches must produce the same grid.
    ///
    /// # Errors
    /// [`BinaryError`] for a short buffer, wrong magic, wrong version, or a header whose declared
    /// grid does not match the bytes present.
    pub fn parse(bytes: &[u8]) -> Result<Self, BinaryError> {
        let total = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        let mut sink = RawDemSink::new(total);
        sink.push(bytes)?;
        sink.finish()
    }

    #[must_use]
    pub fn width(&self) -> u32 {
        self.header.width
    }

    #[must_use]
    pub fn height(&self) -> u32 {
        self.header.height
    }

    /// Row-major index of `(x, y)`, or `None` outside the grid.
    fn index(&self, x: u32, y: u32) -> Option<usize> {
        if x >= self.header.width || y >= self.header.height {
            return None;
        }
        (y as usize)
            .checked_mul(self.header.width as usize)?
            .checked_add(x as usize)
    }

    /// The quantised sample at `(x, y)`, or `None` outside the grid.
    #[must_use]
    pub fn sample_u16(&self, x: u32, y: u32) -> Option<u16> {
        self.samples.get(self.index(x, y)?).copied()
    }

    /// Elevation at `(x, y)` in metres — `offset_m + sample * scale_m`, computed on demand.
    #[must_use]
    pub fn metres(&self, x: u32, y: u32) -> Option<f32> {
        Some(self.header.metres(self.sample_u16(x, y)?))
    }

    /// The whole grid as `f32` metres.
    ///
    /// **This is the one place the loader is allowed to spend a second full-grid allocation**, and
    /// it exists for a single reason: `build_hillshade_image` and the DEM vector grid take
    /// `&[f32]`, and neither is this slice's to change. Everything else should use
    /// [`RawDem::metres`], which allocates nothing. Note the difference from the PNG path's
    /// `meters_cache`: the scale and offset come from the file's own header (`f32`, spec §3.2)
    /// rather than the manifest's `f64` height range, so the two agree to within the `f32`
    /// rounding of those two fields (~6e-5 m over everon's 580.31 m range — 150x finer than the
    /// 8.85 mm the `u16` quantisation itself costs).
    #[must_use]
    pub fn metres_grid(&self) -> Vec<f32> {
        self.samples
            .iter()
            .map(|&s| self.header.metres(s))
            .collect()
    }
}

/// Incremental `TBDE` decoder: feed it the response's chunks, take a [`RawDem`] at the end.
///
/// Holds exactly one growable buffer — the sample vector — and it is allocated once, when the
/// header completes, at the size the header declares. `push` never reallocates it.
pub struct RawDemSink {
    /// File length the transport already knows; the header is checked against it before anything
    /// is allocated. See the module docs.
    total_bytes: u64,
    /// The 32 header bytes as they accumulate — a chunk may deliver them a byte at a time.
    head: [u8; HEADER_BYTES],
    head_len: usize,
    header: Option<TbdeHeader>,
    /// The single allocation. Sized at header completion, filled by `push`.
    samples: Vec<u16>,
    /// Samples written so far.
    filled: usize,
    /// Low byte of a `u16` split across a chunk boundary.
    pending: Option<u8>,
    /// Payload bytes seen — reported by [`RawDemSink::finish`] when they disagree with the header.
    payload_seen: usize,
}

impl RawDemSink {
    /// A sink for a file of exactly `total_bytes` bytes (`content-length`, or the buffer length).
    #[must_use]
    pub fn new(total_bytes: u64) -> Self {
        Self {
            total_bytes,
            head: [0; HEADER_BYTES],
            head_len: 0,
            header: None,
            samples: Vec::new(),
            filled: 0,
            pending: None,
            payload_seen: 0,
        }
    }

    /// The parsed header, once enough bytes have arrived for it.
    #[must_use]
    pub fn header(&self) -> Option<&TbdeHeader> {
        self.header.as_ref()
    }

    /// Feed the next chunk. Header bytes first, then samples, little-endian.
    ///
    /// # Errors
    /// [`BinaryError`] as soon as the header is readable and wrong, or when more payload arrives
    /// than the header declared.
    pub fn push(&mut self, chunk: &[u8]) -> Result<(), BinaryError> {
        let mut rest = chunk;
        if self.header.is_none() {
            let want = HEADER_BYTES - self.head_len;
            let take = want.min(rest.len());
            self.head[self.head_len..self.head_len + take].copy_from_slice(&rest[..take]);
            self.head_len += take;
            rest = &rest[take..];
            if self.head_len < HEADER_BYTES {
                return Ok(());
            }
            self.begin_payload()?;
        }
        self.decode_payload(rest)
    }

    /// Validate the completed header and make the one allocation.
    fn begin_payload(&mut self) -> Result<(), BinaryError> {
        let (header, _) = TbdeHeader::read(&self.head)?;
        // `payload_bytes` is `Option` on purpose (checked arithmetic): on wasm32 a hostile
        // `width * height` wraps `usize`, and this is where that must become an error.
        let declared = header
            .payload_bytes()
            .and_then(|p| p.checked_add(HEADER_BYTES))
            .and_then(|f| u64::try_from(f).ok())
            .ok_or(BinaryError::LengthMismatch {
                what: "TBDE",
                expected: usize::MAX,
                actual: 0,
            })?;
        if declared != self.total_bytes {
            return Err(BinaryError::LengthMismatch {
                what: "TBDE",
                expected: usize::try_from(declared).unwrap_or(usize::MAX),
                actual: usize::try_from(self.total_bytes).unwrap_or(usize::MAX),
            });
        }
        // Safe now: `declared == total_bytes`, so the count is bounded by a file that exists.
        let count = header.sample_count().ok_or(BinaryError::LengthMismatch {
            what: "TBDE",
            expected: usize::MAX,
            actual: 0,
        })?;
        self.samples = vec![0_u16; count];
        self.header = Some(header);
        Ok(())
    }

    /// Little-endian decode of `bytes` into the tail of the sample vector, carrying a split `u16`.
    fn decode_payload(&mut self, bytes: &[u8]) -> Result<(), BinaryError> {
        self.payload_seen = self.payload_seen.saturating_add(bytes.len());
        let mut rest = bytes;
        if let Some(lo) = self.pending.take() {
            let Some((&hi, tail)) = rest.split_first() else {
                self.pending = Some(lo);
                return Ok(());
            };
            self.store(u16::from_le_bytes([lo, hi]))?;
            rest = tail;
        }
        let pairs = rest.len() / 2;
        // The aligned fast path the spec calls for: when the chunk's own bytes sit on a `u16`
        // boundary — `try_cast_slice` tests the SOURCE pointer, not the destination, which is a
        // `Vec<u16>` and therefore always aligned — and the host is little-endian (asserted at
        // compile time in `binary::pod`), the whole chunk is one `copy_from_slice` rather than
        // `pairs` shift-and-ors. Whether it hits is the allocator's business, so the slow branch
        // below is not a fallback for broken input: it is the other half of a coin flip, and
        // `misaligned_payload_decodes_identically_to_the_aligned_one` forces both sides of it.
        if cfg!(target_endian = "little")
            && pairs > 0
            && let Ok(words) = bytemuck::try_cast_slice::<u8, u16>(&rest[..pairs * 2])
        {
            let end = self.filled.saturating_add(words.len());
            if end > self.samples.len() {
                return Err(self.overrun());
            }
            self.samples[self.filled..end].copy_from_slice(words);
            self.filled = end;
        } else {
            for pair in rest[..pairs * 2].chunks_exact(2) {
                self.store(u16::from_le_bytes([pair[0], pair[1]]))?;
            }
        }
        if rest.len() % 2 == 1 {
            self.pending = Some(rest[rest.len() - 1]);
        }
        Ok(())
    }

    fn store(&mut self, sample: u16) -> Result<(), BinaryError> {
        let Some(slot) = self.samples.get_mut(self.filled) else {
            return Err(self.overrun());
        };
        *slot = sample;
        self.filled += 1;
        Ok(())
    }

    fn overrun(&self) -> BinaryError {
        BinaryError::LengthMismatch {
            what: "TBDE",
            expected: self.samples.len() * size_of::<u16>(),
            actual: self.payload_seen,
        }
    }

    /// Finish: the header must have arrived and the payload must be exactly what it declared.
    ///
    /// # Errors
    /// [`BinaryError::Truncated`] when the header never completed;
    /// [`BinaryError::LengthMismatch`] when the payload is short or carries a trailing odd byte.
    pub fn finish(self) -> Result<RawDem, BinaryError> {
        let Some(header) = self.header else {
            return Err(BinaryError::Truncated {
                what: "TBDE",
                expected: HEADER_BYTES,
                actual: self.head_len,
            });
        };
        if self.filled != self.samples.len() || self.pending.is_some() {
            return Err(BinaryError::LengthMismatch {
                what: "TBDE",
                expected: self.samples.len() * size_of::<u16>(),
                actual: self.payload_seen,
            });
        }
        Ok(RawDem {
            header,
            samples: self.samples,
        })
    }
}

/// Frame a header and its samples into the `TBDE` bytes an emitter writes.
///
/// Lives here rather than in the emitter so the writer and the reader are the same file: a field
/// order that drifted would have to drift in both halves of one screen to go unnoticed.
#[must_use]
pub fn to_bytes(header: &TbdeHeader, samples: &[u16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER_BYTES + size_of_val(samples));
    out.extend_from_slice(&header.to_header_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A range whose `f32` scale is exact. The span is written as `65535 / 128` rather than
    /// `511.9921875` so the property is visible: dividing it by 65535 gives exactly `2^-7`, and
    /// `-256` is a power of two, so the header's `f32` arithmetic and the PNG path's `f64`
    /// arithmetic land on the same `f32` for every one of the 65,536 samples. Metres parity is
    /// then a real equality rather than a tolerance that would hide a formula error.
    /// `parse_round_trips_the_grid_and_the_header` asserts the resulting `scale_m` is that `2^-7`.
    const EXACT_MIN_M: f32 = -256.0;
    const EXACT_MAX_M: f32 = EXACT_MIN_M + 65535.0 / 128.0;

    /// everon's shipped range (`manifest.json` `heightRangeMinM` / `heightRangeMaxM`).
    const EVERON_MIN_M: f32 = -204.78;
    const EVERON_MAX_M: f32 = 375.53;

    /// The ticket's 4x3 grid: distinct values, both endpoints, deliberately not square so a
    /// width/height swap cannot pass.
    const GRID_W: u32 = 4;
    const GRID_H: u32 = 3;
    fn grid() -> Vec<u16> {
        vec![
            0, 1, 65535, 32768, 40000, 7, 60000, 100, 12345, 2, 511, 65534,
        ]
    }

    fn framed(w: u32, h: u32, min_m: f32, max_m: f32, samples: &[u16]) -> Vec<u8> {
        to_bytes(&TbdeHeader::new(w, h, min_m, max_m), samples)
    }

    #[test]
    fn parse_round_trips_the_grid_and_the_header() {
        let s = grid();
        let dem =
            RawDem::parse(&framed(GRID_W, GRID_H, EXACT_MIN_M, EXACT_MAX_M, &s)).expect("parse");
        assert_eq!((dem.width(), dem.height()), (GRID_W, GRID_H));
        assert_eq!(dem.samples, s);
        assert_eq!(dem.header.flags, 0);
        assert_eq!(dem.header.reserved, [0_u8; 8]);
        assert_eq!(dem.header.offset_m, EXACT_MIN_M);
        assert_eq!(dem.header.scale_m, (EXACT_MAX_M - EXACT_MIN_M) / 65535.0);
        assert_eq!(
            dem.header.scale_m,
            1.0 / 128.0,
            "EXACT_* must quantise to an exact f32 step — the metres-parity tests rest on it"
        );
    }

    /// `(x, y)` must index row-major with `width` as the stride, and out-of-range must be `None`
    /// rather than a wrapped read. A width/height swap in the header read fails here.
    #[test]
    fn sample_u16_indexes_row_major_and_bounds_check() {
        let s = grid();
        let dem =
            RawDem::parse(&framed(GRID_W, GRID_H, EXACT_MIN_M, EXACT_MAX_M, &s)).expect("parse");
        for y in 0..GRID_H {
            for x in 0..GRID_W {
                assert_eq!(
                    dem.sample_u16(x, y),
                    Some(s[(y * GRID_W + x) as usize]),
                    "({x},{y})"
                );
            }
        }
        assert_eq!(dem.sample_u16(GRID_W, 0), None);
        assert_eq!(dem.sample_u16(0, GRID_H), None);
        assert_eq!(dem.metres(GRID_W, GRID_H), None);
    }

    /// The streaming decoder must produce the same grid as the whole-buffer parse for every chunk
    /// size, including the odd ones that split a `u16` and the ones smaller than the header.
    #[test]
    fn streamed_in_any_chunk_size_matches_the_whole_buffer() {
        let s = grid();
        let bytes = framed(GRID_W, GRID_H, EXACT_MIN_M, EXACT_MAX_M, &s);
        let whole = RawDem::parse(&bytes).expect("parse");
        for step in [1_usize, 2, 3, 5, 7, 11, 31, 32, 33, 47, bytes.len()] {
            let mut sink = RawDemSink::new(bytes.len() as u64);
            for c in bytes.chunks(step) {
                sink.push(c).unwrap_or_else(|e| panic!("step {step}: {e}"));
            }
            let got = sink.finish().unwrap_or_else(|e| panic!("step {step}: {e}"));
            assert_eq!(got, whole, "chunk size {step}");
        }
    }

    /// Both decode branches — the aligned `cast_slice` memcpy and the byte-pair fallback — must
    /// produce the same grid. Alignment is forced by construction rather than left to the
    /// allocator: `repr(align(2))` makes offset 0 castable and offset 1 provably not, and the two
    /// `try_cast_slice` assertions below pin that the two inputs really do take different branches
    /// (without them this test would be two runs of whichever branch the allocator happened to
    /// pick).
    #[test]
    fn misaligned_payload_decodes_identically_to_the_aligned_one() {
        #[repr(align(2))]
        struct Align2([u8; 64]);

        let s = grid();
        let bytes = framed(GRID_W, GRID_H, EXACT_MIN_M, EXACT_MAX_M, &s);
        let payload = &bytes[HEADER_BYTES..];
        let head = &bytes[..HEADER_BYTES];

        let mut even = Align2([0; 64]);
        even.0[..payload.len()].copy_from_slice(payload);
        let even_payload = &even.0[..payload.len()];
        let mut odd = Align2([0; 64]);
        odd.0[1..=payload.len()].copy_from_slice(payload);
        let odd_payload = &odd.0[1..=payload.len()];

        assert!(
            bytemuck::try_cast_slice::<u8, u16>(even_payload).is_ok(),
            "the even buffer must take the cast_slice branch"
        );
        assert!(
            bytemuck::try_cast_slice::<u8, u16>(odd_payload).is_err(),
            "the odd buffer must take the byte-pair branch"
        );

        let decode = |p: &[u8]| {
            let mut sink = RawDemSink::new(bytes.len() as u64);
            sink.push(head).expect("head");
            sink.push(p).expect("payload");
            sink.finish().expect("finish")
        };
        let from_even = decode(even_payload);
        assert_eq!(from_even.samples, s);
        assert_eq!(decode(odd_payload), from_even);
    }

    #[test]
    fn metres_is_offset_plus_sample_times_scale() {
        let s = grid();
        let dem =
            RawDem::parse(&framed(GRID_W, GRID_H, EVERON_MIN_M, EVERON_MAX_M, &s)).expect("parse");
        let scale = (EVERON_MAX_M - EVERON_MIN_M) / 65535.0;
        for y in 0..GRID_H {
            for x in 0..GRID_W {
                let v = s[(y * GRID_W + x) as usize];
                assert_eq!(dem.metres(x, y), Some(EVERON_MIN_M + f32::from(v) * scale));
            }
        }
        assert_eq!(dem.metres_grid().len(), s.len());
        assert_eq!(dem.metres_grid()[0], dem.metres(0, 0).expect("in range"));
    }

    #[test]
    fn bad_magic_and_version_are_reported_not_guessed() {
        let s = grid();
        let mut bytes = framed(GRID_W, GRID_H, EXACT_MIN_M, EXACT_MAX_M, &s);
        bytes[0] = b'X';
        assert!(matches!(
            RawDem::parse(&bytes),
            Err(BinaryError::BadMagic { .. })
        ));
        let mut bytes = framed(GRID_W, GRID_H, EXACT_MIN_M, EXACT_MAX_M, &s);
        bytes[4..6].copy_from_slice(&9_u16.to_le_bytes());
        assert!(matches!(
            RawDem::parse(&bytes),
            Err(BinaryError::UnsupportedVersion { .. })
        ));
        // A git-LFS pointer file is the realistic corruption in this tree.
        assert!(matches!(
            RawDem::parse(b"version https://git-lfs.github.com/spec/v1\n"),
            Err(BinaryError::BadMagic { .. })
        ));
    }

    #[test]
    fn truncated_header_and_short_payload_are_errors() {
        let s = grid();
        let bytes = framed(GRID_W, GRID_H, EXACT_MIN_M, EXACT_MAX_M, &s);
        assert!(matches!(
            RawDem::parse(&bytes[..HEADER_BYTES - 1]),
            Err(BinaryError::Truncated { .. })
        ));
        // The header is intact but two sample bytes are missing: `total_bytes` no longer matches
        // what the header declares, so this is caught before anything is allocated.
        assert!(matches!(
            RawDem::parse(&bytes[..bytes.len() - 2]),
            Err(BinaryError::LengthMismatch { .. })
        ));
        // A sink that is told the right length but never fed the tail fails at `finish`.
        let mut sink = RawDemSink::new(bytes.len() as u64);
        sink.push(&bytes[..bytes.len() - 2]).expect("prefix");
        assert!(matches!(
            sink.finish(),
            Err(BinaryError::LengthMismatch { .. })
        ));
        // A trailing odd byte is a short sample, not a rounding-down.
        let mut sink = RawDemSink::new(bytes.len() as u64);
        sink.push(&bytes[..bytes.len() - 1]).expect("prefix");
        assert!(matches!(
            sink.finish(),
            Err(BinaryError::LengthMismatch { .. })
        ));
    }

    /// More payload than the header declared must be an error, not a silent reallocation — the
    /// whole point of the single allocation is that it cannot grow.
    #[test]
    fn payload_overrun_is_an_error() {
        let s = grid();
        let bytes = framed(GRID_W, GRID_H, EXACT_MIN_M, EXACT_MAX_M, &s);
        let mut sink = RawDemSink::new(bytes.len() as u64);
        sink.push(&bytes).expect("whole file");
        assert!(matches!(
            sink.push(&[1, 2, 3, 4]),
            Err(BinaryError::LengthMismatch { .. })
        ));
    }

    /// **The single-allocation pin** — the claim this whole slice is built on, and the one the
    /// other tests cannot see: they all pass just as happily over a decoder that pushes into a
    /// growing vector and reallocates 24 times on the way to everon's 81.9 MB.
    ///
    /// So this one watches the buffer itself. Capacity alone would not settle it — a `Vec` may
    /// reallocate and land on the same capacity — so the buffer's *address* is pinned too, across
    /// a push per byte, which is the most fragmented delivery a socket can produce. `finish` is
    /// then held to the same standard: it must **move** the buffer out, not copy it.
    #[test]
    fn the_sample_vector_is_allocated_once_and_never_moves() {
        let s = grid();
        let bytes = framed(GRID_W, GRID_H, EXACT_MIN_M, EXACT_MAX_M, &s);
        let mut sink = RawDemSink::new(bytes.len() as u64);
        assert_eq!(
            sink.samples.capacity(),
            0,
            "nothing may be allocated before the header has been validated against the file length"
        );

        let mut byte_at_a_time = bytes.chunks(1);
        for c in byte_at_a_time.by_ref().take(HEADER_BYTES) {
            sink.push(c).expect("header byte");
        }
        let (ptr, cap) = (sink.samples.as_ptr(), sink.samples.capacity());
        assert_eq!(
            sink.samples.len(),
            s.len(),
            "the header must size the buffer to the whole grid at once, not incrementally"
        );

        for c in byte_at_a_time {
            sink.push(c).expect("payload byte");
            assert_eq!(
                sink.samples.as_ptr(),
                ptr,
                "the sample buffer moved: the payload decode reallocated"
            );
            assert_eq!(sink.samples.capacity(), cap, "the sample buffer regrew");
        }

        let dem = sink.finish().expect("finish");
        assert_eq!(
            dem.samples.as_ptr(),
            ptr,
            "`finish` must move the buffer out, not copy it into a second allocation"
        );
        assert_eq!(dem.samples, s);
    }

    /// A hostile header must never reach the allocator. `65535 x 65535` is 8.6 GB of `u16`; the
    /// length check rejects it against a 40-byte file first.
    #[test]
    fn hostile_dimensions_are_rejected_before_allocating() {
        let bytes = framed(65535, 65535, EXACT_MIN_M, EXACT_MAX_M, &[0_u16; 4]);
        assert_eq!(bytes.len(), HEADER_BYTES + 8);
        match RawDem::parse(&bytes) {
            Err(BinaryError::LengthMismatch {
                expected, actual, ..
            }) => {
                let declared: usize = HEADER_BYTES + 65535_usize * 65535 * 2;
                assert_eq!(expected, declared);
                assert_eq!(actual, bytes.len());
            }
            other => panic!("expected LengthMismatch, got {other:?}"),
        }
    }

    /// An empty grid is well-formed and decodes to nothing — the degenerate case must not be an
    /// off-by-one that reports success over a buffer it never looked at.
    #[test]
    fn zero_by_zero_grid_is_empty_not_an_error() {
        let dem = RawDem::parse(&framed(0, 0, EXACT_MIN_M, EXACT_MAX_M, &[])).expect("parse");
        assert!(dem.samples.is_empty());
        assert_eq!(dem.sample_u16(0, 0), None);
    }

    /// **The acceptance test.** The same `u16` grid encoded as a 16-bit PNG and as a `TBDE` file
    /// must decode to the same samples through the two independent decoders, and to the same
    /// metres: `EXACT_*` makes the header's `f32` scale exact, so this is equality, not tolerance.
    #[cfg(feature = "png")]
    #[test]
    fn dem_and_png_decode_to_the_same_grid_and_metres() {
        use super::super::png_decode::decode_png_gray16;
        use super::super::sample::uint16_to_meters;

        let s = grid();
        let mut png_bytes = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut png_bytes, GRID_W, GRID_H);
            enc.set_color(png::ColorType::Grayscale);
            enc.set_depth(png::BitDepth::Sixteen);
            let mut w = enc.write_header().expect("png header");
            let be: Vec<u8> = s.iter().flat_map(|v| v.to_be_bytes()).collect();
            w.write_image_data(&be).expect("png data");
        }
        let (png_raster, pw, ph) = decode_png_gray16(&png_bytes).expect("png decode");
        let dem =
            RawDem::parse(&framed(GRID_W, GRID_H, EXACT_MIN_M, EXACT_MAX_M, &s)).expect("dem");

        assert_eq!((dem.width(), dem.height()), (pw, ph), "dims");
        assert_eq!(dem.samples, png_raster, "u16 grid");
        for y in 0..ph {
            for x in 0..pw {
                let png_m = uint16_to_meters(
                    f64::from(png_raster[(y * pw + x) as usize]),
                    f64::from(EXACT_MIN_M),
                    f64::from(EXACT_MAX_M),
                ) as f32;
                let raw_m = dem.metres(x, y).expect("in range");
                assert!(
                    (f64::from(raw_m) - f64::from(png_m)).abs() <= 1e-6,
                    "({x},{y}): raw {raw_m} vs png {png_m}"
                );
            }
        }
    }

    /// The same equality at everon's shipped height range, where the header's `f32` scale is
    /// **not** exact. The `u16` grid is still identical; metres differ only by the `f32` rounding
    /// of `scale_m` and `offset_m` (spec §3.2 stores both as `f32`), which this bounds at 1e-4 m
    /// — 88x finer than the 8.85 mm the `u16` quantisation itself costs.
    #[cfg(feature = "png")]
    #[test]
    fn everon_range_keeps_the_grid_exact_and_metres_within_f32_rounding() {
        use super::super::sample::uint16_to_meters;

        let s: Vec<u16> = (0..=65535_u32).step_by(97).map(|v| v as u16).collect();
        let w = s.len() as u32;
        let dem = RawDem::parse(&framed(w, 1, EVERON_MIN_M, EVERON_MAX_M, &s)).expect("dem");
        assert_eq!(
            dem.samples, s,
            "u16 grid must be bit-exact regardless of range"
        );
        let mut worst = 0.0_f64;
        for (x, &v) in s.iter().enumerate() {
            let png_m = uint16_to_meters(
                f64::from(v),
                f64::from(EVERON_MIN_M),
                f64::from(EVERON_MAX_M),
            ) as f32;
            let raw_m = dem.metres(x as u32, 0).expect("in range");
            worst = worst.max((f64::from(raw_m) - f64::from(png_m)).abs());
        }
        assert!(
            worst <= 1e-4,
            "worst metre delta {worst} over everon's range"
        );
    }
}
