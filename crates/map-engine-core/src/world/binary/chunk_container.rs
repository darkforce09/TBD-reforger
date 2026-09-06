//! T-935.1 — the fixed 32-byte container headers: `TBDC` chunks, `TBDE` DEM, `TBDB` bathymetry,
//! `TBDS` v2 satellite (spec §3).
//!
//! All four are `#[repr(C)]` `bytemuck::Pod` and all four are **exactly 32 bytes with no padding**,
//! which is not decoration: a 32-byte header keeps the payload that follows 4-aligned (so a
//! 4-aligned file buffer casts to [`ObjectInstancePod`] with no copy) and it means every offset in
//! the format is `32 + …`, computable by a Range reader that has not downloaded the body yet.
//!
//! Field orders come from the spec and were chosen so the compiler inserts nothing; each type
//! carries a `const _: () = assert!(size_of == 32)` so a future edit that reorders or widens a
//! field fails the build instead of silently shifting every committed file by two bytes.
//!
//! # Reading
//!
//! [`ContainerHeader::parse`] is zero-copy and needs a 4-aligned buffer;
//! [`ContainerHeader::read`] copies the 32 bytes out and works on any buffer, which is what a
//! `Vec<u8>` straight off an HTTP fetch usually needs. Both validate magic and version, both
//! return [`BinaryError`] rather than panicking, and both hand back the payload slice so the
//! caller cannot forget where the body starts.

use bytemuck::{Pod, Zeroable};

use super::BinaryError;
use super::pod::{self, ObjectInstancePod, POD_BYTES};

/// Every TBD container header is this long. Also the offset of the payload.
pub const HEADER_BYTES: usize = 32;

/// `objects/chunks/{cx}_{cy}.bin`.
pub const TBDC_MAGIC: [u8; 4] = *b"TBDC";
/// `dem/elevation.dem`.
pub const TBDE_MAGIC: [u8; 4] = *b"TBDE";
/// `water/bathymetry.tbd-bath`.
pub const TBDB_MAGIC: [u8; 4] = *b"TBDB";
/// `satellite/{terrain}-sat.tbd-sat`.
pub const TBDS_MAGIC: [u8; 4] = *b"TBDS";

/// The version this build writes and reads for the three Tier-1/3 containers.
pub const CONTAINER_VERSION: u16 = 1;
/// `TBDS` is the one format with a shipped predecessor: v1 is the hand-packed tile table already in
/// `everon-sat.tbd-sat`, v2 is the rkyv index ([`super::archives::TbdSatIndexV2`]). A reader
/// dispatches on [`peek_version`] before choosing a header type.
pub const TBDS_VERSION_V2: u16 = 2;

/// The shared header behaviour: identify yourself, then split a buffer into header and payload.
///
/// Implementors are `Pod`, so the parse is a cast; the trait exists so the *validation* — length,
/// magic, version, in that order — is written once and cannot drift between four formats.
pub trait ContainerHeader: Pod + Copy + Sized {
    /// The four bytes at offset 0.
    const MAGIC: [u8; 4];
    /// The version this build understands.
    const VERSION: u16;
    /// Human name for error messages (`"TBDC"`).
    const NAME: &'static str;

    /// The magic actually stored in this header instance.
    fn magic_bytes(&self) -> [u8; 4];
    /// The version actually stored in this header instance.
    fn format_version(&self) -> u16;

    /// Magic then version, in that order — a wrong magic must not be reported as a version
    /// problem, because those two send a caller down completely different paths.
    ///
    /// # Errors
    /// [`BinaryError::BadMagic`] or [`BinaryError::UnsupportedVersion`].
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

    /// Zero-copy parse: `(&header, payload)`.
    ///
    /// # Errors
    /// [`BinaryError::Truncated`] under 32 bytes, [`BinaryError::Misaligned`] when `bytes` is not
    /// 4-aligned (use [`ContainerHeader::read`] then), or whatever [`ContainerHeader::validate`]
    /// rejects.
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

