//! `--verify` over manifests that build on each other: an earlier manifest's `rust_path` scope is
//! judged where the later manifests' `path` rows put it, in the order the manifests entered the
//! history (uncommitted ones last), a scope a later move emptied judges nothing, and a scope no
//! later move explains still fails closed.

use std::path::PathBuf;

use super::file_treatment::manifests_folder;
use super::fixture_repository::FixtureRepository;
use super::verify;

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
