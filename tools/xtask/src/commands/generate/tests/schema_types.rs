//! Contract codegen: the freshness check covers every file of the generated module tree
//! independently of whether Git tracks it, every generated file names the codegen command, and
//! the split keeps every typify item while holding each generated file to the production
//! source-size limit.
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
        fs::create_dir_all(schema_dir.join(schema).parent().unwrap()).unwrap();
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
    let expected = render_tree(&scratch.0).unwrap();
    let directory = scratch.0.join(OUTPUT_DIR);
    let error = || compare_tree(&expected, &directory).unwrap_err().to_string();
    let mut perturbed = vec![
        "mod.rs".to_string(),
        "operations/mod.rs".to_string(),
        "community_content/equipment_data_viewer/mod.rs".to_string(),
    ];
    for (_, output) in TARGETS {
        let prefix = format!("{output}/");
        let nested = expected
            .keys()
            .find(|relative| {
                relative
                    .strip_prefix(&prefix)
                    .is_some_and(|inner| inner.contains('/'))
            })
            .cloned()
            .unwrap_or_else(|| format!("{output}/error.rs"));
        perturbed.extend([format!("{output}/mod.rs"), nested]);
    }
    for relative in &perturbed {
        let path = directory.join(relative);
        let correct = fs::read_to_string(&path).unwrap();
        fs::remove_file(&path).unwrap();
        assert!(error().contains("missing generated output"), "{relative}");
        fs::write(&path, format!("{correct}\n// Unrepresented change\n")).unwrap();
        assert!(error().contains("stale generated output"), "{relative}");
        fs::write(&path, correct).unwrap();
    }
    for (schema, output) in TARGETS {
        let stray = directory.join(output).join("hand_written.rs");
        fs::write(&stray, "pub struct HandWritten;\n").unwrap();
        assert!(error().contains("stray generated output"), "{schema}");
        fs::remove_file(&stray).unwrap();

        let single_file = directory.join(format!("{output}.rs"));
        fs::write(&single_file, "// the retired single-file form\n").unwrap();
        assert!(error().contains("stray generated output"), "{schema}");
        fs::remove_file(&single_file).unwrap();
    }
    let stray_domain = directory.join("retired_domain/mod.rs");
    fs::create_dir_all(stray_domain.parent().unwrap()).unwrap();
    fs::write(&stray_domain, "// retired\n").unwrap();
    assert!(error().contains("stray generated output"));
    fs::remove_dir_all(stray_domain.parent().unwrap()).unwrap();
    verify_fresh(&scratch.0).unwrap();
}

#[test]
fn schema_codegen_removes_what_the_schemas_no_longer_produce() {
    let scratch = generated_scratch();
    let (_, output) = TARGETS[0];
    let directory = scratch.0.join(OUTPUT_DIR);
    let module = directory.join(output);
    fs::create_dir_all(module.join("retired_definition")).unwrap();
    fs::write(module.join("retired_definition/mod.rs"), "// retired\n").unwrap();
    fs::write(
        module.with_extension("rs"),
        "// the retired single-file form\n",
    )
    .unwrap();
    fs::create_dir_all(directory.join("retired_domain/retired_schema")).unwrap();
    fs::write(
        directory.join("retired_domain/retired_schema/mod.rs"),
        "// retired\n",
    )
    .unwrap();
    write_generated_modules(&scratch.0).unwrap();
    assert!(!module.join("retired_definition").exists());
    assert!(!module.with_extension("rs").exists());
    assert!(!directory.join("retired_domain").exists());
    verify_fresh(&scratch.0).unwrap();
}

#[test]
fn every_generated_file_names_the_codegen_command() {
    let files = render_tree(&repo_root().unwrap()).unwrap();
    for (relative, source) in &files {
        assert!(
            source.starts_with(
                "// Code generated from JSON Schema using `cargo xtask schema codegen`"
            ),
            "{relative}"
        );
        assert!(
            source.contains("regenerate with: cargo xtask ci schema-codegen"),
            "{relative}"
        );
    }
    for (_, output) in TARGETS {
        let segments: Vec<&str> = output.split('/').collect();
        for depth in 0..segments.len() {
            let folder = segments[..depth].join("/");
            let key = if folder.is_empty() {
                "mod.rs".to_string()
            } else {
                format!("{folder}/mod.rs")
            };
            let declaration = format!("pub mod {};", segments[depth]);
            assert!(
                files[&key].lines().any(|line| line == declaration),
                "{key} declares {}",
                segments[depth]
            );
        }
    }
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
