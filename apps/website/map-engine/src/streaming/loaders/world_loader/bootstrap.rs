//! Role: bootstrap.
//! Position: `streaming/loaders/world_loader` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::BootEvent;
use super::BootSeg;
use super::WorldHost;
use super::fetch_bytes;
use super::fetch_text;
use super::load_glyph_atlas;
use super::parse_manifest_binary;
use super::regions_from_bytes;

impl WorldHost {
    /// Init.
    pub async fn init(&mut self, terrain: &str, report: &dyn Fn(BootEvent)) -> bool {
        let done = || report(BootEvent::Done(BootSeg::World, 1));
        let base = format!("/map-assets/{terrain}");
        let manifest = fetch_text(&format!("{base}/manifest.json")).await;
        done();
        let Some(manifest) = manifest else {
            return false;
        };
        if self.residency.load_manifest_json(&manifest).is_err() {
            return false;
        }
        if self.store.load_manifest_json(&manifest).is_err() {
            return false;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&manifest) else {
            return false;
        };
        let Some(objects) = v.get("objects") else {
            return false;
        };
        let Some(prefabs) = objects.get("prefabsPath").and_then(|x| x.as_str()) else {
            return false;
        };
        let Some(chunks) = objects.get("chunksPath").and_then(|x| x.as_str()) else {
            return false;
        };
        let roads = objects
            .get("roadsPath")
            .and_then(|x| x.as_str())
            .unwrap_or("objects/roads.json.gz");
        let regions = objects
            .get("regionsPath")
            .and_then(|x| x.as_str())
            .unwrap_or("objects/forest-regions.json.gz");
        self.asset_base = base.clone();
        self.chunks_path = chunks.to_string();
        let objects_bin = parse_manifest_binary(&v).objects;

        self.chunks_bin = objects_bin
            .as_ref()
            .filter(|b| b.matches_this_build())
            .map(|b| b.chunks.clone());

        let named = |p: &String| (!p.is_empty()).then(|| p.clone());
        let prefabs_bin = objects_bin.as_ref().and_then(|b| named(&b.prefabs));
        let roads_bin = objects_bin.as_ref().and_then(|b| named(&b.roads));
        let regions_bin = objects_bin.as_ref().and_then(|b| named(&b.regions));

        if self
            .load_prefabs(&base, terrain, prefabs_bin.as_deref(), prefabs)
            .await
        {
            self.occluder.init(&base, &self.residency).await;
        }
        done();
        if let Some(idx) = fetch_text(&format!("{base}/{chunks}/manifest.json")).await {
            let _ = self.residency.load_chunk_index_json(&idx);
        }
        done();

        self.atlas = load_glyph_atlas(report).await;

        self.roads_loaded = self.load_roads(&base, roads_bin.as_deref(), roads).await;
        if self.roads_loaded {
            let runways: Vec<_> = self.store.runway_segments().into_iter().cloned().collect();
            self.residency.set_airfield_bbox_from_runways(&runways);
        }
        done();
        self.landcover_ready = self
            .load_regions(&base, regions_bin.as_deref(), regions)
            .await;
        done();
        self.ready = true;
        true
    }
}

impl WorldHost {
    /// Load prefabs.
    pub(super) async fn load_prefabs(
        &mut self,
        base: &str,
        terrain: &str,
        bin: Option<&str>,
        json: &str,
    ) -> bool {
        if let Some(path) = bin {
            return match fetch_bytes(&format!("{base}/{path}")).await {
                Some(bytes) => match self.residency.load_prefabs(&bytes, terrain) {
                    Ok(_) => true,
                    Err(e) => {
                        web_sys::console::warn_1(
                            &format!("world: prefab archive {path} rejected: {e}").into(),
                        );
                        false
                    }
                },
                None => {
                    web_sys::console::warn_1(
                        &format!("world: prefab archive {path} missing").into(),
                    );
                    false
                }
            };
        }
        match fetch_bytes(&format!("{base}/{json}")).await {
            Some(bytes) => self.residency.load_prefabs(&bytes, terrain).is_ok(),
            None => false,
        }
    }
}

impl WorldHost {
    /// Load roads.
    pub(super) async fn load_roads(&mut self, base: &str, bin: Option<&str>, json: &str) -> bool {
        if let Some(path) = bin {
            return match fetch_bytes(&format!("{base}/{path}")).await {
                Some(bytes) => match self.store.load_roads(&bytes) {
                    Ok(_) => true,
                    Err(e) => {
                        web_sys::console::warn_1(
                            &format!("world: road archive {path} rejected: {e}").into(),
                        );
                        false
                    }
                },
                None => {
                    web_sys::console::warn_1(&format!("world: road archive {path} missing").into());
                    false
                }
            };
        }
        match fetch_bytes(&format!("{base}/{json}")).await {
            Some(bytes) => self.store.load_roads(&bytes).is_ok(),
            None => false,
        }
    }
}

impl WorldHost {
    /// Load regions.
    pub(super) async fn load_regions(&mut self, base: &str, bin: Option<&str>, json: &str) -> bool {
        if let Some(path) = bin {
            return match fetch_bytes(&format!("{base}/{path}")).await {
                Some(bytes) => match regions_from_bytes(&bytes) {
                    Ok(regions) => {
                        self.store.regions = regions;
                        true
                    }
                    Err(e) => {
                        web_sys::console::warn_1(
                            &format!("world: region archive {path} rejected: {e}").into(),
                        );
                        false
                    }
                },
                None => {
                    web_sys::console::warn_1(
                        &format!("world: region archive {path} missing").into(),
                    );
                    false
                }
            };
        }
        match fetch_bytes(&format!("{base}/{json}")).await {
            Some(bytes) => self.store.load_forest_regions_gz(&bytes).is_ok(),
            None => false,
        }
    }
}
