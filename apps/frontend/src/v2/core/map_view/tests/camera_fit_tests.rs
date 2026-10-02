//! Terrain bounds from the manifest and the camera view that fits them.

use super::{fit_view, WorldBounds};
use camera_math::ortho::state::{MAX_ZOOM, MIN_ZOOM};

fn everon() -> WorldBounds {
    WorldBounds::new(0.0, 0.0, 12_800.0, 12_800.0).expect("valid bounds")
}

#[test]
fn reads_world_bounds_from_a_manifest_document() {
    let doc = br#"{"terrainId":"everon","worldBounds":[0,0,12800,12800],"dem":{}}"#;
    assert_eq!(WorldBounds::from_manifest_json(doc), Some(everon()));
}

#[test]
fn reads_the_committed_everon_manifest() {
    let doc = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/terrains/everon/manifest.json"
    ));
    assert_eq!(WorldBounds::from_manifest_json(doc), Some(everon()));
}

#[test]
fn refuses_malformed_or_empty_bounds() {
    assert_eq!(WorldBounds::from_manifest_json(b"not json"), None);
    assert_eq!(
        WorldBounds::from_manifest_json(br#"{"worldBounds":[0,0,1]}"#),
        None
    );
    assert_eq!(
        WorldBounds::from_manifest_json(br#"{"worldBounds":[5,0,5,10]}"#),
        None
    );
    assert_eq!(WorldBounds::new(0.0, 0.0, f64::NAN, 1.0), None);
    assert_eq!(WorldBounds::new(0.0, 3.0, 1.0, 2.0), None);
}

#[test]
fn fits_the_whole_terrain_centred_in_the_container() {
    let view = fit_view(everon(), 800.0, 400.0);
    assert_eq!((view.target_x, view.target_y), (6_400.0, 6_400.0));
    // The limiting axis is the 400 px height: 400 / 12800 = 2^-5 pixels per metre.
    assert!((view.zoom - (-5.0)).abs() < 1e-12, "{}", view.zoom);
    let offset = WorldBounds::new(1_000.0, 2_000.0, 3_000.0, 3_000.0).expect("valid bounds");
    let view = fit_view(offset, 500.0, 500.0);
    assert_eq!((view.target_x, view.target_y), (2_000.0, 2_500.0));
    assert!((view.zoom - (0.25f64).log2()).abs() < 1e-12);
}

#[test]
fn keeps_the_fitted_zoom_inside_the_camera_band() {
    assert_eq!(fit_view(everon(), 1.0, 1.0).zoom, MIN_ZOOM);
    let tiny = WorldBounds::new(0.0, 0.0, 1.0, 1.0).expect("valid bounds");
    assert_eq!(fit_view(tiny, 4_000.0, 4_000.0).zoom, MAX_ZOOM);
    assert_eq!(fit_view(everon(), 0.0, 300.0).zoom, MIN_ZOOM);
}
