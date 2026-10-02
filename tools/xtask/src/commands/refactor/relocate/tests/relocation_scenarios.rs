//! Whole relocation runs on throwaway checkouts: moves, path spellings, relative literals, frozen
//! records, tickets, binaries, refusals and the verification.

use super::file_treatment::manifests_folder;
use super::fixture_repository::FixtureRepository;
use super::{EXAMPLE_MANIFEST, apply, dry_run, verify};
use crate::core::repository_layout::TICKETS_DIR;
use crate::core::repository_layout::documentation::{ARCHIVE_DIR, TICKET_DOCUMENTS_DIR};

#[test]
fn relocate_folder_move_rewrites_root_and_slash_root_spellings() {
    let repo = FixtureRepository::new("root-spellings");
    repo.write("old_assets/terrains/a.json", "{}\n")
        .write("guides/old_assets/README.md", "# Guide about assets\n")
        .write(
            "README.md",
            "See `old_assets/terrains/a.json` and [a](/old_assets/terrains/a.json).\n\
             The guide lives in `guides/old_assets/README.md`.\n\
             Not paths: my_old_assets, old_assets_extra, old_assets.md.\n\
             Pinned: https://example.com/blob/abc/old_assets/terrains/a.json\n\
             Sentence end: old_assets.\n",
        )
        .write(
            "deploy/site.service",
            "Environment=MAP_DIR=/PLACEHOLDER/old_assets/terrains\nWorkingDirectory=/srv/old_assets\n",
        )
        .write(".gitignore", "/old_assets/scratch/\nold_assets/tiles/\n")
        .track();
    let manifest = repo.manifest("path\told_assets\tnew_assets\t\n");

    assert_eq!(dry_run(repo.root(), &manifest), 0);
    assert!(
        repo.exists("old_assets/terrains/a.json"),
        "a dry run moves nothing"
    );
    assert!(
        repo.read("README.md")
            .contains("[a](/old_assets/terrains/a.json)")
    );

    assert_eq!(apply(repo.root(), &manifest), 0);
    assert!(repo.exists("new_assets/terrains/a.json"));
    assert!(!repo.exists("old_assets"));
    assert!(
        repo.tracked()
            .contains(&"new_assets/terrains/a.json".to_string())
    );
    let readme = repo.read("README.md");
    assert!(
        readme.contains("See `new_assets/terrains/a.json` and [a](/new_assets/terrains/a.json).")
    );
    assert!(readme.contains("`guides/old_assets/README.md`"), "{readme}");
    assert!(readme.contains("my_old_assets, old_assets_extra, old_assets.md."));
    assert!(readme.contains("https://example.com/blob/abc/old_assets/terrains/a.json"));
    assert!(readme.contains("Sentence end: new_assets.\n"));
    assert_eq!(
        repo.read("deploy/site.service"),
        "Environment=MAP_DIR=/PLACEHOLDER/new_assets/terrains\nWorkingDirectory=/srv/new_assets\n"
    );
    assert_eq!(
        repo.read(".gitignore"),
        "/new_assets/scratch/\nnew_assets/tiles/\n"
    );
}