    /// Alignment-free parse: copies the 32 bytes out and returns them by value.
    ///
    /// This is the one to call on a `Vec<u8>` from the network — 32 bytes is nothing, and the
    /// alternative is a failed cast on a buffer that is otherwise perfectly good.
    ///
    /// # Errors
    /// As [`ContainerHeader::parse`], minus [`BinaryError::Misaligned`], which cannot occur.
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

    /// This header's own 32 bytes, ready to write.
    fn to_header_bytes(&self) -> [u8; HEADER_BYTES] {
        let mut out = [0_u8; HEADER_BYTES];
        out.copy_from_slice(bytemuck::bytes_of(self));
        out
    }
}

/// Read the `version` field of any TBD container without committing to a header type.
///
/// The `.tbd-sat` reader needs exactly this: v1 and v2 are different formats behind the same
/// magic, and the four bytes that say which arrive in the same Range read as the rest.
///
/// # Errors
/// [`BinaryError::Truncated`] when `bytes` is shorter than the magic plus the version field.
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

/* ───────────────────────────────── TBDC — chunk container ───────────────────────────────── */

/// `objects/chunks/{cx}_{cy}.bin` — `count` [`ObjectInstancePod`] rows and nothing else.
/// File length is exactly `32 + 32 * count`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod, Zeroable)]
pub struct TbdcHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub flags: u16,
    /// Instance rows in the payload.
    pub count: u32,
    /// Chunk index, signed — the same `cx`/`cy` as the `{cx}_{cy}` id.
    pub cx: i16,
    pub cy: i16,
    pub reserved: [u8; 16],
}

const _: () = assert!(size_of::<TbdcHeader>() == HEADER_BYTES);

impl ContainerHeader for TbdcHeader {
    const MAGIC: [u8; 4] = TBDC_MAGIC;
    const VERSION: u16 = CONTAINER_VERSION;
    const NAME: &'static str = "TBDC";
    fn magic_bytes(&self) -> [u8; 4] {
        self.magic
    }
    fn format_version(&self) -> u16 {
        self.version
    }
}

impl TbdcHeader {
    /// A valid header for `count` rows of chunk `(cx, cy)`.
    #[must_use]
    pub fn new(cx: i16, cy: i16, count: u32) -> Self {
        Self {
            magic: TBDC_MAGIC,
            version: CONTAINER_VERSION,
            flags: 0,
            count,
            cx,
            cy,
            reserved: [0; 16],
        }
    }

    /// Payload length this header implies — `None` when the product does not fit `usize`.
    ///
    /// CHECKED BECAUSE THE LOADER IS 32-BIT. `usize` is 4 bytes on `wasm32-unknown-unknown`, the
    /// target this format exists for, so `count as usize * POD_BYTES` wraps for any `count`
    /// above 2^27. Measured on a real wasm32 build under node vs the same code native:
    /// `TbdcHeader::new(0, 0, 0x0800_0000).instances(&[])` is `Err(LengthMismatch)` on x86_64 and
    /// `Ok(0 rows)` on wasm32 — the length check that exists so "a truncated download reads as a
    /// short-but-valid chunk" cannot happen was void on the one platform it is for.
    #[must_use]
    pub fn payload_bytes(&self) -> Option<usize> {
        (self.count as usize).checked_mul(POD_BYTES)
    }

    /// Whole-file length this header implies — `None` on the same overflow.
    #[must_use]
    pub fn file_bytes(&self) -> Option<usize> {
        HEADER_BYTES.checked_add(self.payload_bytes()?)
    }

    /// The payload as instance rows, zero-copy, with the header's `count` enforced.
    ///
    /// The length check is the point: without it a truncated download reads as a short-but-valid
    /// chunk and objects silently disappear from the map.
    ///
    /// # Errors
    /// [`BinaryError::LengthMismatch`] when the payload is not exactly `count * 32` bytes, or
    /// [`BinaryError::Misaligned`] when it cannot be cast in place.
    pub fn instances<'a>(&self, payload: &'a [u8]) -> Result<&'a [ObjectInstancePod], BinaryError> {
        // An overflowing header can never be satisfied by a real buffer, so it is a length
        // mismatch — reported with the count that caused it rather than a wrapped number.
        let Some(expected) = self.payload_bytes() else {
            return Err(BinaryError::LengthMismatch {
                what: Self::NAME,
                expected: usize::MAX,
                actual: payload.len(),
            });
        };
        if payload.len() != expected {
            return Err(BinaryError::LengthMismatch {
                what: Self::NAME,
                expected,
                actual: payload.len(),
            });
        }
        pod::instances_from_bytes(payload)
    }
}

