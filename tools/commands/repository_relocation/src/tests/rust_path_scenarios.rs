//! Whole relocation runs for the `rust_path` and `text` rows, and the committed example manifest.

use super::file_treatment::manifests_folder;
use super::fixture_repository::FixtureRepository;
use super::relocation_modes::EXAMPLE_MANIFEST;
use super::{apply, dry_run, verify};

/// A small crate whose `old_layer::widgets` module moves to `controls`, reached by absolute paths,
/// `use` trees, doc links, `self::` and `super::` chains in both directions.
fn widget_crate(repo: &FixtureRepository) {
    repo.write("app/Cargo.toml", "[package]\nname = \"app\"\n")
        .write(
            "app/src/lib.rs",
            "pub mod old_layer;\n\
             use crate::old_layer::widgets;\n\n\
             /// Builds a [`crate::old_layer::widgets::Widget`] from [`crate::old_layer::widgets`].\n\
             pub fn make() -> crate::old_layer::widgets::Widget {\n    crate::old_layer::widgets::Widget\n}\n\n\
             pub fn again() -> widgets::Widget {\n    widgets::Widget\n}\n\n\
             const LABEL: &str = \"crate::old_layer::widgets::Widget\";\n",
        )
        .write(
            "app/src/old_layer/mod.rs",
            "pub mod other;\npub mod pages;\npub mod widgets;\n\npub use self::widgets::Widget;\n",
        )
        .write("app/src/old_layer/other.rs", "pub struct Thing;\n")
        .write(
            "app/src/old_layer/pages.rs",
            "use crate::old_layer::{other::Thing, widgets::{self, Widget}};\n\n\
             pub fn both() -> (Thing, Widget) {\n    (Thing, widgets::Widget)\n}\n",
        )
        .write(
            "app/src/old_layer/widgets/mod.rs",
            "mod inner;\n\nuse super::other::Thing;\n\npub struct Widget;\n\n\
             pub fn thing() -> Thing {\n    super::other::Thing\n}\n\n\
             #[cfg(test)]\nmod tests {\n    use super::*;\n}\n",
        )
        .write(
            "app/src/old_layer/widgets/inner.rs",
            "use super::super::pages::both;\nuse super::Widget;\n\n\
             pub fn widget() -> Widget {\n    both().1\n}\n",
        )
        .track();
}

#[test]
fn relocate_rust_path_rewrites_use_trees_doc_links_and_super_chains() {
    let repo = FixtureRepository::new("rust-paths");
    widget_crate(&repo);
    let manifest = repo.manifest(
        "path\tapp/src/old_layer/widgets\tapp/src/controls\t\n\
         rust_path\tcrate::old_layer::widgets::\tcrate::controls::\tapp\n",
    );

    assert_eq!(dry_run(repo.root(), &manifest), 0);
    assert_eq!(apply(repo.root(), &manifest), 0);
    let library = repo.read("app/src/lib.rs");
    assert!(
        library.contains("use crate::controls as widgets;\n"),
        "{library}"
    );
    assert!(
        library.contains("[`crate::controls::Widget`] from [`crate::controls`]."),
        "{library}"
    );
    assert!(
        library
            .contains("pub fn make() -> crate::controls::Widget {\n    crate::controls::Widget\n}")
    );
    assert!(
        library.contains("pub fn again() -> widgets::Widget"),
        "{library}"
    );
    assert!(library.contains("\"crate::controls::Widget\""), "{library}");
    assert!(
        repo.read("app/src/old_layer/mod.rs")
            .contains("pub use crate::controls::Widget;\n")
    );
    assert!(repo.read("app/src/old_layer/pages.rs").starts_with(
        "use crate::{old_layer::other::Thing, controls::{self as widgets, Widget}};\n"
    ));
    let widgets = repo.read("app/src/controls/mod.rs");
    assert!(
        widgets.contains("use crate::old_layer::other::Thing;\n"),
        "{widgets}"
    );
    assert!(
        widgets.contains("    crate::old_layer::other::Thing\n"),
        "{widgets}"
    );
    assert!(
        widgets.contains("mod tests {\n    use super::*;\n}"),
        "{widgets}"
    );
    let inner = repo.read("app/src/controls/inner.rs");
    assert!(
        inner.starts_with("use crate::old_layer::pages::both;\nuse super::Widget;\n"),
        "{inner}"
    );
}