#[test]
fn relocate_depth_change_rerelativises_include_and_manifest_dir_literals() {
    let repo = FixtureRepository::new("depth-change");
    repo.write("shared_data/schema.json", "{}\n")
        .write("shared_data/terrains/everon.json", "{}\n")
        .write("apps/web/server_v2/Cargo.toml", "[package]\nname = \"server\"\n")
        .write("apps/web/server_v2/src/fixture.json", "{}\n")
        .write(
            "apps/web/server_v2/src/lib.rs",
            "const SCHEMA: &str = include_str!(\"../../../../shared_data/schema.json\");\n\
             const AGAIN: &str =\n    include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/../../../shared_data/schema.json\"));\n\
             const TERRAINS: &str = \"../../../shared_data/terrains\";\n\
             const LOCAL: &str = include_str!(\"fixture.json\");\n\
             // Falls back to ../../../shared_data/terrains/everon.json when unset.\n",
        )
        .write("tools_dir/checker/Cargo.toml", "[package]\nname = \"checker\"\n")
        .write(
            "tools_dir/checker/src/main.rs",
            "const FIXTURE: &str = include_str!(\"../../../apps/web/server_v2/src/fixture.json\");\n\
             fn main() {\n    let base = std::path::Path::new(env!(\"CARGO_MANIFEST_DIR\")).join(\"../../apps/web/server_v2/src\");\n}\n",
        )
        .track();
    let manifest = repo.manifest("path\tapps/web/server_v2\tapps/server\t\n");

    assert_eq!(apply(repo.root(), &manifest), 0);
    let library = repo.read("apps/server/src/lib.rs");
    assert!(
        library.contains("include_str!(\"../../../shared_data/schema.json\")"),
        "{library}"
    );
    assert!(
        library.contains("\"/../../shared_data/schema.json\""),
        "{library}"
    );
    assert!(
        library.contains("TERRAINS: &str = \"../../shared_data/terrains\";"),
        "{library}"
    );
    assert!(
        library.contains("include_str!(\"fixture.json\")"),
        "{library}"
    );
    assert!(
        library.contains("Falls back to ../../shared_data/terrains/everon.json"),
        "{library}"
    );
    let checker = repo.read("tools_dir/checker/src/main.rs");
    assert!(
        checker.contains("include_str!(\"../../../apps/server/src/fixture.json\")"),
        "{checker}"
    );
    assert!(
        checker.contains(".join(\"../../apps/server/src\")"),
        "{checker}"
    );
}

#[test]
fn relocate_cargo_path_dependencies_follow_both_sides() {
    let repo = FixtureRepository::new("cargo-paths");
    repo.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"apps/web/server_v2\", \"libs/shared\", \"tools_dir/checker\"]\n",
    )
    .write("libs/shared/Cargo.toml", "[package]\nname = \"shared\"\n")
    .write("libs/shared/src/lib.rs", "\n")
    .write(
        "apps/web/server_v2/Cargo.toml",
        "[package]\nname = \"server\"\n\n[lib]\npath = \"src/lib.rs\"\n\n[dependencies]\nshared = { path = \"../../../libs/shared\" }\n",
    )
    .write("apps/web/server_v2/src/lib.rs", "\n")
    .write(
        "tools_dir/checker/Cargo.toml",
        "[package]\nname = \"checker\"\n\n[dependencies]\nserver = { path = \"../../apps/web/server_v2\" }\n",
    )
    .write("tools_dir/checker/src/main.rs", "fn main() {}\n")
    .track();
    let manifest = repo.manifest("path\tapps/web/server_v2\tapps/server\t\n");

    assert_eq!(apply(repo.root(), &manifest), 0);
    assert!(
        repo.read("Cargo.toml")
            .contains("\"apps/server\", \"libs/shared\"")
    );
    let server = repo.read("apps/server/Cargo.toml");
    assert!(server.contains("path = \"src/lib.rs\""), "{server}");
    assert!(
        server.contains("shared = { path = \"../../libs/shared\" }"),
        "{server}"
    );
    let checker = repo.read("tools_dir/checker/Cargo.toml");
    assert!(
        checker.contains("server = { path = \"../../apps/server\" }"),
        "{checker}"
    );
}

