//! Tests for [`super`] — rules 1, 2 and 6 over the `crates/graphics` members.
//!
//! Rules 1 and 2 scan every graphics-category member beside the parked renderer; rule 6 forbids
//! the frontend the wasm-only members and leaves it the CPU-only ones. Each case is pinned red and
//! green against the fixture checkout, and each anti-vacuity path by its exit code.

use super::fixture_repository::*;
use super::*;

/// A wasm-only graphics member: a GPU-side crate the frontend may not reach.
const WASM_ONLY_MEMBER: &str = "gpu_device";
const WASM_ONLY_MANIFEST: &str = "\
[package]
name = \"gpu_device\"

[package.metadata.layout]
category = \"crates/graphics\"
tier = 1
targets = \"wasm32\"
";
const WASM_ONLY_SOURCE: &str = "pub struct GpuDevice;\n";

/// RULE 2, RED, IN A MEMBER — the declaration is reported with the member's path.
#[test]
fn a_map_noun_declared_in_a_graphics_member_fails() {
    let r = Repo::new("member-rule2");
    r.member(
        GRAPHICS_MEMBER,
        "draw/bad.rs",
        "pub struct TerrainTile;\n// a terrain cell is the map engine's word\n",
    );
    let all = r.expect(
        1,
        &[
            "FAIL: map nouns declared inside the pure renderer:",
            "  crates/graphics/render_primitives/src/draw/bad.rs:1:pub struct TerrainTile;",
            RULE2_TAIL[0],
            "1 map-noun declaration(s)",
        ],
    );
    assert!(
        !all.contains("bad.rs:2:"),
        "prose is not a declaration:\n{all}"
    );
}

/// RULE 1, RED, IN A MEMBER — both the source arm and the manifest arm.
#[test]
fn a_graphics_member_importing_the_map_engine_fails() {
    let r = Repo::new("member-rule1");
    r.member(
        GRAPHICS_MEMBER,
        "draw/bad.rs",
        "use map_engine::frame::EngineHandle;\n",
    );
    r.member_manifest(
        GRAPHICS_MEMBER,
        &format!("{GRAPHICS_MEMBER_MANIFEST}me = {{ package = \"map_engine\" }}\n"),
    );
    r.expect(
        1,
        &[
            "FAIL: the graphics layer reaches back into the map engine:",
            "  crates/graphics/render_primitives/src/draw/bad.rs:1:use map_engine::frame::\
             EngineHandle;",
            "  crates/graphics/render_primitives/Cargo.toml:11:me = { package = \"map_engine\" }",
            RULE1_TAIL[0],
            "2 wall breach(es)",
        ],
    );
}

/// RULE 6, RED — a wasm-only member imported by the frontend, by path and by manifest edge; a
/// comment naming it while describing the boundary stays green.
#[test]
fn a_wasm_only_graphics_member_is_off_limits_to_the_frontend() {
    let r = Repo::new("member-rule6");
    r.member_manifest(WASM_ONLY_MEMBER, WASM_ONLY_MANIFEST);
    r.member(WASM_ONLY_MEMBER, "lib.rs", WASM_ONLY_SOURCE);
    r.front(
        "canvas/bad.rs",
        "//! the map engine owns `gpu_device`; this page never depends on it\n\
         use gpu_device::GpuDevice;\n",
    );
    r.front_manifest(&format!(
        "{FRONT_MANIFEST}gpu_device = {{ path = \"../../crates/graphics/gpu_device\" }}\n"
    ));
    let all = r.expect(
        1,
        &[
            "FAIL: the frontend imports the renderer directly:",
            "  apps/frontend/src/canvas/bad.rs:2:use gpu_device::GpuDevice;",
            "  apps/frontend/Cargo.toml:6:gpu_device = { path",
            RULE6_TAIL[0],
            "2 direct-renderer import(s)",
            "across 2 crates/graphics member(s) (1 wasm-only)",
        ],
    );
    assert!(!all.contains("bad.rs:1:"), "prose is not an import:\n{all}");
}

/// RULE 6, GREEN — a CPU-only member is a shared building block the frontend may link.
#[test]
fn a_cpu_only_graphics_member_is_open_to_the_frontend() {
    let r = Repo::new("member-cpu");
    r.front(
        "canvas/mesh.rs",
        "use render_primitives::draw::triangulate::triangulate;\n",
    );
    r.front_manifest(&format!(
        "{FRONT_MANIFEST}render_primitives = {{ path = \"../../crates/graphics/render_primitives\" }}\n"
    ));
    r.expect(0, &["ENGINE-LAYERS: PASS", "(0 wasm-only)"]);
}

/// The anti-vacuity cases of the graphics layer: a member with no source, and a category with no
/// member, are each "nothing to look at"; an unreadable workspace is a check that did not run.
#[test]
fn an_absent_graphics_category_is_not_a_clean_wall() {
    let r = Repo::new("member-empty");
    std::fs::remove_file(r.0.join("crates/graphics/render_primitives/src/draw/instances.rs"))
        .unwrap();
    r.expect(
        1,
        &[
            "FAIL: engine-layers walked 0 .rs file(s) under crates/graphics/render_primitives/src",
            "ENGINE-LAYERS: FAIL (no inputs)",
        ],
    );

    let r = Repo::new("member-none");
    std::fs::remove_dir_all(r.0.join("crates/graphics/render_primitives")).unwrap();
    r.expect(
        1,
        &[
            "FAIL: engine-layers found no workspace member in crates/graphics",
            NOTHING_TAIL[0],
            "ENGINE-LAYERS: FAIL (no inputs)",
        ],
    );

    let r = Repo::new("member-no-workspace");
    std::fs::remove_file(r.0.join("Cargo.toml")).unwrap();
    let (code, out) = super::run(&r.0);
    let all = out.join("\n");
    assert_eq!(code, 2, "{all}");
    assert!(all.contains("workspace members"), "{all}");
    assert!(all.contains("ENGINE-LAYERS: FAIL (did not run)"), "{all}");
}

/// Rule 6's wasm-only matcher, built from two names: each crate's import shapes match, and its
/// prose, a crate-local path sharing its prefix and a CPU-only member do not.
#[test]
fn the_wasm_only_matcher_matches_imports_of_each_named_crate() {
    let identifiers = vec!["gpu_device".to_string(), "gpu_frame".to_string()];
    let p = Pattern::regex(&wasm_only_graphics_import_re(&identifiers)).unwrap();
    for bad in [
        "use gpu_device::GpuDevice;",
        "    let f = gpu_frame::encode(&packet);",
        "extern crate gpu_frame;",
    ] {
        assert!(p.is_match(bad), "should fail the gate: {bad}");
    }
    for ok in [
        "//! the map engine owns `gpu_device`",
        "use crate::gpu_device_settings::Panel;",
        "use render_primitives::draw::triangulate::triangulate;",
    ] {
        assert!(!p.is_match(ok), "should pass the gate: {ok}");
    }
    // The self-probe holds for any member name, the crate the frontend links today included: a
    // probe subject naming a fixed crate would go red the day that crate turned wasm-only.
    for names in [identifiers, vec!["render_primitives".to_string()]] {
        let mut lines = Vec::new();
        assert!(
            super::matcher_probes::probe_wasm_only_graphics_import(&mut lines, &names).is_ok(),
            "{names:?}: {lines:?}"
        );
    }
}
