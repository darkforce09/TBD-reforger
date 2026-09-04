//! T-090.12.5 — the LOS tool's object layer is wired: the viewshed click starts the object wash,
//! the rAF loop steps it, the tool-switch / Esc path cancels it, and the overlay attaches the
//! object verdict through the live occluder seam. The t644 `editor_live` scrub idiom, with the
//! viewport loop and the overlay source scrubbed in as well.

use crate::editor::arsenal::class_r_scrub::live_code;

fn editor_live() -> String {
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    let raw = include_str!("../mission_editor.rs");
    assert_eq!(
        raw.matches(anchor.as_str()).count(),
        1,
        "scrub anchor must be unambiguous"
    );
    let mut src = live_code(&raw[raw.find(anchor.as_str()).expect("counted above")..]);
    src.push_str(&live_code(include_str!("../canvas/gestures.rs")));
    src.push_str(&live_code(include_str!("../canvas/viewport.rs")));
    src
}

fn overlay_live() -> String {
    live_code(include_str!("../tools/los_tool.rs"))
}

fn adapter_live() -> String {
    live_code(include_str!("../tools/los_world_wasm.rs"))
}

/// (viewshed click → object wash) The same `is_viewshed()` commit arm that places the observer
/// and uploads the terrain wash starts the object wash right after `place_viewshed`.
#[test]
fn viewshed_click_starts_the_object_wash_after_the_terrain_upload() {
    let ed = editor_live();
    let place = ed
        .find("place_viewshed(")
        .expect("T-644 viewshed placement");
    let start = ed
        .find("los_world_wasm::start_object_wash()")
        .expect("T-090.12.5: the object wash must start from the viewshed click");
    assert!(
        start > place,
        "the wash starts after the terrain raster is placed"
    );
}

/// (rAF steps the wash) The viewport loop ticks the wash every frame, inside the engine borrow
/// so the merged raster re-upload has the engine.
#[test]
fn raf_loop_ticks_the_object_wash_and_shows_its_progress() {
    let ed = editor_live();
    assert!(
        ed.contains("los_world_wasm::tick_object_wash(e)"),
        "T-090.12.5: the rAF loop must step the object wash"
    );
    assert!(
        ed.contains("los_world_wasm::hud_suffix()"),
        "T-090.12.5: the debug HUD must show the occluder residency + wash progress"
    );
}

/// (clear path) Leaving LoS-viewshed drops the wash with the lane — the same Effect that calls
/// `viewshed_clear` cancels the wash.
#[test]
fn leaving_the_viewshed_cancels_the_object_wash_with_the_lane() {
    let ed = editor_live();
    let clear = ed.find("e.viewshed_clear()").expect("T-644 lane clear");
    let cancel = ed
        .find("los_world_wasm::cancel_object_wash()")
        .expect("T-090.12.5: the wash must be cancelled with the lane");
    assert!(
        cancel > clear && cancel - clear < 400,
        "cancel sits beside viewshed_clear"
    );
}

/// (overlay attaches the object verdict) The shot projection applies the object layer through
/// the wasm adapter, the header reads the combined verdict, and the styling follows the pair.
#[test]
fn overlay_applies_the_object_verdict_and_formats_the_combined_header() {
    let ov = overlay_live();
    assert!(
        ov.contains("los_world::apply_objects("),
        "T-090.12.5: apply_objects on the projected shot"
    );
    assert!(
        ov.contains("los_world_wasm::object_verdict(&shot)"),
        "T-090.12.5: the verdict comes from the live occluder"
    );
    assert!(
        ov.contains("los_world::format_combined("),
        "T-090.12.5: the header is the combined verdict"
    );
    assert!(
        ov.matches("los_world::styling_of(&shot)").count() >= 3,
        "T-090.12.5: line, dot and header classes follow the pair"
    );
    assert!(
        ov.contains("objects: super::los_world::ObjectVerdict::NotLoaded"),
        "T-090.12.5: a shot starts NotLoaded (never fake clear)"
    );
}

/// (honest seam) Every occluder read goes through `with_occluder`, which is `None` while the
/// host is taken — the adapter maps that to `NotLoaded` / a waiting pass, never to clear.
#[test]
fn adapter_reaches_the_occluder_only_through_with_occluder_and_never_fakes_clear() {
    let ad = adapter_live();
    assert!(
        ad.matches("with_occluder(").count() >= 3,
        "object_verdict, cell_test and hud_suffix all go through the seam"
    );
    assert!(
        ad.contains(".unwrap_or(ObjectVerdict::NotLoaded)"),
        "an unreachable occluder is NotLoaded"
    );
    assert!(
        ad.contains("WorldVerdict::Provisional => ObjectCell::Provisional"),
        "a proxy-decided cell is provisional, not hidden or clear"
    );
    assert!(
        !ad.contains("unwrap_or(ObjectVerdict::Clear"),
        "never default to clear"
    );
}

#[test]
fn engine_mount_registers_the_object_wash_bridge_beside_the_camera_hook() {
    let ed = editor_live();
    let cam = ed
        .find("register_editor_cam(engine.clone(), map_host.clone());")
        .expect("the camera hook registration is the anchor");
    let hook = ed
        .find("los_world_wasm::register_object_wash_hook();")
        .expect("the wash bridge is registered at mount");
    assert!(
        hook > cam && hook - cam < 400,
        "the wash bridge registers right after the camera hook (same mount arm)"
    );
}