#[test]
fn relocate_path_attribute_and_markdown_relative_links() {
    let repo = FixtureRepository::new("path-attribute");
    repo.write("crate_a/Cargo.toml", "[package]\nname = \"crate_a\"\n")
        .write(
            "crate_a/src/lib.rs",
            "#[cfg(test)]\n#[path = \"../shared_tests/helpers.rs\"]\nmod helpers;\n",
        )
        .write("crate_a/shared_tests/helpers.rs", "\n")
        .write(
            "crate_a/shared_tests/README.md",
            "# Helpers\n\nSee [the library](../src/lib.rs) and [helpers](helpers.rs#top).\n",
        )
        .write(
            "guide/README.md",
            "# Guide\n\n[helpers](../crate_a/shared_tests/helpers.rs) and\n[ref]: ../crate_a/shared_tests/README.md\n",
        )
        .track();
    let manifest = repo.manifest("path\tcrate_a/shared_tests\tcrate_a/src/support\t\n");

    assert_eq!(apply(repo.root(), &manifest), 0);
    assert!(
        repo.read("crate_a/src/lib.rs")
            .contains("#[path = \"support/helpers.rs\"]")
    );
    assert_eq!(
        repo.read("crate_a/src/support/README.md"),
        "# Helpers\n\nSee [the library](../lib.rs) and [helpers](helpers.rs#top).\n"
    );
    let guide = repo.read("guide/README.md");
    assert!(
        guide.contains("[helpers](../crate_a/src/support/helpers.rs)"),
        "{guide}"
    );
    assert!(
        guide.contains("[ref]: ../crate_a/src/support/README.md"),
        "{guide}"
    );
}

#[test]
fn relocate_frozen_documents_rewrite_links_and_keep_backticks() {
    let repo = FixtureRepository::new("frozen");
    let archived = format!("{ARCHIVE_DIR}/old_note.md");
    let ticket_document = format!("{TICKET_DOCUMENTS_DIR}/specs/old_spec.md");
    let frozen_text = "Moved from `old_assets/a.json`; see [a](/old_assets/a.json).\n";
    repo.write("old_assets/a.json", "{}\n")
        .write(&archived, frozen_text)
        .write(&ticket_document, frozen_text)
        .write(&format!("{ARCHIVE_DIR}/data.tsv"), "old_assets/a.json\n")
        .track();
    let manifest = repo.manifest("path\told_assets\tnew_assets\t\n");

    assert_eq!(apply(repo.root(), &manifest), 0);
    let expected = "Moved from `old_assets/a.json`; see [a](/new_assets/a.json).\n";
    assert_eq!(repo.read(&archived), expected);
    assert_eq!(repo.read(&ticket_document), expected);
    assert_eq!(
        repo.read(&format!("{ARCHIVE_DIR}/data.tsv")),
        "old_assets/a.json\n"
    );
}

#[test]
fn relocate_closed_tickets_rewrite_only_spec_plan_and_owns() {
    let repo = FixtureRepository::new("tickets");
    let closed = format!("{TICKETS_DIR}/T-901.toml");
    let open = format!("{TICKETS_DIR}/T-902.toml");
    let body = |status: &str| {
        format!(
            "id = \"T-90x\"\nstatus = \"{status}\"\nspec = \"old_docs/spec.md\"\nplan = \"old_docs/plan.md\"\n\
             notes = \"\"\"\n[not a table] old_docs/spec.md\n\"\"\"\ncitations = [\"old_docs/spec.md §1\"]\n\
             owns = [\"old_docs/spec.md\"]\n\n[scope]\ndomain = \"old_docs/x\"\n"
        )
    };
    repo.write("old_docs/spec.md", "# Spec\n")
        .write("old_docs/plan.md", "# Plan\n")
        .write(&closed, &body("shipped"))
        .write(&open, &body("ready"))
        .track();
    let manifest = repo.manifest("path\told_docs\tnew_docs\t\n");

    assert_eq!(apply(repo.root(), &manifest), 0);
    let closed_text = repo.read(&closed);
    assert!(closed_text.contains("spec = \"new_docs/spec.md\"\nplan = \"new_docs/plan.md\""));
    assert!(
        closed_text.contains("owns = [\"new_docs/spec.md\"]"),
        "{closed_text}"
    );
    assert!(
        closed_text.contains("[not a table] old_docs/spec.md"),
        "{closed_text}"
    );
    assert!(
        closed_text.contains("citations = [\"old_docs/spec.md §1\"]"),
        "{closed_text}"
    );
    assert!(
        closed_text.contains("domain = \"old_docs/x\""),
        "{closed_text}"
    );
    let open_text = repo.read(&open);
    assert!(!open_text.contains("old_docs"), "{open_text}");
    assert!(
        open_text.contains("citations = [\"new_docs/spec.md §1\"]"),
        "{open_text}"
    );
}