/* ──────────────────────────────────── TBDE — raw DEM ──────────────────────────────────── */

/// `dem/elevation.dem` — `width * height` `u16` samples, row-major, row 0 = north edge (the same
/// orientation as the 16-bit PNG it replaces). Metres are `offset_m + sample * scale_m`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct TbdeHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub flags: u16,
    pub width: u32,
    pub height: u32,
    /// Metres per quantisation step: `(max_m - min_m) / 65535`.
    pub scale_m: f32,
    /// Metres at sample `0` — everon's `-204.78`.
    pub offset_m: f32,
    pub reserved: [u8; 8],
}

const _: () = assert!(size_of::<TbdeHeader>() == HEADER_BYTES);

impl ContainerHeader for TbdeHeader {
    const MAGIC: [u8; 4] = TBDE_MAGIC;
    const VERSION: u16 = CONTAINER_VERSION;
    const NAME: &'static str = "TBDE";
    fn magic_bytes(&self) -> [u8; 4] {
        self.magic
    }
    fn format_version(&self) -> u16 {
        self.version
    }
}

impl TbdeHeader {
    /// A valid header for a `width × height` grid spanning `[min_m, max_m]`.
    #[must_use]
    pub fn new(width: u32, height: u32, min_m: f32, max_m: f32) -> Self {
        Self {
            magic: TBDE_MAGIC,
            version: CONTAINER_VERSION,
            flags: 0,
            width,
            height,
            scale_m: (max_m - min_m) / f32::from(u16::MAX),
            offset_m: min_m,
            reserved: [0; 8],
        }
    }

    /// Samples the header implies — `None` on 32-bit overflow (see `TbdcHeader::payload_bytes`).
    #[must_use]
    pub fn sample_count(&self) -> Option<usize> {
        (self.width as usize).checked_mul(self.height as usize)
    }

    /// Payload length this header implies — `None` on 32-bit overflow (see `TbdcHeader`).
    #[must_use]
    pub fn payload_bytes(&self) -> Option<usize> {
        self.sample_count()?.checked_mul(size_of::<u16>())
    }

    /// The payload as `u16` samples, zero-copy, with `width * height` enforced.
    ///
    /// # Errors
    /// [`BinaryError::LengthMismatch`] or [`BinaryError::Misaligned`].
    pub fn samples<'a>(&self, payload: &'a [u8]) -> Result<&'a [u16], BinaryError> {
        let Some(expected) = self.payload_bytes() else {
            return Err(BinaryError::LengthMismatch {
                what: Self::NAME,
                expected: usize::MAX,
                actual: payload.len(),
            });
        };
        if payload.len() != expected {
            return Err(BinaryError::LengthMismatch {
                what: Self::NAME,
                expected,
                actual: payload.len(),
            });
        }
        bytemuck::try_cast_slice(payload).map_err(|_| BinaryError::Misaligned {
            what: Self::NAME,
            align: align_of::<u16>(),
        })
    }

    /// Decode one quantised sample to metres.
    #[must_use]
    pub fn metres(&self, sample: u16) -> f32 {
        self.offset_m + f32::from(sample) * self.scale_m
    }
}

/* ─────────────────────────────────── TBDB — bathymetry ─────────────────────────────────── */

/// `water/bathymetry.tbd-bath` — a mip pyramid of depth + water-mask levels.
///
/// Level `L` is `w = max(1, width >> L)` by `h = max(1, height >> L)`: `w*h` `u16` depths (metres
/// are `v * depth_scale`), then `w*h` `u8` mask bytes (`1` = water), then padding to a multiple of
/// 4 bytes so the next level's depths stay 2-aligned and every offset is header-computable.
/// Downsampling is `depth = max` of the 2×2 block and `mask = any water` — the conservative
/// direction, so a coarse level never claims dry ground that is wet.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct TbdbHeader {
    pub magic: [u8; 4],
    pub version: u16,
    /// Levels present, `L = 0..mip_count`.
    pub mip_count: u16,
    pub width: u32,
    pub height: u32,
    /// Metres per depth unit.
    pub depth_scale: f32,
    pub reserved: [u8; 12],
}

