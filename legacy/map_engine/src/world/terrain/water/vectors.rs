//! Role: vectors.
//! Position: `world/terrain/water` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use bytemuck::cast_slice;
use rkyv::Archived;

use crate::io::archives::codec::BinaryError;
use crate::io::archives::codec::access_checked;
use crate::io::archives::version::ARCHIVE_SCHEMA_VERSION;
use crate::io::archives::water::WaterVectorsArchive;
use crate::io::containers::header::ContainerHeader;
use crate::io::containers::tbdb::TbdbHeader;

/// `manifest.water.encoding` this build reads (spec §5). A block naming anything else describes a container this code does not implement, and reading it anyway is how a mask answers confidently about the wrong terrain.
pub const TBDB_ENCODING_V1: &str = "tbdb-v1";

/// The alignment [`access_checked`] needs for a [`WaterVectorsArchive`] buffer.
pub const WATER_VECTORS_ALIGN: usize = 16;

const _: () = assert!(
    WATER_VECTORS_ALIGN.is_multiple_of(align_of::<Archived<WaterVectorsArchive>>()),
    "WATER_VECTORS_ALIGN must be a multiple of the archived type's own alignment"
);

/// Which texel of a level does a texel of the level above it fold into?.
#[must_use]
pub fn downsample_index(src_index: u32, dst_dim: u32) -> u32 {
    (src_index >> 1).min(dst_dim.saturating_sub(1))
}

/// One decoded level of a [`Bathymetry`] pyramid: the `u16` depth grid (metres are `v * depth_scale`, [`TbdbHeader::metres`]) and the `u8` water mask (`0` dry, non-zero water), both `width * height` long and both borrowed from the container with no copy.
#[derive(Clone, Copy, Debug)]
pub struct BathymetryLevel<'a> {
    /// Width.
    pub width: u32,

    /// Height.
    pub height: u32,

    /// Depth.
    pub depth: &'a [u16],

    /// Mask.
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

/// Which byte range of a `.tbd-bath` a loader should actually fetch ([`suffix_plan`]): everon's full pyramid is 655,359,980 B and the levels a placement guard needs are the small ones at the end. `file_offset` already includes the 32-byte header, so it is an HTTP `Range` start as-is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SuffixPlan {
    /// The finest level carried; levels below it read as [`WaterAt::Unknown`].
    pub first_level: u16,

    /// File offset.
    pub file_offset: u64,

    /// Bytes.
    pub bytes: u64,
}

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

/// Suffix plan.
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
#[derive(Clone, Debug)]
pub struct Bathymetry {
    header: TbdbHeader,
    first_level: u16,
    payload: Vec<u32>,
}

impl Bathymetry {
    /// Parse a whole `.tbd-bath` file.
    pub fn from_bytes(raw: &[u8]) -> Result<Self, BinaryError> {
        let (header, payload) = TbdbHeader::read(raw)?;
        Self::assemble(header, 0, payload, raw.len())
    }

    /// Parse the level suffix a [`SuffixPlan`] fetched: `header` is the whole file's 32-byte header (Range-read separately), `payload` the bytes from `first_level`'s depth block to the end of the file.
    pub fn from_level_suffix(
        header: TbdbHeader,
        first_level: u16,
        payload: &[u8],
    ) -> Result<Self, BinaryError> {
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

    /// The 32-byte header as written — **the whole file's**, even when only a suffix is held, so the world→texel ladder is unchanged.
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

    /// One level's two grids, or `None` past [`Bathymetry::level_count`] — or *below* [`Bathymetry::finest_level`], when only a suffix was fetched.
    #[must_use]
    pub fn level(&self, level: u16) -> Option<BathymetryLevel<'_>> {
        if level < self.first_level {
            return None;
        }
        let span = self.header.level_span(level)?;

        let base = self.header.level_span(self.first_level)?.depth_offset;
        let depth_off = span.depth_offset.checked_sub(base)?;
        let mask_off = span.mask_offset.checked_sub(base)?;
        let bytes: &[u8] = cast_slice(&self.payload);
        let depth_end = depth_off.checked_add(span.depth_bytes)?;
        let mask_end = mask_off.checked_add(span.mask_bytes)?;
        let depth_bytes = bytes.get(depth_off..depth_end)?;
        let mask = bytes.get(mask_off..mask_end)?;

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
    /// **No reading here** — outside the container's world extent, or a mip level it does not hold. *Not* a claim that the ground is dry.
    Unknown,

    /// Dry ground.
    Dry,

    /// The container records water, `depth_m` metres deep.
    Water { depth_m: f32 },
}

/// The placement guard's view of a loaded `.tbd-bath`: a [`Bathymetry`] plus the world rectangle it covers, which is the manifest's `worldBounds` (`[min_x, min_z, max_x, max_z]`).
#[derive(Clone, Debug)]
pub struct WaterMask {
    bathymetry: Bathymetry,
    bounds: [f64; 4],
}

impl WaterMask {
    /// `None` when `bounds` is not a finite, positive-area rectangle — a degenerate extent would divide by zero and fold the whole map onto one texel.
    #[must_use]
    pub fn new(bathymetry: Bathymetry, bounds: [f64; 4]) -> Option<Self> {
        let [min_x, min_z, max_x, max_z] = bounds;
        let ok = bounds.iter().all(|v| v.is_finite()) && max_x > min_x && max_z > min_z;
        ok.then_some(Self { bathymetry, bounds })
    }

