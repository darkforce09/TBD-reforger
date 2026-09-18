use super::*;

/// T-946 — the wasm scope reaches the engine crates the SPA compiles, not just its own path.
///
/// Wave 237 changed `apps/website/map-engine` only, and the gate printed
/// `wasm32 (frontend) PASS` next to `trunk build SKIP (frontend untouched this wave)` — a
/// success reported over code neither step had compiled. The scope is derived from
/// `path = "…"` dependencies, so it follows the graph instead of a hand-kept list.
#[test]
fn the_wasm_scope_follows_the_frontends_dependency_graph() {
    let root = crate::core::repository_root::test_repo_root();
    let scope = wasm_scope_prefixes(&root);
    println!("── wasm scope ── {scope:?}");
    assert!(
        scope.iter().any(|d| d == FRONTEND_DIR),
        "the frontend itself is always in scope: {scope:?}"
    );
    for engine in ["apps/website/map-engine", "apps/website/graphics-engine"] {
        assert!(
            scope.iter().any(|d| d == engine),
            "{engine} is compiled into the SPA's wasm and must be in scope: {scope:?}"
        );
    }
    // The exact change that fooled the gate.
    assert!(
        wasm_scope_touched(
            &root,
            ["apps/website/map-engine/src/io/density/tbdd.rs"].into_iter()
        ),
        "a map-engine-core source change must put the SPA in scope"
    );
    assert!(
        wasm_scope_touched(&root, ["apps/website/map-engine/Cargo.toml"].into_iter()),
        "and so must its manifest — wave 237 made a dependency unconditional there"
    );
    // Something the SPA genuinely does not compile stays out.
    assert!(
        !wasm_scope_touched(
            &root,
            ["apps/website/api_v2/src/core/database/mod.rs"].into_iter()
        ),
        "a backend-only change must not force the most expensive step in the gate"
    );
}

/// T-946.64 follow-up, filed by the wave-255 verify: **the dependency graph is not the frontend
/// suite's whole input set.**
///
/// `frontend_tests_changed` originally scoped itself on `wasm_scope_touched` alone. But the
/// suite compiles files from outside that graph through `include_str!`, and wave 255 itself
/// changed one of them — `packages/tbd-schema/schema/mission.schema.json`, compiled by
/// `v2/apps/editor/ui/inspector/zones_panel/zone_schema_vocabulary.rs` and asserted over by
/// `zone_rule_fields_cover_the_whole_vocabulary`, which is documented to fail loudly on a new
/// `$defs/zoneRules` key. A slice whose diff was only that file would have printed "frontend
/// untouched", skipped the suite, and reported PASS over the one test that would have caught it.
///
/// The negative half is the load-bearing one: the fix must NOT be
/// `compiled_include_input_paths()` wholesale, because that would drag the API's own include
/// inputs into the frontend's scope and make a backend-only slice run the most expensive step in
/// the gate — the thing the test above deliberately forbids.
#[test]
fn the_frontends_include_str_inputs_are_in_scope_and_the_apis_are_not() {
    let root = crate::core::repository_root::test_repo_root();
    let scoped = frontend_include_inputs(&root);
    println!("── include inputs ── wasm-scope {}", scoped.len());
    // Non-vacuity: an empty scoped list would make every assertion below trivially true, and
    // an empty list is EXACTLY the failure mode here — it reads as "nothing is in scope" and
    // silently skips the suite. This assertion is what caught the cwd-relative walk that
    // `frontend_include_inputs` now pins with an absolute root.
    assert!(
        !scoped.is_empty(),
        "the SPA compiles include_str! inputs; an empty list means the walk broke"
    );
    // NOT compared against `compiled_include_input_paths()`: that one resolves against the
    // process CWD and answers 0 from a test binary, so the comparison would be vacuous in
    // exactly the direction this test exists to rule out. The negative cases below carry the
    // scoping guarantee instead.
    // The exact file wave 255 changed.
    assert!(
        frontend_include_input_touched(
            &root,
            ["packages/tbd-schema/schema/mission.schema.json"].into_iter()
        ),
        "mission.schema.json is include_str!'d by the SPA and must put it in scope; \
         scoped inputs: {scoped:?}"
    );
    // And the negative: a backend-only change stays out, include inputs and all.
    assert!(
        !frontend_include_input_touched(
            &root,
            ["apps/website/api_v2/src/core/database/mod.rs"].into_iter()
        ),
        "a backend-only change must not reach the frontend suite"
    );
    // A path nobody includes is not in scope either — this is a membership test, not a
    // "does the file exist" test.
    assert!(
        !frontend_include_input_touched(&root, ["README.md"].into_iter()),
        "an un-included file must not put the SPA in scope"
    );
}

