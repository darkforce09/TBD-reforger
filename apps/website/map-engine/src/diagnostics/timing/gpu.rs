//! Role: gpu.
//! Position: `diagnostics/timing` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::frame::buffers::readback::ReadbackLane;
use std::cell::Cell;
use std::rc::Rc;

/// Now ms.
pub(crate) fn now_ms() -> f64 {
    js_sys::Date::now()
}

/// Perf now ms.
pub(crate) fn perf_now_ms() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map_or_else(now_ms, |p| p.now())
}

/// Gpu timer.
pub(crate) struct GpuTimer {
    /// Query set.
    pub(crate) query_set: wgpu::QuerySet,

    /// Resolve buf.
    pub(crate) resolve_buf: wgpu::Buffer,

    /// Read buf.
    pub(crate) read_buf: wgpu::Buffer,

    /// Period ns.
    pub(crate) period_ns: f32,

    /// Last ms.
    pub(crate) last_ms: Rc<Cell<f64>>,

    /// Lane.
    pub(crate) lane: Rc<ReadbackLane>,
}

impl GpuTimer {
    /// New.
    pub(crate) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
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
    /// Kick readback.
    pub(crate) fn kick_readback(&self) {
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
