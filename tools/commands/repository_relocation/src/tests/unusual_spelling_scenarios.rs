//! Whole relocation runs on throwaway checkouts for the spellings a plain path scan misses: a path
//! glued to a `\n`, `\t`, `\r` or `\0` escape, a `file://` URL, a `/../` piece joined to a base, a
//! token that climbs back out of a folder (`apps/../from`) and one whose lead names no folder
//! (`7/../..`, a test datum), the live README beside the manifests, frozen areas the tool names by
//! their place after the moves, the dry run's verification of the planned tree, and the spellings
//! that look like paths and name none: a lone `/` and a synthetic fixture path that only starts
//! with a moved folder's name.

use super::file_treatment::manifests_folder;
use super::fixture_repository::FixtureRepository;
use super::{apply, dry_run, verify};
use repository_layout::ARCHIVE_DIR;
use repository_layout::documentation::DOCUMENTATION_ROOT;

#[test]
fn relocate_spellings_after_control_escapes_are_rewritten_and_verified() {
    let repo = FixtureRepository::new("control-escapes");
    let listing = r#"const TREE: &str = "apps/a.rs\0old_assets/x.json\0README.md";
const MESSAGE: &str = "missing: {x}\nold_assets/scratch/ is gitignored\told_assets/a.json";
const NOT_AN_ESCAPE: &str = "C:\\nold_assets";
"#;
    repo.write("old_assets/a.json", "{}\n")
        .write("old_assets/x.json", "{}\n")
        .write("apps/a.rs", "\n")
        .write("src/listing.rs", listing)
        .write(
            "data/help.json",
            "{\"help\": \"see\\nold_assets/a.json\\r\"}\n",
        )
        .track();
    let manifest = repo.manifest("path\told_assets\tnew_assets\t\n");

    assert_eq!(dry_run(repo.root(), &manifest), 0);
    assert_eq!(apply(repo.root(), &manifest), 0);
    assert_eq!(
        repo.read("src/listing.rs"),
        r#"const TREE: &str = "apps/a.rs\0new_assets/x.json\0README.md";
const MESSAGE: &str = "missing: {x}\nnew_assets/scratch/ is gitignored\tnew_assets/a.json";
const NOT_AN_ESCAPE: &str = "C:\\nold_assets";
"#
    );
    assert_eq!(
        repo.read("data/help.json"),
        "{\"help\": \"see\\nnew_assets/a.json\\r\"}\n"
    );
    assert_eq!(verify(repo.root(), Some(&manifest)), 0);

    repo.write("planted.txt", "listing: a.rs\\0old_assets/a.json\n")
        .track();
    assert_eq!(verify(repo.root(), Some(&manifest)), 1);
}

#[test]
fn relocate_escape_spelling_that_also_names_a_tracked_path_is_unresolved() {
    let repo = FixtureRepository::new("ambiguous-escape");
    let listing = r#"const LISTING: &str = "x.rs\nold_assets/a.json";
"#;
    repo.write("old_assets/a.json", "{}\n")
        .write("nold_assets/b.json", "{}\n")
        .write("src/listing.rs", listing)
        .track();
    let manifest = repo.manifest("path\told_assets\tnew_assets\t\n");

    assert_eq!(dry_run(repo.root(), &manifest), 1);
    assert_eq!(apply(repo.root(), &manifest), 1);
    assert_eq!(repo.read("src/listing.rs"), listing);
    assert!(repo.exists("old_assets/a.json"), "nothing moved");
}

