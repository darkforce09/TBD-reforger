//! T-935.9 — the SPA's water host: the `TBDB` bathymetry mask a placement guard queries, plus the
//! `water_vectors.rkyv` lake/river/pond archive.
//!
//! Dormant until a terrain manifest carries a `water` block (spec §5) — which is T-935.13's job, so
//! this loader is a no-op for every terrain shipping today and the editor is byte-identical without
//! it. That is deliberate: the whole binary migration is a manifest edit, not a code change.
//!
//! # Why the bathymetry is Range-fetched, not downloaded
//!
//! Everon's pyramid is **655,359,980 bytes**, and 491 MB of that is level 0 (12800² texels of
//! `u16` depth + `u8` mask). A browser cannot hold it and does not need to: a `TBDB` header says
//! where every level lives, so this loader reads the 32-byte header with one Range request, asks
//! [`suffix_plan`] for the finest level whose tail fits [`MAX_BATHYMETRY_BYTES`], and Range-reads
//! that tail. Everon at 16 MiB lands on level 3 — 1600×1600, 8 m per texel, 10.2 MB.
//!
//! **A coarser level is the safe direction, not a compromise.** The pyramid folds with
//! `mask = any water` and `depth = max`, so an 8 m texel reports water when *any* of its 64 one-
//! metre samples is wet. A placement guard reading a coarse level therefore refuses slightly more
//! ground than it strictly must, and never allows ground it should not. The world→texel ladder is
//! unchanged either way: [`Bathymetry::from_level_suffix`] keeps the *whole file's* header, so the
//! answers are the full container's answers at those levels rather than a resampling of them.
//!
//! # What a guard gets when there is no file
//!
//! Nothing loads → [`WaterHost::mask`] is `None` → [`super::is_known_dry_land`] is `false`
//! everywhere. "No reading" and "off the map" and "in a lake" all answer the same way at the call
//! site, which is the only answer that cannot put a unit in a lake. The decode itself lives in
//! `map_engine_core::world::water` — `world_assets` is `wasm32`-only and this repo has no
//! wasm-bindgen-test harness, so everything testable is on the other side of that boundary
//! (T-935.7 split `map_labels.rkyv` the same way).

use map_engine_core::world::binary::chunk_container::{ContainerHeader, TbdbHeader};
use map_engine_core::world::{
    suffix_plan, Bathymetry, WaterBlock, WaterMask, WaterVectors, TBDB_ENCODING_V1,
};

use crate::editor::mission_editor::boot_progress::{BootEvent, BootSeg};

use super::fetch::{fetch_bytes, fetch_range_outcome, RangeOutcome};

/// The most bathymetry this loader will pull. 16 MiB puts everon on level 3 (8 m texels) and the
/// whole of any terrain under ~4600² texels on level 0 — see the module docs for why coarser is
/// the safe direction. Sized against the satellite bundle (42–152 MB), which this must not rival.
pub const MAX_BATHYMETRY_BYTES: u64 = 16 << 20;

/// T-628: the two files this host fetches when the manifest declares them. Declared immediately
/// before the first request and never when the block is absent, so a terrain with no water block
/// leaves nothing outstanding.
const WATER_FILES: u64 = 2;

/// Loaded water for the current terrain. Both lanes are independently optional: a terrain can ship
/// vectors without bathymetry or the other way round, and a failure in one must not cost the other.
#[derive(Default)]
pub struct WaterHost {
    mask: Option<WaterMask>,
    vectors: Option<WaterVectors>,
}

impl WaterHost {
    pub fn new() -> Self {
        Self::default()
    }

    /// The placement-guard mask, or `None` when this terrain ships no readable bathymetry.
    pub fn mask(&self) -> Option<&WaterMask> {
        self.mask.as_ref()
    }

