//! The frontend source trees the whole-frontend source pins walk.
//!
//! **Role:** one answer to "which source trees make up the frontend" for the pins that walk all
//! of it: the keymap census's one-extractor pin and the validation seam's defined-once pin.
//! **Position:** test support of this crate; reads the repository through the root finder of
//! `frontend_test_support`.
//! **Signals & state:** none; reads the file system.
//! **Invariants:** the list holds the app's `src` and every `crates/frontend/<layer>/<crate>/src`
//! and always the five Mission Creator crates' trees; a missing one fails the calling test.

/// The source trees the whole-frontend pins walk: the app's `src` and the `src` of every crate
/// under `crates/frontend/<layer>/`. Fails closed when a Mission Creator crate's tree is missing,
/// so a moved crate cannot shrink the walk in silence.
pub(crate) fn frontend_source_roots(repository: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut roots = vec![repository.join("apps/frontend/src")];
    let layers = std::fs::read_dir(repository.join("crates/frontend"))
        .unwrap_or_else(|e| panic!("cannot read crates/frontend: {e}"));
    for layer in layers {
        let layer = layer.expect("read_dir entry").path();
        if !layer.is_dir() {
            continue;
        }
        let crates = std::fs::read_dir(&layer)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", layer.display()));
        for crate_folder in crates {
            let src = crate_folder.expect("read_dir entry").path().join("src");
            if src.is_dir() {
                roots.push(src);
            }
        }
    }
    roots.sort();
    for editor_crate in [
        "mission_creator_state",
        "mission_creator_engine_bridge",
        "mission_creator_session",
        "mission_creator_arsenal",
        "mission_creator_workspace",
    ] {
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
