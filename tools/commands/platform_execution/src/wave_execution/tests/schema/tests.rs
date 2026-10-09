use super::*;

#[test]
fn cksum_matches_the_coreutils_tool() {
    // If this drifts, the bash gate and this one fight over target/gate-schema and each pays a
    // cold rebuild. Compared against the real `cksum` so the interop claim is measured.
    use std::io::Write;
    let data = b"the quick brown fox\n";
    let mut child = std::process::Command::new("cksum")
        .current_dir(std::env::temp_dir())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("cksum on PATH");
    child.stdin.take().unwrap().write_all(data).unwrap();
    let out = child.wait_with_output().unwrap();
    let want: String = String::from_utf8_lossy(&out.stdout)
        .trim_end()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    assert_eq!(cksum(data), want);
}

#[test]
fn empty_input_matches_too() {
    assert_eq!(cksum(b""), format!("{}{}", 4294967295u32, 0));
}

/// A read that returns only some of the names leaves a one-way subset check green over the
/// hole, so the set must match EXACTLY — an empty or partial read is a hard fail in
/// `gate_schema`.
///
/// Both sides come from code: the task table and the pinned constant. No `if …exists()` guard
/// skips the comparison, because a guard makes the test vacuous the moment its subject goes away.
#[test]
fn the_task_table_and_the_pinned_set_agree() {
    let mut got = task_validate_gates();
    assert!(
        !got.is_empty(),
        "the schema-validate task row vanished — gate_schema would refuse, and so does this"
    );
    got.sort();
    let mut want: Vec<String> = VALIDATE_GATES.iter().map(|s| (*s).to_string()).collect();
    want.sort();
    assert_eq!(
        got, want,
        "`xtask schema list-gates` disagrees with GATE_SCHEMA_VALIDATE_GATES"
    );
}

/// xtask's build closure as the workspace manifests declare it, sorted: xtask itself, its direct
/// path dependencies and the members reachable only through them (the ticket model's id macros,
/// the world export pipeline's world crates). A crate entering or leaving the closure changes this
/// list in the same change, so the assertion below stays an exact-set comparison.
const XTASK_BUILD_CLOSURE: &[&str] = &[
    "crates/contracts/fleet_wire_contract",
    "crates/foundation/content_digest",
    "crates/foundation/deterministic_random",
    "crates/foundation/newtype_ids",
    "crates/foundation/repository_root",
    "crates/foundation/time_source",
    "crates/geometry/geometry_primitives",
    "crates/geometry/map_coordinates",
    "crates/geometry/spatial_indexes",
    "crates/graphics/render_primitives",
    "crates/line_of_sight/interior_line_of_sight",
    "crates/line_of_sight/terrain_line_of_sight",
    "crates/line_of_sight/world_line_of_sight",
    "crates/map_overlay/label_layout",
    "crates/map_overlay/map_draw_lanes",
    "crates/terrain/road_network",
    "crates/terrain/terrain_elevation",
    "crates/world_formats/prefab_catalog",
    "crates/world_formats/world_chunks",
    "crates/world_formats/world_file_formats",
    "crates/world_formats/world_store",
    "crates/world_objects/building_interiors",
    "crates/world_objects/place_names",
    "crates/world_objects/vegetation",
    "tools/checks/documentation_checks",
    "tools/checks/mod_script_checks",
    "tools/checks/repository_checks",
    "tools/commands/agent_context_guards",
    "tools/commands/api_readiness_checks",
    "tools/commands/ballistics_oracle_tooling",
    "tools/commands/ci_task_catalog",
    "tools/commands/database_operations",
    "tools/commands/deployment",
    "tools/commands/enfusion_mcp",
    "tools/commands/mod_operations",
    "tools/commands/platform_execution",
    "tools/commands/remote_debugging",
    "tools/commands/repository_relocation",
    "tools/commands/schema_tooling",
    "tools/commands/staging_procedures",
    "tools/commands/workstation_setup",
    "tools/enfusion/enfusion_pak",
    "tools/enfusion/enfusion_script_index",
    "tools/foundation/deploy_settings",
    "tools/foundation/process_runner",
    "tools/foundation/repository_laws",
    "tools/foundation/repository_layout",
    "tools/foundation/verification_core",
    "tools/map_assets/blueprint_compiler",
    "tools/map_assets/map_asset_verification",
    "tools/map_assets/world_export_pipeline",
    "tools/staging/staging_load_plan",
    "tools/tickets/ticket_metrics",
    "tools/tickets/ticket_model",
    "tools/tickets/ticket_registry",
    "tools/tickets/ticket_wave_lock",
    "tools/xtask",
];

/// The stamp roots are xtask's build closure, derived from the manifests: xtask itself, its
/// direct path dependencies, the members only reachable through them, and nothing it does not
/// build. A hand list missed every crate born after it was written, and two trees could then
/// share the private target under one stamp while a dependency differed. The derived roots equal
/// [`XTASK_BUILD_CLOSURE`] exactly, in a stable order: a missing root and an extra root both fail.
#[test]
fn the_stamp_roots_are_the_whole_build_closure_of_xtask_and_nothing_else() {
    let root = tool_test_support::test_repo_root();
    let roots = xtask_build_closure(&root).expect("the workspace manifests read");
    let mut sorted = roots.clone();
    sorted.sort();
    assert_eq!(
        roots, sorted,
        "the stamp roots are listed in a stable order"
    );
    let mut want: Vec<String> = XTASK_BUILD_CLOSURE
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    want.sort();
    assert_eq!(
        roots, want,
        "the stamp roots differ from xtask's build closure"
    );
    for unwanted in [
        "apps/api",
        "apps/frontend",
        "crates/map_rendering/map_renderer",
        "tools/foundation/tool_test_support",
    ] {
        assert!(
            !roots.iter().any(|folder| folder == unwanted),
            "{unwanted} is not built with xtask, yet it is a stamp root"
        );
    }
}
