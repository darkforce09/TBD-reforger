//! T-935.9 — the runtime water reader: the `TBDB` bathymetry pyramid and the
//! `water/water_vectors.rkyv` archive (spec §3.3, §4).
//!
//! # Why this lives in the core and not in the SPA
//!
//! [`WaterMask::is_water`] is a **placement guard's** question, and a wrong answer is a squad
//! spawned in a lake. `world_assets` is `#![cfg(target_arch = "wasm32")]` and this repo has no
//! wasm-bindgen-test harness, so anything decided over there is guarded by compilation alone. Every
//! decision that can be wrong — the payload-length check, the world→texel mapping, the mip fold,
//! the schema-version gate — is therefore made here, where `cargo test` executes it. The SPA module
//! is the fetch shim on top ([`crate::world`] is what it links).
//!
//! # What the query answers, precisely
//!
//! [`WaterMask::sample`] is three-valued on purpose:
//!
//! | Where | [`WaterAt`] | [`WaterMask::is_water`] | [`WaterMask::depth_m`] |
//! |---|---|---|---|
//! | inside the extent, mask byte 0 | [`WaterAt::Dry`] | `false` | `Some(metres)` (0.0) |
//! | inside the extent, mask byte non-zero | [`WaterAt::Water`] | `true` | `Some(metres)` |
//! | **outside the world extent** | [`WaterAt::Unknown`] | **`false`** | **`None`** |
//! | a mip level this container does not hold | [`WaterAt::Unknown`] | `false` | `None` |
//! | **no file at all** | — the host holds `Option<WaterMask>` and it is `None` | — | — |
//!
//! `is_water` answering `false` off the map is not a claim that the ground there is dry: it is the
//! absence of a reading, which is why [`WaterAt::Unknown`] exists and why the guard-facing
//! predicate is [`WaterMask::is_known_dry_land`] — `false` for water, `false` off the map, and
//! (because a caller holding no mask holds no reading either) `false` by construction when the
//! container was never loaded.
//!
//! # The grid convention
//!
//! `TBD_MapExportWater.c` walks `wx = px * worldSize / (w - 1)`, so a raster texel is a **point
//! sample on a vertex grid**: texel 0 sits exactly on the world minimum and texel `w - 1` exactly
//! on the maximum, and row 0 is `z = z_min` (no flip — unlike `TBDE`, whose row 0 is the north
//! edge). The query is therefore *nearest sample*, not a cell lookup, which is what
//! [`WaterMask::texel_at_level`] implements.
//!
//! # The mip fold agrees with the emitter by construction
//!
//! [`downsample_index`] is the *only* definition of "which coarse texel does this fine texel fold
//! into", and both sides call it: `tbd-tools`' `map water` builds level `L + 1` by folding level
//! `L` through it, and [`WaterMask::texel_at_level`] walks a level-0 texel down the same ladder one
//! step at a time. Two independent formulas would drift the moment a dimension stopped being a
//! power of two (everon's 12800 stops at level 9: `25 >> 1 == 12`, not 12.5), and the drift would
//! be a coarse texel that reads the wrong lake.

use bytemuck::cast_slice;
use rkyv::Archived;

use super::binary::archives::{ARCHIVE_SCHEMA_VERSION, WaterVectorsArchive};
use super::binary::chunk_container::{ContainerHeader, TbdbHeader};
use super::binary::{BinaryError, access_checked};

/// `manifest.water.encoding` this build reads (spec §5). A block naming anything else describes a
/// container this code does not implement, and reading it anyway is how a mask answers confidently
/// about the wrong terrain.
pub const TBDB_ENCODING_V1: &str = "tbdb-v1";

/// The alignment [`access_checked`] needs for a [`WaterVectorsArchive`] buffer.
pub const WATER_VECTORS_ALIGN: usize = 16;

const _: () = assert!(
    WATER_VECTORS_ALIGN.is_multiple_of(align_of::<Archived<WaterVectorsArchive>>()),
    "WATER_VECTORS_ALIGN must be a multiple of the archived type's own alignment"
);

/// Which texel of a level does a texel of the level above it fold into?
///
/// The 2×2 halving, plus the clamp that makes an **odd** dimension safe: level `L + 1` is
/// `max(1, dim >> 1)` wide, so when `dim` is odd the last source column has no partner and would
/// index one past the end. Clamping it into the final destination texel means that texel absorbs
/// three sources instead of two — which is the conservative direction for both reductions
/// (`depth = max`, `mask = any water`): a coarse texel never claims dry ground that is wet.
///
/// Called by the emitter to build a level and by [`WaterMask::texel_at_level`] to read one. That
/// is deliberate — see the module docs.
#[must_use]
pub fn downsample_index(src_index: u32, dst_dim: u32) -> u32 {
    (src_index >> 1).min(dst_dim.saturating_sub(1))
}

/// One decoded level of a [`Bathymetry`] pyramid: the `u16` depth grid and the `u8` water mask,
/// both `width * height` long and both borrowed from the container with no copy.
#[derive(Clone, Copy, Debug)]
pub struct BathymetryLevel<'a> {
    pub width: u32,
    pub height: u32,
    /// Quantised depth; metres are `v * depth_scale` ([`TbdbHeader::metres`]).
    pub depth: &'a [u16],
    /// `0` = dry, non-zero = water.
    pub mask: &'a [u8],
}

impl BathymetryLevel<'_> {
    /// Row-major index of `(tx, tz)`, or `None` when either is off this level.
    #[must_use]
    pub fn index(&self, tx: u32, tz: u32) -> Option<usize> {
        if tx >= self.width || tz >= self.height {
            return None;
        }
        (tz as usize)
            .checked_mul(self.width as usize)?
            .checked_add(tx as usize)
    }
}

/// Which byte range of a `.tbd-bath` a loader should actually fetch: everon's full pyramid is
/// 655,359,980 B, and the levels a placement guard needs are the small ones at the end.
///
/// Produced by [`suffix_plan`]. `file_offset` already includes the 32-byte header, so it is an
/// HTTP `Range` start as-is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SuffixPlan {
    /// The finest level the plan carries; levels below it are not fetched and read as
    /// [`WaterAt::Unknown`].
    pub first_level: u16,
    pub file_offset: u64,
    pub bytes: u64,
}

