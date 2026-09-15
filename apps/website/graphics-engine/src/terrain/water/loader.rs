//! Role: loader.
//! Position: `terrain/water` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::formats::containers::header::ContainerHeader;
use crate::formats::containers::tbdb::TbdbHeader;
use crate::streaming::loaders::manifest::WaterBlock;
use crate::terrain::water::vectors::Bathymetry;
use crate::terrain::water::vectors::TBDB_ENCODING_V1;
use crate::terrain::water::vectors::WaterMask;
use crate::terrain::water::vectors::WaterVectors;
use crate::terrain::water::vectors::suffix_plan;

use crate::streaming::bridge::progress::BootEvent;
use crate::streaming::bridge::progress::BootSeg;

use crate::streaming::loaders::fetch::RangeOutcome;
use crate::streaming::loaders::fetch::fetch_bytes;
use crate::streaming::loaders::fetch::fetch_range_outcome;

/// The most bathymetry this loader will pull. 16 MiB puts everon on level 3 (8 m texels) and the whole of any terrain under ~4600² texels on level 0 — see the module docs for why coarser is the safe direction. Sized against the satellite bundle (42–152 MB), which this must not rival.
pub const MAX_BATHYMETRY_BYTES: u64 = 16 << 20;

const WATER_FILES: u64 = 2;

/// Loaded water for the current terrain. Both lanes are independently optional: a terrain can ship vectors without bathymetry or the other way round, and a failure in one must not cost the other.
#[derive(Default)]
pub struct WaterHost {
    mask: Option<WaterMask>,
    vectors: Option<WaterVectors>,
}

impl WaterHost {
    /// New.
    pub fn new() -> Self {
        Self::default()
    }

    /// The placement-guard mask, or `None` when this terrain ships no readable bathymetry.
    pub fn mask(&self) -> Option<&WaterMask> {
        self.mask.as_ref()
    }

    /// The lake / river / pond archive, read in place.
    #[allow(dead_code)]
    pub fn vectors(&self) -> Option<&WaterVectors> {
        self.vectors.as_ref()
    }

    /// Load both files **if and only if** the manifest carries a `water` block this build reads.
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

fn readable_block(water: Option<&serde_json::Value>) -> Option<WaterBlock> {
    let block: WaterBlock = serde_json::from_value(water?.clone()).ok()?;
    (!block.vectors.is_empty()
        && !block.bathymetry.is_empty()
        && block.encoding == TBDB_ENCODING_V1)
        .then_some(block)
}

async fn load_bathymetry(url: &str, bounds: [f64; 4]) -> Option<WaterMask> {
    let head = match fetch_range_outcome(url, 0, (size_of::<TbdbHeader>() - 1) as u64).await {
        RangeOutcome::Body(b) => b.bytes,

        RangeOutcome::RateLimited { .. } | RangeOutcome::Failed { .. } => return None,
    };

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