    /// The lake / river / pond archive, read in place.
    ///
    /// `dead_code`-allowed for the same reason as [`super::with_water_mask`]: the archive is
    /// fetched and validated here because the ticket's acceptance is that both files load, and its
    /// first renderer is a later ticket. The reading it wraps — alignment, `access_checked`, the
    /// schema-version gate — is natively tested in `map_engine_core::world::water`.
    #[allow(dead_code)]
    pub fn vectors(&self) -> Option<&WaterVectors> {
        self.vectors.as_ref()
    }

    /// Load both files **if and only if** the manifest carries a `water` block this build reads.
    ///
    /// `water` is the raw `manifest.water` value: `bootstrap` keeps it untyped so a malformed
    /// block costs the water layer rather than failing the manifest parse the DEM and satellite
    /// lanes also depend on. `bounds` is the manifest's `worldBounds`.
    ///
    /// Every step falls back rather than fails — an absent block, an encoding this build does not
    /// implement, a 404, a server that ignores `Range`, a truncated container, an archive from a
    /// future schema. The cost of any of them is a `None` mask, which every caller reads as "no
    /// reading here".
    pub async fn init(
        &mut self,
        base: &str,
        water: Option<&serde_json::Value>,
        bounds: [f64; 4],
        report: &dyn Fn(BootEvent),
    ) {
        let Some(block) = readable_block(water) else {
            return;
        };
        report(BootEvent::Files(BootSeg::World, WATER_FILES));

        if let Some(raw) = fetch_bytes(&format!("{base}/{}", block.vectors)).await {
            self.vectors = WaterVectors::from_bytes(&raw).ok();
        }
        report(BootEvent::Done(BootSeg::World, 1));

        let url = format!("{base}/{}", block.bathymetry);
        self.mask = load_bathymetry(&url, bounds).await;
        report(BootEvent::Done(BootSeg::World, 1));
    }
}

/// The manifest's `water` block, when it names files this build can read.
///
/// `WaterBlock` is `serde(default)`, so a hand-edited manifest missing a field deserialises with
/// empty strings rather than failing — which must mean "fall back", not "assume the default path".
/// The encoding check is the same guard `dem_load::raw_block_is_readable` applies: a block naming a
/// container this code does not implement is a manifest written for a different reader, and reading
/// it anyway is how a mask answers confidently about the wrong terrain.
fn readable_block(water: Option<&serde_json::Value>) -> Option<WaterBlock> {
    let block: WaterBlock = serde_json::from_value(water?.clone()).ok()?;
    (!block.vectors.is_empty()
        && !block.bathymetry.is_empty()
        && block.encoding == TBDB_ENCODING_V1)
        .then_some(block)
}

/// Header Range read, then a plan, then the tail. `None` on any failure — see [`WaterHost::init`].
async fn load_bathymetry(url: &str, bounds: [f64; 4]) -> Option<WaterMask> {
    let head = match fetch_range_outcome(url, 0, (size_of::<TbdbHeader>() - 1) as u64).await {
        RangeOutcome::Body(b) => b.bytes,
        // A 200 here means the server ignored `Range` and is about to hand back 655 MB; the
        // helper already refused it. Rate limiting and 404 land here too, and all three mean the
        // same thing to a guard: no reading.
        RangeOutcome::RateLimited { .. } | RangeOutcome::Failed { .. } => return None,
    };
    // `read`, not `parse`: 32 bytes off the network arrive at whatever alignment the allocator
    // chose, and a failed cast on an otherwise-good header would cost the whole layer.
    let (header, _) = TbdbHeader::read(&head).ok()?;
    let plan = suffix_plan(&header, MAX_BATHYMETRY_BYTES)?;
    let end = plan.file_offset.checked_add(plan.bytes)?.checked_sub(1)?;
    let tail = match fetch_range_outcome(url, plan.file_offset, end).await {
        RangeOutcome::Body(b) => b.bytes,
        RangeOutcome::RateLimited { .. } | RangeOutcome::Failed { .. } => return None,
    };
    let bath = Bathymetry::from_level_suffix(header, plan.first_level, &tail).ok()?;
    WaterMask::new(bath, bounds)
}