#[test]
fn relocate_file_urls_carrying_a_repository_path_are_rewritten() {
    let repo = FixtureRepository::new("file-urls");
    repo.write("old_docs/runbook.md", "# Runbook\n")
        .write(
            "deploy/site.service",
            "Documentation=file:///PLACEHOLDER/old_docs/runbook.md\n\
             Documentation=file:///old_docs/runbook.md\n\
             Documentation=file://old_docs/runbook.md\n\
             Documentation=https://example.com/old_docs/runbook.md\n",
        )
        .track();
    let manifest = repo.manifest("path\told_docs\tnew_docs\t\n");

    assert_eq!(dry_run(repo.root(), &manifest), 0);
    assert_eq!(apply(repo.root(), &manifest), 0);
    assert_eq!(
        repo.read("deploy/site.service"),
        "Documentation=file:///PLACEHOLDER/new_docs/runbook.md\n\
         Documentation=file:///new_docs/runbook.md\n\
         Documentation=file://new_docs/runbook.md\n\
         Documentation=https://example.com/old_docs/runbook.md\n"
    );
    assert_eq!(verify(repo.root(), Some(&manifest)), 0);

    repo.write(
        "deploy/other.service",
        "Documentation=file:///srv/checkout/old_docs/runbook.md\n",
    )
    .track();
    assert_eq!(verify(repo.root(), Some(&manifest)), 1);
}

#[test]
fn relocate_slash_led_climbs_in_macro_arguments_follow_their_anchor() {
    let repo = FixtureRepository::new("slash-climbs");
    let settings = r#"fn embed() -> String {
    let path = format!("{}{}{}", "/../../../shared_data/definitions/", "mission", ".schema.json");
    format!("\"{path}\"")
}
// The embed reads "/../../../shared_data/definitions/mission.schema.json" from the crate folder.
const SCHEMA: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../shared_data/definitions/mission.schema.json"));
"#;
    repo.write("shared_data/definitions/mission.schema.json", "{}\n")
        .write(
            "apps/web/client/Cargo.toml",
            "[package]\nname = \"client\"\n",
        )
        .write("apps/web/client/src/tests/settings.rs", settings)
        .track();
    let manifest = repo.manifest("path\tshared_data\tcontracts\t\n");

    assert_eq!(dry_run(repo.root(), &manifest), 0);
    assert_eq!(apply(repo.root(), &manifest), 0);
    assert_eq!(
        repo.read("apps/web/client/src/tests/settings.rs"),
        settings.replace("shared_data", "contracts")
    );
}

#[test]
fn relocate_slash_led_climb_that_names_nothing_is_unresolved() {
    let repo = FixtureRepository::new("stale-slash-climb");
    let writer = r#"fn golden(root: &str) -> String {
    format!("{}{}", root, "/../../shared_data/definitions/mission.schema.json")
}
"#;
    repo.write("shared_data/definitions/mission.schema.json", "{}\n")
        .write(
            "apps/web/client/Cargo.toml",
            "[package]\nname = \"client\"\n",
        )
        .write("apps/web/client/src/writer.rs", writer)
        .track();
    let manifest = repo.manifest("path\tshared_data\tcontracts\t\n");

    assert_eq!(dry_run(repo.root(), &manifest), 1);
    assert_eq!(apply(repo.root(), &manifest), 1);
    assert_eq!(repo.read("apps/web/client/src/writer.rs"), writer);
}

#[test]
fn relocate_interior_climbs_keep_their_shape() {
    let repo = FixtureRepository::new("interior-climbs");
    repo.write("old_assets/a.json", "{}\n")
        .write("apps/x.rs", "\n")
        .write(
            "tools_dir/checker/Cargo.toml",
            "[package]\nname = \"checker\"\n",
        )
        .write(
            "tools_dir/checker/src/scope.rs",
            "const CASES: [&str; 1] = [\"apps/../old_assets\"];\n",
        )
        .write(
            "docs/scope.md",
            "A value like `apps/../old_assets/a.json` climbs out of its folder.\n",
        )
        .track();
    let manifest = repo.manifest("path\told_assets\tnew_assets\t\n");

    assert_eq!(dry_run(repo.root(), &manifest), 0);
    assert_eq!(apply(repo.root(), &manifest), 0);
    assert_eq!(
        repo.read("tools_dir/checker/src/scope.rs"),
        "const CASES: [&str; 1] = [\"apps/../new_assets\"];\n"
    );
    assert_eq!(
        repo.read("docs/scope.md"),
        "A value like `apps/../new_assets/a.json` climbs out of its folder.\n"
    );
}