#[test]
fn relocate_closed_tickets_rewrite_their_owns_entries() {
    let repo = FixtureRepository::new("ticket-owns");
    let listed = format!("{TICKETS_DIR}/T-903.toml");
    let inline = format!("{TICKETS_DIR}/T-904.toml");
    let listed_body = |path: &str| {
        format!(
            "id = \"T-903\"\nstatus = \"shipped\"\nsummary = \"Moved old_docs/runbook.md\"\n\
             owns = [\n    \"apps/x/.env.example\",\n    \"{path}\",\n]\n\n[scope]\ndomain = \"repo\"\n"
        )
    };
    repo.write("old_docs/runbook.md", "# Runbook\n")
        .write("old_docs/plans/t-904.md", "# Plan\n")
        .write(&listed, &listed_body("old_docs/runbook.md"))
        .write(
            &inline,
            "id = \"T-904\"\nstatus = \"cancelled\"\nowns = [\"old_docs/plans/t-904.md\"]\n",
        )
        .track();
    let manifest = repo.manifest("path\told_docs\tnew_docs\t\n");

    assert_eq!(apply(repo.root(), &manifest), 0);
    assert_eq!(
        repo.read(&listed),
        listed_body("new_docs/runbook.md").replace(
            "summary = \"Moved new_docs/runbook.md\"",
            "summary = \"Moved old_docs/runbook.md\""
        )
    );
    assert_eq!(
        repo.read(&inline),
        "id = \"T-904\"\nstatus = \"cancelled\"\nowns = [\"new_docs/plans/t-904.md\"]\n"
    );
    assert_eq!(verify(repo.root(), Some(&manifest)), 0);

    repo.write(&listed, &listed_body("old_docs/runbook.md"));
    assert_eq!(verify(repo.root(), Some(&manifest)), 1);
}

#[test]
fn relocate_files_moved_into_a_frozen_area_take_its_treatment() {
    let repo = FixtureRepository::new("into-archive");
    let plan = "# Plan\n\nWe will move `old_assets/a.json`; see [a](../old_assets/a.json).\n";
    let rows = "from\tto\nold_assets/a.json\tnew_assets/a.json\n";
    let archived = format!("{ARCHIVE_DIR}/records");
    repo.write("old_assets/a.json", "{}\n")
        .write("records/plan.md", plan)
        .write("records/rows.tsv", rows)
        .track();
    let manifest = repo.manifest(&format!(
        "path\trecords/plan.md\t{archived}/plan.md\t\n\
         path\trecords/rows.tsv\t{archived}/rows.tsv\t\n\
         path\told_assets\tnew_assets\t\n"
    ));

    assert_eq!(apply(repo.root(), &manifest), 0);
    let climb = "../".repeat(archived.split('/').count());
    assert_eq!(
        repo.read(&format!("{archived}/plan.md")),
        format!("# Plan\n\nWe will move `old_assets/a.json`; see [a]({climb}new_assets/a.json).\n")
    );
    assert_eq!(repo.read(&format!("{archived}/rows.tsv")), rows);
    assert_eq!(verify(repo.root(), Some(&manifest)), 0);
}

