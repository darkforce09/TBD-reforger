//! Exact re-implementations of the JS numeric primitives whose semantics differ from Rust's, so
//! ports stay bit-identical (Class R).

/// `Math.round` — round half **up** (toward +∞), i.e. `floor(x + 0.5)`. Rust's `f64::round` rounds
/// half **away from zero**, which differs for negative half-integers (`Math.round(-2.5) === -2` but
/// `(-2.5f64).round() == -3.0`). Every port that mirrors a JS `Math.round` uses this.
#[inline]
#[must_use]
pub(crate) fn round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// ═══ T-582 — the zone draw tool's round trip, proved through the REAL save/reload path ═══
///
/// **Why this proof lives in this file.** T-582's brief assigns it `js.rs`, on the premise that this
/// is where "wasm bindings for T-211's eleven mutators" belong. That premise is obsolete and the
/// report says so: this crate declares "No `wasm-bindgen` / `web-sys` here" at the top of `lib.rs`,
/// the `map-engine-wasm` shim it names was deleted at T-418, and the Leptos SPA now links
/// `map-engine-core` **directly** with `features = ["doc"]` (Cargo.toml) and calls
/// `MissionDocCore` as plain Rust. There is no JS boundary left to bind, so no binding was written.
///
/// What the file assignment *does* still buy is the one thing the frontend crate cannot give: this
/// module is compiled with the `doc` feature under
/// `cargo test -p map-engine-core --features doc,mission`, so a test here can drive the real
/// document. The SPA's own `editor_ops.rs` is `#![cfg(target_arch = "wasm32")]` and its
/// `map-engine-core/doc` dependency is wasm32-only, so **no test in the frontend crate can touch a
/// `MissionDocCore` at all**. This is the only file in T-582's scope where the round trip executes.
///
/// **Why a round trip and not a render assertion.** A test that asserted "the tool renders a zone"
/// would pass over a zone whose shape was dropped on the way to the wire — which is exactly how
/// T-211 caught its own bug. So this drives the whole path the author's click actually takes:
///
/// ```text
///   mutators  →  small_maps_json()  →  compile_payload()  →  hydrate()  →  zones_json()
///   (the tool)   (what save reads)     (what is POSTed)     (reload)      (what renders)
/// ```
///
/// …and asserts the geometry survives **whole** — every vertex, every field — not merely that a
/// zone with the right id came back.
///
/// **The landmine this walks past.** `zones` is in `store.rs`'s `is_known_editor_payload_top_level`
/// but deliberately NOT in `compile.rs`'s `KNOWN_EDITOR_PAYLOAD_TOP_LEVEL_KEYS`; the round trip
/// works today through a projection into `payloadExtras.zones` that `compile_payload` promotes to
/// the payload root. Adding `"zones"` to that key list ALONE silently drops every authored zone.
/// This slice touched neither file. The test below is the tripwire from the tool's side: it goes red
/// the moment that route stops carrying geometry, whatever the cause.
#[cfg(all(test, feature = "doc", feature = "mission"))]
mod zone_round_trip {
    use crate::doc::MissionDocCore;
    use crate::mission::compile::compile_payload;
    use serde_json::Value;

    /// The layer id `editor_ops::DEFAULT_LAYER_ID` mints; hydrate needs one and zones do not use it.
    const DEFAULT_LAYER: &str = "layer-1";