#[test]
fn relocate_climb_whose_lead_names_no_folder_stays_as_written() {
    let repo = FixtureRepository::new("climb-without-lead-folder");
    let cases = "// An id such as 7/../.. climbs out of the profile route.\nconst CASES: [(&str, &str); 1] = [(\"7/../..\", \"traversal through the id\")];\n";
    repo.write("apps/api/Cargo.toml", "[package]\nname = \"api\"\n")
        .write("apps/api/src/identity/services/profile.rs", "\n")
        .write("apps/api/src/identity/services/tests/profile.rs", cases)
        .write(
            "crates/identity/Cargo.toml",
            "[package]\nname = \"identity\"\n",
        )
        .track();
    let manifest = repo.manifest("path\tapps/api/src/identity\tcrates/identity/src\t\n");

    assert_eq!(dry_run(repo.root(), &manifest), 0);
    assert_eq!(apply(repo.root(), &manifest), 0);
    assert_eq!(
        repo.read("crates/identity/src/services/tests/profile.rs"),
        cases
    );
    assert_eq!(verify(repo.root(), Some(&manifest)), 0);
}

#[test]
fn relocate_manifests_folder_readme_is_live_and_its_manifests_stay_excluded() {
    let repo = FixtureRepository::new("manifests-readme");
    let folder = manifests_folder();
    let stage = "kind\tfrom\tto\tscope\npath\told_assets\tnew_assets\t\n";
    repo.write("old_assets/a.json", "{}\n")
        .write(
            &format!("{folder}/README.md"),
            "Stage one moved [assets](/old_assets/a.json).\n",
        )
        .write(&format!("{folder}/stage_01.tsv"), stage)
        .track();
    let manifest = repo.manifest("path\told_assets\tnew_assets\t\n");

    assert_eq!(apply(repo.root(), &manifest), 0);
    assert_eq!(
        repo.read(&format!("{folder}/README.md")),
        "Stage one moved [assets](/new_assets/a.json).\n"
    );
    assert_eq!(repo.read(&format!("{folder}/stage_01.tsv")), stage);
    assert_eq!(verify(repo.root(), None), 0);

    repo.write(
        &format!("{folder}/README.md"),
        "Stage one moved [assets](/old_assets/a.json).\n",
    );
    assert_eq!(verify(repo.root(), None), 1);
}

#[test]
fn relocate_frozen_areas_named_after_the_moves_are_found_before_them() {
    let repo = FixtureRepository::new("frozen-areas-before");
    let old_root = "old_documentation";
    let archive_below_root = ARCHIVE_DIR
        .strip_prefix(DOCUMENTATION_ROOT)
        .expect("the archive lies in the documentation tree");
    let history = "We moved `old_assets/a.json`.\n";
    repo.write("old_assets/a.json", "{}\n")
        .write(
            &format!("{old_root}{archive_below_root}/history.md"),
            history,
        )
        .write(
            &format!("{old_root}/guide.md"),
            "See `old_assets/a.json`.\n",
        )
        .track();
    let manifest = repo.manifest(&format!(
        "path\t{old_root}\t{DOCUMENTATION_ROOT}\t\npath\told_assets\tnew_assets\t\n"
    ));

    assert_eq!(apply(repo.root(), &manifest), 0);
    assert_eq!(repo.read(&format!("{ARCHIVE_DIR}/history.md")), history);
    assert_eq!(
        repo.read(&format!("{DOCUMENTATION_ROOT}/guide.md")),
        "See `new_assets/a.json`.\n"
    );
}

#[test]
fn relocate_dry_run_verifies_the_planned_tree_and_fails_on_a_hidden_leftover() {
    let repo = FixtureRepository::new("planned-leftover");
    repo.write("old_assets/a.json", "{}\n")
        .write("README.md", "Data: alpha/a.json\n")
        .track();
    let manifest = repo.manifest("path\told_assets\tnew_assets\t\ntext\talpha\told_assets\t\n");

    assert_eq!(
        dry_run(repo.root(), &manifest),
        1,
        "the text row spells the retired path again in the planned README"
    );
    assert_eq!(apply(repo.root(), &manifest), 1);
    assert!(repo.exists("old_assets/a.json"), "nothing moved");
    assert_eq!(repo.read("README.md"), "Data: alpha/a.json\n");
}

