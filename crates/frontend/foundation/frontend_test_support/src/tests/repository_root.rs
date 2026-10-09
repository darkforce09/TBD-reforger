//! The repository file reads: their root is the workspace root above the calling crate, the
//! calling source file resolves to its folder, and a folder with no root above it is refused.

use super::{repository_root, repository_text, source_file_folder};
use std::path::Path;

/// From this crate's manifest folder the walk ends at the folder holding the root marker, the root
/// `Cargo.lock`, the root manifest with its `[workspace]` table, and the `contracts/` tree the
/// goldens live in.
#[test]
fn the_root_is_the_workspace_folder_above_the_crate() {
    let root = repository_root(env!("CARGO_MANIFEST_DIR"));
    assert!(
        root.join(::repository_root::ROOT_MARKER).is_file(),
        "{}",
        root.display()
    );
    assert!(root.join("Cargo.lock").is_file(), "{}", root.display());
    assert!(
        root.join("contracts/fixtures/api_goldens").is_dir(),
        "{}",
        root.display()
    );
    assert!(
        Path::new(env!("CARGO_MANIFEST_DIR")).starts_with(&root),
        "the root is an ancestor of the manifest folder"
    );
    assert!(
        repository_text(env!("CARGO_MANIFEST_DIR"), "Cargo.toml")
            .lines()
            .any(|line| line.trim() == "[workspace]"),
        "the root manifest declares the workspace"
    );
}

/// The folder of this very file resolves through `file!()`, wherever the crate sits.
#[test]
fn the_calling_source_file_resolves_to_its_folder() {
    let folder = source_file_folder(env!("CARGO_MANIFEST_DIR"), file!());
    assert!(
        folder.join("repository_root.rs").is_file(),
        "{}",
        folder.display()
    );
}

/// A folder with no workspace above it is refused with the path it searched, never answered with
/// a guess.
#[test]
fn a_folder_outside_any_workspace_has_no_root() {
    let outside = std::env::temp_dir();
    let refused = std::panic::catch_unwind(|| repository_root(&outside.to_string_lossy()));
    let message = refused.expect_err("no workspace root above the temporary folder");
    let message = message
        .downcast_ref::<String>()
        .expect("the refusal is a formatted message");
    assert!(message.contains("no repository root"), "{message}");
}
