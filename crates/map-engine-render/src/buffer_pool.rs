//! T-938.1 — persistent GPU instance buffers for slot/cluster icon lanes.
//!
//! Pre-pool, `upload_slot_role_lane` copied bytes and called `create_buffer_init` on every
//! upload. This module keeps one VERTEX|COPY_DST buffer per lane id, writes with
//! `queue.write_buffer`, doubles capacity on demand, and never shrinks.
//!
//! Native tests exercise the allocation policy against a CPU shadow (wgpu is wasm32-only).

use std::collections::HashMap;

/// wgpu `COPY_BUFFER_ALIGNMENT` — `write_buffer` size/offset must be a multiple of 4.
const ALIGN: usize = 4;

/// Result of a host-side pooled write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WriteOutcome {
    /// A new buffer object was allocated (first write or growth).
    pub created: bool,
    /// Logical pooled capacity in bytes (never shrinks).
    pub capacity: usize,
}

struct PooledLane {
    capacity: usize,
    contents: Vec<u8>,
    #[cfg(target_arch = "wasm32")]
    gpu: Option<wgpu::Buffer>,
}

/// Persistent instance buffers keyed by lane id (`LaneRole as u32`).
#[derive(Default)]
pub struct LanePool {
    slots: HashMap<u32, PooledLane>,
    creates: u32,
}

impl LanePool {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// How many buffer objects this pool has created (tests + HUD).
    #[must_use]
    pub fn creates(&self) -> u32 {
        self.creates
    }

    /// Logical capacity for `lane`, or 0 if the lane has never been written.
    #[must_use]
    pub fn capacity(&self, lane: u32) -> usize {
        self.slots.get(&lane).map_or(0, |s| s.capacity)
    }

    /// Last written bytes for `lane` (exact payload, not the unused tail).
    #[must_use]
    pub fn contents(&self, lane: u32) -> &[u8] {
        self.slots.get(&lane).map_or(&[], |s| s.contents.as_slice())
    }

    /// Drop every pooled buffer. Next write allocates again.
    pub fn clear(&mut self) {
        self.slots.clear();
    }

    /// Host write used by unit tests and as the growth policy for the GPU path.
    pub fn write(&mut self, lane: u32, bytes: &[u8]) -> WriteOutcome {
        self.write_inner(lane, bytes, None)
    }

    fn write_inner(
        &mut self,
        lane: u32,
        bytes: &[u8],
        convert: Option<fn(&mut [u8])>,
    ) -> WriteOutcome {
        let current = self.slots.get(&lane).map_or(0, |s| s.capacity);
        let new_cap = grow_capacity(current, bytes.len());
        let created = current == 0 || new_cap > current;
        if created {
            self.creates += 1;
        }
        let slot = self.slots.entry(lane).or_insert_with(|| PooledLane {
            capacity: new_cap,
            contents: Vec::with_capacity(new_cap),
            #[cfg(target_arch = "wasm32")]
            gpu: None,
        });
        if new_cap > slot.capacity {
            slot.capacity = new_cap;
            #[cfg(target_arch = "wasm32")]
            {
                slot.gpu = None;
            }
        }
        slot.contents.clear();
        slot.contents.extend_from_slice(bytes);
        if let Some(convert) = convert {
            convert(&mut slot.contents);
        }
        WriteOutcome {
            created,
            capacity: slot.capacity,
        }
    }

    /// Upload `bytes` into a persistent VERTEX|COPY_DST buffer, converting in place first.
    ///
    /// Returns a cloned wgpu buffer handle plus whether the GPU object is new (bind groups
    /// that keyed on buffer identity must rebuild only then).
    #[cfg(target_arch = "wasm32")]
    pub fn write_gpu(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        lane: u32,
        bytes: &[u8],
        convert: fn(&mut [u8]),
    ) -> (wgpu::Buffer, bool) {
        let outcome = self.write_inner(lane, bytes, Some(convert));
        let gpu = {
            let slot = self
                .slots
                .get_mut(&lane)
                .expect("write_inner inserts the lane");
            if outcome.created || slot.gpu.is_none() {
                slot.gpu = Some(device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("slot-icon-lane"),
                    size: slot.capacity as u64,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }));
            }
            slot.gpu.as_ref().expect("pooled gpu buffer").clone()
        };
        let contents = self
            .slots
            .get(&lane)
            .expect("write_inner inserts the lane")
            .contents
            .as_slice();
        queue.write_buffer(&gpu, 0, contents);
        (gpu, outcome.created)
    }
}

/// First alloc is `needed` (4-byte aligned). Later growth doubles until it fits. Never shrinks.
#[must_use]
pub fn grow_capacity(current: usize, needed: usize) -> usize {
    let needed = needed.next_multiple_of(ALIGN);
    if needed <= current {
        return current;
    }
    if current == 0 {
        return needed;
    }
    let mut cap = current;
    while cap < needed {
        let next = cap.saturating_mul(2);
        if next == cap {
            return needed.max(cap);
        }
        cap = next;
    }
    cap.max(needed).next_multiple_of(ALIGN)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_size_uploads_allocate_once() {
        let mut pool = LanePool::new();
        let bytes = [7u8; 20];
        pool.write(1, &bytes);
        pool.write(1, &bytes);
        assert_eq!(
            pool.creates(),
            1,
            "two equal-size uploads must create one buffer"
        );
    }

    #[test]
    fn growth_keeps_content() {
        let mut pool = LanePool::new();
        pool.write(1, &[1, 2, 3, 4]);
        assert_eq!(pool.creates(), 1);
        pool.write(1, &[9, 8, 7, 6, 5, 4, 3, 2]);
        assert_eq!(pool.creates(), 2);
        assert!(
            pool.capacity(1) >= 8,
            "grown capacity must cover the larger write"
        );
        assert_eq!(pool.contents(1), &[9, 8, 7, 6, 5, 4, 3, 2]);
    }

    #[test]
    fn smaller_write_reuses_and_never_shrinks() {
        let mut pool = LanePool::new();
        pool.write(1, &[3u8; 64]);
        let cap = pool.capacity(1);
        assert_eq!(pool.creates(), 1);
        pool.write(1, &[1, 2, 3, 4]);
        assert_eq!(pool.creates(), 1, "smaller write must reuse the buffer");
        assert_eq!(pool.capacity(1), cap, "capacity must never shrink");
        assert_eq!(pool.contents(1), &[1, 2, 3, 4]);
    }

    #[test]
    fn grow_capacity_doubles_and_never_shrinks() {
        assert_eq!(grow_capacity(0, 20), 20);
        assert_eq!(grow_capacity(20, 20), 20);
        assert_eq!(grow_capacity(20, 21), 40);
        assert_eq!(grow_capacity(40, 4), 40);
    }
}