const _: () = assert!(size_of::<TbdbHeader>() == HEADER_BYTES);

impl ContainerHeader for TbdbHeader {
    const MAGIC: [u8; 4] = TBDB_MAGIC;
    const VERSION: u16 = CONTAINER_VERSION;
    const NAME: &'static str = "TBDB";
    fn magic_bytes(&self) -> [u8; 4] {
        self.magic
    }
    fn format_version(&self) -> u16 {
        self.version
    }
}

/// Where one bathymetry level's two blocks live, **relative to the start of the payload** (add
/// [`HEADER_BYTES`] for a file offset / HTTP Range).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LevelSpan {
    pub width: u32,
    pub height: u32,
    pub depth_offset: usize,
    pub depth_bytes: usize,
    pub mask_offset: usize,
    pub mask_bytes: usize,
    /// `depth_bytes + mask_bytes` rounded up to a multiple of 4 — the stride to the next level.
    pub stride: usize,
}

impl TbdbHeader {
    /// A valid header for a `width × height` base with `mip_count` levels.
    #[must_use]
    pub fn new(width: u32, height: u32, mip_count: u16, depth_scale: f32) -> Self {
        Self {
            magic: TBDB_MAGIC,
            version: CONTAINER_VERSION,
            mip_count,
            width,
            height,
            depth_scale,
            reserved: [0; 12],
        }
    }

    /// Dimensions of level `level`, or `None` past `mip_count`.
    #[must_use]
    pub fn level_dims(&self, level: u16) -> Option<(u32, u32)> {
        if level >= self.mip_count {
            return None;
        }
        let shift = u32::from(level);
        Some(((self.width >> shift).max(1), (self.height >> shift).max(1)))
    }

    /// Offsets and sizes of level `level`, or `None` past `mip_count`. Computed from the header
    /// alone, which is what lets a Range reader fetch one level without the file.
    #[must_use]
    pub fn level_span(&self, level: u16) -> Option<LevelSpan> {
        let mut offset = 0_usize;
        for l in 0..=level {
            let (w, h) = self.level_dims(l)?;
            let texels = (w as usize).checked_mul(h as usize)?;
            let depth_bytes = texels.checked_mul(size_of::<u16>())?;
            let mask_bytes = texels;
            let stride = depth_bytes
                .checked_add(mask_bytes)?
                .checked_next_multiple_of(4)?;
            if l == level {
                return Some(LevelSpan {
                    width: w,
                    height: h,
                    depth_offset: offset,
                    depth_bytes,
                    mask_offset: offset.checked_add(depth_bytes)?,
                    mask_bytes,
                    stride,
                });
            }
            offset += stride;
        }
        None
    }

    /// Payload length the whole pyramid implies.
    #[must_use]
    pub fn payload_bytes(&self) -> usize {
        (0..self.mip_count)
            .filter_map(|l| self.level_span(l))
            .map(|s| s.stride)
            .sum()
    }

    /// Decode one quantised depth to metres.
    #[must_use]
    pub fn metres(&self, depth: u16) -> f32 {
        f32::from(depth) * self.depth_scale
    }
}

/* ───────────────────────────────── TBDS v2 — satellite ───────────────────────────────── */

/// `satellite/{terrain}-sat.tbd-sat` **version 2** — a fixed 32-byte header, then `index_len` bytes
/// of rkyv [`TbdSatIndexV2`](super::archives::TbdSatIndexV2), then the tile bytes.
///
/// Tile offsets recorded in the index are relative to the *end of the index*, i.e. absolute file
/// offset `32 + index_len + tile.offset` — see [`TbdsHeader::tiles_offset`]. Version 1 is the
/// shipped hand-packed table; dispatch with [`peek_version`] before parsing as v2.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod, Zeroable)]
pub struct TbdsHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub flags: u16,
    /// Bytes of rkyv index immediately after this header.
    pub index_len: u32,
    pub reserved: [u8; 20],
}

