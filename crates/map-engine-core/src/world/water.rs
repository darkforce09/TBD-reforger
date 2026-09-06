//! T-935.9 — the runtime water reader: the `TBDB` bathymetry pyramid and the
//! `water/water_vectors.rkyv` archive (spec §3.3, §4).
//!
//! # Why this lives in the core and not in the SPA
//!
//! [`WaterMask::is_water`] is a **placement guard's** question, and a wrong answer is a squad
//! spawned in a lake. `world_assets` is `#![cfg(target_arch = "wasm32")]` and this repo has no
//! wasm-bindgen-test harness, so anything decided over there is guarded by compilation alone. Every
//! decision that can be wrong — the payload-length check, the world→texel mapping, the mip fold,
//! the schema-version gate — is therefore made here, where `cargo test` executes it.
//!
//! # What the query answers, precisely
//!
//! | Where | [`WaterAt`] | [`WaterMask::is_water`] | [`WaterMask::depth_m`] |
//! |---|---|---|---|
//! | inside the extent, mask byte 0 | [`WaterAt::Dry`] | `false` | `Some(metres)` (0.0) |
//! | inside the extent, mask byte non-zero | [`WaterAt::Water`] | `true` | `Some(metres)` |
//! | **outside the world extent** | [`WaterAt::Unknown`] | **`false`** | **`None`** |
//! | a mip level this container does not hold | [`WaterAt::Unknown`] | `false` | `None` |
//! | **no file at all** | the host holds `Option<WaterMask>` and it is `None` | — | — |
//!
//! `is_water` answering `false` off the map is not a claim that the ground there is dry: it is the
//! absence of a reading, which is why [`WaterAt::Unknown`] exists and why the guard-facing
//! predicate is [`WaterMask::is_known_dry_land`] — `false` for water, `false` off the map, and
//! (because a caller holding no mask holds no reading either) `false` by construction when the
//! container was never loaded.
//!
//! # The grid convention, and the fold that agrees with the emitter by construction
//!
//! `TBD_MapExportWater.c` walks `wx = px * worldSize / (w - 1)`, so a raster texel is a **point
//! sample on a vertex grid**: texel 0 sits exactly on the world minimum and texel `w - 1` exactly
//! on the maximum, and row 0 is `z = z_min` (no flip — unlike `TBDE`, whose row 0 is the north
//! edge). The query is therefore *nearest sample*, not a cell lookup.
//!
//! [`downsample_index`] is then the *only* definition of "which coarse texel does this fine texel
//! fold into", and both sides call it: `tbd-tools`' `map water` builds level `L + 1` by folding
//! level `L` through it, and [`WaterMask::texel_at_level`] walks a level-0 texel down the same
//! ladder one step at a time. Two independent formulas would drift the moment a dimension stopped
//! being a power of two (everon's 12800 stops at level 9: `25 >> 1 == 12`, not 12.5), and the drift
//! would be a coarse texel that reads the wrong lake.

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
/// Called by the emitter to build a level and by [`WaterMask::texel_at_level`] to read one — that
/// is deliberate, see the module docs.
#[must_use]
pub fn downsample_index(src_index: u32, dst_dim: u32) -> u32 {
    (src_index >> 1).min(dst_dim.saturating_sub(1))
}

/// One decoded level of a [`Bathymetry`] pyramid: the `u16` depth grid (metres are
/// `v * depth_scale`, [`TbdbHeader::metres`]) and the `u8` water mask (`0` dry, non-zero water),
/// both `width * height` long and both borrowed from the container with no copy.
#[derive(Clone, Copy, Debug)]
pub struct BathymetryLevel<'a> {
    pub width: u32,
    pub height: u32,
    pub depth: &'a [u16],
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

/// Which byte range of a `.tbd-bath` a loader should actually fetch ([`suffix_plan`]): everon's
/// full pyramid is 655,359,980 B and the levels a placement guard needs are the small ones at the
/// end. `file_offset` already includes the 32-byte header, so it is an HTTP `Range` start as-is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SuffixPlan {
    /// The finest level carried; levels below it read as [`WaterAt::Unknown`].
    pub first_level: u16,
    pub file_offset: u64,
    pub bytes: u64,
}

