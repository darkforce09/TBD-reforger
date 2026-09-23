//! Renders a partitioned schema output as the files of its module directory. Every file carries
//! the generated-code banner, imports exactly the types it names from the schema module's root,
//! and stays within the production source-size limit the file-length gate enforces. A
//! definition too large for one file becomes a directory: its own type in `mod.rs` and one file
//! per derived type.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};
use heck::ToSnakeCase;
use syn::visit::Visit;

use super::module_plan::{DefinitionGroup, PartitionedOutput};

/// The most lines a production source file may have (CLAUDE.md law 7; the gate's SIZE-3 rule).
pub(super) const PRODUCTION_LINE_LIMIT: usize = 500;

/// Formats Rust source the way `rustfmt` would.
pub(super) type Formatter<'a> = &'a dyn Fn(&str) -> Result<String>;

/// Every file of one schema's module directory, keyed by its path inside that directory.
pub(super) fn render_files(
    output: PartitionedOutput,
    schema_file: &str,
    format: Formatter<'_>,
) -> Result<BTreeMap<String, String>> {
    let context = RenderContext {
        banner: banner(schema_file),
        known: output.type_names(),
        support: output
            .support
            .iter()
            .map(|module| module.name.clone())
            .collect(),
        format,
    };
    let mut files = BTreeMap::new();
    let mut declarations = String::new();
    for module in output.support {
        let mut source = context.banner.clone();
        for line in &module.documentation {
            source.push_str(&format!("//!{line}\n"));
        }
        source.push('\n');
        source.push_str(&unparse(module.items));
        insert(
            &mut files,
            format!("{}.rs", module.name),
            (context.format)(&source)?,
        )?;
        declarations.push_str(&format!("pub mod {};\n", module.name));
    }
    let mut definitions: Vec<String> = Vec::new();
    for group in &output.groups {
        let module = group.name.to_snake_case();
        if context.support.contains(&module) {
            bail!(
                "{schema_file}: definition `{}` collides with typify's `{module}` module",
                group.name
            );
        }
        let items: Vec<syn::Item> = group
            .blocks
            .iter()
            .flat_map(|block| block.items.clone())
            .collect();
        let single = context.render(&items, 1, &BTreeSet::new(), "")?;
        if single.lines().count() <= PRODUCTION_LINE_LIMIT {
            insert(&mut files, format!("{module}.rs"), single)?;
        } else {
            context.render_directory(&mut files, &module, group)?;
        }
        definitions.push(module);
    }
    definitions.sort();
    let mut root = context.banner.clone();
    root.push_str(&format!(
        "//! Types generated from `contracts_v2/definitions/{schema_file}`, one module per schema \
         definition.\n\n{declarations}"
    ));
    for module in &definitions {
        root.push_str(&format!("mod {module};\npub use {module}::*;\n"));
    }
    insert(&mut files, "mod.rs".to_string(), (context.format)(&root)?)?;
    for (path, source) in &files {
        let lines = source.lines().count();
        if lines > PRODUCTION_LINE_LIMIT {
            bail!(
                "{schema_file}: generated {path} would be {lines} lines (limit \
                 {PRODUCTION_LINE_LIMIT}); split the schema definition it holds"
            );
        }
    }
    Ok(files)
}

struct RenderContext<'a> {
    banner: String,
    known: BTreeSet<String>,
    support: Vec<String>,
    format: Formatter<'a>,
}

impl RenderContext<'_> {
    /// One file at `depth` below the schema module's root. It imports every schema type it
    /// names that it neither defines nor receives through its own glob re-exports, and reaches
    /// typify's support modules through the root.
    fn render(
        &self,
        items: &[syn::Item],
        depth: usize,
        reexported: &BTreeSet<String>,
        preamble: &str,
    ) -> Result<String> {
        let root = vec!["super"; depth].join("::");
        let defined: BTreeSet<String> = items.iter().filter_map(defined_type).collect();
        let mut named = NamedTypes {
            known: &self.known,
            found: BTreeSet::new(),
        };
        for item in items {
            named.visit_item(item);
        }
        let imports: Vec<String> = named
            .found
            .into_iter()
            .filter(|name| !defined.contains(name) && !reexported.contains(name))
            .collect();
        let mut source = self.banner.clone();
        source.push_str(preamble);
        if !imports.is_empty() {
            source.push_str(&format!("use {root}::{{{}}};\n\n", imports.join(", ")));
        }
        source.push_str(&unparse(items.to_vec()));
        for module in &self.support {
            source = source.replace(&format!("self::{module}::"), &format!("{root}::{module}::"));
        }
        (self.format)(&source)
    }

    /// A definition too large for one file: `module/mod.rs` holds the definition's own type and
    /// re-exports one file per derived type, named by what the derived type adds to the
    /// definition's name.
    fn render_directory(
        &self,
        files: &mut BTreeMap<String, String>,
        module: &str,
        group: &DefinitionGroup,
    ) -> Result<()> {
        let mut own_items = Vec::new();
        let mut children: Vec<String> = Vec::new();
        let mut reexported = BTreeSet::new();
        for block in &group.blocks {
            if block.name == group.name {
                own_items.extend(block.items.iter().cloned());
                continue;
            }
            let child = block
                .name
                .strip_prefix(group.name.as_str())
                .filter(|suffix| !suffix.is_empty())
                .unwrap_or(&block.name)
                .to_snake_case();
            if child == "mod" || children.contains(&child) {
                bail!(
                    "definition `{}` derives two types named `{child}`",
                    group.name
                );
            }
            let source = self.render(&block.items, 2, &BTreeSet::new(), "")?;
            insert(files, format!("{module}/{child}.rs"), source)?;
            reexported.insert(block.name.clone());
            children.push(child);
        }
        children.sort();
        let mut preamble = format!(
            "//! The `{}` definition and the types typify derives from it.\n\n",
            group.name
        );
        for child in &children {
            preamble.push_str(&format!("mod {child};\npub use {child}::*;\n"));
        }
        preamble.push('\n');
        let source = self.render(&own_items, 1, &reexported, &preamble)?;
        insert(files, format!("{module}/mod.rs"), source)
    }
}

/// Collects the schema types a file names: the first segment of every relative path.
struct NamedTypes<'a> {
    known: &'a BTreeSet<String>,
    found: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for NamedTypes<'_> {
    fn visit_path(&mut self, path: &'ast syn::Path) {
        if path.leading_colon.is_none()
            && let Some(first) = path.segments.first()
        {
            let name = first.ident.to_string();
            if self.known.contains(&name) {
                self.found.insert(name);
            }
        }
        syn::visit::visit_path(self, path);
    }
}

fn defined_type(item: &syn::Item) -> Option<String> {
    match item {
        syn::Item::Struct(item) => Some(item.ident.to_string()),
        syn::Item::Enum(item) => Some(item.ident.to_string()),
        syn::Item::Type(item) => Some(item.ident.to_string()),
        _ => None,
    }
}

fn unparse(items: Vec<syn::Item>) -> String {
    prettyplease::unparse(&syn::File {
        shebang: None,
        attrs: Vec::new(),
        items,
    })
}

fn banner(schema_file: &str) -> String {
    format!(
        "// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.\n\
         // Source: contracts_v2/definitions/{schema_file} — regenerate with: cargo xtask ci schema-codegen\n\n"
    )
}

fn insert(files: &mut BTreeMap<String, String>, path: String, source: String) -> Result<()> {
    if files.contains_key(&path) {
        bail!("two generated files would share the path {path}");
    }
    files.insert(path, source);
    Ok(())
}
