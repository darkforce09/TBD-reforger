//! Contract codegen: the freshness check covers every file of every generated module directory
//! independently of whether Git tracks it, and the split keeps every typify item while holding
//! each generated file to the production source-size limit.
use super::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("remove owned codegen scratch directory");
    }
}
fn scratch() -> Scratch {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "tbd-schema-{}-{now}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&path).expect("exclusively create scratch directory");
    Scratch(path)
}

/// A scratch repository holding copies of every target schema and freshly generated modules.
fn generated_scratch() -> Scratch {
    let source = repo_root().unwrap();
    let scratch = scratch();
    let schema_dir = contract_definitions_dir(&scratch.0);
    fs::create_dir_all(&schema_dir).unwrap();
    for (schema, _) in TARGETS {
        fs::copy(
            contract_definitions_dir(&source).join(schema),
            schema_dir.join(schema),
        )
        .unwrap();
    }
    write_generated_modules(&scratch.0).unwrap();
    scratch
}

#[test]
fn schema_freshness_detects_missing_changed_and_stray_outputs_without_git() {
    let scratch = generated_scratch();
    assert!(!scratch.0.join(".git").exists());
    verify_fresh(&scratch.0).unwrap();
    for (schema, output) in TARGETS {
        let expected = render_module(&scratch.0, schema).unwrap();
        let directory = scratch.0.join(API_SOURCE_DIR).join(output);
        let error = |directory: &Path| {
            compare_module(&expected, directory)
                .unwrap_err()
                .to_string()
        };
        let nested = expected
            .keys()
            .find(|relative| relative.contains('/'))
            .cloned()
            .unwrap_or_else(|| "mod.rs".to_string());
        for relative in ["mod.rs".to_string(), nested] {
            let path = directory.join(&relative);
            let correct = fs::read_to_string(&path).unwrap();
            fs::remove_file(&path).unwrap();
            assert!(
                error(&directory).contains("missing generated output"),
                "{schema} {relative}"
            );
            fs::write(&path, format!("{correct}\n// Unrepresented change\n")).unwrap();
            assert!(
                error(&directory).contains("stale generated output"),
                "{schema} {relative}"
            );
            fs::write(&path, correct).unwrap();
        }

        let stray = directory.join("hand_written.rs");
        fs::write(&stray, "pub struct HandWritten;\n").unwrap();
        assert!(
            error(&directory).contains("stray generated output"),
            "{schema}"
        );
        fs::remove_file(&stray).unwrap();

        let single_file = directory.with_extension("rs");
        fs::write(&single_file, "// the retired single-file form\n").unwrap();
        assert!(
            error(&directory).contains("stray generated output"),
            "{schema}"
        );
        fs::remove_file(&single_file).unwrap();
        compare_module(&expected, &directory).unwrap();
    }
    verify_fresh(&scratch.0).unwrap();
}

#[test]
fn schema_codegen_removes_what_the_schemas_no_longer_produce() {
    let scratch = generated_scratch();
    let (_, output) = TARGETS[0];
    let directory = scratch.0.join(API_SOURCE_DIR).join(output);
    fs::create_dir_all(directory.join("retired_definition")).unwrap();
    fs::write(directory.join("retired_definition/mod.rs"), "// retired\n").unwrap();
    fs::write(
        directory.with_extension("rs"),
        "// the retired single-file form\n",
    )
    .unwrap();
    write_generated_modules(&scratch.0).unwrap();
    assert!(!directory.join("retired_definition").exists());
    assert!(!directory.with_extension("rs").exists());
    verify_fresh(&scratch.0).unwrap();
}

/// The type and impl items of a parsed file, counted by their rendered signature, plus the
/// items inside inline modules.
fn item_signatures(items: &[syn::Item], out: &mut Vec<String>) {
    for item in items {
        match item {
            syn::Item::Mod(module) => {
                if let Some((_, inner)) = &module.content {
                    item_signatures(inner, out);
                }
            }
            syn::Item::Use(_) => {}
            syn::Item::Struct(item) => out.push(format!("struct {}", item.ident)),
            syn::Item::Enum(item) => out.push(format!("enum {}", item.ident)),
            syn::Item::Type(item) => out.push(format!("type {}", item.ident)),
            syn::Item::Impl(item) => {
                let mut implementation = item.clone();
                implementation.items.clear();
                implementation.attrs.clear();
                out.push(
                    prettyplease::unparse(&syn::File {
                        shebang: None,
                        attrs: Vec::new(),
                        items: vec![syn::Item::Impl(implementation)],
                    })
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .replace("self :: error ::", "error ::")
                    .replace("super :: super :: error ::", "error ::")
                    .replace("super :: error ::", "error ::"),
                );
            }
            other => out.push(format!(
                "unexpected {}",
                prettyplease::unparse(&syn::File {
                    shebang: None,
                    attrs: Vec::new(),
                    items: vec![other.clone()],
                })
            )),
        }
    }
}

#[test]
fn schema_split_keeps_every_typify_item_and_bounds_every_file() {
    let root = repo_root().unwrap();
    for (schema, _) in TARGETS {
        let (typified, _) = typify_output(&root, schema).unwrap();
        let mut expected = Vec::new();
        item_signatures(&typified.items, &mut expected);
        let files = render_module(&root, schema).unwrap();
        let mut rendered = Vec::new();
        for (path, source) in &files {
            let lines = source.lines().count();
            assert!(
                lines <= module_files::PRODUCTION_LINE_LIMIT,
                "{schema}: {path} has {lines} lines"
            );
            let parsed =
                syn::parse_file(source).unwrap_or_else(|e| panic!("{schema}: {path}: {e}"));
            item_signatures(&parsed.items, &mut rendered);
        }
        expected.sort();
        rendered.sort();
        assert_eq!(
            rendered, expected,
            "{schema}: the split must keep exactly typify's items"
        );
        assert!(files.contains_key("mod.rs"), "{schema}");
        assert!(files.contains_key("error.rs"), "{schema}");
    }
}
