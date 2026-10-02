//! Role: ingest.
//! Position: `streaming/loaders/world_loader` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::BootEvent;
use super::BootSeg;
use super::BridgeHandle;
use super::EngineHandle;
use super::FETCH_CONCURRENCY;
use super::PendingChunk;
use super::WorldHost;
use super::chunk_bin_path;
use super::fetch_bytes;

impl WorldHost {
    /// Fetch and queue.
    pub(super) async fn fetch_and_queue(&mut self, ids: Vec<String>, report: &dyn Fn(BootEvent)) {
        report(BootEvent::Files(BootSeg::World, ids.len() as u64));

        self.residency.mark_inflight(&ids);
        let base = self.asset_base.clone();
        let chunks = self.chunks_path.clone();
        let chunks_bin = self.chunks_bin.clone();
        let mut fetched = Vec::with_capacity(ids.len());
        self.crossing_allocs.pass_fetch_batch += 1;

        for batch in ids.chunks(FETCH_CONCURRENCY) {
            let futs = batch.iter().map(|id| {
                let rel = chunks_bin
                    .as_deref()
                    .and_then(|template| chunk_bin_path(template, id));
                let binary = rel.is_some();
                let url = rel.map_or_else(
                    || format!("{base}/{chunks}/{id}.json.gz"),
                    |rel| format!("{base}/{rel}"),
                );
                let id = id.clone();
                async move {
                    let bytes = fetch_bytes(&url).await;
                    PendingChunk { id, bytes, binary }
                }
            });
            for item in futures::future::join_all(futs).await {
                report(BootEvent::Done(BootSeg::World, 1));
                fetched.push(item);
            }
        }

        for item in fetched.into_iter().rev() {
            self.pending.push_front(item);
        }
    }
}

impl WorldHost {
    /// Drain.
    pub(super) fn drain(&mut self, engine: &EngineHandle, bridge: &BridgeHandle) -> bool {
        let now = js_sys::Date::now();
        self.residency.begin_ingest_frame_at(now);
        let mut applied = 0u32;

        const MAX_PER_SETTLE: u32 = 24;
        while !self.pending.is_empty() && applied < MAX_PER_SETTLE {
            let Some(next) = self.pending.pop_front() else {
                break;
            };
            match next.bytes {
                Some(bytes) => {
                    let applied = if next.binary {
                        match self.residency.ingest_chunk_bin(&next.id, &bytes) {
                            Ok(_) => true,
                            Err(e) => {
                                web_sys::console::warn_1(
                                    &format!("world: chunk {} rejected: {e}", next.id).into(),
                                );
                                false
                            }
                        }
                    } else {
                        self.residency.ingest_chunk_gz(&next.id, &bytes).is_ok()
                    };
                    if !applied {
                        self.residency.note_fetch_failure(&next.id);
                    }
                }
                None => self.residency.note_fetch_failure(&next.id),
            }
            applied += 1;
        }
        if applied > 0 {
            self.residency.end_ingest_frame_at(js_sys::Date::now());
            self.push_to_engine(engine, bridge);
        }
        applied > 0
    }
}