#[test]
fn relocate_lone_separator_literals_name_no_path() {
    let repo = FixtureRepository::new("lone-separator");
    let cases = "pub const CASES: &[(&str, bool)] = &[\n    (\"http://[::1]/\", true),\n    (\"/\", false),\n    (\"//\", false),\n];\n";
    repo.write("web_shared/url_cases.rs", cases)
        .write("web_shared/README.md", "# Shared URL cases\n")
        .write("web_shared/kept.rs", "pub const KEPT: bool = true;\n")
        .track();
    let manifest = repo.manifest(
        "path\tweb_shared/url_cases.rs\tguard/src/cases.rs\t\npath\tweb_shared/README.md\tguard/README.md\t\n",
    );

    assert_eq!(dry_run(repo.root(), &manifest), 0);
    assert_eq!(apply(repo.root(), &manifest), 0);
    assert_eq!(repo.read("guard/src/cases.rs"), cases);
    assert_eq!(verify(repo.root(), Some(&manifest)), 0);
}

#[test]
fn relocate_plain_fixture_paths_under_a_moved_folder_name_stay_as_written() {
    let repo = FixtureRepository::new("synthetic-fixtures");
    let fixtures = "fn fixture(repo: &Repo) {\n    repo.write(\"deploy/site.service\", \"[Unit]\\n\");\n    repo.ignore(\"deploy/secrets.env\");\n    // Writes deploy/site.service beside the tracked deploy/units/agent.service.\n    let tracked = \"deploy/units/agent.service\";\n    let from_root = \"tools_dir/checker/deploy/units/agent.service\";\n}\n";
    let expected = fixtures
        .replace(
            "tracked = \"deploy/units/agent.service\"",
            "tracked = \"../../deploy/units/agent.service\"",
        )
        .replace(
            "\"tools_dir/checker/deploy/units/agent.service\"",
            "\"deploy/units/agent.service\"",
        );
    repo.write(
        "tools_dir/checker/Cargo.toml",
        "[package]\nname = \"checker\"\n",
    )
    .write("tools_dir/checker/deploy/units/agent.service", "[Unit]\n")
    .write("tools_dir/checker/src/tests/fixtures.rs", fixtures)
    .track();
    let manifest = repo.manifest("path\ttools_dir/checker/deploy\tdeploy\t\n");

    assert_eq!(dry_run(repo.root(), &manifest), 0);
    assert_eq!(apply(repo.root(), &manifest), 0);
    assert_eq!(
        repo.read("tools_dir/checker/src/tests/fixtures.rs"),
        expected
    );
    assert_eq!(verify(repo.root(), Some(&manifest)), 0);
}

/// The same rule over a lead of two named segments, the case stage S9 pins: a datum whose lead no
/// anchor holds as a folder stays as written, while `apps/../old_assets` (lead `apps`) is rewritten.
#[test]
fn relocate_climbs_led_by_a_folder_no_anchor_holds_stay_as_written() {
    let repo = FixtureRepository::new("unanchored-climbs");
    let profile = "const ID: &str = \"7/../..\";\nconst DATA: &str = \"apps/../old_assets\";\n";
    repo.write("old_assets/a.json", "{}\n")
        .write("apps/api/Cargo.toml", "[package]\nname = \"api\"\n")
        .write("apps/api/src/lib.rs", "\n")
        .write("apps/api/src/identity/services/tests/profile.rs", profile)
        .track();
    let manifest = repo.manifest(
        "path\tapps/api/src/identity/services/tests/profile.rs\tapps/api/src/kernel/discord/tests/profile.rs\t\n\
         path\told_assets\tnew_assets\t\n",
    );

    assert_eq!(dry_run(repo.root(), &manifest), 0);
    assert_eq!(apply(repo.root(), &manifest), 0);
    assert_eq!(
        repo.read("apps/api/src/kernel/discord/tests/profile.rs"),
        "const ID: &str = \"7/../..\";\nconst DATA: &str = \"apps/../new_assets\";\n"
    );
    assert_eq!(verify(repo.root(), Some(&manifest)), 0);
}