const _: () = assert!(size_of::<TbdsHeader>() == HEADER_BYTES);

impl ContainerHeader for TbdsHeader {
    const MAGIC: [u8; 4] = TBDS_MAGIC;
    const VERSION: u16 = TBDS_VERSION_V2;
    const NAME: &'static str = "TBDS";
    fn magic_bytes(&self) -> [u8; 4] {
        self.magic
    }
    fn format_version(&self) -> u16 {
        self.version
    }
}

impl TbdsHeader {
    /// A valid v2 header for an index of `index_len` bytes.
    #[must_use]
    pub fn new(index_len: u32) -> Self {
        Self {
            magic: TBDS_MAGIC,
            version: TBDS_VERSION_V2,
            flags: 0,
            index_len,
            reserved: [0; 20],
        }
    }

    /// File offset where tile bytes begin — the origin every `Tile::offset` is measured from.
    #[must_use]
    pub fn tiles_offset(&self) -> usize {
        HEADER_BYTES + self.index_len as usize
    }

    /// The rkyv index bytes out of a payload (a payload being everything after the header), ready
    /// for [`access_checked`](super::access_checked).
    ///
    /// # Errors
    /// [`BinaryError::Truncated`] when the payload is shorter than `index_len`.
    pub fn index<'a>(&self, payload: &'a [u8]) -> Result<&'a [u8], BinaryError> {
        let n = self.index_len as usize;
        payload.get(..n).ok_or(BinaryError::Truncated {
            what: Self::NAME,
            expected: n,
            actual: payload.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `AlignedBuf` guarantees a 4-aligned byte buffer so the zero-copy `parse` path is exercised
    /// deterministically instead of at the allocator's whim.
    #[repr(align(4))]
    struct AlignedBuf([u8; 128]);

    fn framed<H: ContainerHeader>(h: &H, payload: &[u8]) -> AlignedBuf {
        let mut b = AlignedBuf([0; 128]);
        b.0[..HEADER_BYTES].copy_from_slice(&h.to_header_bytes());
        b.0[HEADER_BYTES..HEADER_BYTES + payload.len()].copy_from_slice(payload);
        b
    }

    #[test]
    fn all_headers_are_thirty_two_bytes() {
        assert_eq!(size_of::<TbdcHeader>(), HEADER_BYTES);
        assert_eq!(size_of::<TbdeHeader>(), HEADER_BYTES);
        assert_eq!(size_of::<TbdbHeader>(), HEADER_BYTES);
        assert_eq!(size_of::<TbdsHeader>(), HEADER_BYTES);
    }

    /// Padding-free: the sum of the declared fields must reach 32 with nothing left over. A
    /// reordering that introduced two bytes of slack would keep `size_of == 32` only by dropping
    /// bytes from `reserved`, which this catches.
    #[test]
    fn header_fields_sum_to_thirty_two() {
        assert_eq!(4 + 2 + 2 + 4 + 2 + 2 + 16, HEADER_BYTES); // TBDC
        assert_eq!(4 + 2 + 2 + 4 + 4 + 4 + 4 + 8, HEADER_BYTES); // TBDE
        assert_eq!(4 + 2 + 2 + 4 + 4 + 4 + 12, HEADER_BYTES); // TBDB
        assert_eq!(4 + 2 + 2 + 4 + 20, HEADER_BYTES); // TBDS
    }

    #[test]
    fn tbdc_round_trips_and_reads_its_instances() {
        let rows = [ObjectInstancePod::identity(), ObjectInstancePod::identity()];
        let head = TbdcHeader::new(-7, 12, 2);
        let buf = framed(&head, pod::instances_to_bytes(&rows));
        let file = &buf.0[..head.file_bytes().expect("no overflow in a fixture")];

        let (h, payload) = TbdcHeader::parse(file).expect("aligned parse");
        assert_eq!(*h, head);
        assert_eq!(h.cx, -7);
        assert_eq!(h.cy, 12);
        assert_eq!(h.file_bytes(), Some(HEADER_BYTES + 64));
        assert_eq!(h.instances(payload).expect("cast"), &rows[..]);

        let (owned, payload2) = TbdcHeader::read(file).expect("unaligned-safe read");
        assert_eq!(owned, head);
        assert_eq!(payload2.len(), 64);
    }

    /// The header bytes themselves are the format — magic at 0, LE version at 4, LE count at 8,
    /// LE `cx`/`cy` at 12/14.
    #[test]
    fn tbdc_header_bytes_are_the_wire_layout() {
        let b = TbdcHeader::new(-7, 12, 258).to_header_bytes();
        assert_eq!(&b[0..4], b"TBDC");
        assert_eq!(&b[4..6], &1_u16.to_le_bytes());
        assert_eq!(&b[6..8], &0_u16.to_le_bytes());
        assert_eq!(&b[8..12], &258_u32.to_le_bytes());
        assert_eq!(&b[12..14], &(-7_i16).to_le_bytes());
        assert_eq!(&b[14..16], &12_i16.to_le_bytes());
        assert_eq!(&b[16..32], &[0_u8; 16]);
    }

    #[test]
    fn tbde_round_trips_and_decodes_metres() {
        let head = TbdeHeader::new(4, 2, -204.78, 375.53);
        let samples: [u16; 8] = [0, 1, 2, 3, 4, 5, 6, u16::MAX];
        let buf = framed(&head, bytemuck::cast_slice(&samples));
        let file = &buf.0[..HEADER_BYTES + head.payload_bytes().expect("no overflow in a fixture")];

        let (h, payload) = TbdeHeader::parse(file).expect("aligned parse");
        assert_eq!(*h, head);
        assert_eq!(h.sample_count(), Some(8));
        assert_eq!(h.samples(payload).expect("cast"), &samples[..]);
        assert!((h.metres(0) - (-204.78)).abs() < 1e-2);
        assert!((h.metres(u16::MAX) - 375.53).abs() < 1e-2);
    }

    #[test]
    fn tbde_header_bytes_are_the_wire_layout() {
        let head = TbdeHeader::new(6400, 6400, -204.78, 375.53);
        let b = head.to_header_bytes();
        assert_eq!(&b[0..4], b"TBDE");
        assert_eq!(&b[4..6], &1_u16.to_le_bytes());
        assert_eq!(&b[8..12], &6400_u32.to_le_bytes());
        assert_eq!(&b[12..16], &6400_u32.to_le_bytes());
        assert_eq!(&b[16..20], &head.scale_m.to_le_bytes());
        assert_eq!(&b[20..24], &(-204.78_f32).to_le_bytes());
        assert_eq!(&b[24..32], &[0_u8; 8]);
    }

    #[test]
    fn tbdb_level_spans_are_header_computable() {
        let head = TbdbHeader::new(4, 4, 3, 0.5);
        assert_eq!(head.level_dims(0), Some((4, 4)));
        assert_eq!(head.level_dims(1), Some((2, 2)));
        assert_eq!(head.level_dims(2), Some((1, 1)));
        assert_eq!(head.level_dims(3), None);

        // L0: 16 texels → 32 B depth + 16 B mask = 48 (already ×4).
        let l0 = head.level_span(0).expect("L0");
        assert_eq!(
            (
                l0.depth_offset,
                l0.depth_bytes,
                l0.mask_offset,
                l0.mask_bytes
            ),
            (0, 32, 32, 16)
        );
        assert_eq!(l0.stride, 48);
        // L1: 4 texels → 8 + 4 = 12 (already ×4), starting right after L0.
        let l1 = head.level_span(1).expect("L1");
        assert_eq!((l1.depth_offset, l1.stride), (48, 12));
        // L2: 1 texel → 2 + 1 = 3, padded up to 4. This is the padding rule.
        let l2 = head.level_span(2).expect("L2");
        assert_eq!((l2.depth_offset, l2.depth_bytes, l2.mask_bytes), (60, 2, 1));
        assert_eq!(l2.stride, 4);
        assert_eq!(head.level_span(3), None);
        assert_eq!(head.payload_bytes(), 48 + 12 + 4);
        assert!((head.metres(10) - 5.0).abs() < 1e-6);
    }

    /// Non-square and non-power-of-two: the `max(1, …)` clamp must stop a dimension collapsing to
    /// zero, which would make a level's stride 0 and every deeper offset wrong.
    /// T-935.1 follow-up — the length check must hold on a 32-bit `usize`, which is what the
    /// loader actually runs on.
    ///
    /// `wasm32-unknown-unknown` has a 4-byte `usize`, so `count as usize * POD_BYTES` wrapped for
    /// any count above 2^27 and a header claiming 134,217,728 instances "matched" an EMPTY
    /// payload. Measured by the wave 237 verifier on a real wasm32 build under node against the
    /// same code native: `Err(LengthMismatch)` on x86_64, `Ok(0 rows)` on wasm32. The arithmetic
    /// is checked now, so the count that overflows a 32-bit address space is refused on BOTH.
    ///
    /// Written to be meaningful on a 64-bit host too: `u32::MAX` instances at 32 bytes each
    /// overflows nothing on 64-bit, so the 64-bit half asserts the honest large answer while the
    /// 32-bit half asserts the refusal.
    #[test]
    fn a_count_that_cannot_fit_the_address_space_is_refused_not_wrapped() {
        // 2^27 instances × 32 B = 2^32 B — exactly one past a 32-bit usize.
        let head = TbdcHeader::new(0, 0, 0x0800_0000);
        let bytes = head.payload_bytes();
        println!("── usize {} bits ── payload_bytes = {bytes:?}", usize::BITS);
        if usize::BITS == 32 {
            assert_eq!(bytes, None, "the product does not fit and must not wrap");
        } else {
            assert_eq!(bytes, Some(0x1_0000_0000));
        }
        // Either way an EMPTY payload is never a valid answer for it.
        let err = head
            .instances(&[])
            .expect_err("an empty payload cannot satisfy 2^27 instances");
        println!("── instances(&[]) ── {err}");
        assert!(matches!(err, BinaryError::LengthMismatch { .. }));
    }

    #[test]
    fn tbdb_levels_clamp_at_one_texel() {
        let head = TbdbHeader::new(3, 1, 3, 1.0);
        assert_eq!(head.level_dims(0), Some((3, 1)));
        assert_eq!(head.level_dims(1), Some((1, 1)));
        assert_eq!(head.level_dims(2), Some((1, 1)));
        assert_eq!(head.payload_bytes(), 12 + 4 + 4);
    }

    #[test]
    fn tbdb_round_trips() {
        let head = TbdbHeader::new(2, 2, 1, 0.25);
        let payload = vec![0_u8; head.payload_bytes()];
        let buf = framed(&head, &payload);
        let (h, p) = TbdbHeader::parse(&buf.0[..HEADER_BYTES + payload.len()]).expect("parse");
        assert_eq!(*h, head);
        assert_eq!(p.len(), head.payload_bytes());
    }

    #[test]
    fn tbds_v2_frames_its_index() {
        let head = TbdsHeader::new(9);
        let payload = [1_u8, 2, 3, 4, 5, 6, 7, 8, 9, 99, 99];
        let buf = framed(&head, &payload);
        let (h, p) = TbdsHeader::parse(&buf.0[..HEADER_BYTES + payload.len()]).expect("parse");
        assert_eq!(*h, head);
        assert_eq!(h.version, TBDS_VERSION_V2);
        assert_eq!(h.tiles_offset(), HEADER_BYTES + 9);
        assert_eq!(h.index(p).expect("index"), &payload[..9]);
        assert!(
            TbdsHeader::new(64).index(p).is_err(),
            "index longer than payload must be Err"
        );
    }

    /// A v1 `.tbd-sat` must be *dispatched*, not misparsed: `peek_version` reports v1 and the v2
    /// header type refuses it.
    #[test]
    fn tbds_v1_is_rejected_by_the_v2_header_but_peekable() {
        let mut b = AlignedBuf([0; 128]);
        b.0[..4].copy_from_slice(&TBDS_MAGIC);
        b.0[4..6].copy_from_slice(&1_u16.to_le_bytes());
        assert_eq!(peek_version(&b.0).expect("peek"), (TBDS_MAGIC, 1));
        let err = TbdsHeader::parse(&b.0[..HEADER_BYTES]).expect_err("v1 is not v2");
        assert_eq!(
            err,
            BinaryError::UnsupportedVersion {
                what: "TBDS",
                expected: 2,
                actual: 1
            }
        );
    }

    #[test]
    fn truncated_buffer_is_err_not_panic() {
        let head = TbdcHeader::new(0, 0, 0);
        let buf = framed(&head, &[]);
        for n in 0..HEADER_BYTES {
            let err = TbdcHeader::parse(&buf.0[..n]).expect_err("short header");
            assert!(matches!(err, BinaryError::Truncated { .. }), "{n}: {err}");
            assert!(TbdcHeader::read(&buf.0[..n]).is_err(), "{n}");
        }
        assert!(peek_version(&buf.0[..5]).is_err());
    }

    #[test]
    fn wrong_magic_is_err_not_panic() {
        let mut buf = framed(&TbdcHeader::new(0, 0, 0), &[]);
        buf.0[..4].copy_from_slice(b"vers"); // a git-LFS pointer, the trap this really catches
        let err = TbdcHeader::parse(&buf.0[..HEADER_BYTES]).expect_err("wrong magic");
        assert_eq!(
            err,
            BinaryError::BadMagic {
                what: "TBDC",
                expected: TBDC_MAGIC,
                actual: *b"vers"
            }
        );
        assert!(err.to_string().contains("b\"vers\""), "{err}");
    }

    /// Magic is checked before version: a `TBDE` buffer handed to the `TBDC` parser must say
    /// "bad magic", not "unsupported version", because the two send callers different ways.
    #[test]
    fn magic_is_checked_before_version() {
        let buf = framed(&TbdeHeader::new(1, 1, 0.0, 1.0), &[]);
        let err = TbdcHeader::parse(&buf.0[..HEADER_BYTES]).expect_err("TBDE is not TBDC");
        assert!(matches!(err, BinaryError::BadMagic { .. }), "{err}");
    }

    #[test]
    fn wrong_version_is_err_not_panic() {
        let mut head = TbdcHeader::new(0, 0, 0);
        head.version = 99;
        let buf = framed(&head, &[]);
        let err = TbdcHeader::parse(&buf.0[..HEADER_BYTES]).expect_err("v99");
        assert_eq!(
            err,
            BinaryError::UnsupportedVersion {
                what: "TBDC",
                expected: 1,
                actual: 99
            }
        );
    }

    /// A count that disagrees with the payload is the truncated-download case, and it must be an
    /// error rather than a short read.
    #[test]
    fn count_payload_mismatch_is_err() {
        let head = TbdcHeader::new(0, 0, 4);
        let rows = [ObjectInstancePod::identity()];
        let err = head
            .instances(pod::instances_to_bytes(&rows))
            .expect_err("1 row for a count of 4");
        assert_eq!(
            err,
            BinaryError::LengthMismatch {
                what: "TBDC",
                expected: 128,
                actual: 32
            }
        );
        let short = TbdeHeader::new(4, 4, 0.0, 1.0);
        assert!(short.samples(&[0_u8; 4]).is_err());
    }

    /// `read` is what a `Vec<u8>` off the network needs: `parse` fails on a 1-aligned buffer, and
    /// `read` returns the identical header from the identical bytes.
    #[test]
    fn misaligned_buffer_parses_via_read_only() {
        let head = TbdcHeader::new(3, 4, 0);
        // 4-aligned backing, so "one byte in" is deterministically misaligned rather than
        // whatever the allocator or the stack frame happened to hand out.
        let mut v = AlignedBuf([0; 128]);
        v.0[1..=HEADER_BYTES].copy_from_slice(&head.to_header_bytes());
        let err = TbdcHeader::parse(&v.0[1..=HEADER_BYTES]).expect_err("odd address");
        assert!(matches!(err, BinaryError::Misaligned { .. }), "{err}");
        let (owned, payload) = TbdcHeader::read(&v.0[1..=HEADER_BYTES]).expect("read copies");
        assert_eq!(owned, head);
        assert!(payload.is_empty());
    }
}
