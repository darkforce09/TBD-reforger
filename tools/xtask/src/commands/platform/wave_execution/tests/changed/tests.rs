use super::*;

/// The wasm scope reaches the engine crates the SPA compiles, not just its own path.
///
/// Wave 237 changed `legacy/map_engine` only, and the gate printed
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
    for engine in ["legacy/map_engine", "legacy/graphics_engine"] {
        assert!(
            scope.iter().any(|d| d == engine),
            "{engine} is compiled into the SPA's wasm and must be in scope: {scope:?}"
        );
    }
    // A map engine source file the SPA compiles into its wasm.
    assert!(
        wasm_scope_touched(&root, ["legacy/map_engine/src/frame/encode.rs"].into_iter()),
        "a map_engine source change must put the SPA in scope"
    );
    assert!(
        wasm_scope_touched(&root, ["legacy/map_engine/Cargo.toml"].into_iter()),
        "and so must its manifest — wave 237 made a dependency unconditional there"
    );
    // Something the SPA genuinely does not compile stays out.
    assert!(
        !wasm_scope_touched(&root, ["apps/api/src/core/database/mod.rs"].into_iter()),
        "a backend-only change must not force the most expensive step in the gate"
    );
}

/// Follow-up, filed by the wave-255 verify: **the dependency graph is not the frontend
/// suite's whole input set.**
///
/// `frontend_tests_changed` originally scoped itself on `wasm_scope_touched` alone. But the
/// suite compiles files from outside that graph through `include_str!`, and wave 255 itself
/// changed one of them — `contracts/definitions/mission.schema.json`, compiled by
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
            ["contracts/definitions/mission.schema.json"].into_iter()
        ),
        "mission.schema.json is include_str!'d by the SPA and must put it in scope; \
         scoped inputs: {scoped:?}"
    );
    // And the negative: a backend-only change stays out, include inputs and all.
    assert!(
        !frontend_include_input_touched(&root, ["apps/api/src/core/database/mod.rs"].into_iter()),
        "a backend-only change must not reach the frontend suite"
    );
    // A golden response the frontend embeds through a macro that completes a folder prefix per
    // call site: the folder sits outside every wasm-scope crate, so only the include inputs can
    // put a golden-only change in scope.
    assert!(
        frontend_include_input_touched(
            &root,
            ["contracts/fixtures/api_goldens/GET__me.json"].into_iter()
        ),
        "the golden responses the frontend embeds must put it in scope"
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
    // The subjects are real `path = "../…"` values out of the workspace manifests: the frontend
    // climbs out of `apps/` to reach the map engine, and the map engine reaches the renderer
    // beside it. Each expected half is the join of its own inputs, so the test is about `..`
    // resolution rather than about a crate name.
    assert_eq!(
        join_rel("apps/frontend", "../../legacy/map_engine").as_deref(),
        Some("legacy/map_engine")
    );
    assert_eq!(
        join_rel("legacy/map_engine", "../graphics_engine").as_deref(),
        Some("legacy/graphics_engine")
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
    // The real workspace: apps/api is edition 2024, and hardcoding 2021 made every
    // slice touching it fail a gate it did not cause. `file_edition` takes a repository-relative
    // path, so the test runs at the repository root under the process-wide cwd lock: another
    // test may move the cwd at any moment otherwise.
    let cwd = crate::commands::platform::wave_execution::testcwd::CwdGuard::enter(
        &crate::core::repository_root::test_repo_root(),
    );
    let edition = file_edition("apps/api/src/lib.rs");
    drop(cwd);
    assert_eq!(edition, "2024");
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
    // `cargo test` sets the CWD to the PACKAGE root (`tools/xtask/`), not the workspace root, so a
    // bare `Cargo.toml` here is xtask's own manifest and has no `[workspace]` at all. Walk up
    // for the real one; at runtime the driver has already `cd`-ed to the repo root.
    // The cwd is shared test state — resolve the root AND chdir under the one
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
    let members = members.expect("the root manifest's workspace members");
    assert!(
        !members.is_empty(),
        "parsed ZERO workspace members out of the root Cargo.toml"
    );
    // The parse must reach the LAST member too — a range that stopped at the first `]` would
    // silently drop crates, and a dropped member is a crate judged on someone else's artifacts.
    assert!(
        members.contains(&"tools/xtask".to_string()),
        "members: {members:?}"
    );
    assert!(
        members.contains(&"tools/developer_tools".to_string()),
        "members: {members:?}"
    );
    assert!(
        members.contains(&"apps/api".to_string()),
        "members: {members:?}"
    );
}

/// A scratch folder under the system temporary directory holding `files` (path, contents), each
/// path relative to the folder.
fn scratch_tree(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("wave-changed-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for (path, contents) in files {
        let file = root.join(path);
        std::fs::create_dir_all(file.parent().expect("a parent folder")).expect("mkdir");
        std::fs::write(&file, contents).expect("write");
    }
    root
}

/// The wasm scope reaches a `crates/` member the frontend takes from `[workspace.dependencies]`
/// (`workspace = true`, inline or dotted, in a development table too) as surely as an engine it
/// names by `path` in a target-specific table, and leaves out a `crates/` member only another
/// application depends on.
#[test]
fn the_wasm_scope_follows_workspace_inherited_edges_into_crates_members() {
    let root = scratch_tree(
        "scope",
        &[
            (
                "Cargo.toml",
                "[workspace]\nmembers = [\"apps/frontend\", \"apps/api\", \"legacy/engine\", \
                 \"legacy/renderer\", \"crates/*/*\"]\n\n[workspace.dependencies]\n\
                 policy = { path = \"crates/contracts/policy\" }\n\
                 guard = { path = \"crates/foundation/guard\" }\n\
                 unrelated = { path = \"crates/foundation/unrelated\" }\nserde = \"1\"\n",
            ),
            (
                "apps/frontend/Cargo.toml",
                "[package]\nname = \"frontend\"\n\n[dependencies]\npolicy = { workspace = true }\n\
                 serde = { workspace = true }\n\n\
                 [target.'cfg(target_arch = \"wasm32\")'.dependencies]\n\
                 engine = { path = \"../../legacy/engine\" }\n\n\
                 [dev-dependencies]\nguard.workspace = true\n",
            ),
            (
                "apps/api/Cargo.toml",
                "[package]\nname = \"api\"\n\n[dependencies]\nunrelated = { workspace = true }\n\
                 engine = { path = \"../../legacy/engine\" }\n",
            ),
            (
                "legacy/engine/Cargo.toml",
                "[package]\nname = \"engine\"\n\n[dependencies]\n\
                 renderer = { path = \"../renderer\" }\n",
            ),
            (
                "legacy/renderer/Cargo.toml",
                "[package]\nname = \"renderer\"\n",
            ),
            (
                "crates/contracts/policy/Cargo.toml",
                "[package]\nname = \"policy\"\n",
            ),
            (
                "crates/foundation/guard/Cargo.toml",
                "[package]\nname = \"guard\"\n",
            ),
            (
                "crates/foundation/unrelated/Cargo.toml",
                "[package]\nname = \"unrelated\"\n",
            ),
            (
                "crates/foundation/README.md",
                "a file a member glob also matches\n",
            ),
        ],
    );
    let scope = wasm_scope_prefixes(&root);
    let touched = |path: &str| wasm_scope_touched(&root, [path].into_iter());
    let verdicts = (
        touched("crates/contracts/policy/src/lib.rs"),
        touched("crates/foundation/guard/src/lib.rs"),
        touched("crates/foundation/unrelated/src/lib.rs"),
        touched("apps/api/src/lib.rs"),
    );
    let _ = std::fs::remove_dir_all(&root);
    assert_eq!(
        scope,
        [
            "apps/frontend",
            "crates/contracts/policy",
            "crates/foundation/guard",
            "legacy/engine",
            "legacy/renderer",
        ]
    );
    assert_eq!(verdicts, (true, true, false, false), "{scope:?}");
}

/// The member list `touch_workspace` invalidates expands a glob into the member folders it
/// matches (a matched folder without a manifest, or a file, is not a member) and drops an
/// excluded folder; read literally, a glob names a folder that does not exist and the members
/// behind it keep their fingerprints.
#[test]
fn workspace_members_expand_globs_and_drop_excluded_folders() {
    let root = scratch_tree(
        "members",
        &[
            (
                "Cargo.toml",
                "[workspace]\nmembers = [\"apps/one\", \"crates/*/*\"]\n\
                 exclude = [\"crates/foundation/parked\"]\n",
            ),
            ("apps/one/Cargo.toml", "[package]\nname = \"one\"\n"),
            (
                "crates/contracts/policy/Cargo.toml",
                "[package]\nname = \"policy\"\n",
            ),
            (
                "crates/foundation/guard/Cargo.toml",
                "[package]\nname = \"guard\"\n",
            ),
            (
                "crates/foundation/parked/Cargo.toml",
                "[package]\nname = \"parked\"\n",
            ),
            (
                "crates/foundation/notes/README.md",
                "a folder without a manifest\n",
            ),
            (
                "crates/foundation/README.md",
                "a file the glob also matches\n",
            ),
        ],
    );
    let cwd = crate::commands::platform::wave_execution::testcwd::CwdGuard::enter(&root);
    let members = workspace_members();
    drop(cwd);
    let _ = std::fs::remove_dir_all(&root);
    assert_eq!(
        members.expect("a readable workspace"),
        [
            "apps/one",
            "crates/contracts/policy",
            "crates/foundation/guard"
        ]
    );
}

/// A `.rs` file outside every package has no owning package, and resolves instead to the
/// packages that `include!` that very file — not to a package that includes another file of the
/// same name.
#[test]
fn a_fragment_with_no_package_ancestor_resolves_to_the_packages_that_include_it() {
    let root = scratch_tree(
        "orphan",
        &[
            (
                "fragments/case_table.rs",
                "pub const CASES: [&str; 0] = [];\n",
            ),
            ("consumer/Cargo.toml", "[package]\nname = \"consumer\"\n"),
            (
                "consumer/src/lib.rs",
                "include!(\"../../fragments/case_table.rs\");\n",
            ),
            ("bystander/Cargo.toml", "[package]\nname = \"bystander\"\n"),
            (
                "bystander/src/lib.rs",
                "include!(\"../other/case_table.rs\");\n",
            ),
            (
                "bystander/other/case_table.rs",
                "pub const CASES: [&str; 0] = [];\n",
            ),
        ],
    );
    let fragment = root.join("fragments/case_table.rs").display().to_string();
    let consumer = root.join("consumer").display().to_string();
    let bystander = root.join("bystander").display().to_string();
    let owner = owning_package_dir(&fragment);
    let consumers = include_consumers_under(&fragment, &[consumer.clone(), bystander]);
    let _ = std::fs::remove_dir_all(&root);
    assert_eq!(owner, None, "the fragment sits outside every package");
    assert_eq!(consumers, [consumer]);
}