#[test]
fn join_rel_resolves_dotdot_and_refuses_to_climb_out() {
    // The subjects are real `path = "../…"` values out of the website manifests. The engine
    // split renamed that crate, and 4057d82b9 rewrote the EXPECTED halves here without the
    // inputs — leaving `../graphics-engine` asserted to resolve to `map-engine`, which no path
    // join could ever do. Both halves now say the same thing, so the test is about `..`
    // resolution again rather than about a crate name.
    assert_eq!(
        join_rel("apps/website/frontend", "../map-engine").as_deref(),
        Some("apps/website/map-engine")
    );
    assert_eq!(
        join_rel("apps/website/map-engine", "../graphics-engine").as_deref(),
        Some("apps/website/graphics-engine")
    );
    assert_eq!(
        join_rel("crates", "../../elsewhere"),
        None,
        "cannot climb out"
    );
}

#[test]
fn edition_falls_back_to_2021_when_nothing_says_otherwise() {
    assert_eq!(file_edition("/definitely/not/a/repo/x.rs"), "2021");
}

#[test]
fn edition_is_read_from_the_nearest_manifest() {
    // The real workspace: apps/website/api_v2 is edition 2024, and hardcoding 2021 made every
    // slice touching it fail a gate it did not cause.
    if Path::new("apps/website/api_v2/Cargo.toml").is_file() {
        assert_eq!(file_edition("apps/website/api_v2/src/lib.rs"), "2024");
    }
}

#[test]
fn realpath_m_normalises_without_touching_the_disk() {
    assert_eq!(
        realpath_m(Path::new("/a/b/../c/./d")),
        PathBuf::from("/a/c/d")
    );
}

#[test]
fn workspace_members_parse_is_not_empty_on_the_real_manifest() {
    // A manifest reformat that parses to the empty set would make touch_workspace "succeed"
    // having touched nothing, which is the same lie one level up — touch_workspace refuses on
    // it, and this pins the parser that feeds it.
    //
    // `cargo test` sets the CWD to the PACKAGE root (`tools_v2/xtask/`), not the workspace root, so a
    // bare `Cargo.toml` here is xtask's own manifest and has no `[workspace]` at all. Walk up
    // for the real one; at runtime the driver has already `cd`-ed to the repo root.
    // T-923: the cwd is shared test state — resolve the root AND chdir under the one
    // process-wide lock in [`crate::commands::platform::wave_execution::testcwd`], or a concurrent scratch-repo test
    // (whose tree carries `.ai/tickets/ROOT`) becomes the "repo root" this test reads.
    let Some(cwd) =
        crate::commands::platform::wave_execution::testcwd::CwdGuard::enter_resolved(|| {
            crate::core::repository_root::find_repo_root().ok()
        })
    else {
        return;
    };
    let members = workspace_members();
    drop(cwd);
    assert!(
        !members.is_empty(),
        "parsed ZERO workspace members out of the root Cargo.toml"
    );
    // The parse must reach the LAST member too — a range that stopped at the first `]` would
    // silently drop crates, and a dropped member is a crate judged on someone else's artifacts.
    assert!(
        members.contains(&"tools_v2/xtask".to_string()),
        "members: {members:?}"
    );
    assert!(
        members.contains(&"tools_v2/developer-tools".to_string()),
        "members: {members:?}"
    );
    assert!(
        members.contains(&"apps/website/api_v2".to_string()),
        "members: {members:?}"
    );
}
