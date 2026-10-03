//! The viewport pin, the in-flight marks, the fetch-failure cap, chunk insert and LRU eviction.
//!
//! **Role:** turns a viewport into the pinned chunk set and the chunk ids to fetch, inserts and
//! evicts resident chunks, and keeps the in-flight and failure bookkeeping.
//! **Position:** `chunk_scheduler`; methods of
//! [`crate::state::ChunkResidency`]; called through the composed world
//! residency by the world loader and the residency tests.
//! **Signals & state:** mutates the residency's pin, in-flight, failure, chunk, LRU, index and
//! event state.
//! **Invariants:** the pin lists `chunk_ids_for_viewport` ids intersected with the chunk index;
//! eviction keeps at least `max(LRU_MIN_CHUNKS, 3 × pinned)` chunks and never takes a pinned or
//! known-empty one; a chunk at `FETCH_FAILURE_CAP` failures becomes an empty stub.

use crate::draw_rebuild::DrawRebuild;
use crate::draw_rebuild::ViewportUpdate;
use crate::state::ChunkResidency;
use crate::state::ResidencyEvent;
use map_coordinates::chunk_math::chunk_ids_for_viewport;
use map_draw_lanes::zoom_gates::building_visible;
use prefab_catalog::render_classes::class_code;
use world_chunks::ChunkId;
use world_chunks::world_chunk::WorldChunk;

/// Canonical draw cull margin m value.
pub const DRAW_CULL_MARGIN_M: f64 = 0.0;

/// LRU floor (`chunkStore.ts` `LRU_MIN_CHUNKS`).
pub const LRU_MIN_CHUNKS: usize = 64;

/// Canonical fetch failure cap value.
pub const FETCH_FAILURE_CAP: u8 = 3;

/// Splits interleaved `[x0, y0, x1, y1, …]` positions into their x and y columns.
fn deinterleave(positions: &[f32], count: u32) -> (Vec<f32>, Vec<f32>) {
    let n = count as usize;
    let mut xs = Vec::with_capacity(n);
    let mut ys = Vec::with_capacity(n);
    for i in 0..n {
        xs.push(positions[2 * i]);
        ys.push(positions[2 * i + 1]);
    }
    (xs, ys)
}

impl ChunkResidency {
    /// `runViewport(bbox, deckZoom)` — pin the viewport chunk set, re-touch resident members, evict, and return the **missing** ids to fetch (already marked in-flight so an overlapping viewport won't re-request them) with the draw rebuild the viewport asks for. The missing ids are empty when below the building band or when the chunk set is unchanged.
    pub fn set_viewport(
        &mut self,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        deck_zoom: f64,
    ) -> ViewportUpdate {
        let prev_zoom = self.deck_zoom;
        self.deck_zoom = deck_zoom;
        self.last_viewport = [min_x, min_y, max_x, max_y];

        let world_want =
            building_visible(deck_zoom) || self.min_importance_zoom.is_some_and(|m| deck_zoom >= m);
        if !world_want {
            let rebuild = if !self.pinned_ids.is_empty() {
                self.pinned_ids.clear();
                self.pinned_set.clear();
                self.pinned_key.clear();
                DrawRebuild::AllBuffers
            } else {
                DrawRebuild::DrawSetAndGlyphs
            };
            return ViewportUpdate {
                missing: Vec::new(),
                rebuild,
            };
        }
        let chunk_size_m = match &self.manifest {
            Some(m) => m.chunk_size_m,
            None => {
                return ViewportUpdate {
                    missing: Vec::new(),
                    rebuild: DrawRebuild::Nothing,
                };
            }
        };
        let extra_ring = i64::from(self.has_oversized);
        let mut ids = chunk_ids_for_viewport(
            [min_x, min_y, max_x, max_y],
            self.terrain,
            chunk_size_m,
            extra_ring,
        );
        if let Some(cells) = &self.cell_ids {
            ids.retain(|id| cells.contains(id));
        }
        let key = ids.join(",");
        if key == self.pinned_key {
            let rebuild = DrawRebuild::ZoomUnderUnchangedPin {
                previous_zoom: prev_zoom,
                zoom: deck_zoom,
            };

            if self.pin_settled() || !self.inflight.is_empty() {
                return ViewportUpdate {
                    missing: Vec::new(),
                    rebuild,
                };
            }
            let missing: Vec<String> = self
                .pinned_ids
                .iter()
                .filter(|id| !self.chunks.contains_key(*id))
                .cloned()
                .collect();
            for id in &missing {
                self.inflight.insert(id.clone());
            }
            return ViewportUpdate {
                missing: missing.into_iter().map(ChunkId::new).collect(),
                rebuild,
            };
        }
        self.pinned_ids = ids.clone();
        self.pinned_set = ids.iter().cloned().collect();
        self.pinned_key = key;

        self.fetch_failures.clear();

        for id in &ids {
            if self.chunks.contains_key(id) {
                self.use_tick += 1;
                self.last_used.insert(id.clone(), self.use_tick);
            }
        }
        let missing: Vec<String> = ids
            .iter()
            .filter(|id| !self.chunks.contains_key(*id) && !self.inflight.contains(*id))
            .cloned()
            .collect();
        for id in &missing {
            self.inflight.insert(id.clone());
        }
        self.evict();
        ViewportUpdate {
            missing: missing.into_iter().map(ChunkId::new).collect(),
            rebuild: DrawRebuild::AllBuffers,
        }
    }
}