#[test]
fn relocate_binaries_attributes_and_large_file_pointers_move_unedited() {
    let repo = FixtureRepository::new("binaries");
    let binary = b"old_assets\0\x01\x02 binary payload";
    let pointer =
        "version https://git-lfs.github.com/spec/v1\noid sha256:00\nsize 12\nold_assets/x\n";
    repo.write_bytes("old_assets/blob.bin", binary)
        .write("old_assets/pointer.png", pointer)
        .write(
            "old_assets/marked.dat",
            "old_assets/marked.dat is text marked binary\n",
        )
        .write(".gitattributes", "old_assets/**/*.dat binary\n")
        .track();
    let manifest = repo.manifest("path\told_assets\tnew_assets\t\n");

    assert_eq!(apply(repo.root(), &manifest), 0);
    assert_eq!(repo.read_bytes("new_assets/blob.bin"), binary);
    assert_eq!(repo.read("new_assets/pointer.png"), pointer);
    assert_eq!(
        repo.read("new_assets/marked.dat"),
        "old_assets/marked.dat is text marked binary\n"
    );
    assert_eq!(repo.read(".gitattributes"), "new_assets/**/*.dat binary\n");
}

#[test]
fn relocate_unresolvable_literal_fails_apply_with_nothing_written() {
    let repo = FixtureRepository::new("unresolvable");
    let main = "fn main() {\n    let data = \"../data/x.json\";\n}\n";
    repo.write("crate_b/Cargo.toml", "[package]\nname = \"crate_b\"\n")
        .write("crate_b/src/main.rs", main)
        .write("crate_b/data/x.json", "{}\n")
        .write("data/x.json", "{}\n")
        .write("README.md", "Data lives in `data/x.json`.\n")
        .track();
    let manifest = repo.manifest("path\tdata\tdatasets\t\n");

    assert_eq!(dry_run(repo.root(), &manifest), 1);
    assert_eq!(apply(repo.root(), &manifest), 1);
    assert!(repo.exists("data/x.json"), "nothing moved");
    assert!(!repo.exists("datasets"));
    assert_eq!(repo.read("crate_b/src/main.rs"), main);
    assert_eq!(repo.read("README.md"), "Data lives in `data/x.json`.\n");
}

#[test]
fn relocate_refuses_a_move_into_an_existing_path() {
    let repo = FixtureRepository::new("refusal");
    repo.write("old_assets/a.json", "{}\n")
        .write("new_assets/b.json", "{}\n")
        .track();
    let manifest = repo.manifest("path\told_assets\tnew_assets\t\n");
    assert_eq!(dry_run(repo.root(), &manifest), 2);
    let missing = repo.manifest("path\tabsent_folder\tnew_place\t\n");
    assert_eq!(apply(repo.root(), &missing), 2);
    assert!(repo.exists("old_assets/a.json"));
}

#[test]
fn relocate_verify_is_red_on_a_planted_retired_spelling_and_green_after_removal() {
    let repo = FixtureRepository::new("verify");
    repo.write("new_assets/a.json", "{}\n")
        .write(
            "guides/old_assets/README.md",
            "# Another folder of that name\n",
        )
        .write(
            "README.md",
            "See `new_assets/a.json`, `guides/old_assets/README.md` and ./guides/old_assets/.\n",
        )
        .write(
            &format!("{ARCHIVE_DIR}/history.md"),
            "We moved `old_assets/a.json`.\n",
        )
        .track();
    let manifest = repo.manifest("path\told_assets\tnew_assets\t\n");
    assert_eq!(verify(repo.root(), Some(&manifest)), 0);

    repo.write("notes.md", "Planted: [a](/old_assets/a.json)\n");
    repo.track();
    assert_eq!(verify(repo.root(), Some(&manifest)), 1);

    repo.write("notes.md", "Planted: [a](/new_assets/a.json)\n");
    assert_eq!(verify(repo.root(), Some(&manifest)), 0);
}

