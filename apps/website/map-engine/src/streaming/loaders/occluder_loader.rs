//! Role: occluder loader.
//! Position: `streaming/loaders` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use std::collections::HashMap;
use std::sync::Arc;

use crate::spatial::bvh::sidecar::BvhSidecar;
use crate::spatial::world_los::descriptor::ArchiveBoot;
use crate::spatial::world_los::descriptor::BlasManifest;
use crate::spatial::world_los::descriptor::BuildingArchiveBytes;
use crate::spatial::world_los::descriptor::PrefabDescriptor;
use crate::spatial::world_los::state::WorldOccluder;
use crate::streaming::loaders::manifest::parse_manifest_binary;
use crate::streaming::scheduler::chunk_math::TerrainSizeM;
use crate::streaming::scheduler::state::ResidencyEvent;
use crate::streaming::scheduler::state::WorldResidency;

use crate::streaming::loaders::fetch::fetch_bytes;
use crate::streaming::loaders::fetch::fetch_text;

const FETCH_CONCURRENCY: usize = 12;

const FAILURE_CAP: u8 = 3;

const WANT_PER_PASS: usize = 96;

const DRAIN_ROUNDS: usize = 8;

/// Occluder host.
pub struct OccluderHost {
    occ: WorldOccluder,
    base: String,
    manifest: Option<BlasManifest>,

    archive: Option<BuildingArchiveBytes>,

    archive_census: (usize, usize, usize),
    failed: HashMap<String, u8>,
    ready: bool,
}

impl OccluderHost {
    /// New.
    #[must_use]
    pub fn new() -> Self {
        Self {
            occ: WorldOccluder::new(512.0, TerrainSizeM::default()),
            base: String::new(),
            manifest: None,
            archive: None,
            archive_census: (0, 0, 0),
            failed: HashMap::new(),
            ready: false,
        }
    }

    /// Init.
    pub async fn init(&mut self, base: &str, residency: &WorldResidency) {
        self.base = base.to_string();
        self.occ = WorldOccluder::new(residency.chunk_size_m(), residency.terrain());
        self.occ.set_prefabs(residency.prefab_rows());
        let _archive = self.init_from_archive(base).await;

        self.manifest = fetch_text(&format!("{base}/prefabs/blas-manifest.json"))
            .await
            .and_then(|t| serde_json::from_str(&t).ok());
        self.ready = true;
        let hot = hot_pids(self.manifest.as_ref());
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

    async fn init_from_archive(&mut self, base: &str) -> bool {
        let Some(rel) = self.archive_rel_path(base).await else {
            return false;
        };
        let Some(bytes) = fetch_bytes(&format!("{base}/{rel}")).await else {
            return false;
        };

        let held = BuildingArchiveBytes::new(&bytes);
        let Ok(archive) = held.archive() else {
            return false;
        };
        let boot = ArchiveBoot::from_archive(archive);
        self.archive_census = (boot.census.len(), boot.blocking, boot.unusable);
        for d in boot.census {
            self.occ.insert_descriptor(d);
        }
        self.archive = Some(held);
        self.log_archive_boot(bytes.len());
        true
    }

    fn log_archive_boot(&self, bytes: usize) {
        let (census, blocking, unusable) = self.archive_census();
        let levels = self
            .building_archive()
            .and_then(|a| a.archive().ok())
            .map(|a| a.blueprints.iter().map(|b| b.levels.len()).sum::<usize>())
            .unwrap_or_default();
        crate::diagnostics::platform::console::log!(
            "occluder: building_blueprints.rkyv {} KB - {census} non-blocking prefabs seeded, \
             {blocking} descriptors still JSON, {unusable} unresolved, {} blueprints \
             ({levels} levels, zero-copy)",
            bytes / 1024,
            self.archive
                .as_ref()
                .and_then(|a| a.archive().ok())
                .map_or(0, |a| a.blueprints.len()),
        );
    }

    async fn archive_rel_path(&self, base: &str) -> Option<String> {
        let text = fetch_text(&format!("{base}/manifest.json")).await?;
        let raw: serde_json::Value = serde_json::from_str(&text).ok()?;
        let rel = parse_manifest_binary(&raw).buildings?.archive;
        (!rel.is_empty()).then_some(rel)
    }

    /// The validated building archive, for readers that walk blueprint levels in place (viewshed, LOS). `archive()` re-runs `access_checked` and borrows — it never deserialises.
    #[must_use]
    pub fn building_archive(&self) -> Option<&BuildingArchiveBytes> {
        self.archive.as_ref()
    }

    /// `(census pids inserted from the archive, blocking pids still read as JSON, unresolvable rows)` — `(0, 0, 0)` when the JSON branch is in use. For the boot log and the LOS bench.
    #[must_use]
    pub fn archive_census(&self) -> (usize, usize, usize) {
        self.archive_census
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

    /// Mirror the residency's inserts / evictions, then fetch what the resident chunks still wait for (descriptors first, then BLAS). Returns whether anything changed.
    pub async fn run_viewport(&mut self, residency: &mut WorldResidency) -> bool {
        if !self.ready {
            return false;
        }
        let mut work = false;
        for ev in residency.take_residency_events() {
            match ev {
                ResidencyEvent::Inserted(id) => {
                    if let Some(c) = residency.chunk(&id)
                        && c.count > 0
                    {
                        self.occ.insert_chunk(&id, c);
                        work = true;
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

            let blas = want.blas;
            if !blas.is_empty() {
                round |= self.fetch_blas(&blas).await;
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

    /// Occluder.
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

fn hot_pids(manifest: Option<&BlasManifest>) -> Vec<u16> {
    manifest
        .map(|m| {
            m.hot
                .iter()
                .filter_map(|p| u16::try_from(*p).ok())
                .collect()
        })
        .unwrap_or_default()
}

impl Default for OccluderHost {
    fn default() -> Self {
        Self::new()
    }
}