impl ChunkResidency {
    /// Counts one failed fetch of chunk `id`: its in-flight mark is released so a later viewport
    /// asks again, and at `FETCH_FAILURE_CAP` failures it becomes an empty stub.
    pub fn note_fetch_failure(&mut self, id: &ChunkId) {
        let n = self.fetch_failures.entry(id.to_string()).or_insert(0);
        *n += 1;
        if *n >= FETCH_FAILURE_CAP {
            self.fetch_failures.remove(id.as_str());
            self.note_undelivered(id);
        } else {
            self.inflight.remove(id.as_str());
        }
    }
}

impl ChunkResidency {
    /// Cache a requested-but-undelivered chunk (missing/empty file) as hydrated-empty so it is never re-requested (`requestMissing`'s `applied.set(id, [])`).
    pub fn note_undelivered(&mut self, id: &ChunkId) {
        self.insert_chunk(
            id,
            WorldChunk {
                id: id.clone(),
                ..Default::default()
            },
        );
    }
}

impl ChunkResidency {
    /// Insert chunk.
    pub(super) fn insert_chunk(&mut self, id: &ChunkId, chunk: WorldChunk) {
        let building_code = class_code("building");
        let building_count = chunk.rows_by_class.get(&building_code).map_or(0, |rows| {
            rows.iter()
                .filter(|&&r| {
                    self.building_by_u16
                        .contains_key(&chunk.prefab_idx[r as usize])
                })
                .count() as u32
        });
        let (xs, ys) = deinterleave(&chunk.positions, chunk.count);
        self.index.insert_chunk(id, &xs, &ys, &chunk.cls_codes);
        self.building_counts.insert(id.to_string(), building_count);
        self.residency_events
            .push(ResidencyEvent::Inserted(id.clone()));
        self.chunks.insert(id.to_string(), chunk);
        self.use_tick += 1;
        self.last_used.insert(id.to_string(), self.use_tick);
        self.insert_counter += 1;
        self.inserted_seq
            .insert(id.to_string(), self.insert_counter);
        self.inflight.remove(id.as_str());
        self.chunks_applied += 1;
        self.content_epoch += 1;
    }
}

impl ChunkResidency {
    /// Evict.
    pub(super) fn evict(&mut self) {
        let cap = LRU_MIN_CHUNKS.max(3 * self.pinned_ids.len());
        if self.chunks.len() <= cap {
            return;
        }

        let mut candidates: Vec<String> = self
            .chunks
            .keys()
            .filter(|id| !self.pinned_set.contains(*id) && !self.known_empty.contains(*id))
            .cloned()
            .collect();
        candidates.sort_by(|a, b| {
            let la = self.last_used.get(a).copied().unwrap_or(0);
            let lb = self.last_used.get(b).copied().unwrap_or(0);
            la.cmp(&lb).then_with(|| {
                let sa = self.inserted_seq.get(a).copied().unwrap_or(0);
                let sb = self.inserted_seq.get(b).copied().unwrap_or(0);
                sa.cmp(&sb)
            })
        });
        for id in candidates {
            if self.chunks.len() <= cap {
                break;
            }
            self.chunks.remove(&id);
            self.last_used.remove(&id);
            self.inserted_seq.remove(&id);
            self.building_counts.remove(&id);
            let chunk_id = ChunkId::new(id.as_str());
            self.index.remove_chunk(&chunk_id);
            self.residency_events
                .push(ResidencyEvent::Evicted(chunk_id));
            self.eviction_log.push(id);
            self.content_epoch += 1;
        }
    }
}

impl ChunkResidency {
    /// Ordered eviction victims since construction — parity surface (Class S eviction-order log).
    #[must_use]
    pub fn eviction_log(&self) -> Vec<String> {
        self.eviction_log.clone()
    }
}

impl ChunkResidency {
    /// Inflight count.
    #[must_use]
    pub fn inflight_count(&self) -> usize {
        self.inflight.len()
    }
}

impl ChunkResidency {
    /// Clear inflight.
    pub fn clear_inflight(&mut self) {
        self.inflight.clear();
    }
}

impl ChunkResidency {
    /// Release one in-flight mark after a soft fetch failure (host may retry next settle).
    pub fn release_inflight(&mut self, id: &ChunkId) {
        self.inflight.remove(id.as_str());
    }
}

impl ChunkResidency {
    /// Drop a resident (or empty-stub) chunk so the next `set_viewport` re-requests it. Used by the Leptos host to recover from a soft HTTP failure that must not be cached as a permanent empty stub (tree-glyph zoom probes need real instance rows).
    pub fn invalidate_chunk(&mut self, id: &ChunkId) {
        let key = id.as_str();
        if self.chunks.remove(key).is_some() {
            self.residency_events
                .push(ResidencyEvent::Evicted(id.clone()));
            self.index.remove_chunk(id);
            self.building_counts.remove(key);
            self.last_used.remove(key);
            self.inserted_seq.remove(key);
            self.content_epoch += 1;
        }
        self.inflight.remove(key);

        self.known_empty.remove(key);
        self.fetch_failures.remove(key);
    }
}

impl ChunkResidency {
    /// Mark ids as in-flight (not yet resident). Used after `clear_inflight` when starting a replacement fetch so concurrent same-key `set_viewport` does not re-queue them.
    pub fn mark_inflight(&mut self, ids: &[ChunkId]) {
        for id in ids {
            if !self.chunks.contains_key(id.as_str()) {
                self.inflight.insert(id.to_string());
            }
        }
    }
}

impl ChunkResidency {
    /// True when every pinned id is either resident or known-empty (present in `chunks`). Empty pin set (gate closed) counts as settled.
    #[must_use]
    pub fn pin_settled(&self) -> bool {
        self.pinned_ids
            .iter()
            .all(|id| self.chunks.contains_key(id))
    }
}
