//! Tests for [`super`] — the files a module declaration loads, by the Rust reference's path rules.

use super::*;

fn paths(files: &[ModuleFile]) -> Vec<(String, bool)> {
    files
        .iter()
        .map(|file| (file.path.to_string_lossy().into_owned(), file.owns_folder))
        .collect()
}

fn owned(path: &str) -> (String, bool) {
    (path.to_owned(), true)
}

fn named(path: &str) -> (String, bool) {
    (path.to_owned(), false)
}

#[test]
fn module_declarations_a_plain_declaration_has_two_candidates_beside_a_folder_owner() {
    let files = declared_module_files(Path::new("src/lib.rs"), true, "pub mod data;\n");
    assert_eq!(
        paths(&files),
        vec![named("src/data.rs"), owned("src/data/mod.rs")]
    );
}

#[test]
fn module_declarations_a_named_file_resolves_its_children_under_its_stem() {
    let files = declared_module_files(Path::new("src/data.rs"), false, "mod model;\n");
    assert_eq!(
        paths(&files),
        vec![named("src/data/model.rs"), owned("src/data/model/mod.rs")]
    );
}

#[test]
fn module_declarations_a_path_attribute_resolves_beside_the_declaring_file() {
    for (attributes, expected) in [
        (
            "#[cfg(test)]\n#[path = \"tests/data.rs\"]\nmod tests;\n",
            "src/tests/data.rs",
        ),
        (
            "#[cfg(test)] #[path = \"tests/data.rs\"] mod tests;\n",
            "src/tests/data.rs",
        ),
        (
            "#[cfg_attr(test, path = \"tests/data.rs\")]\nmod tests;\n",
            "src/tests/data.rs",
        ),
        (
            "/// Docs.\n#[path = \"../tests/frontend.rs\"]\npub(crate) mod tests;\n",
            "tests/frontend.rs",
        ),
    ] {
        let files = declared_module_files(Path::new("src/data.rs"), false, attributes);
        assert_eq!(paths(&files), vec![owned(expected)], "{attributes}");
    }
}

#[test]
fn module_declarations_inline_modules_add_folders_under_the_declaring_files_base() {
    let text =
        "mod outer {\n    mod inner;\n    #[path = \"x.rs\"]\n    mod pathed;\n}\nmod after;\n";
    let files = declared_module_files(Path::new("src/data.rs"), false, text);
    assert_eq!(
        paths(&files),
        vec![
            named("src/data/outer/inner.rs"),
            owned("src/data/outer/inner/mod.rs"),
            owned("src/data/outer/x.rs"),
            named("src/data/after.rs"),
            owned("src/data/after/mod.rs"),
        ]
    );
}

#[test]
fn module_declarations_quoted_commented_and_inline_bodies_declare_no_file() {
    let text = "// mod commented;\n/* mod blocked; */\nconst SOURCE: &str = \"mod quoted;\";\n\
                mod inline { fn f() {} }\nlet module = 1;\nfn modify() {}\n\
                macro_rules! m { ($name:ident) => { mod $name; } }\n";
    let files = declared_module_files(Path::new("src/lib.rs"), true, text);
    assert_eq!(paths(&files), Vec::<(String, bool)>::new());
}

#[test]
fn module_declarations_an_attribute_applies_to_the_next_item_only() {
    let text = "#[path = \"elsewhere.rs\"]\nfn helper() {}\nmod plain;\n";
    let files = declared_module_files(Path::new("src/lib.rs"), true, text);
    assert_eq!(
        paths(&files),
        vec![named("src/plain.rs"), owned("src/plain/mod.rs")]
    );
}

#[test]
fn module_declarations_parent_segments_fold() {
    assert_eq!(
        normalised(Path::new("src/a/../tests/./b.rs")),
        PathBuf::from("src/tests/b.rs")
    );
}
