//! `--verify` over manifests that build on each other: an earlier manifest's `rust_path` scope is
//! judged where the later manifests' `path` rows put it, in the order the manifests entered the
//! history (uncommitted ones last), an order a move of the manifests folder leaves unchanged; a
//! scope a later move, or the manifest's own rows file by file, emptied judges nothing, and a
//! scope no move explains still fails closed.

use std::path::PathBuf;

use super::file_treatment::manifests_folder;
use super::fixture_repository::FixtureRepository;
use super::manifest_chronology::chronological_manifests;
use super::repository_files::git;
use super::{apply, dry_run, verify};

const HEADER: &str = "kind\tfrom\tto\tscope\n";

/// The rewrite of the widgets layer, scoped to the folder a later manifest moves or empties.
const SCOPED_REWRITE: &str = "rust_path\tcrate::old_layer::\tcrate::controls::\tapp/src/widgets\n";

fn manifest_path(name: &str) -> String {
    format!("{}/{name}", manifests_folder())
}

fn crate_with_widgets(tag: &str) -> FixtureRepository {
    let repo = FixtureRepository::new(tag);
    repo.write("app/Cargo.toml", "[package]\nname = \"app\"\n")
        .write("app/src/lib.rs", "pub mod controls;\n");
    repo
}

#[test]
fn relocate_verify_judges_an_earlier_scope_where_a_later_manifest_moved_it() {
    let repo = crate_with_widgets("chronology-moved-scope");
    // Committed first although its name sorts last: the history, not the name, orders them.
    repo.write(
        &manifest_path("stage_b_rewrite.tsv"),
        &format!("{HEADER}{SCOPED_REWRITE}"),
    )
    .write(
        "app/src/widgets/mod.rs",
        "pub use crate::controls::Widget;\n",
    )
    .commit("stage b: rewrite the widgets layer");
    std::fs::remove_dir_all(repo.root().join("app/src/widgets")).expect("move the scope away");
    repo.write(
        &manifest_path("stage_a_move.tsv"),
        &format!("{HEADER}path\tapp/src/widgets\tapp/src/controls\t\n"),
    )
    .write(
        "app/src/controls/mod.rs",
        "pub use crate::controls::Widget;\n",
    )
    .commit("stage a: move the widgets folder");

    assert_eq!(
        verify(repo.root(), None),
        0,
        "the rewrite's scope is judged at the folder the later move made"
    );
    let named = PathBuf::from(repo.root()).join(manifest_path("stage_b_rewrite.tsv"));
    assert_eq!(
        verify(repo.root(), Some(&named)),
        0,
        "a named committed manifest composes through the manifests after it"
    );

    repo.write(
        "app/src/controls/mod.rs",
        "pub use crate::old_layer::Widget;\n",
    );
    assert_eq!(
        verify(repo.root(), None),
        1,
        "a retired prefix in the moved scope is still found"
    );
}

#[test]
fn relocate_verify_passes_a_scope_a_later_manifest_emptied_and_fails_an_unexplained_one() {
    let repo = crate_with_widgets("chronology-emptied-scope");
    repo.write(
        &manifest_path("stage_b_rewrite.tsv"),
        &format!("{HEADER}{SCOPED_REWRITE}"),
    )
    .write(
        "app/src/widgets/mod.rs",
        "pub use crate::controls::Widget;\n",
    )
    .commit("stage b: rewrite the widgets layer");
    std::fs::remove_dir_all(repo.root().join("app/src/widgets")).expect("empty the scope");
    // Not committed yet: an uncommitted manifest comes after every committed one.
    repo.write(
        &manifest_path("stage_a_flatten.tsv"),
        &format!("{HEADER}path\tapp/src/widgets/mod.rs\tapp/src/controls.rs\t\n"),
    )
    .write("app/src/controls.rs", "pub use crate::controls::Widget;\n")
    .track();

    assert_eq!(
        verify(repo.root(), None),
        0,
        "a scope whose files a later move took away judges nothing there"
    );

    repo.write(
        &manifest_path("stage_c_typo.tsv"),
        &format!("{HEADER}rust_path\tcrate::gone::\tcrate::controls::\tapp/src/nowhere\n"),
    )
    .track();
    assert_eq!(
        verify(repo.root(), None),
        2,
        "a missing scope no later manifest explains is a did-not-run"
    );
}

