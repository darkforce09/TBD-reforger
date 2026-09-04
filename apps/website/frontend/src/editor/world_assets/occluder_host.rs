//! T-090.12.5 — the SPA's mirror of chunk residency into the world occluder
//! (`map_engine_core::world::occluder::WorldOccluder`) plus the lazy fetch of prefab descriptors
//! and BLAS sidecars from `/map-assets/<terrain>/prefabs/`. Owned by [`super::world_host::WorldHost`]
//! and run at the tail of every viewport pass; readers reach it through
//! `world_assets::with_occluder`. Same fetch path and browser HTTP cache as the chunks.

use std::collections::HashMap;
use std::sync::Arc;

use map_engine_core::bvh::BvhSidecar;
use map_engine_core::world::occluder::{BlasManifest, PrefabDescriptor, WorldOccluder};
use map_engine_core::world::{ResidencyEvent, TerrainSizeM, WorldResidency};

use super::fetch::{fetch_bytes, fetch_text};

const FETCH_CONCURRENCY: usize = 12;
/// A descriptor / BLAS that failed this many times is not asked for again this session.
const FAILURE_CAP: u8 = 3;
/// Descriptors and BLAS fetched per viewport pass (six passes per settle).
const WANT_PER_PASS: usize = 96;
/// Rounds of `wanted` a single viewport pass drains (each up to [`WANT_PER_PASS`] descriptors +
/// BLAS). The settle loop caps itself at 12 passes; without an inner drain a village at zoom 1
/// (≈ 1,300 BLAS) was left 89 BLAS short until the next camera move.
const DRAIN_ROUNDS: usize = 8;

pub struct OccluderHost {
    occ: WorldOccluder,
    base: String,
    manifest: Option<BlasManifest>,
    failed: HashMap<String, u8>,
    ready: bool,
}

impl OccluderHost {
    #[must_use]
    pub fn new() -> Self {
        Self {
            occ: WorldOccluder::new(512.0, TerrainSizeM::default()),
            base: String::new(),
            manifest: None,
            failed: HashMap::new(),
            ready: false,
        }
    }

    /// After the residency loaded its manifest + prefabs: size the occluder, take the catalogue,
    /// fetch the library manifest and prefetch the hot set (descriptors + their BLAS).
    pub async fn init(&mut self, base: &str, residency: &WorldResidency) {
        self.base = base.to_string();
        self.occ = WorldOccluder::new(residency.chunk_size_m(), residency.terrain());
        self.occ.set_prefabs(residency.prefab_rows());
        self.manifest = fetch_text(&format!("{base}/prefabs/blas-manifest.json"))
            .await
            .and_then(|t| serde_json::from_str(&t).ok());
        self.ready = true;
        let hot: Vec<u16> = self
            .manifest
            .as_ref()
            .map(|m| {
                m.hot
                    .iter()
                    .filter_map(|p| u16::try_from(*p).ok())
                    .collect()
            })
            .unwrap_or_default();
        if !hot.is_empty() {
            self.fetch_descriptors(&hot).await;
            let paths: Vec<String> = hot
                .iter()
                .filter_map(|pid| self.occ.descriptor_of(*pid))
                .flat_map(|d| {
                    d.blas_paths()
                        .iter()
                        .map(|s| (*s).to_string())
                        .collect::<Vec<_>>()
                })
                .collect();
            let mut distinct: Vec<String> = Vec::new();
            for p in paths {
                if !distinct.contains(&p) {
                    distinct.push(p);
                }
            }
            self.fetch_blas(&distinct).await;
            self.occ.refresh();
        }
    }

    fn failed_out(&self, key: &str) -> bool {
        self.failed.get(key).copied().unwrap_or(0) >= FAILURE_CAP
    }

    fn note_failure(&mut self, key: String) {
        *self.failed.entry(key).or_insert(0) += 1;
    }

    async fn fetch_descriptors(&mut self, pids: &[u16]) -> bool {
        let base = self.base.clone();
        let mut work = false;
        let wanted: Vec<u16> = pids
            .iter()
            .copied()
            .filter(|p| !self.failed_out(&format!("d{p}")))
            .collect();
        for batch in wanted.chunks(FETCH_CONCURRENCY) {
            let futs = batch.iter().map(|pid| {
                let url = format!("{base}/prefabs/descriptors/{pid}.json");
                let pid = *pid;
                async move { (pid, fetch_text(&url).await) }
            });
            for (pid, text) in futures::future::join_all(futs).await {
                match text.and_then(|t| serde_json::from_str::<PrefabDescriptor>(&t).ok()) {
                    Some(d) => {
                        self.occ.insert_descriptor(d);
                        work = true;
                    }
                    None => self.note_failure(format!("d{pid}")),
                }
            }
        }
        work
    }

    async fn fetch_blas(&mut self, paths: &[String]) -> bool {
        let base = self.base.clone();
        let mut work = false;
        let wanted: Vec<String> = paths
            .iter()
            .filter(|p| !self.failed_out(p))
            .cloned()
            .collect();
        for batch in wanted.chunks(FETCH_CONCURRENCY) {
            let futs = batch.iter().map(|rel| {
                let url = format!("{base}/prefabs/{rel}");
                let rel = rel.clone();
                async move { (rel, fetch_bytes(&url).await) }
            });
            for (rel, bytes) in futures::future::join_all(futs).await {
                match bytes.and_then(|b| BvhSidecar::parse(&b).ok()) {
                    Some(sc) => {
                        self.occ.insert_blas(&rel, Arc::new(sc));
                        work = true;
                    }
                    None => self.note_failure(rel),
                }
            }
        }
        work
    }

    /// Mirror the residency's inserts / evictions, then fetch what the resident chunks still
    /// wait for (descriptors first, then BLAS). Returns whether anything changed.
    pub async fn run_viewport(&mut self, residency: &mut WorldResidency) -> bool {
        if !self.ready {
            return false;
        }
        let mut work = false;
        for ev in residency.take_residency_events() {
            match ev {
                ResidencyEvent::Inserted(id) => {
                    if let Some(c) = residency.chunk(&id) {
                        if c.count > 0 {
                            self.occ.insert_chunk(&id, c);
                            work = true;
                        }
                    }
                }
                ResidencyEvent::Evicted(id) => {
                    self.occ.remove_chunk(&id);
                    work = true;
                }
            }
        }
        let ids = self.occ.resident_chunk_ids();
        for _ in 0..DRAIN_ROUNDS {
            let mut round = false;
            let want = self.occ.wanted(&ids, WANT_PER_PASS);
            if !want.descriptors.is_empty() {
                round |= self.fetch_descriptors(&want.descriptors).await;
            }
            let want = self.occ.wanted(&ids, WANT_PER_PASS);
            if !want.blas.is_empty() {
                round |= self.fetch_blas(&want.blas).await;
            }
            if !round {
                break;
            }
            work = true;
            self.occ.refresh();
        }
        if work {
            self.occ.refresh();
        }
        work
    }

    #[must_use]
    pub fn occluder(&self) -> &WorldOccluder {
        &self.occ
    }

    /// `(keys at the failure cap, up to five of them)` — the fetches this session gave up on.
    #[must_use]
    pub fn failed_summary(&self) -> (usize, Vec<String>) {
        let mut out: Vec<String> = self
            .failed
            .iter()
            .filter(|(_, n)| **n >= FAILURE_CAP)
            .map(|(k, _)| k.clone())
            .collect();
        out.sort();
        let n = out.len();
        out.truncate(5);
        (n, out)
    }
}

impl Default for OccluderHost {
    fn default() -> Self {
        Self::new()
    }
}
