//! Role: metrics.
//! Position: `streaming/loaders/world_loader` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::JsValue;
use super::VecDeque;

/// Crossing sample.
#[derive(Clone, Copy, Debug)]
pub(super) struct CrossingSample {
    /// Staging.
    pub(super) staging: u32,

    /// Fetch batch.
    pub(super) fetch_batch: u32,

    /// Packed clones.
    pub(super) packed_clones: u32,
}

/// Crossing alloc probe.
pub(super) struct CrossingAllocProbe {
    /// Pass staging.
    pub(super) pass_staging: u32,

    /// Pass fetch batch.
    pub(super) pass_fetch_batch: u32,

    /// Pass clones.
    pub(super) pass_clones: u32,

    /// Total warm.
    pub(super) total_warm: u32,

    /// Warm.
    pub(super) warm: VecDeque<CrossingSample>,
}

impl CrossingAllocProbe {
    /// New.
    pub(super) fn new() -> Self {
        Self {
            pass_staging: 0,
            pass_fetch_batch: 0,
            pass_clones: 0,
            total_warm: 0,
            warm: VecDeque::new(),
        }
    }
}

impl CrossingAllocProbe {
    /// Reset pass.
    pub(super) fn reset_pass(&mut self) {
        self.pass_staging = 0;
        self.pass_fetch_batch = 0;
        self.pass_clones = 0;
    }
}

impl CrossingAllocProbe {
    /// Commit warm.
    pub(super) fn commit_warm(&mut self) {
        let sample = CrossingSample {
            staging: self.pass_staging,
            fetch_batch: self.pass_fetch_batch,
            packed_clones: self.pass_clones,
        };
        self.total_warm += 1;
        self.warm.push_back(sample);
        if self.warm.len() > 8 {
            self.warm.pop_front();
        }
        publish_crossing_probe(self.total_warm, self.warm.make_contiguous());
        if chunk_alloc_log_enabled() {
            web_sys::console::log_1(
                &format!(
                    "t9382 crossing #{} staging={} fetch_batch={} packed_clones={}",
                    self.total_warm, sample.staging, sample.fetch_batch, sample.packed_clones
                )
                .into(),
            );
        }
    }
}

/// Chunk alloc log enabled.
pub(super) fn chunk_alloc_log_enabled() -> bool {
    let search = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .unwrap_or_default();
    if search.contains("t9382=1") || search.contains("t9382=true") {
        return true;
    }
    web_sys::window()
        .and_then(|w| js_sys::Reflect::get(&w, &JsValue::from_str("__t9382Log")).ok())
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

/// Publish crossing probe.
pub(super) fn publish_crossing_probe(total_warm: u32, warm: &[CrossingSample]) {
    let Some(win) = web_sys::window() else {
        return;
    };
    let obj = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("count"),
        &JsValue::from_f64(f64::from(total_warm)),
    );
    let samples = js_sys::Array::new();
    let mut max_staging = 0u32;
    for s in warm {
        max_staging = max_staging.max(s.staging);
        let row = js_sys::Object::new();
        let _ = js_sys::Reflect::set(
            &row,
            &JsValue::from_str("staging"),
            &JsValue::from_f64(f64::from(s.staging)),
        );
        let _ = js_sys::Reflect::set(
            &row,
            &JsValue::from_str("fetch_batch"),
            &JsValue::from_f64(f64::from(s.fetch_batch)),
        );
        let _ = js_sys::Reflect::set(
            &row,
            &JsValue::from_str("packed_clones"),
            &JsValue::from_f64(f64::from(s.packed_clones)),
        );
        samples.push(&row);
    }
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("samples"), &samples);
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("max_staging"),
        &JsValue::from_f64(f64::from(max_staging)),
    );
    let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__t9382"), &obj);
}