    /// Author the two shapes the draw tool can produce, plus the identity and rules the Attributes
    /// panel writes. Values are deliberately awkward: a polygon that is not axis-aligned, a radius
    /// and centre with a decimal that survives the 0.1 m grid, and a `rules` key from the closed
    /// T-241 vocabulary.
    fn authored() -> MissionDocCore {
        let core = MissionDocCore::new();
        core.add_circle_zone("z1", "boundary", 1234.5, 6789.2, 250.7);
        core.set_zone_label("z1", Some("Area of Operations"));
        core.set_zone_faction("z1", Some("blufor"));
        core.set_zone_rules("z1", Some(r#"{"graceSeconds":45,"penalty":"kill"}"#));

        core.add_polygon_zone(
            "z2",
            "objective_capture",
            &[100.0, 200.0, 340.5, 210.25, 275.0, 480.75, 120.0, 460.0],
        );
        core.set_zone_label("z2", Some("Hilltop"));
        core.set_zone_rules("z2", Some(r#"{"captureSeconds":180,"contestable":false}"#));
        core
    }

    /// Save exactly as the editor does, then reload into a FRESH document exactly as a page load
    /// does. Returns the reloaded document's `zones` map.
    fn save_then_reload(core: &MissionDocCore) -> Value {
        let payload = compile_payload(&core.small_maps_json(), &core.slots_json(), false);
        let fresh = MissionDocCore::new();
        fresh.hydrate(&payload.to_string(), DEFAULT_LAYER);
        serde_json::from_str(&fresh.zones_json()).expect("zones_json is JSON")
    }

    /// THE test. A zone drawn by the tool must come back from a save+reload byte-for-byte intact.
    #[test]
    fn zone_geometry_survives_save_and_reload() {
        let core = authored();
        let before: Value = serde_json::from_str(&core.zones_json()).expect("zones_json is JSON");
        let after = save_then_reload(&core);

        // Non-vacuity first: if the tool authored nothing, every assertion below is trivially true.
        assert_eq!(
            core.zone_count(),
            2,
            "the tool must have authored two zones"
        );
        assert!(
            before.get("z1").is_some() && before.get("z2").is_some(),
            "precondition: both zones exist before the save"
        );

        // ── The circle, field by field ────────────────────────────────────────────────────────
        let c = after
            .get("z1")
            .expect("the circle zone must survive the round trip");
        assert_eq!(c["type"], "boundary");
        assert_eq!(c["label"], "Area of Operations");
        assert_eq!(c["faction"], "blufor");
        let circle = &c["shape"]["circle"];
        assert_eq!(
            circle["x"].as_f64(),
            Some(1234.5),
            "centre x must survive whole"
        );
        assert_eq!(circle["z"].as_f64(), Some(6789.2), "centre z, not y");
        assert_eq!(
            circle["r"].as_f64(),
            Some(250.7),
            "the radius is the geometry — a dropped r is the T-211 bug class"
        );
        assert!(
            c["shape"].get("polygon").is_none(),
            "$defs/shape is a oneOf: a circle row carrying a polygon key is schema-INVALID"
        );
        // `rules` is opaque all the way through — the tool never re-spells these names.
        assert_eq!(c["rules"]["graceSeconds"].as_f64(), Some(45.0));
        assert_eq!(c["rules"]["penalty"], "kill");

        // ── The polygon, vertex by vertex ─────────────────────────────────────────────────────
        let p = after
            .get("z2")
            .expect("the polygon zone must survive the round trip");
        assert_eq!(p["type"], "objective_capture");
        assert_eq!(p["label"], "Hilltop");
        let ring = p["shape"]["polygon"]
            .as_array()
            .expect("the ring must survive as an array");
        assert_eq!(ring.len(), 4, "every vertex, not just the first");
        let flat: Vec<f64> = ring
            .iter()
            .flat_map(|v| v.as_array().expect("a [x,z] pair").iter())
            .map(|n| n.as_f64().expect("numeric vertex"))
            .collect();
        assert_eq!(
            flat,
            vec![100.0, 200.0, 340.5, 210.25, 275.0, 480.75, 120.0, 460.0],
            "the ring must come back in order and unrounded — a reordered or truncated ring is a \
             different play area"
        );
        assert!(
            p["shape"].get("circle").is_none(),
            "$defs/shape oneOf, from the other side"
        );
        assert_eq!(p["rules"]["captureSeconds"].as_f64(), Some(180.0));
        assert_eq!(p["rules"]["contestable"], false);

        // ── And the whole thing, so a field added later cannot slip through unchecked ──────────
        assert_eq!(
            before, after,
            "the reloaded zones map must equal the authored one exactly"
        );
    }

    /// The wire route itself. The round trip above would still pass if `zones` reached the payload
    /// by some other means, so this pins WHERE it travels: `small_maps_json` projects into
    /// `payloadExtras.zones`, and `compile_payload` promotes that to the payload root, which is
    /// where `flatten.rs`'s `EditorPayload.zones` reads it.
    ///
    /// This is the assertion that goes red if someone "tidies" `KNOWN_EDITOR_PAYLOAD_TOP_LEVEL_KEYS`
    /// by adding `"zones"` without also teaching `compile_payload` to emit it: the promotion is
    /// skipped, the root array vanishes, and the zone never reaches the mod.
    #[test]
    fn zones_reach_the_payload_root_through_the_projection() {
        let core = authored();
        let small: Value = serde_json::from_str(&core.small_maps_json()).expect("JSON");

        // The canonical by-id emit, shaped like `entitiesById`.
        assert!(
            small["zonesById"]["z1"].is_object(),
            "small_maps_json must carry the canonical by-id map"
        );
        // The transitional side-channel that actually closes the trip today.
        let parked = small["payloadExtras"]["zones"]
            .as_array()
            .expect("the projection must park an ordered array");
        assert_eq!(parked.len(), 2);

        let payload = compile_payload(&core.small_maps_json(), &core.slots_json(), false);
        let root = payload["zones"]
            .as_array()
            .expect("compile_payload must promote payloadExtras.zones to the payload ROOT");
        assert_eq!(
            root.len(),
            2,
            "both zones must reach the root array flatten reads"
        );
        assert_eq!(
            root, parked,
            "promotion must not reshape or reorder the rows"
        );
    }

    /// A document that authored zones and then deleted them all must not re-emit a stale parked
    /// array — absence has to be expressible, or a deleted play area would come back on reload.
    #[test]
    fn deleting_every_zone_survives_the_round_trip_too() {
        let core = authored();
        core.remove_zone("z1");
        core.remove_zone("z2");
        assert_eq!(core.zone_count(), 0);

        let after = save_then_reload(&core);
        assert_eq!(
            after.as_object().map(serde_json::Map::len),
            Some(0),
            "a cleared play area must stay cleared across a save and reload"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::round;

    #[test]
    fn matches_js_math_round() {
        assert_eq!(round(2.5), 3.0);
        assert_eq!(round(-2.5), -2.0); // JS: -2 (Rust f64::round would give -3)
        assert_eq!(round(0.5), 1.0);
        assert_eq!(round(-0.5), 0.0); // JS: -0
        assert_eq!(round(2.4), 2.0);
        assert_eq!(round(2.6), 3.0);
    }
}
