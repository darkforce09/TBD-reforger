//! Role: loader.
//! Position: `world/environment/vegetation` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::culling::lod::class_visible;
use crate::io::density::tbdd::decode_tbdd;
use crate::renderers::primitives::compose::FOREST_OUTLINE_RGBA;
use crate::renderers::primitives::compose::compose_contour_hairlines;
use crate::world::environment::vegetation::density::CHUNKS_PER_AXIS;
use crate::world::environment::vegetation::density::EVERON_DENSITY_BINS;
use crate::world::environment::vegetation::density::ISLAND_CORNERS;
use crate::world::environment::vegetation::density::pack_island_r8_yflip;
use crate::world::environment::vegetation::density::stitch_chunk_into_island;
use crate::world::environment::vegetation::mass::CANOPY_MASS_ISO;
use crate::world::environment::vegetation::mass::forest_fill_alpha;
use crate::world::environment::vegetation::mass::forest_outline_segments_from_corners;

use crate::core::context::handles::EngineHandle;
use crate::streaming::bridge::progress::BootEvent;
use crate::streaming::bridge::progress::BootSeg;

use crate::streaming::bridge::statistics::BridgeHandle;
use crate::streaming::bridge::statistics::publish;
use crate::streaming::bridge::statistics::publish_engine;
use crate::streaming::loaders::fetch::fetch_bytes;

/// Planned density bins.
#[must_use]
pub const fn planned_density_bins() -> u64 {
    (CHUNKS_PER_AXIS * CHUNKS_PER_AXIS) as u64
}

const FETCH_CONCURRENCY: usize = 12;
const FETCH_RETRIES: usize = 3;
const WORLD_M: f64 = 12_800.0;
const CELL_M: f64 = 8.0;

/// Forest mass host.
pub struct ForestMassHost {
    asset_base: String,
    ready: bool,

    uploaded: bool,
    bins_ok: u32,
    last_params: (u64, bool, bool),
}

impl ForestMassHost {
    /// New.
    pub fn new() -> Self {
        Self {
            asset_base: String::new(),
            ready: false,
            uploaded: false,
            bins_ok: 0,
            last_params: (u64::MAX, false, false),
        }
    }

    /// Init.
    pub fn init(&mut self, terrain: &str) {
        self.asset_base = format!("/map-assets/{terrain}");
        self.ready = true;
        self.uploaded = false;
        self.bins_ok = 0;
        self.last_params = (u64::MAX, false, false);
    }

    /// True once the island density texture is on the GPU.
    pub fn is_uploaded(&self) -> bool {
        self.uploaded
    }

    /// Boot: fetch all 625 bins (retry), stitch, upload once + MS outlines. Settle: LOD params only.
    pub async fn run_viewport(
        &mut self,
        engine: &EngineHandle,
        bridge: &BridgeHandle,
        report: &dyn Fn(BootEvent),
    ) -> bool {
        if !self.ready {
            return false;
        }
        if !self.uploaded {
            return self.boot_upload(engine, bridge, report).await;
        }
        self.apply_params(engine, bridge)
    }

    async fn boot_upload(
        &mut self,
        engine: &EngineHandle,
        bridge: &BridgeHandle,
        report: &dyn Fn(BootEvent),
    ) -> bool {
        let base = self.asset_base.clone();
        let mut island = vec![0u16; ISLAND_CORNERS * ISLAND_CORNERS];
        let mut pending: Vec<(u32, u32)> = Vec::with_capacity(EVERON_DENSITY_BINS as usize);
        for cy in 0..CHUNKS_PER_AXIS as u32 {
            for cx in 0..CHUNKS_PER_AXIS as u32 {
                pending.push((cx, cy));
            }
        }
        let mut bins_ok = 0u32;
        for _attempt in 0..FETCH_RETRIES {
            if pending.is_empty() {
                break;
            }
            let mut still = Vec::new();
            for batch in pending.chunks(FETCH_CONCURRENCY) {
                let futs = batch.iter().map(|&(cx, cy)| {
                    let url = format!("{base}/objects/density/{cx}_{cy}.bin");
                    async move { (cx, cy, fetch_bytes(&url).await) }
                });
                for (cx, cy, bytes) in futures::future::join_all(futs).await {
                    let mut ok = false;
                    if let Some(b) = bytes
                        && let Ok(grid) = decode_tbdd(&b)
                        && let Some(tree) = grid.channels.first()
                        && tree.len() == 65 * 65
                    {
                        stitch_chunk_into_island(&mut island, cx, cy, tree);
                        ok = true;
                        bins_ok += 1;
                    }

                    if ok {
                        report(BootEvent::Done(BootSeg::World, 1));
                    } else {
                        still.push((cx, cy));
                    }
                }
            }
            pending = still;
        }
        self.bins_ok = bins_ok;
        {
            let mut b = bridge.borrow_mut();
            b.forest_bins_ok = bins_ok;
        }
        publish(bridge);

        if bins_ok != EVERON_DENSITY_BINS {
            return true;
        }

        let (rgba, bpr) = pack_island_r8_yflip(&island);
        let outline_segs = forest_outline_segments_from_corners(
            &island,
            ISLAND_CORNERS,
            ISLAND_CORNERS,
            0.0,
            0.0,
            CELL_M,
            CANOPY_MASS_ISO,
        );
        let hair = compose_contour_hairlines(&outline_segs, FOREST_OUTLINE_RGBA);

        {
            let mut g = engine.borrow_mut();
            let Some(e) = g.as_mut() else {
                return false;
            };
            if e.forest_density_upload(
                0.0,
                0.0,
                WORLD_M,
                WORLD_M,
                ISLAND_CORNERS as u32,
                ISLAND_CORNERS as u32,
                &rgba,
                bpr,
                bins_ok,
            )
            .is_err()
            {
                return false;
            }
            e.upload_hairline_segments(
                crate::overlay::lanes::role_id::FOREST_OUTLINE,
                &hair.verts,
                hair.segment_count,
                false,
            );
            e.forest_outline_set_stored(hair.segment_count);
        }
        self.uploaded = true;
        self.last_params = (u64::MAX, false, false);
        self.apply_params(engine, bridge);
        true
    }

    fn apply_params(&mut self, engine: &EngineHandle, bridge: &BridgeHandle) -> bool {
        let zoom = engine.borrow().as_ref().map(|e| e.zoom()).unwrap_or(-2.0);
        let fill_on = class_visible("forestFill", zoom);
        let outline_on = class_visible("forestOutline", zoom);
        let alpha = forest_fill_alpha(zoom);
        let state = (alpha.to_bits(), fill_on, outline_on);
        if state == self.last_params {
            return false;
        }
        if let Some(e) = engine.borrow_mut().as_mut() {
            #[allow(clippy::cast_possible_truncation)]
            e.forest_density_set_params(alpha as f32, fill_on, outline_on);
            publish_engine(bridge, e);
        }
        {
            let mut b = bridge.borrow_mut();
            b.forest_bins_ok = self.bins_ok;
        }
        publish(bridge);
        self.last_params = state;
        true
    }
}

impl Default for ForestMassHost {
    fn default() -> Self {
        Self::new()
    }
}