/// Offsets of a level suffix **within the payload**: `(bytes before `first_level`, bytes from
/// `first_level` to the end)`. All arithmetic checked — a hostile `width * height * 2` wraps a
/// 32-bit `usize` on wasm, and a wrapped length sizes an allocation for a grid the file has not got.
fn payload_span(header: &TbdbHeader, first_level: u16) -> Result<(usize, usize), BinaryError> {
    let h = *header;
    let bad = || BinaryError::LengthMismatch {
        what: "TBDB",
        expected: 0,
        actual: usize::from(first_level),
    };
    if h.width == 0 || h.height == 0 || h.mip_count == 0 || first_level >= h.mip_count {
        return Err(bad());
    }
    let (mut before, mut after) = (0_usize, 0_usize);
    for level in 0..h.mip_count {
        let n = h.level_span(level).ok_or_else(bad)?.stride;
        let slot = if level < first_level {
            &mut before
        } else {
            &mut after
        };
        *slot = slot.checked_add(n).ok_or_else(bad)?;
    }
    Ok((before, after))
}

/// The **finest** level whose suffix — that level and every coarser one — fits `budget_bytes`.
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
/// **A partial pyramid is a first-class shape.** A browser cannot hold everon's 655 MB level 0 and
/// does not need to: a loader Range-fetches a *suffix* ([`suffix_plan`]). The **header is kept
/// whole** either way — [`WaterMask::texel_at_level`] still folds from a level-0 texel down the
/// same ladder — so a partial container gives exactly the full one's answers at those levels, not
/// an approximation of them.
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
    /// * [`BinaryError::LengthMismatch`] — a zero-sized pyramid (`width`, `height` or `mip_count`
    ///   of 0), a level whose size overflows `usize`, or a total that is not the multiple of 4 the
    ///   padding rule guarantees.
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
        // T-946 — A SUFFIX MUST BE EXACTLY ITS SUFFIX, and this is a placement-guard input.
        //
        // `<` alone accepts anything at least long enough, and the levels are then indexed FROM
        // THE FRONT of `payload`. So a 206 that starts at the wrong offset — or a server that
        // ignores `Range` and sends a longer body from byte 0 — is read as if it began at
        // `first_level`, and every texel lands in the wrong level. The wave 240 verifier drove
        // exactly that: an all-water 16x16 terrain read through a mis-started 206 answered
        // `known_dry` for 8 of 16 probes. A mask that says DRY over open water is the one answer
        // a placement guard must never get, so the length is now an equality.
        if payload.len() != bytes {
            return Err(BinaryError::LengthMismatch {
                what: "TBDB",
                expected: bytes,
                actual: payload.len(),
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

    /// The 32-byte header as written — **the whole file's**, even when only a suffix is held, so
    /// the world→texel ladder is unchanged.
    #[must_use]
    pub fn header(&self) -> &TbdbHeader {
        &self.header
    }

    /// The finest level present (`0` for a whole file).
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
        // Every level starts on a 4-multiple stride, so this cast never copies and never fails;
        // `try_` rather than `cast_slice` so a future layout change is an error rather than a
        // panic in a placement guard.
        let depth = bytemuck::try_cast_slice::<u8, u16>(depth_bytes).ok()?;
        let (width, height) = (span.width, span.height);
        Some(BathymetryLevel {
            width,
            height,
            depth,
            mask,
        })
    }
}

/// What the mask knows about one world point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WaterAt {
    /// **No reading here** — outside the container's world extent, or a mip level it does not
    /// hold. *Not* a claim that the ground is dry.
    Unknown,
    /// Dry ground.
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

    /// What level `level` records at `(x, z)`. Every step that cannot answer says
    /// [`WaterAt::Unknown`] rather than guessing.
    #[must_use]
    pub fn sample_at_level(&self, x: f64, z: f64, level: u16) -> WaterAt {
        let read = || {
            let (tx, tz) = self.texel_at_level(x, z, level)?;
            let grid = self.bathymetry.level(level)?;
            let i = grid.index(tx, tz)?;
            Some((*grid.mask.get(i)?, *grid.depth.get(i)?))
        };
        match read() {
            None => WaterAt::Unknown,
            Some((0, _)) => WaterAt::Dry,
            Some((_, depth)) => WaterAt::Water {
                depth_m: self.bathymetry.header.metres(depth),
            },
        }
    }

    /// What the **finest level this container actually holds** records at `(x, z)` — level 0 for a
    /// whole file, [`Bathymetry::finest_level`] for a Range-fetched suffix.
    #[must_use]
    pub fn sample(&self, x: f64, z: f64) -> WaterAt {
        self.sample_at_level(x, z, self.bathymetry.finest_level())
    }

    /// **`true` only where the container records water.** Off the map — and for a level the file
    /// does not carry — the answer is `false` because there is no reading, *not* because the ground
    /// is dry. A caller that must not drop a unit in a lake wants
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
            match self.bathymetry.header.level_dims(level) {
                Some((w, _)) if span / f64::from(w) <= target_m => best = level,
                _ => break,
            }
        }
        best
    }
}