    /// [`Bathymetry::from_bytes`] then [`WaterMask::new`].
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

    /// The texel of level `level` covering `(x, z)`, reached by folding the level-0 texel down the ladder one [`downsample_index`] step at a time — the emitter's own construction, run backwards. `None` outside the extent or past the last level.
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

    /// What level `level` records at `(x, z)`. Every step that cannot answer says [`WaterAt::Unknown`] rather than guessing.
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

    /// What the **finest level this container actually holds** records at `(x, z)` — level 0 for a whole file, [`Bathymetry::finest_level`] for a Range-fetched suffix.
    #[must_use]
    pub fn sample(&self, x: f64, z: f64) -> WaterAt {
        self.sample_at_level(x, z, self.bathymetry.finest_level())
    }

    /// **`true` only where the container records water.** Off the map — and for a level the file does not carry — the answer is `false` because there is no reading, *not* because the ground is dry. A caller that must not drop a unit in a lake wants [`WaterMask::is_known_dry_land`], which refuses both.
    #[must_use]
    pub fn is_water(&self, x: f64, z: f64) -> bool {
        matches!(self.sample(x, z), WaterAt::Water { .. })
    }

    /// The predicate a placement guard actually wants: is `(x, z)` **known** dry ground?.
    #[must_use]
    pub fn is_known_dry_land(&self, x: f64, z: f64) -> bool {
        matches!(self.sample(x, z), WaterAt::Dry)
    }

    /// Metres of water at `(x, z)`: `Some(0.0)` on dry ground (the raster records a zero there), `None` where there is no reading at all.
    #[must_use]
    pub fn depth_m(&self, x: f64, z: f64) -> Option<f32> {
        match self.sample(x, z) {
            WaterAt::Unknown => None,
            WaterAt::Dry => Some(0.0),
            WaterAt::Water { depth_m } => Some(depth_m),
        }
    }

    /// Mip selection: the **coarsest** level whose texels are still no wider than `target_m` metres, or [`Bathymetry::finest_level`] when even that is coarser than the target (there is nothing finer in this container to pick).
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

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn nearest_sample(t: f64, dim: u32) -> u32 {
    if dim <= 1 {
        return 0;
    }
    let last = f64::from(dim - 1);

    (t * last).round().clamp(0.0, last) as u32
}

/// A whole `water/water_vectors.rkyv` file, kept as the bytes it arrived as.
#[derive(Clone, Debug)]
pub struct WaterVectors {
    buf: Vec<u8>,
    pad: usize,
}

impl WaterVectors {
    /// Copy onto the archive's alignment, **validate**, then check the schema version.
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
    pub fn archive(&self) -> Result<&Archived<WaterVectorsArchive>, BinaryError> {
        access_checked::<WaterVectorsArchive>(&self.buf[self.pad..])
    }

    /// `(lakes, rivers, ponds)` counts — the cheap summary a loader logs.
    pub fn counts(&self) -> Result<(usize, usize, usize), BinaryError> {
        let a = self.archive()?;
        Ok((a.lakes.len(), a.rivers.len(), a.ponds.len()))
    }
}

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
#[path = "tests/vectors_tests.rs"]
mod tests;
