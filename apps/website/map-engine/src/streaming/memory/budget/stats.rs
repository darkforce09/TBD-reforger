//! Role: stats.
//! Position: `streaming/memory/budget` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::*;

impl Ledger {
    /// The debug-HUD tail: reserved bytes against the budget, and the current satellite floor.
    #[must_use]
    pub fn hud_suffix(&self) -> String {
        if self.held_total() == 0 && self.sat_floor.is_none() {
            return String::new();
        }
        let sat = match (self.sat_floor, self.sat_raised) {
            (Some(f), 0) => format!(" · sat L{f}"),
            (Some(f), n) => format!(" · sat L{f} (+{n})"),
            (None, _) => String::new(),
        };
        format!(
            " · mem {}/{}MB{sat}",
            self.held_total() / MIB,
            self.budget / MIB
        )
    }
}

/// The debug-HUD tail. See [`Ledger::hud_suffix`].
#[must_use]
pub fn hud_suffix() -> String {
    with_ledger(|l| l.hud_suffix())
}

/// Publish.
#[cfg(target_arch = "wasm32")]
pub fn publish() {
    use wasm_bindgen::JsValue;
    let Some(win) = web_sys::window() else {
        return;
    };
    let obj = js_sys::Object::new();
    let set = |o: &js_sys::Object, k: &str, v: f64| {
        let _ = js_sys::Reflect::set(o, &JsValue::from_str(k), &JsValue::from_f64(v));
    };
    with_ledger(|l| {
        set(&obj, "budget", l.budget() as f64);
        set(&obj, "held", l.held_total() as f64);
        set(&obj, "peak", l.total_peak() as f64);
        set(&obj, "heap", heap_bytes() as f64);
        set(
            &obj,
            "satFloor",
            l.satellite_floor().map_or(-1.0, |f| f as f64),
        );
        set(&obj, "satRaised", f64::from(l.satellite_raised()));
        let assets = js_sys::Object::new();
        for a in Asset::ALL {
            let e = l.entry(a);
            let row = js_sys::Object::new();
            set(&row, "held", e.held as f64);
            set(&row, "peak", e.peak as f64);
            set(&row, "growth", e.growth as f64);
            let _ = js_sys::Reflect::set(&assets, &JsValue::from_str(a.name()), &row);
        }
        let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("assets"), &assets);
    });
    let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__t9386"), &obj);
}

/// Publish.
#[cfg(not(target_arch = "wasm32"))]
pub fn publish() {}
