//! The frontend source trees the whole-frontend source pins walk.
//!
//! **Role:** one answer to "which source trees make up the frontend" for the pins that walk all
//! of it: the keymap census's one-extractor pin and the validation seam's defined-once pin.
//! **Position:** test support of this crate; takes the walk of
//! `frontend_test_support::frontend_source_roots` over the repository its root finder returns.
//! **Signals & state:** none; reads the file system.
//! **Invariants:** the list holds the `src` of every `crates/frontend/<layer>/<crate>`, the app's
//! in `shell/` among them, each package once, and always the five Mission Creator crates' trees;
//! a missing one or a package listed twice fails the calling test.

use frontend_test_support::frontend_source_roots::{
    assert_each_package_read_once, frontend_source_roots as frontend_package_source_roots,
};
use std::path::{Path, PathBuf};

/// The Mission Creator crates under `crates/frontend/workspaces/` whose trees the pins never run
/// without.
const MISSION_CREATOR_CRATES: [&str; 5] = [
    "mission_creator_state",
    "mission_creator_engine_bridge",
    "mission_creator_session",
    "mission_creator_arsenal",
    "mission_creator_workspace",
];

/// The source trees the whole-frontend pins walk: the `src` of every crate under
/// `crates/frontend/<layer>/`, one per package. Fails closed when a Mission Creator crate's tree
/// is missing, so a moved crate cannot shrink the walk in silence.
pub(crate) fn frontend_source_roots(repository: &Path) -> Vec<PathBuf> {
    let roots = frontend_package_source_roots(repository);
    for editor_crate in MISSION_CREATOR_CRATES {
        let src = repository
            .join("crates/frontend/workspaces")
            .join(editor_crate)
            .join("src");
        assert!(
            roots.contains(&src),
            "the walk lost the {editor_crate} source tree ({})",
            src.display()
        );
    }
    roots
}

/// The whole-frontend pins read each frontend package once: the app's tree in `shell/` is walked
/// once, as one of the frontend packages.
#[test]
fn the_whole_frontend_walk_reads_every_package_once() {
    let repository =
        frontend_test_support::repository_root::repository_root(env!("CARGO_MANIFEST_DIR"));
    let roots = frontend_source_roots(&repository);
    assert_each_package_read_once(&roots);
    let app = repository.join("crates/frontend/shell/frontend_application/src");
    assert!(
        roots.contains(&app),
        "the walk skips the app's {}",
        app.display()
    );
}