#[test]
fn relocate_nested_moves_take_the_longest_row() {
    let repo = FixtureRepository::new("nested");
    repo.write("docs_root/old_assets/README.md", "# Assets docs\n")
        .write("docs_root/runbook.md", "# Runbook\n")
        .write("old_assets/a.json", "{}\n")
        .write(
            "README.md",
            "[r](/docs_root/runbook.md) [d](/docs_root/old_assets/README.md) [a](/old_assets/a.json)\n",
        )
        .track();
    let manifest = repo.manifest(
        "path\tdocs_root\tdocumentation_root\t\npath\tdocs_root/old_assets\tdocumentation_root/assets\t\npath\told_assets\tassets\t\n",
    );

    assert_eq!(apply(repo.root(), &manifest), 0);
    assert!(repo.exists("documentation_root/assets/README.md"));
    assert!(repo.exists("documentation_root/runbook.md"));
    assert!(repo.exists("assets/a.json"));
    assert_eq!(
        repo.read("README.md"),
        "[r](/documentation_root/runbook.md) [d](/documentation_root/assets/README.md) [a](/assets/a.json)\n"
    );
}

#[test]
fn relocate_prose_read_from_another_crate_folder_follows_the_move() {
    let repo = FixtureRepository::new("crate-prose");
    repo.write("shared_data/terrains/a.json", "{}\n")
        .write(
            "apps/web/server_v2/Cargo.toml",
            "[package]\nname = \"server\"\n",
        )
        .write("apps/web/server_v2/src/lib.rs", "\n")
        .write(
            "notes/server.md",
            "The server reads `../../../shared_data/terrains` from its crate folder.\n",
        )
        .track();
    let manifest = repo.manifest("path\tshared_data\tdata_sets\t\n");
    assert_eq!(apply(repo.root(), &manifest), 0);
    assert_eq!(
        repo.read("notes/server.md"),
        "The server reads `../../../data_sets/terrains` from its crate folder.\n"
    );
}

#[test]
fn relocate_moved_path_behind_an_unreadable_climb_is_unresolved() {
    let repo = FixtureRepository::new("unreadable-climb");
    let note = "Stale: `../../../../shared_data/terrains` names nothing from here.\n";
    repo.write("shared_data/terrains/a.json", "{}\n")
        .write("notes/stale.md", note)
        .track();
    let manifest = repo.manifest("path\tshared_data\tdata_sets\t\n");
    assert_eq!(dry_run(repo.root(), &manifest), 1);
    assert_eq!(apply(repo.root(), &manifest), 1);
    assert_eq!(repo.read("notes/stale.md"), note);
    assert!(repo.exists("shared_data/terrains/a.json"));
}

#[test]
fn relocate_verify_without_a_manifest_judges_every_committed_stage_manifest() {
    let repo = FixtureRepository::new("committed-manifests");
    repo.write("new_assets/a.json", "{}\n")
        .write("README.md", "See `new_assets/a.json`.\n")
        .track();
    assert_eq!(
        verify(repo.root(), None),
        2,
        "a missing manifests folder is a did-not-run"
    );

    let folder = manifests_folder();
    let header = "kind\tfrom\tto\tscope\n";
    repo.write(
        &format!("{folder}/{EXAMPLE_MANIFEST}"),
        &format!("{header}path\tnew_assets\tnewer_assets\t\n"),
    )
    .write(
        &format!("{folder}/stage_01.tsv"),
        &format!("{header}path\told_assets\tnew_assets\t\n"),
    )
    .track();
    assert_eq!(
        verify(repo.root(), None),
        0,
        "the example is never judged; the stage row holds"
    );

    repo.write("notes.md", "Planted: `old_assets/a.json`\n")
        .track();
    assert_eq!(verify(repo.root(), None), 1);

    repo.write(&format!("{folder}/stage_02.tsv"), "kind\tfrom\n")
        .track();
    std::fs::remove_file(repo.root().join("notes.md")).expect("remove the planted note");
    assert_eq!(
        verify(repo.root(), None),
        2,
        "an invalid stage manifest is a did-not-run"
    );
}