/// Nearest sample index on a vertex grid of `dim` samples spanning `t ∈ [0, 1]`.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn nearest_sample(t: f64, dim: u32) -> u32 {
    if dim <= 1 {
        return 0;
    }
    let last = f64::from(dim - 1);
    // Clamped to `[0, dim - 1]`, so the cast is exact and cannot saturate.
    (t * last).round().clamp(0.0, last) as u32
}

/// A whole `water/water_vectors.rkyv` file, kept as the bytes it arrived as.
///
/// Zero-copy per spec §6: the file is fetched once, copied **once** onto an alignment
/// [`access_checked`] accepts, validated **once** here, and every later read borrows that same
/// buffer. Nothing is deserialised into owned lakes and rivers.
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

/// `raw` copied into a buffer whose byte at `pad` sits on [`WATER_VECTORS_ALIGN`]. Capacity is
/// reserved up front so `extend_from_slice` cannot reallocate and move the alignment out from
/// under the offset just measured; the result is re-checked anyway. (`world::locations` has the
/// same six lines privately for `map_labels.rkyv`; neither file is the other's to edit.)
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
        let mut out = bytemuck::bytes_of(&head).to_vec();
        let (mut w, mut h) = (width as usize, height as usize);
        let mut depth = vec![0_u16; w * h];
        let mut mask = vec![0_u8; w * h];
        for &(x, z, d) in wet {
            depth[z as usize * w + x as usize] = d;
            mask[z as usize * w + x as usize] = 1;
        }
        for level in 0..mips {
            out.extend(depth.iter().flat_map(|d| d.to_le_bytes()));
            out.extend_from_slice(&mask);
            out.resize(out.len().next_multiple_of(4), 0);
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

    /// World coordinate of level-0 texel `t` on a `dim`-sample vertex grid over `[0, span]`.
    fn world_of(t: u32, dim: u32, span: f64) -> f64 {
        f64::from(t) * span / f64::from(dim - 1)
    }

    /// Which refusal a parse gave, as a short name — lets the refusal tables below read one line
    /// per case instead of a four-line `assert!(matches!(…))` each.
    fn refusal<T>(r: Result<T, BinaryError>) -> &'static str {
        match r {
            Ok(_) => "ok",
            Err(BinaryError::Truncated { .. }) => "truncated",
            Err(BinaryError::BadMagic { .. }) => "magic",
            Err(BinaryError::UnsupportedVersion { .. }) => "version",
            Err(BinaryError::Misaligned { .. }) => "align",
            Err(BinaryError::LengthMismatch { .. }) => "length",
            Err(BinaryError::Archive { .. }) => "archive",
        }
    }

    /// THE SLICE'S PIN. Every level agrees with level 0: a coarse texel is water whenever ANY
    /// level-0 texel folding into it is water, and its depth is the MAX of that set. Checked by
    /// walking level 0 and folding, so the assertion cannot be satisfied by a mask that is uniform.
    /// The header and the level geometry are checked here too — they are what the fold indexes.
    #[test]
    fn every_mip_level_agrees_with_level_zero() {
        // Several wet texels at different depths, so "max" is distinguishable from "first",
        // "last" and "any".
        let wet = [(0_u32, 0_u32, 7_u16), (1, 2, 30), (3, 3, 12)];
        let b = Bathymetry::from_bytes(&synth(W, H, MIPS, &wet)).expect("parse");
        assert_eq!((b.width(), b.height(), b.level_count()), (W, H, MIPS));
        assert_eq!((b.header().depth_scale, b.finest_level()), (SCALE, 0));
        assert!(b.level(MIPS).is_none(), "past the last level");
        for (level, dims) in [(0_u16, (4_u32, 4_u32)), (1, (2, 2)), (2, (1, 1))] {
            let g = b.level(level).expect("level present");
            let n = (dims.0 * dims.1) as usize;
            assert_eq!(
                ((g.width, g.height), g.depth.len(), g.mask.len()),
                (dims, n, n)
            );
        }
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

    /// The three predicates agree with [`WaterMask::sample`] on both answers it can give inside
    /// the extent, and the lake survives every level of the pyramid.
    #[test]
    fn wet_and_dry_texels_read_consistently_across_the_predicates() {
        let m = mask_4x4();
        let (x, z) = (world_of(WET.0, W, 4.0), world_of(WET.1, H, 4.0));
        assert_eq!(m.sample(x, z), WaterAt::Water { depth_m: 3.0 });
        assert!(m.is_water(x, z) && !m.is_known_dry_land(x, z));
        assert_eq!(m.depth_m(x, z), Some(3.0));
        for level in 0..MIPS {
            let want = WaterAt::Water { depth_m: 3.0 };
            assert_eq!(m.sample_at_level(x, z, level), want, "level {level}");
        }
        let (dx, dz) = (world_of(3, W, 4.0), world_of(0, H, 4.0));
        assert_eq!(m.sample(dx, dz), WaterAt::Dry);
        assert!(!m.is_water(dx, dz) && m.is_known_dry_land(dx, dz));
        assert_eq!(m.depth_m(dx, dz), Some(0.0));
    }

    /// The answer OUTSIDE the map, pinned in every direction and for NaN. `Unknown`, never `Dry`:
    /// a guard that read "dry" off the map would place a unit on nothing at all.
    #[test]
    fn outside_the_map_is_unknown_not_dry() {
        let m = mask_4x4();
        #[rustfmt::skip]
        let off = [
            (-0.001, 2.0), (4.001, 2.0), (2.0, -0.001), (2.0, 4.001),
            (-1e9, -1e9), (1e9, 1e9), (f64::NAN, 2.0), (2.0, f64::NAN),
            (f64::INFINITY, 2.0), (2.0, f64::NEG_INFINITY),
        ];
        for (x, z) in off {
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
        // The other unknown: a level this container does not hold.
        for level in [MIPS, u16::MAX] {
            assert_eq!(m.sample_at_level(2.0, 2.0, level), WaterAt::Unknown);
            assert!(m.texel_at_level(2.0, 2.0, level).is_none());
        }
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
            let (before, _) = payload_span(full.header(), first).expect("span");
            let start = size_of::<TbdbHeader>() + before;
            let part = Bathymetry::from_level_suffix(*full.header(), first, &bytes[start..])
                .expect("suffix parses");
            assert_eq!(part.finest_level(), first);
            assert_eq!((part.width(), part.height()), (W, H), "header stays whole");
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
        // The in-container twin: 4 m over 4 samples means 1 m texels at level 0, 2 m at 1, 4 m
        // at 2. 0.5 m is finer than anything the pyramid has, and 100 m is capped by it.
        let m = mask_4x4();
        for (target, want) in [(0.5, 0_u16), (1.0, 0), (2.0, 1), (100.0, 2)] {
            assert_eq!(m.level_for_texel_size_m(target), want, "{target} m");
        }
    }

    /// T-946 — an OVER-LONG suffix is refused too, because it means the bytes do not start where
    /// the reader thinks they do.
    ///
    /// The levels are indexed from the front of the payload, so a 206 that begins at the wrong
    /// offset, or a server that ignores `Range` and returns the whole file, is read as though it
    /// began at `first_level`: every texel lands in the wrong level. The wave 240 verifier drove
    /// an all-water terrain through a mis-started 206 and got `known_dry` for half its probes.
    #[test]
    fn a_suffix_longer_than_its_level_is_refused() {
        let whole = synth(W, H, MIPS, &[(1, 2, 30)]);
        let header = TbdbHeader::new(W, H, MIPS, SCALE);
        let (skip, bytes) = payload_span(&header, 1).expect("span");
        let payload = &whole[size_of::<TbdbHeader>() + skip..];
        assert_eq!(payload.len(), bytes, "the fixture's own suffix is exact");
        assert!(Bathymetry::from_level_suffix(header, 1, payload).is_ok());

        // The whole payload handed over as if it were level 1's suffix — the mis-started 206.
        let from_zero = &whole[size_of::<TbdbHeader>()..];
        assert!(
            from_zero.len() > bytes,
            "the fixture must actually be longer: {} vs {bytes}",
            from_zero.len()
        );
        let err = Bathymetry::from_level_suffix(header, 1, from_zero)
            .expect_err("a body that does not start at first_level must not be read");
        println!("── over-long suffix ── {err}");
        assert!(matches!(err, BinaryError::LengthMismatch { .. }));
    }

    #[test]
    fn a_truncated_suffix_is_refused() {
        let bytes = synth(W, H, MIPS, &[(1, 2, 30)]);
        let head = TbdbHeader::new(W, H, MIPS, SCALE);
        let (before, after) = payload_span(&head, 1).expect("span");
        let start = size_of::<TbdbHeader>() + before;
        let mut bad_magic = head;
        bad_magic.magic = *b"vers";
        let short = &bytes[start..start + after - 4];
        #[rustfmt::skip]
        let cases = [
            ("whole suffix", head, 1_u16, &bytes[start..], "ok"),
            ("4 B short", head, 1, short, "truncated"),
            ("no such level", head, MIPS, &bytes[start..], "length"),
            ("LFS header", bad_magic, 0, &bytes[32..], "magic"),
        ];
        for (what, h, first, tail, want) in cases {
            let got = refusal(Bathymetry::from_level_suffix(h, first, tail));
            assert_eq!(got, want, "{what}");
        }
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
        // src 4 is the odd tail: it must clamp into the last dst texel, not index past it.
        for (src, dst_dim, want) in [
            (0, 2, 0),
            (1, 2, 0),
            (2, 2, 1),
            (3, 2, 1),
            (4, 2, 1),
            (9, 1, 0),
        ] {
            assert_eq!(downsample_index(src, dst_dim), want, "{src} into {dst_dim}");
        }
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

    /// Every way a `.tbd-bath` — or the extent it is read against — can be wrong, and the refusal
    /// each one must get. A header that claims 40000² must not allocate for it; a zero-sized grid
    /// must be refused rather than folded onto one texel that then answers for the whole map; a
    /// degenerate extent would divide by zero and do the same.
    #[test]
    fn a_malformed_container_or_extent_is_refused_not_guessed() {
        assert_eq!(HEADER_BYTES, 32);
        let good = synth(W, H, MIPS, &[(1, 2, 30)]);
        #[rustfmt::skip]
        let extents = [
            [0.0, 0.0, 0.0, 4.0], [0.0, 0.0, 4.0, 0.0], [4.0, 0.0, 0.0, 4.0],
            [0.0, 0.0, f64::NAN, 4.0], [0.0, 0.0, f64::INFINITY, 4.0],
        ];
        for bounds in extents {
            let b = Bathymetry::from_bytes(&good).expect("parse");
            assert!(WaterMask::new(b, bounds).is_none(), "{bounds:?}");
            assert!(WaterMask::from_bytes(&good, bounds).is_err(), "{bounds:?}");
        }
        let patch = |at: std::ops::Range<usize>, v: &[u8]| {
            let mut b = good.clone();
            b[at].copy_from_slice(v);
            b
        };
        let cut = good[..good.len() - 4].to_vec();
        #[rustfmt::skip]
        let cases: [(&str, Vec<u8>, &str); 7] = [
            ("head cut off", good[..8].to_vec(), "truncated"),
            ("last level cut", cut, "truncated"),
            ("an LFS pointer", patch(0..4, b"vers"), "magic"),
            ("a future version", patch(4..6, &9_u16.to_le_bytes()), "version"),
            ("mip_count 0", patch(6..8, &0_u16.to_le_bytes()), "length"),
            ("width 0", patch(8..12, &0_u32.to_le_bytes()), "length"),
            ("height 0", patch(12..16, &0_u32.to_le_bytes()), "length"),
        ];
        for (what, bytes, want) in cases {
            assert_eq!(refusal(Bathymetry::from_bytes(&bytes)), want, "{what}");
        }
        let mut big = patch(8..12, &40_000_u32.to_le_bytes());
        big[12..16].copy_from_slice(&40_000_u32.to_le_bytes());
        assert_eq!(refusal(Bathymetry::from_bytes(&big)), "truncated", "40000²");
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

    #[rustfmt::skip]
    fn vectors_archive(schema_version: u16) -> rkyv::util::AlignedVec {
        let lake = WaterBody { id: "lake_1".into(), surface_y: 84.762,
                               ring: vec![[0.0, 0.0], [4.0, 0.0], [4.0, 4.0]] };
        let river = WaterLine { id: "river_1".into(), width_m: 12.5,
                                centerline: vec![[0.0, 1.0], [4.0, 1.0]] };
        to_bytes(&WaterVectorsArchive { schema_version, lakes: vec![lake],
                                        rivers: vec![river], ponds: Vec::new() })
            .expect("serialise")
    }

    /// The archive is read in place off a 1-aligned network buffer — and refuses what it must.
    /// Rule 16: rkyv proves the buffer is READABLE, never that this build agrees with the writer
    /// about what the fields mean, so a shifted schema is refused even though it validates.
    #[test]
    fn water_vectors_read_in_place_and_refuse_a_foreign_or_corrupt_file() {
        let good = vectors_archive(ARCHIVE_SCHEMA_VERSION);
        let mut shifted = vec![0_u8];
        shifted.extend_from_slice(&good);
        let v = WaterVectors::from_bytes(&shifted[1..]).expect("parse");
        assert_eq!(v.counts().expect("counts"), (1, 1, 0));
        let a = v.archive().expect("archive");
        assert_eq!(a.lakes[0].id.as_str(), "lake_1");
        assert_eq!(a.lakes[0].surface_y.to_native(), 84.762);
        assert_eq!(a.lakes[0].ring.len(), 3);
        assert_eq!(a.rivers[0].width_m.to_native(), 12.5);
        assert_eq!(a.rivers[0].centerline.len(), 2);

        let future = vectors_archive(ARCHIVE_SCHEMA_VERSION + 1);
        assert!(
            access_checked::<WaterVectorsArchive>(&future).is_ok(),
            "the future-schema buffer must be structurally valid, or this proves nothing"
        );
        let mut flipped = good.to_vec();
        let n = flipped.len();
        flipped[n - 3] ^= 0xFF;
        #[rustfmt::skip]
        let cases = [
            ("a future schema", future.to_vec(), "version"),
            ("half a file", good[..good.len() / 2].to_vec(), "archive"),
            ("no file", Vec::new(), "archive"),
        ];
        for (what, bytes, want) in cases {
            assert_eq!(refusal(WaterVectors::from_bytes(&bytes)), want, "{what}");
        }
        assert_ne!(
            refusal(WaterVectors::from_bytes(&flipped)),
            "ok",
            "bit flip"
        );
    }
}