#[test]
fn relocate_rust_path_verify_finds_a_surviving_prefix() {
    let repo = FixtureRepository::new("rust-verify");
    repo.write("app/Cargo.toml", "[package]\nname = \"app\"\n")
        .write(
            "app/src/lib.rs",
            "pub mod controls;\npub use crate::controls::Widget;\n",
        )
        .write("app/src/controls/mod.rs", "pub struct Widget;\n")
        .write(
            "outside/note.rs",
            "// crate::old_layer::widgets::Widget outside the scope\n",
        )
        .track();
    let manifest =
        repo.manifest("rust_path\tcrate::old_layer::widgets::\tcrate::controls::\tapp\n");
    assert_eq!(verify(repo.root(), Some(&manifest)), 0);

    repo.write(
        "app/src/lib.rs",
        "pub mod controls;\npub use crate::old_layer::{widgets::Widget};\n",
    );
    assert_eq!(verify(repo.root(), Some(&manifest)), 1);
}

#[test]
fn relocate_text_rows_respect_word_boundaries() {
    let repo = FixtureRepository::new("text-rows");
    repo.write(
        "Cargo.toml",
        "[dependencies]\nshop-server = { path = \"server\" }\nshop-server-types = { path = \"types\" }\n",
    )
    .write(
        "src/main.rs",
        "use shop_server::Thing;\nuse shop_server_types::Other;\n// unit: tbd-shop-server.service\n",
    )
    .write(
        "notes.md",
        "Run `cargo test -p shop-server` (not shop-server-types, not myshop-server).\n",
    )
    .track();
    let manifest = repo.manifest("text\tshop-server\tserver\t\ntext\tshop_server\tserver\t\n");

    assert_eq!(apply(repo.root(), &manifest), 0);
    assert_eq!(
        repo.read("Cargo.toml"),
        "[dependencies]\nserver = { path = \"server\" }\nshop-server-types = { path = \"types\" }\n"
    );
    assert_eq!(
        repo.read("src/main.rs"),
        "use server::Thing;\nuse shop_server_types::Other;\n// unit: tbd-shop-server.service\n"
    );
    assert_eq!(
        repo.read("notes.md"),
        "Run `cargo test -p server` (not shop-server-types, not myshop-server).\n"
    );
}

#[test]
fn relocate_text_row_glob_scope_limits_the_files() {
    let repo = FixtureRepository::new("text-glob");
    repo.write("a/one.toml", "name = \"shop-server\"\n")
        .write("a/one.md", "shop-server\n")
        .write("b/two.toml", "name = \"shop-server\"\n")
        .track();
    let manifest = repo.manifest("text\tshop-server\tserver\ta/**/*.toml\n");
    assert_eq!(apply(repo.root(), &manifest), 0);
    assert_eq!(repo.read("a/one.toml"), "name = \"server\"\n");
    assert_eq!(repo.read("a/one.md"), "shop-server\n");
    assert_eq!(repo.read("b/two.toml"), "name = \"shop-server\"\n");
}

#[test]
fn relocate_example_manifest_parses_and_applies() {
    let root = tool_test_support::test_repo_root();
    let example = root.join(manifests_folder()).join(EXAMPLE_MANIFEST);
    let text = std::fs::read_to_string(&example).expect("the example manifest is committed");
    let rows = super::manifest::parse_manifest(&text).expect("the example manifest parses");
    assert_eq!(rows.len(), 3, "one row of each kind");

    let repo = FixtureRepository::new("example");
    repo.write("example_tree/Cargo.toml", "[package]\nname = \"widgets-v1\"\n")
        .write(
            "example_tree/src/lib.rs",
            "pub mod widgets_v1;\n\n/// The [`crate::widgets_v1::Gadget`].\npub use crate::widgets_v1::Gadget;\n",
        )
        .write("example_tree/src/widgets_v1/mod.rs", "pub struct Gadget;\n")
        .write(
            "example_tree/README.md",
            "The `widgets-v1` crate keeps its code in `example_tree/src/widgets_v1/`.\n",
        )
        .track();
    let manifest = repo.manifest("");
    std::fs::write(&manifest, &text).expect("copy the example manifest");

    assert_eq!(apply(repo.root(), &manifest), 0);
    assert!(repo.exists("example_tree/src/widgets/mod.rs"));
    assert_eq!(
        repo.read("example_tree/src/lib.rs"),
        "pub mod widgets_v1;\n\n/// The [`crate::widgets::Gadget`].\npub use crate::widgets::Gadget;\n"
    );
    assert_eq!(
        repo.read("example_tree/Cargo.toml"),
        "[package]\nname = \"widgets\"\n"
    );
    assert_eq!(
        repo.read("example_tree/README.md"),
        "The `widgets` crate keeps its code in `example_tree/src/widgets/`.\n"
    );
}
