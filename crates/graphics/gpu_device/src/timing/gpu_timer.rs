//! The timestamp-query timer of one render pass.
//!
//! **Role:** [`crate::timing::gpu_timer::GpuTimer`] holds a two-entry timestamp query set, the
//! buffer the pair resolves into, the mappable buffer it is copied to, the queue's timestamp
//! period and the last measured pass time; [`crate::timing::gpu_timer::GpuTimer::kick_readback`]
//! maps the copy and stores the elapsed milliseconds.
//! **Position:** a renderer creates it after a [`crate::context::gpu_context::GpuContext`] with
//! timestamp queries enabled, writes the pass's begin and end timestamps into
//! [`crate::timing::gpu_timer::GpuTimer::query_set`], and hands the query set,
//! [`crate::timing::gpu_timer::GpuTimer::resolve_buffer`] and
//! [`crate::timing::gpu_timer::GpuTimer::read_buffer`] to `gpu_frame`'s `frame::present::submit`,
//! which resolves and copies them before the submit; it writes timestamps only while
//! [`crate::timing::gpu_timer::GpuTimer::readback_in_flight`] is false, and its statistics read
//! [`crate::timing::gpu_timer::GpuTimer::last_sample_ms`].
//! **Signals & state:** the last pass time and the readback lane, shared with the `map_async`
//! callback; every field is private.
//! **Invariants:** at most one readback is in flight ([`crate::buffers::readback::ReadbackLane`]):
//! `kick_readback` returns without mapping while one is, and the renderer writes timestamps only
//! when none is, so the read buffer is never mapped twice; the pass time is (end − start) × period
//! / 10⁶, and there is no sample after a failed mapping.

use crate::buffers::readback::ReadbackLane;
use std::cell::Cell;
use std::rc::Rc;

/// The timestamp queries of one render pass and the last pass time read back from them.
pub struct GpuTimer {
    /// The two-entry timestamp query set: index 0 at the pass's beginning, 1 at its end.
    query_set: wgpu::QuerySet,

    /// The 16-byte `QUERY_RESOLVE | COPY_SRC` buffer the query pair resolves into.
    resolve_buf: wgpu::Buffer,

    /// The 16-byte `MAP_READ | COPY_DST` buffer the resolved pair is copied to and read from.
    read_buf: wgpu::Buffer,

    /// Nanoseconds per timestamp tick, from the queue.
    period_ns: f32,

    /// The last measured pass time in milliseconds.
    last_ms: Rc<Cell<f64>>,

    /// The one-mapping-at-a-time guard of `read_buf`, and whether `last_ms` is fresh.
    lane: Rc<ReadbackLane>,
}

impl GpuTimer {
    /// Create the query set and both buffers on `device`, with the timestamp period of `queue`.
    #[must_use]
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("frame-timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count: 2,
        });
        let resolve_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("timestamp-resolve"),
            size: 16,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let read_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("timestamp-read"),
            size: 16,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            query_set,
            resolve_buf,
            read_buf,
            period_ns: queue.get_timestamp_period(),
            last_ms: Rc::new(Cell::new(0.0)),
            lane: Rc::new(ReadbackLane::new()),
        }
    }
}

impl GpuTimer {
    /// The two-entry timestamp query set the render pass writes its begin and end timestamps
    /// into.
    #[must_use]
    pub fn query_set(&self) -> &wgpu::QuerySet {
        &self.query_set
    }

    /// The 16-byte buffer the query pair resolves into.
    #[must_use]
    pub fn resolve_buffer(&self) -> &wgpu::Buffer {
        &self.resolve_buf
    }

    /// The 16-byte mappable buffer the resolved pair is copied to and read from.
    #[must_use]
    pub fn read_buffer(&self) -> &wgpu::Buffer {
        &self.read_buf
    }

    /// Whether a readback is in flight; the renderer writes no timestamps while one is.
    #[must_use]
    pub fn readback_in_flight(&self) -> bool {
        self.lane.in_flight()
    }

    /// The last measured pass time in milliseconds, or `None` before the first readback lands or
    /// after a failed one.
    #[must_use]
    pub fn last_sample_ms(&self) -> Option<f64> {
        self.lane.has_sample().then(|| self.last_ms.get())
    }
}

impl GpuTimer {
    /// Map `read_buf` and store the pass time in `last_ms` when the mapping lands; returns at
    /// once while an earlier readback is still in flight.
    pub fn kick_readback(&self) {
        if !self.lane.begin() {
            return;
        }
        let buf = self.read_buf.clone();
        let last = self.last_ms.clone();
        let lane = self.lane.clone();
        let period = f64::from(self.period_ns);
        self.read_buf
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |res| {
                if lane.settle(res.is_ok()) {
                    {
                        let data = buf.slice(..).get_mapped_range();
                        let t0 = u64::from_le_bytes(data[0..8].try_into().expect("8 bytes"));
                        let t1 = u64::from_le_bytes(data[8..16].try_into().expect("8 bytes"));
                        last.set(t1.saturating_sub(t0) as f64 * period / 1.0e6);
                        lane.record_sample();
                    }
                    buf.unmap();
                }
            });
    }
}
