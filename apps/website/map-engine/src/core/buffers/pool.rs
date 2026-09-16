//! Role: pool.
//! Position: `core/buffers` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use std::collections::HashMap;

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
    /// New.
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
#[path = "tests/pool_tests.rs"]
mod tests;