/// Offsets of a level suffix **within the payload**: `(bytes before `first_level`, bytes from
/// `first_level` to the end)`. All arithmetic checked — a hostile `width * height * 2` wraps a
/// 32-bit `usize` on wasm, and a wrapped length sizes an allocation for a grid the file has not
/// got.
fn payload_span(header: &TbdbHeader, first_level: u16) -> Result<(usize, usize), BinaryError> {
    let bad = BinaryError::LengthMismatch {
        what: "TBDB",
        expected: 0,
        actual: usize::from(first_level),
    };
    if header.width == 0
        || header.height == 0
        || header.mip_count == 0
        || first_level >= header.mip_count
    {
        return Err(bad);
    }
    let (mut before, mut after) = (0_usize, 0_usize);
    for level in 0..header.mip_count {
        let stride = header.level_span(level).ok_or_else(|| bad.clone())?.stride;
        let slot = if level < first_level {
            &mut before
        } else {
            &mut after
        };
        *slot = slot.checked_add(stride).ok_or_else(|| bad.clone())?;
    }
    Ok((before, after))
}

/// The **finest** level whose suffix — that level and every coarser one — fits `budget_bytes`.
///
/// `None` when even the coarsest level does not fit, or the header is degenerate. Coarser is
/// always safe to fall back to: the fold is `depth = max` / `mask = any water`, so a coarse texel
/// over-reports water and never under-reports it, and a placement guard that reads a coarse level
/// refuses more ground than it has to rather than less.
#[must_use]
pub fn suffix_plan(header: &TbdbHeader, budget_bytes: u64) -> Option<SuffixPlan> {
    let mut best: Option<SuffixPlan> = None;
    for first_level in (0..header.mip_count).rev() {
        let (before, after) = payload_span(header, first_level).ok()?;
        let bytes = u64::try_from(after).ok()?;
        if bytes > budget_bytes {
            break;
        }
        best = Some(SuffixPlan {
            first_level,
            file_offset: u64::try_from(size_of::<TbdbHeader>().checked_add(before)?).ok()?,
            bytes,
        });
    }
    best
}

/// A `water/bathymetry.tbd-bath`, or the **coarse tail of one**, validated once and read in place.
///
/// The payload is kept as `Vec<u32>` rather than `Vec<u8>` purely as an **alignment vehicle**: a
/// `Vec<u8>` is 1-aligned, the format pads every level to a multiple of 4 bytes so every level's
/// depth block starts 4-aligned, and a `Vec<u32>` is the only allocation that lets
/// `bytemuck::try_cast_slice::<u8, u16>` succeed on those blocks without a second copy per query.
/// The words are never *interpreted* as numbers — [`u32::from_ne_bytes`] in, `cast_slice` out, so
/// the bytes round-trip exactly (`binary::pod` asserts the host is little-endian at compile time).
///
/// # Why a partial pyramid is a first-class shape
///
/// A browser cannot hold everon's 655 MB level 0, and it does not need to: the header alone says
/// where every level lives, so a loader Range-fetches a *suffix* ([`suffix_plan`]) and gets a
/// complete, self-consistent mask at a coarser texel size. The **header is kept whole** either way
/// — [`WaterMask::texel_at_level`] still folds from a level-0 texel down the same ladder — so the
/// answers a partial container gives are exactly the answers the full one gives at those levels,
/// not an approximation of them.
#[derive(Clone, Debug)]
pub struct Bathymetry {
    header: TbdbHeader,
    first_level: u16,
    payload: Vec<u32>,
}

impl Bathymetry {
    /// Parse a whole `.tbd-bath` file.
    ///
    /// # Errors
    /// * [`BinaryError::Truncated`] — under 32 bytes, or a payload shorter than the header's
    ///   pyramid.
    /// * [`BinaryError::BadMagic`] / [`BinaryError::UnsupportedVersion`] — not a `TBDB` v1 file.
    /// * [`BinaryError::LengthMismatch`] — a header describing a zero-sized pyramid (`width`,
    ///   `height` or `mip_count` of 0), a level whose size overflows `usize`, or a pyramid whose
    ///   total is not the multiple of 4 the padding rule guarantees.
    pub fn from_bytes(raw: &[u8]) -> Result<Self, BinaryError> {
        let (header, payload) = TbdbHeader::read(raw)?;
        Self::assemble(header, 0, payload, raw.len())
    }

    /// Parse the level suffix a [`SuffixPlan`] fetched: `header` is the whole file's 32-byte
    /// header (Range-read separately), `payload` the bytes from `first_level`'s depth block to the
    /// end of the file.
    ///
    /// # Errors
    /// As [`Bathymetry::from_bytes`], plus [`BinaryError::LengthMismatch`] when `first_level` is
    /// not a level this header has.
    pub fn from_level_suffix(
        header: TbdbHeader,
        first_level: u16,
        payload: &[u8],
    ) -> Result<Self, BinaryError> {
        // The header arrived over the wire too, on its own request — validate it here rather than
        // trusting that whoever produced it did.
        header.validate()?;
        let len = payload.len().saturating_add(size_of::<TbdbHeader>());
        Self::assemble(header, first_level, payload, len)
    }

    fn assemble(
        header: TbdbHeader,
        first_level: u16,
        payload: &[u8],
        raw_len: usize,
    ) -> Result<Self, BinaryError> {
        let (_, bytes) = payload_span(&header, first_level)?;
        if payload.len() < bytes {
            return Err(BinaryError::Truncated {
                what: "TBDB",
                expected: bytes.saturating_add(size_of::<TbdbHeader>()),
                actual: raw_len,
            });
        }
        if !bytes.is_multiple_of(4) {
            return Err(BinaryError::LengthMismatch {
                what: "TBDB",
                expected: bytes.next_multiple_of(4),
                actual: bytes,
            });
        }
        Ok(Self {
            header,
            first_level,
            payload: payload[..bytes]
                .chunks_exact(4)
                .map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
                .collect(),
        })
    }