/// The file names of the checkout's stage manifests, oldest first.
fn manifest_order(repo: &FixtureRepository) -> Vec<String> {
    chronological_manifests(repo.root())
        .expect("order the fixture manifests")
        .iter()
        .map(|path| {
            path.file_name()
                .expect("a manifest file name")
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

#[test]
fn relocate_manifest_order_survives_a_move_of_the_manifests_folder() {
    let repo = FixtureRepository::new("chronology-folder-move");
    let earlier = "documentation/stage_manifests";
    let row = |n: usize| format!("{HEADER}path\tapp/step_{n}\tapp/done_{n}\t\n");
    repo.write(&format!("{earlier}/zeta_first.tsv"), &row(1))
        .commit("stage one");
    repo.write(&format!("{earlier}/alpha_second.tsv"), &row(2))
        .write(&format!("{earlier}/beta_second.tsv"), &row(3))
        .commit("stage two");
    repo.write(&format!("{earlier}/mid_third.tsv"), &row(4))
        .commit("stage three");
    let committed = [
        "zeta_first.tsv",
        "alpha_second.tsv",
        "beta_second.tsv",
        "mid_third.tsv",
    ];

    let target = manifests_folder();
    let parent = target.rsplit_once('/').map_or("", |(folder, _)| folder);
    std::fs::create_dir_all(repo.root().join(parent)).expect("create the destination parent");
    git(repo.root(), &["mv", earlier, &target]).expect("git mv the manifests folder");
    assert_eq!(
        manifest_order(&repo),
        committed,
        "a staged move of the folder keeps every manifest's place"
    );

    repo.commit("move the manifests folder");
    repo.write(&manifest_path("aardvark_fourth.tsv"), &row(5))
        .commit("stage four");
    repo.write(&manifest_path("aaa_untracked.tsv"), &row(6));
    let mut expected = committed.to_vec();
    expected.extend(["aardvark_fourth.tsv", "aaa_untracked.tsv"]);
    assert_eq!(
        manifest_order(&repo),
        expected,
        "a committed move keeps the order; later and untracked manifests follow"
    );
}

#[test]
fn relocate_manifest_that_empties_its_own_scope_file_by_file_applies_and_verifies() {
    let repo = crate_with_widgets("own-rows-empty-scope");
    repo.write("app/src/widgets/a.rs", "use crate::old_layer::A;\n")
        .write("app/src/widgets/b.rs", "use crate::old_layer::B;\n")
        .track();
    let manifest = repo.manifest(
        "path\tapp/src/widgets/a.rs\tapp/src/controls/a.rs\t\n\
         path\tapp/src/widgets/b.rs\tapp/src/controls/b.rs\t\n\
         rust_path\tcrate::old_layer::\tcrate::controls::\tapp/src/widgets\n",
    );

    assert_eq!(
        dry_run(repo.root(), &manifest),
        0,
        "the planned tree's verification composes the scope the rows emptied"
    );
    assert_eq!(apply(repo.root(), &manifest), 0);
    assert_eq!(
        repo.read("app/src/controls/a.rs"),
        "use crate::controls::A;\n"
    );
    assert!(
        !repo
            .tracked()
            .iter()
            .any(|path| path.starts_with("app/src/widgets/")),
        "the rows emptied the scope"
    );
    assert_eq!(verify(repo.root(), Some(&manifest)), 0);
}

#[test]
fn relocate_verify_composes_a_scope_emptied_by_its_own_rows_through_later_manifests() {
    let repo = crate_with_widgets("chronology-own-rows-emptied");
    repo.write(
        &manifest_path("stage_b_split.tsv"),
        &format!(
            "{HEADER}path\tapp/src/widgets/a.rs\tapp/src/controls/a.rs\t\n\
             path\tapp/src/widgets/b.rs\tapp/src/controls/b.rs\t\n{SCOPED_REWRITE}"
        ),
    )
    .write("app/src/controls/a.rs", "use crate::controls::A;\n")
    .write("app/src/controls/b.rs", "use crate::controls::B;\n")
    .commit("stage b: split the widgets folder file by file");
    repo.write(
        &manifest_path("stage_a_later.tsv"),
        &format!("{HEADER}path\tapp/src/controls/b.rs\tapp/src/parts/b.rs\t\n"),
    )
    .commit("stage a: a later move");
    std::fs::remove_file(repo.root().join("app/src/controls/b.rs")).expect("move b away");
    repo.write("app/src/parts/b.rs", "use crate::controls::B;\n")
        .track();

    assert_eq!(
        verify(repo.root(), None),
        0,
        "a scope its own manifest emptied file by file holds, later manifests composed"
    );
    let named = PathBuf::from(repo.root()).join(manifest_path("stage_b_split.tsv"));
    assert_eq!(verify(repo.root(), Some(&named)), 0);

    repo.write(
        &manifest_path("stage_c_partial.tsv"),
        &format!(
            "{HEADER}path\tapp/src/gadgets/a.rs\tapp/src/parts/a.rs\t\n\
             rust_path\tcrate::gone::\tcrate::parts::\tapp/src/nowhere\n"
        ),
    )
    .track();
    assert_eq!(
        verify(repo.root(), None),
        2,
        "a missing scope none of its manifest's rows lies below is still a did-not-run"
    );
}
