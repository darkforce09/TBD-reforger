//! T-178 — island TBDD density canopy (shader path). Replaces progressive 512 m mesh fill.

use map_engine_core::geometry::density_island::{
    pack_island_rgba_yflip, stitch_chunk_into_island, CHUNKS_PER_AXIS, EVERON_DENSITY_BINS,
    ISLAND_CORNERS,
};
use map_engine_core::geometry::forest_mass::forest_fill_alpha;
use map_engine_core::geometry::tbdd::decode_tbdd;
use map_engine_core::world::class_visible;

use crate::select_tool::EngineHandle;

use super::bridge::{publish_engine, BridgeHandle};
use super::fetch::fetch_bytes;

const FETCH_CONCURRENCY: usize = 12;
const WORLD_M: f64 = 12_800.0;

pub struct ForestMassHost {
    asset_base: String,
    ready: bool,
    /// Island density committed once.
    uploaded: bool,
    last_params: (u64, bool, bool),
}

impl ForestMassHost {
    pub fn new() -> Self {
        Self {
            asset_base: String::new(),
            ready: false,
            uploaded: false,
            last_params: (u64::MAX, false, false),
        }
    }

    pub fn init(&mut self, terrain: &str) {
        self.asset_base = format!("/map-assets/{terrain}");
        self.ready = true;
        self.uploaded = false;
        self.last_params = (u64::MAX, false, false);
    }

    /// True once the island density texture is on the GPU.
    pub fn is_uploaded(&self) -> bool {
        self.uploaded
    }

    /// Boot: fetch all 625 bins, stitch, upload once. Settle: LOD params only.
    /// Returns whether work happened (fetch/upload or param change).
    pub async fn run_viewport(&mut self, engine: &EngineHandle, bridge: &BridgeHandle) -> bool {
        if !self.ready {
            return false;
        }
        if !self.uploaded {
            let did = self.boot_upload(engine, bridge).await;
            return did;
        }
        self.apply_params(engine, bridge)
    }

    async fn boot_upload(&mut self, engine: &EngineHandle, bridge: &BridgeHandle) -> bool {
        let base = self.asset_base.clone();
        let mut island = vec![0u16; ISLAND_CORNERS * ISLAND_CORNERS];
        let mut ids: Vec<(u32, u32)> = Vec::with_capacity(EVERON_DENSITY_BINS as usize);
        for cy in 0..CHUNKS_PER_AXIS as u32 {
            for cx in 0..CHUNKS_PER_AXIS as u32 {
                ids.push((cx, cy));
            }
        }
        for batch in ids.chunks(FETCH_CONCURRENCY) {
            let futs = batch.iter().map(|&(cx, cy)| {
                let url = format!("{base}/objects/density/{cx}_{cy}.bin");
                async move { (cx, cy, fetch_bytes(&url).await) }
            });
            for (cx, cy, bytes) in futures::future::join_all(futs).await {
                if let Some(b) = bytes {
                    if let Ok(grid) = decode_tbdd(&b) {
                        if let Some(tree) = grid.channels.first() {
                            if tree.len() == 65 * 65 {
                                stitch_chunk_into_island(&mut island, cx, cy, tree);
                            }
                        }
                    }
                }
                // Missing/decode-fail → leave zeros (resolved hole).
            }
        }
        let rgba = pack_island_rgba_yflip(&island);
        let ok = {
            let mut g = engine.borrow_mut();
            let Some(e) = g.as_mut() else {
                return false;
            };
            e.forest_density_upload(
                0.0,
                0.0,
                WORLD_M,
                WORLD_M,
                ISLAND_CORNERS as u32,
                ISLAND_CORNERS as u32,
                &rgba,
            )
            .is_ok()
        };
        if !ok {
            return false;
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
        self.last_params = state;
        true
    }
}

impl Default for ForestMassHost {
    fn default() -> Self {
        Self::new()
    }
}
