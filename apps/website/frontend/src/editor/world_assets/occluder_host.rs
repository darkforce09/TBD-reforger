//! T-090.12.5 — the SPA's mirror of chunk residency into the world occluder
//! (`map_engine_core::world::occluder::WorldOccluder`) plus the lazy fetch of prefab descriptors
//! and BLAS sidecars from `/map-assets/<terrain>/prefabs/`. Owned by [`super::world_host::WorldHost`]
//! and run at the tail of every viewport pass; readers reach it through
//! `world_assets::with_occluder`. Same fetch path and browser HTTP cache as the chunks.
//!
//! ── T-935.8: the archive branch ──────────────────────────────────────────────────────────────
//! When the terrain manifest carries a `buildings` block (spec §5), boot reads ONE file —
//! `prefabs/building_blueprints.rkyv` — instead of `prefabs/blas-manifest.json` (947 KB of JSON
//! whose only runtime use was the hot list). That single validated fetch carries:
//!
//! * the **census**: every `blocks: false` prefab, inserted at once, so those pids are routed to
//!   `no_block` and never fetched at all (301 of everon's 1623 descriptors today);
//! * the **BLAS index**: every prefab's `.bvh` fetch list. T-946 removed the fold that consumed
//!   it — it ran AFTER the awaited descriptor fetch, so `wanted()` had already named every
//!   sidecar it could have added. Queueing these concurrently with the descriptors is the real
//!   saving and is its own ticket; until then the archive's index is not read at runtime;
//! * the **blueprint levels**, kept as bytes so LOS and viewshed read them zero-copy through
//!   [`OccluderHost::building_archive`] — `access_checked` hands back a borrowed view, nothing is
//!   deserialised.
//!
//! **What the archive deliberately does NOT replace.** Its descriptor row
//! (`world::binary::archives::OccluderDescriptor`) carries no [`InstanceRecord`]s, so a `blocks:
//! true` prefab rebuilt from it has no placed child geometry. Inserting one would put the pid in
//! `WorldOccluder::descriptors`, fail `try_expand` on an empty instance list, and then be skipped
//! by `wanted()` forever — the prefab would trace against its coarse AABB for the rest of the
//! session with no error anywhere. So `ArchiveBoot` hands back `blocks: false` rows only and the
//! JSON descriptor fetch below stays exactly as it is for the 1322 blocking prefabs. It is a
//! silently wrong sightline that is being avoided here, not a crash.
//!
//! [`InstanceRecord`]: map_engine_core::building_compound::InstanceRecord

use std::collections::HashMap;
use std::sync::Arc;

