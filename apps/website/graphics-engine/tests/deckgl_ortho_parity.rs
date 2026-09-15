//! Role: deckgl ortho parity.
//! Position: `apps/website/graphics-engine/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use serde_json::Value;
use website_graphics_engine::camera::ortho::state::OrthoCamera;

fn ulp_distance(a: f64, b: f64) -> u64 {
    #[allow(clippy::float_cmp)]
    if a == b {
        return 0;
    }
    if a.is_nan() || b.is_nan() {
        return u64::MAX;
    }
    fn flip(bits: i64) -> i128 {
        let x = i128::from(bits);
        if x < 0 { i128::from(i64::MIN) - x } else { x }
    }
    let fa = flip(a.to_bits() as i64);
    let fb = flip(b.to_bits() as i64);
    (fa - fb).unsigned_abs().try_into().unwrap_or(u64::MAX)
}

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/deckgl_ortho_goldens.json"))
        .expect("golden fixture parses")
}

fn f(v: &Value) -> f64 {
    v.as_f64().expect("number")
}

fn floats(v: &Value) -> Vec<f64> {
    v.as_array().expect("array").iter().map(f).collect()
}

struct Case {
    id: String,
    width: f64,
    height: f64,
    zoom: f64,
    target: [f64; 2],
    scale: f64,
}

fn parse_case(v: &Value) -> Case {
    let t = floats(&v["target"]);
    Case {
        id: v["id"].as_str().expect("id").to_owned(),
        width: f(&v["width"]),
        height: f(&v["height"]),
        zoom: f(&v["zoom"]),
        target: [t[0], t[1]],
        scale: f(&v["scale"]),
    }
}

fn camera_own_scale(c: &Case) -> OrthoCamera {
    OrthoCamera::new(c.width, c.height, c.target[0], c.target[1], c.zoom)
}

fn camera_injected_scale(c: &Case) -> OrthoCamera {
    OrthoCamera::with_scale_for_test(c.width, c.height, c.target[0], c.target[1], c.zoom, c.scale)
}

fn assert_ulp_slice(got: &[f64], expected: &[f64], max_ulp: u64, what: &str, id: &str) {
    assert_eq!(got.len(), expected.len(), "{id}: {what} length");
    for (i, (g, e)) in got.iter().zip(expected).enumerate() {
        let d = ulp_distance(*g, *e);
        assert!(
            d <= max_ulp,
            "{id}: {what}[{i}] ULP {d} > {max_ulp} (got {g:e}, expected {e:e})"
        );
    }
}

fn check_case(v: &Value, cam: &OrthoCamera, matrix_ulp: u64, projection_ulp: u64, id: &str) {
    if let Some(vm) = v.get("viewMatrix") {
        assert_ulp_slice(
            &cam.view_matrix(),
            &floats(vm),
            matrix_ulp,
            "viewMatrix",
            id,
        );
        assert_ulp_slice(
            &cam.projection_matrix(),
            &floats(&v["projectionMatrix"]),
            matrix_ulp,
            "projectionMatrix",
            id,
        );
        assert_ulp_slice(
            &cam.view_projection(),
            &floats(&v["viewProjectionMatrix"]),
            matrix_ulp,
            "viewProjectionMatrix",
            id,
        );
        assert_ulp_slice(
            &cam.pixel_projection(),
            &floats(&v["pixelProjectionMatrix"]),
            matrix_ulp,
            "pixelProjectionMatrix",
            id,
        );
        assert_ulp_slice(
            &cam.pixel_unprojection().expect("invertible"),
            &floats(&v["pixelUnprojectionMatrix"]),
            matrix_ulp,
            "pixelUnprojectionMatrix",
            id,
        );
    }

    for (i, probe) in v["probes"].as_array().expect("probes").iter().enumerate() {
        let world = floats(&probe["world"]);
        let projected = cam.project([world[0], world[1], world[2]]);
        assert_ulp_slice(
            &projected,
            &floats(&probe["project"]),
            projection_ulp,
            &format!("probes[{i}].project"),
            id,
        );
        let rt = cam.unproject_xy(projected[0], projected[1]);
        assert_ulp_slice(
            &rt,
            &floats(&probe["roundTrip"]),
            projection_ulp,
            &format!("probes[{i}].roundTrip"),
            id,
        );
    }

    for (i, u) in v["unprojects"]
        .as_array()
        .expect("unprojects")
        .iter()
        .enumerate()
    {
        let pixel = floats(&u["pixel"]);
        let world = cam.unproject_xy(pixel[0], pixel[1]);
        assert_ulp_slice(
            &world,
            &floats(&u["world"]),
            projection_ulp,
            &format!("unprojects[{i}]"),
            id,
        );
    }

    assert_ulp_slice(
        &cam.visible_world_rect(),
        &floats(&v["bounds"]),
        projection_ulp,
        "bounds",
        id,
    );
}