    /// The 32-byte header as it was written — **the whole file's**, even when only a suffix is
    /// held, so the world→texel ladder is unchanged.
    #[must_use]
    pub fn header(&self) -> &TbdbHeader {
        &self.header
    }

    /// The finest level actually present (`0` for a whole file).
    #[must_use]
    pub fn finest_level(&self) -> u16 {
        self.first_level
    }

    /// Level-0 grid width in texels.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.header.width
    }

    /// Level-0 grid height in texels.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.header.height
    }

    /// Levels in the pyramid; the coarsest is `level_count() - 1`.
    #[must_use]
    pub fn level_count(&self) -> u16 {
        self.header.mip_count
    }

    /// One level's two grids, or `None` past [`Bathymetry::level_count`] — or *below*
    /// [`Bathymetry::finest_level`], when only a suffix was fetched.
    ///
    /// Zero-copy: both slices point into the buffer the parse allocated.
    #[must_use]
    pub fn level(&self, level: u16) -> Option<BathymetryLevel<'_>> {
        if level < self.first_level {
            return None;
        }
        let span = self.header.level_span(level)?;
        // Offsets in `LevelSpan` are relative to the start of the WHOLE payload; this buffer starts
        // at `first_level`'s depth block. Both are sums of 4-multiple strides, so the difference is
        // still 4-aligned and the depth cast still succeeds without a copy.
        let base = self.header.level_span(self.first_level)?.depth_offset;
        let depth_off = span.depth_offset.checked_sub(base)?;
        let mask_off = span.mask_offset.checked_sub(base)?;
        let bytes: &[u8] = cast_slice(&self.payload);
        let depth_end = depth_off.checked_add(span.depth_bytes)?;
        let mask_end = mask_off.checked_add(span.mask_bytes)?;
        let depth_bytes = bytes.get(depth_off..depth_end)?;
        let mask = bytes.get(mask_off..mask_end)?;
        // Every level starts on a stride that is a multiple of 4, so this cast never copies and
        // never fails; `try_` rather than `cast_slice` so a future layout change is an error
        // instead of a panic in a placement guard.
        let depth = bytemuck::try_cast_slice::<u8, u16>(depth_bytes).ok()?;
        Some(BathymetryLevel {
            width: span.width,
            height: span.height,
            depth,
            mask,
        })
    }
}

/// What the mask knows about one world point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WaterAt {
    /// **No reading here.** The point is outside the container's world extent, or the requested
    /// mip level is not in the file. This is *not* a claim that the ground is dry.
    Unknown,
    /// The container records dry ground.
    Dry,
    /// The container records water, `depth_m` metres deep.
    Water { depth_m: f32 },
}

/// The placement guard's view of a loaded `.tbd-bath`: a [`Bathymetry`] plus the world rectangle it
/// covers, which is the manifest's `worldBounds` (`[min_x, min_z, max_x, max_z]`).
#[derive(Clone, Debug)]
pub struct WaterMask {
    bathymetry: Bathymetry,
    bounds: [f64; 4],
}

impl WaterMask {
    /// `None` when `bounds` is not a finite, positive-area rectangle — a degenerate extent would
    /// divide by zero and fold the whole map onto one texel.
    #[must_use]
    pub fn new(bathymetry: Bathymetry, bounds: [f64; 4]) -> Option<Self> {
        let [min_x, min_z, max_x, max_z] = bounds;
        let ok = bounds.iter().all(|v| v.is_finite()) && max_x > min_x && max_z > min_z;
        ok.then_some(Self { bathymetry, bounds })
    }

    /// [`Bathymetry::from_bytes`] then [`WaterMask::new`].
    ///
    /// # Errors
    /// As [`Bathymetry::from_bytes`], plus [`BinaryError::LengthMismatch`] when `bounds` is
    /// degenerate (there is no container error for "the caller's extent is nonsense", and the two
    /// have the same consequence: no usable mask).
    pub fn from_bytes(raw: &[u8], bounds: [f64; 4]) -> Result<Self, BinaryError> {
        let bath = Bathymetry::from_bytes(raw)?;
        let texels = bath.width();
        Self::new(bath, bounds).ok_or(BinaryError::LengthMismatch {
            what: "TBDB world bounds",
            expected: 0,
            actual: texels as usize,
        })
    }

    /// The container behind the mask.
    #[must_use]
    pub fn bathymetry(&self) -> &Bathymetry {
        &self.bathymetry
    }

    /// `[min_x, min_z, max_x, max_z]` in world metres.
    #[must_use]
    pub fn bounds(&self) -> [f64; 4] {
        self.bounds
    }

    /// The level-0 texel nearest `(x, z)`, or `None` outside the extent.
    ///
    /// Nearest *sample*, not a cell lookup — see the module docs on the grid convention. A `NaN`
    /// coordinate fails the range test and is therefore outside, which is the answer that cannot
    /// place anything.
    #[must_use]
    pub fn texel(&self, x: f64, z: f64) -> Option<(u32, u32)> {
        let [min_x, min_z, max_x, max_z] = self.bounds;
        if !(x >= min_x && x <= max_x && z >= min_z && z <= max_z) {
            return None;
        }
        Some((
            nearest_sample((x - min_x) / (max_x - min_x), self.bathymetry.width()),
            nearest_sample((z - min_z) / (max_z - min_z), self.bathymetry.height()),
        ))
    }

    /// The texel of level `level` covering `(x, z)`, reached by folding the level-0 texel down the
    /// ladder one [`downsample_index`] step at a time — the emitter's own construction, run
    /// backwards. `None` outside the extent or past the last level.
    #[must_use]
    pub fn texel_at_level(&self, x: f64, z: f64, level: u16) -> Option<(u32, u32)> {
        let (mut tx, mut tz) = self.texel(x, z)?;
        for l in 1..=level {
            let (w, h) = self.bathymetry.header.level_dims(l)?;
            tx = downsample_index(tx, w);
            tz = downsample_index(tz, h);
        }
        Some((tx, tz))
    }