use map_engine_core::bvh::BvhSidecar;
use map_engine_core::world::occluder::descriptor::{ArchiveBoot, BuildingArchiveBytes};
use map_engine_core::world::occluder::{BlasManifest, PrefabDescriptor, WorldOccluder};
use map_engine_core::world::{parse_manifest_binary, ResidencyEvent, TerrainSizeM, WorldResidency};

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
    /// T-935.8 — `prefabs/building_blueprints.rkyv`, held as the 8-aligned bytes it was fetched
    /// as. Kept rather than deserialised so the blueprint levels stay zero-copy for viewshed and
    /// LOS; `None` on every manifest without a `buildings` block, which is the shipped state
    /// until T-935.13.
    archive: Option<BuildingArchiveBytes>,
    /// `(census inserted, blocking rows left to JSON, rows the archive could not resolve)`.
    archive_census: (usize, usize, usize),
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
            archive: None,
            archive_census: (0, 0, 0),
            failed: HashMap::new(),
            ready: false,
        }
    }

    /// After the residency loaded its manifest + prefabs: size the occluder, take the catalogue,
    /// then either read the T-935.8 archive (one fetch) or fall back to the library manifest plus
    /// the hot-set prefetch (descriptors + their BLAS).
    pub async fn init(&mut self, base: &str, residency: &WorldResidency) {
        self.base = base.to_string();
        self.occ = WorldOccluder::new(residency.chunk_size_m(), residency.terrain());
        self.occ.set_prefabs(residency.prefab_rows());
        if self.init_from_archive(base).await {
            self.ready = true;
            self.occ.refresh();
            return;
        }
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

    /// T-935.8 — the archive boot. `true` when `prefabs/building_blueprints.rkyv` was fetched,
    /// validated and seeded; `false` means "use the JSON branch", and every `false` here is a
    /// fallback rather than a failure: a terrain with no `buildings` block, an archive that is not
    /// there, or one that does not validate. A malformed archive must cost a slower boot, never a
    /// blank map — which is also why nothing is inserted before `access_checked` has passed.
    async fn init_from_archive(&mut self, base: &str) -> bool {
        let Some(rel) = self.archive_rel_path(base).await else {
            return false;
        };
        let Some(bytes) = fetch_bytes(&format!("{base}/{rel}")).await else {
            return false;
        };
        // `fetch_bytes` hands back a 1-aligned `Vec<u8>`; `access_checked` validates alignment, so
        // the holder's copy into a `Vec<u64>` is what makes the read deterministic rather than a
        // coin flip that reads as "the archive is corrupt".
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

    /// The boot receipt, read back through the public accessors — which is the point: it walks the
    /// held bytes with `access_checked` exactly the way a viewshed reader will, so a browser where
    /// the zero-copy path does not work says so in the console at boot instead of at the first
    /// sightline.
    fn log_archive_boot(&self, bytes: usize) {
        let (census, blocking, unusable) = self.archive_census();
        let levels = self
            .building_archive()
            .and_then(|a| a.archive().ok())
            .map(|a| a.blueprints.iter().map(|b| b.levels.len()).sum::<usize>())
            .unwrap_or_default();
        leptos::logging::log!(
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

    /// `buildings.archive` from `<base>/manifest.json`, or `None` when the block is absent.
    ///
    /// The manifest is read here rather than passed in because `init`'s two callers
    /// ([`super::world_host::WorldHost`] and the `/debug/world-los` bench) both already fetched it
    /// — this is a browser-cache hit, not a second download, and it keeps the archive decision in
    /// the one file that acts on it.
    async fn archive_rel_path(&self, base: &str) -> Option<String> {
        let text = fetch_text(&format!("{base}/manifest.json")).await?;
        let raw: serde_json::Value = serde_json::from_str(&text).ok()?;
        let rel = parse_manifest_binary(&raw).buildings?.archive;
        (!rel.is_empty()).then_some(rel)
    }

    /// The validated building archive, for readers that walk blueprint levels in place (viewshed,
    /// LOS). `archive()` re-runs `access_checked` and borrows — it never deserialises.
    #[must_use]
    pub fn building_archive(&self) -> Option<&BuildingArchiveBytes> {
        self.archive.as_ref()
    }

    /// `(census pids inserted from the archive, blocking pids still read as JSON, unresolvable
    /// rows)` — `(0, 0, 0)` when the JSON branch is in use. For the boot log and the LOS bench.
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
            // T-946 — THE ARCHIVE SIDECAR FOLD IS GONE, because it could not do what it said.
            //
            // It claimed to carry "the sidecars of the pids whose descriptors are being fetched
            // right now, in the SAME round". But `fetch_descriptors` is AWAITED above, so by the
            // time this line runs every descriptor that parsed is already in `self.occ` and the
            // `wanted()` call on the line above already names its sidecars — the fold's own
            // `!out.contains(p)` then removed them again. The only paths it could still add
            // belonged to pids whose descriptor 404'd or failed to parse, and `try_expand` can
            // never attach geometry to a descriptor it does not have. So the mechanism bought no
            // round-trip and spent real fetches and real bytes of the 48 MB budget on geometry
            // that provably could not be used. Found by the wave 238 verifier.
            //
            // Issuing the archive's sidecars CONCURRENTLY with the descriptor fetch would deliver
            // the intended saving; that is a change to this loop's ordering and belongs in its own
            // ticket rather than in a fix for a mechanism that was inert.
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
