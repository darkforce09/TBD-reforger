//! Renders the module tree above the per-schema modules: one `mod.rs` for the generated root and
//! for every folder between it and a schema's module, declaring that folder's children with a
//! documentation line each. The schema table alone decides the tree, so adding, moving or removing
//! a schema re-renders every `mod.rs` it touches and nothing in the tree is written by hand.

use std::collections::{BTreeMap, BTreeSet};

use crate::error::{Result, refuse};

use super::module_files::Formatter;

/// The banner every module-tree file opens with: generated, never edited, and the command that
/// writes it.
const TREE_BANNER: &str = "// Code generated from JSON Schema using `cargo xtask schema codegen` \
                           (typify). DO NOT EDIT.\n// Source: contracts/definitions/ — regenerate \
                           with: cargo xtask ci schema-codegen\n\n";

/// Every `mod.rs` of the module tree, keyed by its path inside the generated root. `targets`
/// pairs each schema file with the `/`-separated module path its types are written to.
pub(super) fn render_tree_files(
    targets: &[(&str, &str)],
    format: Formatter<'_>,
) -> Result<BTreeMap<String, String>> {
    let modules: BTreeMap<&str, &str> = targets
        .iter()
        .map(|(schema, module)| (*module, *schema))
        .collect();
    if modules.len() != targets.len() {
        refuse!("two schemas share one generated module path");
    }
    let mut folders: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for module in modules.keys() {
        let segments: Vec<&str> = module.split('/').collect();
        for depth in 0..segments.len() {
            let folder = segments[..depth].join("/");
            if let Some(schema) = modules.get(folder.as_str()) {
                refuse!("the module of {schema} ({folder}) would also hold the module {module}");
            }
            folders
                .entry(folder)
                .or_default()
                .insert(segments[depth].to_string());
        }
    }
    let mut files = BTreeMap::new();
    for (folder, children) in &folders {
        let mut source = TREE_BANNER.to_string();
        source.push_str(&format!("//! {}\n\n", folder_description(folder, targets)));
        for child in children {
            let path = join(folder, child);
            let description = match modules.get(path.as_str()) {
                Some(schema) => {
                    format!("Types generated from `contracts/definitions/{schema}`.")
                }
                None => folder_description(&path, targets),
            };
            source.push_str(&format!("/// {description}\npub mod {child};\n"));
        }
        let key = join(folder, "mod.rs");
        files.insert(key, format(&source)?);
    }
    Ok(files)
}

/// What a folder of the tree holds: the generated root, an API domain, or the schemas of one
/// `contracts/definitions/` subfolder.
fn folder_description(folder: &str, targets: &[(&str, &str)]) -> String {
    if folder.is_empty() {
        return "Contract types generated from the JSON Schemas in `contracts/definitions/`, one \
                module per API domain that serves or reads them."
            .to_string();
    }
    if !folder.contains('/') {
        return format!(
            "Types generated from the contract schemas the API's `{folder}` domain serves or reads."
        );
    }
    let prefix = format!("{folder}/");
    let schema_folders: BTreeSet<&str> = targets
        .iter()
        .filter(|(_, module)| module.starts_with(&prefix))
        .map(|(schema, _)| schema.rsplit_once('/').map_or("", |(parent, _)| parent))
        .collect();
    match schema_folders.into_iter().collect::<Vec<_>>().as_slice() {
        [single] if !single.is_empty() => {
            format!("Types generated from the schemas in `contracts/definitions/{single}/`.")
        }
        _ => format!("Types generated from the contract schemas grouped under `{folder}`."),
    }
}

/// `folder/name`, or `name` at the root.
fn join(folder: &str, name: &str) -> String {
    if folder.is_empty() {
        name.to_string()
    } else {
        format!("{folder}/{name}")
    }
}

#[cfg(test)]
#[path = "tests/module_tree.rs"]
mod tests;