    /// What level `level` records at `(x, z)`.
    #[must_use]
    pub fn sample_at_level(&self, x: f64, z: f64, level: u16) -> WaterAt {
        let Some((tx, tz)) = self.texel_at_level(x, z, level) else {
            return WaterAt::Unknown;
        };
        let Some(grid) = self.bathymetry.level(level) else {
            return WaterAt::Unknown;
        };
        let Some(i) = grid.index(tx, tz) else {
            return WaterAt::Unknown;
        };
        let (Some(&mask), Some(&depth)) = (grid.mask.get(i), grid.depth.get(i)) else {
            return WaterAt::Unknown;
        };
        if mask == 0 {
            WaterAt::Dry
        } else {
            WaterAt::Water {
                depth_m: self.bathymetry.header.metres(depth),
            }
        }
    }

    /// What the **finest level this container actually holds** records at `(x, z)` — level 0 for a
    /// whole file, [`Bathymetry::finest_level`] for a Range-fetched suffix.
    #[must_use]
    pub fn sample(&self, x: f64, z: f64) -> WaterAt {
        self.sample_at_level(x, z, self.bathymetry.finest_level())
    }

    /// **`true` only where the container records water.**
    ///
    /// Off the map — and for a mip level the file does not carry — the answer is `false`, because
    /// there is no reading, not because the ground is dry (see [`WaterAt::Unknown`] and the module
    /// docs). A caller that must not drop a unit in a lake wants
    /// [`WaterMask::is_known_dry_land`], which refuses both.
    #[must_use]
    pub fn is_water(&self, x: f64, z: f64) -> bool {
        matches!(self.sample(x, z), WaterAt::Water { .. })
    }

    /// The predicate a placement guard actually wants: is `(x, z)` **known** dry ground?
    ///
    /// `false` in water, `false` off the map, `false` for a level the file does not carry — and,
    /// because a host that never loaded a container holds `None` rather than a `WaterMask`, the
    /// missing-file case is `false` too the moment the caller writes
    /// `mask.is_some_and(|m| m.is_known_dry_land(x, z))`. Every unknown answers "not here".
    #[must_use]
    pub fn is_known_dry_land(&self, x: f64, z: f64) -> bool {
        matches!(self.sample(x, z), WaterAt::Dry)
    }

    /// Metres of water at `(x, z)`: `Some(0.0)` on dry ground (the raster records a zero there),
    /// `None` where there is no reading at all.
    #[must_use]
    pub fn depth_m(&self, x: f64, z: f64) -> Option<f32> {
        match self.sample(x, z) {
            WaterAt::Unknown => None,
            WaterAt::Dry => Some(0.0),
            WaterAt::Water { depth_m } => Some(depth_m),
        }
    }

    /// Mip selection: the **coarsest** level whose texels are still no wider than `target_m`
    /// metres, or [`Bathymetry::finest_level`] when even that is coarser than the target (there is
    /// nothing finer in this container to pick).
    ///
    /// A caller rasterising a mask overlay at 32 m/px reads level 5 of everon instead of touching
    /// 163 M level-0 texels; a caller asking a single placement question uses the finest it has.
    #[must_use]
    pub fn level_for_texel_size_m(&self, target_m: f64) -> u16 {
        let [min_x, _, max_x, _] = self.bounds;
        let span = max_x - min_x;
        let finest = self.bathymetry.finest_level();
        let mut best = finest;
        for level in finest..self.bathymetry.level_count() {
            let Some((w, _)) = self.bathymetry.header.level_dims(level) else {
                break;
            };
            if span / f64::from(w) > target_m {
                break;
            }
            best = level;
        }
        best
    }
}

/// Nearest sample index on a vertex grid of `dim` samples spanning `t ∈ [0, 1]`.
fn nearest_sample(t: f64, dim: u32) -> u32 {
    if dim <= 1 {
        return 0;
    }
    let last = f64::from(dim - 1);
    let idx = (t * last).round().clamp(0.0, last);
    // Clamped to `[0, dim - 1]` on the line above, so the cast is exact and cannot saturate.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let out = idx as u32;
    out
}

/// A whole `water/water_vectors.rkyv` file, kept as the bytes it arrived as.
///
/// Zero-copy per spec §6: the file is fetched once, copied **once** onto an alignment
/// [`access_checked`] accepts, validated **once** here, and every later read is a borrow of that
/// same buffer. Nothing is deserialised into owned lakes and rivers.
#[derive(Clone, Debug)]
pub struct WaterVectors {
    buf: Vec<u8>,
    pad: usize,
}

impl WaterVectors {
    /// Copy onto the archive's alignment, **validate**, then check the schema version.
    ///
    /// The version check is not redundant with `access_checked`: rkyv proves the buffer is
    /// *readable*, never that this build and the writer agree on what the fields **mean**. An
    /// archive whose `surface_y` had become a metre offset rather than an absolute height would
    /// validate perfectly and drown a shoreline.
    ///
    /// # Errors
    /// * [`BinaryError::Misaligned`] — the copy could not be placed on the boundary.
    /// * [`BinaryError::Archive`] — rkyv validation rejected the buffer.
    /// * [`BinaryError::UnsupportedVersion`] — a well-formed archive from a different schema.
    pub fn from_bytes(raw: &[u8]) -> Result<Self, BinaryError> {
        let (buf, pad) = aligned_copy(raw).ok_or(BinaryError::Misaligned {
            what: "WaterVectorsArchive",
            align: WATER_VECTORS_ALIGN,
        })?;
        let archive = access_checked::<WaterVectorsArchive>(&buf[pad..])?;
        let version = archive.schema_version.to_native();
        if version != ARCHIVE_SCHEMA_VERSION {
            return Err(BinaryError::UnsupportedVersion {
                what: "WaterVectorsArchive",
                expected: ARCHIVE_SCHEMA_VERSION,
                actual: version,
            });
        }
        Ok(Self { buf, pad })
    }