#[test]
fn t1_integer_zoom_cases_bit_exact() {
    let fx = fixture();
    let mut checked = 0;
    for v in fx["cases"].as_array().expect("cases") {
        let c = parse_case(v);
        if c.zoom.fract() != 0.0 {
            continue;
        }
        check_case(v, &camera_own_scale(&c), 0, 0, &c.id);
        checked += 1;
    }
    assert_eq!(checked, 180, "6 integer zooms × 6 sizes × 5 targets");
}

#[test]
fn t2_scale_drift_at_most_one_ulp() {
    let fx = fixture();
    for v in fx["cases"].as_array().expect("cases") {
        let c = parse_case(v);
        let d = ulp_distance(c.zoom.exp2(), c.scale);
        assert!(d <= 1, "{}: scale ULP {d} > 1", c.id);
    }
}

#[test]
fn t3_scale_injected_pipeline_bit_exact_all_cases() {
    let fx = fixture();
    let cases = fx["cases"].as_array().expect("cases");
    assert_eq!(cases.len(), 300);
    for v in cases {
        let c = parse_case(v);
        check_case(v, &camera_injected_scale(&c), 0, 0, &c.id);
    }
}

#[test]
fn t4_end_to_end_bound_all_cases() {
    let fx = fixture();
    for v in fx["cases"].as_array().expect("cases") {
        let c = parse_case(v);
        check_case(v, &camera_own_scale(&c), 2, 4, &c.id);
    }
}

#[test]
fn closed_form_anchor_case() {
    let cam = OrthoCamera::new(800.0, 600.0, 6400.0, 6400.0, 0.0);

    let v = cam.view_matrix();
    let expected_v: [f64; 16] = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -6400.0, -6400.0, -1.0, 1.0,
    ];
    assert_eq!(v, expected_v);

    let p = cam.projection_matrix();
    let lr = 1.0 / (-400.0 - 400.0);
    let bt = 1.0 / (-300.0 - 300.0);
    let nf = 1.0 / (0.1 - 1000.0);
    assert_eq!(p[0], -2.0 * lr);
    assert_eq!(p[5], -2.0 * bt);
    assert_eq!(p[10], 2.0 * nf);
    assert_eq!(p[12], 0.0);
    assert_eq!(p[13], 0.0);
    assert_eq!(p[14], (1000.0 + 0.1) * nf);
    assert_eq!(p[15], 1.0);

    assert_eq!(cam.view_projection()[12], -16.0);

    let center = cam.project([6400.0, 6400.0, 0.0]);
    assert_eq!(center[0], 400.0);
    assert!((center[1] - 300.0).abs() <= 1e-9);

    let east = cam.project([6500.0, 6400.0, 0.0]);
    assert!((east[0] - 500.0).abs() <= 1e-9);
    assert!((east[1] - 300.0).abs() <= 1e-9);

    let north = cam.project([6400.0, 6500.0, 0.0]);
    assert!((north[0] - 400.0).abs() <= 1e-9);
    assert!((north[1] - 200.0).abs() <= 1e-9);
}