    /// The archived view: lakes, rivers and ponds read in place.
    ///
    /// # Errors
    /// [`BinaryError::Archive`] — unreachable after a successful [`WaterVectors::from_bytes`], and
    /// returned rather than unwrapped because a placement guard is not a place to panic.
    pub fn archive(&self) -> Result<&Archived<WaterVectorsArchive>, BinaryError> {
        access_checked::<WaterVectorsArchive>(&self.buf[self.pad..])
    }

    /// `(lakes, rivers, ponds)` counts — the cheap summary a loader logs.
    ///
    /// # Errors
    /// As [`WaterVectors::archive`].
    pub fn counts(&self) -> Result<(usize, usize, usize), BinaryError> {
        let a = self.archive()?;
        Ok((a.lakes.len(), a.rivers.len(), a.ponds.len()))
    }
}

/// `raw` copied into a buffer whose byte at `pad` sits on [`WATER_VECTORS_ALIGN`].
///
/// Capacity is reserved up front so `extend_from_slice` cannot reallocate and move the alignment
/// out from under the offset just measured; the result is re-checked anyway. (The same six lines
/// exist privately in `world::locations` for `map_labels.rkyv`; both are private to their module
/// and neither file is the other's to edit.)
fn aligned_copy(raw: &[u8]) -> Option<(Vec<u8>, usize)> {
    let mut buf: Vec<u8> = Vec::with_capacity(raw.len() + WATER_VECTORS_ALIGN);
    let pad = buf.as_ptr().align_offset(WATER_VECTORS_ALIGN);
    if pad >= WATER_VECTORS_ALIGN {
        return None;
    }
    buf.resize(pad, 0);
    buf.extend_from_slice(raw);
    (buf[pad..].as_ptr().align_offset(WATER_VECTORS_ALIGN) == 0).then_some((buf, pad))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::binary::archives::{WaterBody, WaterLine};
    use crate::world::binary::chunk_container::HEADER_BYTES;
    use crate::world::binary::to_bytes;

    /// The 4×4 / 3-mip synthetic grid the slice is specified against: one wet texel at (1, 2),
    /// depth 30 (→ 3.0 m at a 0.1 scale), everything else dry.
    const W: u32 = 4;
    const H: u32 = 4;
    const MIPS: u16 = 3;
    const SCALE: f32 = 0.1;
    const WET: (u32, u32) = (1, 2);

    /// Build a `TBDB` the way the emitter does, from a level-0 depth grid, using the SHARED
    /// [`downsample_index`] fold. This is the reader's oracle for shape only — the emitter's own
    /// bytes are pinned in `tbd-tools`.
    fn synth(width: u32, height: u32, mips: u16, wet: &[(u32, u32, u16)]) -> Vec<u8> {
        let head = TbdbHeader::new(width, height, mips, SCALE);
        let mut out = Vec::new();
        out.extend_from_slice(bytemuck::bytes_of(&head));
        let (mut w, mut h) = (width as usize, height as usize);
        let mut depth = vec![0_u16; w * h];
        let mut mask = vec![0_u8; w * h];
        for &(x, z, d) in wet {
            depth[z as usize * w + x as usize] = d;
            mask[z as usize * w + x as usize] = 1;
        }
        for level in 0..mips {
            for d in &depth {
                out.extend_from_slice(&d.to_le_bytes());
            }
            out.extend_from_slice(&mask);
            while !out.len().is_multiple_of(4) {
                out.push(0);
            }
            if level + 1 == mips {
                break;
            }
            let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
            let mut nd = vec![0_u16; nw * nh];
            let mut nm = vec![0_u8; nw * nh];
            for z in 0..h {
                let dz = downsample_index(z as u32, nh as u32) as usize;
                for x in 0..w {
                    let dx = downsample_index(x as u32, nw as u32) as usize;
                    nd[dz * nw + dx] = nd[dz * nw + dx].max(depth[z * w + x]);
                    nm[dz * nw + dx] |= u8::from(mask[z * w + x] != 0);
                }
            }
            (w, h, depth, mask) = (nw, nh, nd, nm);
        }
        out
    }

    fn mask_4x4() -> WaterMask {
        let bytes = synth(W, H, MIPS, &[(WET.0, WET.1, 30)]);
        WaterMask::from_bytes(&bytes, [0.0, 0.0, 4.0, 4.0]).expect("4x4 mask")
    }

    /// World coordinate of the centre of level-0 texel `t` on a `dim`-sample vertex grid over
    /// `[0, span]`.
    fn world_of(t: u32, dim: u32, span: f64) -> f64 {
        f64::from(t) * span / f64::from(dim - 1)
    }

    #[test]
    fn a_synthetic_container_round_trips_its_header_and_levels() {
        let b = Bathymetry::from_bytes(&synth(W, H, MIPS, &[(1, 2, 30)])).expect("parse");
        assert_eq!((b.width(), b.height(), b.level_count()), (W, H, MIPS));
        assert_eq!(b.header().depth_scale, SCALE);
        for (level, dims) in [(0_u16, (4_u32, 4_u32)), (1, (2, 2)), (2, (1, 1))] {
            let g = b.level(level).expect("level present");
            assert_eq!((g.width, g.height), dims, "level {level}");
            assert_eq!(g.depth.len(), (dims.0 * dims.1) as usize);
            assert_eq!(g.mask.len(), (dims.0 * dims.1) as usize);
        }
        assert!(b.level(MIPS).is_none(), "past the last level");
    }

    /// THE SLICE'S PIN. Every level agrees with level 0: a coarse texel is water whenever ANY
    /// level-0 texel folding into it is water, and its depth is the MAX of that set. Checked by
    /// walking level 0 and folding, so the assertion cannot be satisfied by a mask that is uniform.
    #[test]
    fn every_mip_level_agrees_with_level_zero() {
        // Several wet texels at different depths, so "max" is distinguishable from "first",
        // "last" and "any".
        let wet = [(0_u32, 0_u32, 7_u16), (1, 2, 30), (3, 3, 12)];
        let b = Bathymetry::from_bytes(&synth(W, H, MIPS, &wet)).expect("parse");
        let base = b.level(0).expect("level 0");
        for level in 1..b.level_count() {
            let grid = b.level(level).expect("level present");
            let mut want_mask = vec![0_u8; (grid.width * grid.height) as usize];
            let mut want_depth = vec![0_u16; (grid.width * grid.height) as usize];
            for z in 0..base.height {
                for x in 0..base.width {
                    // Fold the level-0 texel down the ladder exactly as the reader does.
                    let (mut tx, mut tz) = (x, z);
                    for l in 1..=level {
                        let (w, h) = b.header().level_dims(l).expect("dims");
                        tx = downsample_index(tx, w);
                        tz = downsample_index(tz, h);
                    }
                    let src = base.index(x, z).expect("src index");
                    let dst = grid.index(tx, tz).expect("dst index");
                    want_mask[dst] |= u8::from(base.mask[src] != 0);
                    want_depth[dst] = want_depth[dst].max(base.depth[src]);
                }
            }
            let got_mask: Vec<u8> = grid.mask.iter().map(|&m| u8::from(m != 0)).collect();
            assert_eq!(got_mask, want_mask, "level {level} mask");
            assert_eq!(grid.depth, &want_depth[..], "level {level} depth");
            assert!(
                want_mask.contains(&1) && want_depth.iter().any(|&d| d > 0),
                "level {level} oracle is vacuous — nothing wet folded into it"
            );
        }
    }

    #[test]
    fn the_wet_texel_reads_as_water_at_every_level() {
        let m = mask_4x4();
        let (x, z) = (world_of(WET.0, W, 4.0), world_of(WET.1, H, 4.0));
        assert_eq!(m.sample(x, z), WaterAt::Water { depth_m: 3.0 });
        assert!(m.is_water(x, z));
        assert!(!m.is_known_dry_land(x, z));
        assert_eq!(m.depth_m(x, z), Some(3.0));
        for level in 0..MIPS {
            assert_eq!(
                m.sample_at_level(x, z, level),
                WaterAt::Water { depth_m: 3.0 },
                "level {level} lost the lake"
            );
        }
    }

    #[test]
    fn a_dry_texel_reads_as_dry_at_level_zero() {
        let m = mask_4x4();
        let (x, z) = (world_of(3, W, 4.0), world_of(0, H, 4.0));
        assert_eq!(m.sample(x, z), WaterAt::Dry);
        assert!(!m.is_water(x, z));
        assert!(m.is_known_dry_land(x, z));
        assert_eq!(m.depth_m(x, z), Some(0.0));
    }

    /// The answer OUTSIDE the map, pinned in every direction and for NaN. `Unknown`, never `Dry`:
    /// a guard that read "dry" off the map would place a unit on nothing at all.
    #[test]
    fn outside_the_map_is_unknown_not_dry() {
        let m = mask_4x4();
        for (x, z) in [
            (-0.001, 2.0),
            (4.001, 2.0),
            (2.0, -0.001),
            (2.0, 4.001),
            (-1e9, -1e9),
            (1e9, 1e9),
            (f64::NAN, 2.0),
            (2.0, f64::NAN),
            (f64::INFINITY, 2.0),
            (2.0, f64::NEG_INFINITY),
        ] {
            assert_eq!(m.sample(x, z), WaterAt::Unknown, "({x}, {z})");
            assert!(!m.is_water(x, z), "({x}, {z}) claimed water off the map");
            assert!(
                !m.is_known_dry_land(x, z),
                "({x}, {z}) claimed KNOWN DRY LAND off the map — this is the answer that puts a \
                 unit outside the world"
            );
            assert_eq!(m.depth_m(x, z), None, "({x}, {z})");
            assert!(m.texel(x, z).is_none(), "({x}, {z})");
        }
        // The extremes of the extent are INSIDE — the grid is a vertex grid, so the corners are
        // samples, not the far edges of a cell.
        for (x, z) in [(0.0, 0.0), (4.0, 4.0), (0.0, 4.0), (4.0, 0.0)] {
            assert_ne!(m.sample(x, z), WaterAt::Unknown, "corner ({x}, {z})");
        }
    }

    #[test]
    fn a_level_the_file_does_not_carry_is_unknown() {
        let m = mask_4x4();
        assert_eq!(m.sample_at_level(2.0, 2.0, MIPS), WaterAt::Unknown);
        assert_eq!(m.sample_at_level(2.0, 2.0, u16::MAX), WaterAt::Unknown);
        assert!(m.texel_at_level(2.0, 2.0, MIPS).is_none());
    }

    /// A Range-fetched suffix must answer **exactly** what the whole file answers at the levels it
    /// holds — the header is kept whole precisely so the fold does not shift — and `Unknown` for
    /// the fine levels it does not.
    #[test]
    fn a_level_suffix_answers_identically_to_the_whole_file() {
        let bytes = synth(W, H, MIPS, &[(WET.0, WET.1, 30)]);
        let full = Bathymetry::from_bytes(&bytes).expect("full");
        assert_eq!(full.finest_level(), 0);
        for first in 1..MIPS {
            let plan = suffix_plan(full.header(), u64::MAX).expect("plan");
            let (before, _) = payload_span(full.header(), first).expect("span");
            let start = size_of::<TbdbHeader>() + before;
            let part = Bathymetry::from_level_suffix(*full.header(), first, &bytes[start..])
                .expect("suffix parses");
            assert_eq!(part.finest_level(), first);
            assert_eq!((part.width(), part.height()), (W, H), "header stays whole");
            assert_eq!(plan.first_level, 0, "an unbounded budget takes everything");

            let a = WaterMask::new(full.clone(), [0.0, 0.0, 4.0, 4.0]).expect("full mask");
            let b = WaterMask::new(part, [0.0, 0.0, 4.0, 4.0]).expect("part mask");
            for tz in 0..H {
                for tx in 0..W {
                    let (x, z) = (world_of(tx, W, 4.0), world_of(tz, H, 4.0));
                    for level in 0..MIPS {
                        let want = if level < first {
                            WaterAt::Unknown
                        } else {
                            a.sample_at_level(x, z, level)
                        };
                        assert_eq!(
                            b.sample_at_level(x, z, level),
                            want,
                            "suffix from {first}, level {level} at ({x}, {z})"
                        );
                    }
                    // `sample` follows the finest level the container HOLDS, not level 0.
                    assert_eq!(b.sample(x, z), a.sample_at_level(x, z, first));
                }
            }
            assert_eq!(
                b.level_for_texel_size_m(0.0),
                first,
                "cannot go finer than it has"
            );
        }
    }

    /// The plan a browser loader runs on: pick the finest level whose tail fits the budget, and
    /// give a `Range` start that already includes the header.
    #[test]
    fn suffix_plan_picks_the_finest_level_inside_the_budget() {
        // everon: 12800² × 14 levels. Level 0 alone is 491,520,000 B.
        let everon = TbdbHeader::new(12_800, 12_800, 14, 0.1);
        let whole = suffix_plan(&everon, u64::MAX).expect("whole");
        assert_eq!(whole.first_level, 0);
        assert_eq!(whole.file_offset, 32);
        assert_eq!(whole.bytes, 655_359_948);
        for (budget, level, bytes) in [
            (16_u64 << 20, 3_u16, 10_239_948_u64),
            (4 << 20, 4, 2_559_948),
            (1 << 20, 5, 639_948),
            // Level 9's tail is 2448 B, so 1 KiB stops at level 10 (12×12, 432+108+28+4).
            (1024, 10, 572),
        ] {
            let p = suffix_plan(&everon, budget).expect("plan");
            assert_eq!((p.first_level, p.bytes), (level, bytes), "budget {budget}");
            assert!(p.bytes <= budget, "budget {budget} overrun");
            let (before, _) = payload_span(&everon, p.first_level).expect("span");
            assert_eq!(p.file_offset, 32 + before as u64);
        }
        // A budget under the coarsest level's 4 bytes has no plan at all.
        assert!(suffix_plan(&everon, 3).is_none());
        assert!(suffix_plan(&TbdbHeader::new(0, 0, 0, 0.1), u64::MAX).is_none());
    }

    #[test]
    fn a_truncated_suffix_is_refused() {
        let bytes = synth(W, H, MIPS, &[(1, 2, 30)]);
        let head = TbdbHeader::new(W, H, MIPS, SCALE);
        let (before, after) = payload_span(&head, 1).expect("span");
        let start = size_of::<TbdbHeader>() + before;
        assert!(Bathymetry::from_level_suffix(head, 1, &bytes[start..]).is_ok());
        assert!(matches!(
            Bathymetry::from_level_suffix(head, 1, &bytes[start..start + after - 4]),
            Err(BinaryError::Truncated { .. })
        ));
        assert!(matches!(
            Bathymetry::from_level_suffix(head, MIPS, &bytes[start..]),
            Err(BinaryError::LengthMismatch { .. })
        ));
        let mut bad = head;
        bad.magic = *b"vers";
        assert!(matches!(
            Bathymetry::from_level_suffix(bad, 0, &bytes[32..]),
            Err(BinaryError::BadMagic { .. })
        ));
    }

    /// Corners map to the first and last sample, and the mapping is monotone across the span —
    /// the whole of "a wrong answer puts a unit in a lake" is this function being right.
    #[test]
    fn the_world_to_texel_mapping_is_a_vertex_grid() {
        let m = mask_4x4();
        assert_eq!(m.texel(0.0, 0.0), Some((0, 0)));
        assert_eq!(m.texel(4.0, 4.0), Some((3, 3)));
        for t in 0..W {
            let x = world_of(t, W, 4.0);
            assert_eq!(m.texel(x, 0.0), Some((t, 0)), "sample {t} at x={x}");
        }
        // Halfway between two samples rounds to the nearer one, not to the lower.
        let mid = (world_of(1, W, 4.0) + world_of(2, W, 4.0)) / 2.0;
        assert_eq!(m.texel(mid + 0.01, 0.0), Some((2, 0)));
        assert_eq!(m.texel(mid - 0.01, 0.0), Some((1, 0)));
    }

    /// Odd dimensions are where a second, independently written fold would drift: 5 → 2 → 1, and
    /// the last destination texel has to absorb THREE sources, not two.
    #[test]
    fn odd_dimensions_fold_without_running_off_the_end() {
        assert_eq!(downsample_index(0, 2), 0);
        assert_eq!(downsample_index(1, 2), 0);
        assert_eq!(downsample_index(2, 2), 1);
        assert_eq!(downsample_index(3, 2), 1);
        assert_eq!(
            downsample_index(4, 2),
            1,
            "the odd tail must clamp, not overrun"
        );
        assert_eq!(downsample_index(9, 1), 0);
        let bytes = synth(5, 5, 3, &[(4, 4, 9)]);
        let b = Bathymetry::from_bytes(&bytes).expect("parse 5x5");
        for level in 0..3 {
            let g = b.level(level).expect("level");
            let want = (5_u32 >> level).max(1);
            assert_eq!((g.width, g.height), (want, want), "level {level}");
            assert!(
                g.mask.iter().any(|&m| m != 0),
                "level {level} lost the corner lake"
            );
        }
    }

    #[test]
    fn mip_selection_picks_the_coarsest_level_within_the_budget() {
        let m = mask_4x4();
        // 4 m over 4 samples: level 0 texels are 1 m, level 1 are 2 m, level 2 are 4 m.
        assert_eq!(
            m.level_for_texel_size_m(0.5),
            0,
            "nothing is finer than 1 m"
        );
        assert_eq!(m.level_for_texel_size_m(1.0), 0);
        assert_eq!(m.level_for_texel_size_m(2.0), 1);
        assert_eq!(m.level_for_texel_size_m(100.0), 2, "capped by the pyramid");
    }

    #[test]
    fn a_degenerate_extent_has_no_mask() {
        let b = || Bathymetry::from_bytes(&synth(W, H, MIPS, &[(1, 2, 30)])).expect("parse");
        for bounds in [
            [0.0, 0.0, 0.0, 4.0],
            [0.0, 0.0, 4.0, 0.0],
            [4.0, 0.0, 0.0, 4.0],
            [0.0, 0.0, f64::NAN, 4.0],
            [0.0, 0.0, f64::INFINITY, 4.0],
        ] {
            assert!(WaterMask::new(b(), bounds).is_none(), "{bounds:?}");
            assert!(WaterMask::from_bytes(&synth(W, H, MIPS, &[]), bounds).is_err());
        }
    }

    #[test]
    fn a_malformed_container_is_refused_not_guessed() {
        let good = synth(W, H, MIPS, &[(1, 2, 30)]);
        assert!(matches!(
            Bathymetry::from_bytes(&good[..8]),
            Err(BinaryError::Truncated { .. })
        ));
        let mut magic = good.clone();
        magic[..4].copy_from_slice(b"vers"); // an LFS pointer file
        assert!(matches!(
            Bathymetry::from_bytes(&magic),
            Err(BinaryError::BadMagic { .. })
        ));
        let mut version = good.clone();
        version[4..6].copy_from_slice(&9_u16.to_le_bytes());
        assert!(matches!(
            Bathymetry::from_bytes(&version),
            Err(BinaryError::UnsupportedVersion { .. })
        ));
        // A header claiming a pyramid larger than the payload must not allocate for it.
        let mut big = good.clone();
        big[8..12].copy_from_slice(&40_000_u32.to_le_bytes());
        big[12..16].copy_from_slice(&40_000_u32.to_le_bytes());
        assert!(matches!(
            Bathymetry::from_bytes(&big),
            Err(BinaryError::Truncated { .. })
        ));
        for zeroed in [8..12_usize, 12..16] {
            let mut z = good.clone();
            z[zeroed].copy_from_slice(&0_u32.to_le_bytes());
            assert!(
                matches!(
                    Bathymetry::from_bytes(&z),
                    Err(BinaryError::LengthMismatch { .. })
                ),
                "a zero-sized grid must be refused, not folded onto one texel"
            );
        }
        let mut no_levels = good.clone();
        no_levels[6..8].copy_from_slice(&0_u16.to_le_bytes());
        assert!(matches!(
            Bathymetry::from_bytes(&no_levels),
            Err(BinaryError::LengthMismatch { .. })
        ));
        // Truncated in the middle of the last level.
        assert!(matches!(
            Bathymetry::from_bytes(&good[..good.len() - 4]),
            Err(BinaryError::Truncated { .. })
        ));
        assert_eq!(HEADER_BYTES, 32);
    }

    /// A `Vec<u8>` off the network is 1-aligned. The parse must survive that, and read the same
    /// grid as an aligned buffer would.
    #[test]
    fn a_misaligned_file_buffer_parses_identically() {
        let good = synth(W, H, MIPS, &[(1, 2, 30)]);
        let mut shifted = vec![0_u8];
        shifted.extend_from_slice(&good);
        let a = Bathymetry::from_bytes(&good).expect("aligned");
        let b = Bathymetry::from_bytes(&shifted[1..]).expect("misaligned");
        for level in 0..MIPS {
            let (ga, gb) = (a.level(level).expect("a"), b.level(level).expect("b"));
            assert_eq!(ga.depth, gb.depth, "level {level}");
            assert_eq!(ga.mask, gb.mask, "level {level}");
        }
    }

    fn vectors_archive(schema_version: u16) -> rkyv::util::AlignedVec {
        to_bytes(&WaterVectorsArchive {
            schema_version,
            lakes: vec![WaterBody {
                id: "lake_1".into(),
                surface_y: 84.762,
                ring: vec![[0.0, 0.0], [4.0, 0.0], [4.0, 4.0]],
            }],
            rivers: vec![WaterLine {
                id: "river_1".into(),
                width_m: 12.5,
                centerline: vec![[0.0, 1.0], [4.0, 1.0]],
            }],
            ponds: Vec::new(),
        })
        .expect("serialise")
    }

    #[test]
    fn water_vectors_read_in_place_from_an_unaligned_buffer() {
        let bytes = vectors_archive(ARCHIVE_SCHEMA_VERSION);
        let mut shifted = vec![0_u8];
        shifted.extend_from_slice(&bytes);
        let v = WaterVectors::from_bytes(&shifted[1..]).expect("parse");
        assert_eq!(v.counts().expect("counts"), (1, 1, 0));
        let a = v.archive().expect("archive");
        assert_eq!(a.lakes[0].id.as_str(), "lake_1");
        assert_eq!(a.lakes[0].surface_y.to_native(), 84.762);
        assert_eq!(a.lakes[0].ring.len(), 3);
        assert_eq!(a.rivers[0].width_m.to_native(), 12.5);
        assert_eq!(a.rivers[0].centerline.len(), 2);
    }

    /// Rule: rkyv proves the buffer is READABLE, never that this build agrees with the writer
    /// about what the fields mean. A shifted schema must be refused even though it validates.
    #[test]
    fn a_foreign_schema_version_is_refused_even_though_it_validates() {
        let bytes = vectors_archive(ARCHIVE_SCHEMA_VERSION + 1);
        assert!(
            access_checked::<WaterVectorsArchive>(&bytes).is_ok(),
            "the buffer must be structurally valid, or this test proves nothing"
        );
        assert!(matches!(
            WaterVectors::from_bytes(&bytes),
            Err(BinaryError::UnsupportedVersion {
                expected: ARCHIVE_SCHEMA_VERSION,
                ..
            })
        ));
    }

    #[test]
    fn a_corrupt_vectors_file_is_refused() {
        let bytes = vectors_archive(ARCHIVE_SCHEMA_VERSION);
        assert!(matches!(
            WaterVectors::from_bytes(&bytes[..bytes.len() / 2]),
            Err(BinaryError::Archive { .. })
        ));
        assert!(matches!(
            WaterVectors::from_bytes(&[]),
            Err(BinaryError::Archive { .. })
        ));
        let mut flipped = bytes.to_vec();
        let n = flipped.len();
        flipped[n - 3] ^= 0xFF;
        assert!(WaterVectors::from_bytes(&flipped).is_err());
    }
}
