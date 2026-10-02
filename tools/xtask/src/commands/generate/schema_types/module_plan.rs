//! Partitions one schema's typify output by schema definition, so the generated module mirrors
//! the contract. Typify's support modules (`error`) stand alone, and every schema definition
//! owns its type, the helper types typify derives from it and all of their impls. Typify names a
//! derived type by appending to its definition's name, so a type belongs to the longest
//! definition name it starts with; a type that starts with none is its own definition.

use std::collections::BTreeSet;

use anyhow::{Result, bail};
use heck::ToPascalCase;

/// Opens the `<details>` block typify appends to a type's documentation to quote its schema.
const SCHEMA_DOCUMENT_OPENING: &str = "<details><summary>JSON schema</summary>";
/// Closes that block.
const SCHEMA_DOCUMENT_CLOSING: &str = "</details>";

/// One type typify emitted, with the impls that follow it.
pub(super) struct TypeBlock {
    pub(super) name: String,
    pub(super) items: Vec<syn::Item>,
}

/// A schema definition's share of the output, in typify's order.
pub(super) struct DefinitionGroup {
    pub(super) name: String,
    pub(super) blocks: Vec<TypeBlock>,
}

/// One of typify's support modules: its documentation lines and its body.
pub(super) struct SupportModule {
    pub(super) name: String,
    pub(super) documentation: Vec<String>,
    pub(super) items: Vec<syn::Item>,
}

/// Typify's output for one schema, partitioned.
pub(super) struct PartitionedOutput {
    pub(super) support: Vec<SupportModule>,
    pub(super) groups: Vec<DefinitionGroup>,
}

impl PartitionedOutput {
    /// Every type the schema's module defines.
    pub(super) fn type_names(&self) -> BTreeSet<String> {
        self.groups
            .iter()
            .flat_map(|group| group.blocks.iter().map(|block| block.name.clone()))
            .collect()
    }
}

/// The Rust names typify gives the schema's definitions and its titled root, sorted.
pub(super) fn definition_names(schema: &serde_json::Value) -> Vec<String> {
    let mut names: Vec<String> = ["definitions", "$defs"]
        .iter()
        .filter_map(|key| schema.get(*key)?.as_object())
        .flat_map(|definitions| definitions.keys().map(|key| key.to_pascal_case()))
        .collect();
    if let Some(title) = schema.get("title").and_then(serde_json::Value::as_str) {
        names.push(title.to_pascal_case());
    }
    names.sort();
    names.dedup();
    names
}

/// The definition that owns a type: the longest definition name the type's name starts with at
/// a word boundary, or the type itself.
pub(super) fn owning_definition(type_name: &str, definitions: &[String]) -> String {
    definitions
        .iter()
        .filter(|definition| {
            type_name
                .strip_prefix(definition.as_str())
                .is_some_and(|rest| {
                    rest.chars()
                        .next()
                        .is_none_or(|next| next.is_ascii_uppercase() || next.is_ascii_digit())
                })
        })
        .max_by_key(|definition| definition.len())
        .cloned()
        .unwrap_or_else(|| type_name.to_string())
}

/// Partition typify's items. Anything other than support modules, types and impls is refused,
/// so no generated item can be dropped or misplaced silently.
pub(super) fn partition(file: syn::File, definitions: &[String]) -> Result<PartitionedOutput> {
    let mut support = Vec::new();
    let mut blocks: Vec<TypeBlock> = Vec::new();
    for mut item in file.items {
        if let syn::Item::Mod(module) = item {
            support.push(support_module(module)?);
            continue;
        }
        if let Some(name) = type_name(&item) {
            if let Some(attributes) = type_attributes(&mut item) {
                strip_schema_document(attributes)?;
            }
            blocks.push(TypeBlock {
                name,
                items: vec![item],
            });
            continue;
        }
        let syn::Item::Impl(implementation) = item else {
            bail!(
                "typify emitted an unexpected top-level item ({}); extend the schema module split",
                item_kind(&item)
            );
        };
        let owner = impl_owner(&implementation, &blocks)?;
        blocks[owner].items.push(syn::Item::Impl(implementation));
    }
    let mut groups: Vec<DefinitionGroup> = Vec::new();
    for block in blocks {
        let owner = owning_definition(&block.name, definitions);
        match groups.iter_mut().find(|group| group.name == owner) {
            Some(group) => group.blocks.push(block),
            None => groups.push(DefinitionGroup {
                name: owner,
                blocks: vec![block],
            }),
        }
    }
    Ok(PartitionedOutput { support, groups })
}

fn support_module(module: syn::ItemMod) -> Result<SupportModule> {
    let name = module.ident.to_string();
    let Some((_, items)) = module.content else {
        bail!("typify emitted the module `{name}` without a body");
    };
    Ok(SupportModule {
        documentation: module.attrs.iter().filter_map(documentation_text).collect(),
        name,
        items,
    })
}

fn type_name(item: &syn::Item) -> Option<String> {
    match item {
        syn::Item::Struct(item) => Some(item.ident.to_string()),
        syn::Item::Enum(item) => Some(item.ident.to_string()),
        syn::Item::Type(item) => Some(item.ident.to_string()),
        _ => None,
    }
}

fn type_attributes(item: &mut syn::Item) -> Option<&mut Vec<syn::Attribute>> {
    match item {
        syn::Item::Struct(item) => Some(&mut item.attrs),
        syn::Item::Enum(item) => Some(&mut item.attrs),
        syn::Item::Type(item) => Some(&mut item.attrs),
        _ => None,
    }
}

/// The block an impl belongs to: the type it implements, or — for an impl on a foreign type
/// such as `From<Type> for String`, which typify emits right after `Type` — the preceding type.
fn impl_owner(implementation: &syn::ItemImpl, blocks: &[TypeBlock]) -> Result<usize> {
    let implemented = match &*implementation.self_ty {
        syn::Type::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string()),
        _ => None,
    };
    if let Some(index) = implemented
        .as_deref()
        .and_then(|name| blocks.iter().position(|block| block.name == name))
    {
        return Ok(index);
    }
    match blocks.len() {
        0 => bail!("typify emitted an impl before any type"),
        count => Ok(count - 1),
    }
}

fn item_kind(item: &syn::Item) -> &'static str {
    match item {
        syn::Item::Const(_) => "const",
        syn::Item::Fn(_) => "fn",
        syn::Item::Macro(_) => "macro invocation",
        syn::Item::Static(_) => "static",
        syn::Item::Trait(_) => "trait",
        syn::Item::Use(_) => "use",
        _ => "other item",
    }
}

/// The text of a `#[doc = "…"]` attribute.
pub(super) fn documentation_text(attribute: &syn::Attribute) -> Option<String> {
    if !attribute.path().is_ident("doc") {
        return None;
    }
    let syn::Meta::NameValue(pair) = &attribute.meta else {
        return None;
    };
    let syn::Expr::Lit(syn::ExprLit {
        lit: syn::Lit::Str(text),
        ..
    }) = &pair.value
    else {
        return None;
    };
    Some(text.value())
}

/// Remove the quoted JSON Schema from a type's documentation and keep its description. Typify
/// writes the quote either as one documentation line per attribute or inside one attribute;
/// the blank lines that only separated the description from the quote go with it.
pub(super) fn strip_schema_document(attributes: &mut Vec<syn::Attribute>) -> Result<()> {
    let Some(opening) = attributes.iter().position(|attribute| {
        documentation_text(attribute).is_some_and(|text| text.contains(SCHEMA_DOCUMENT_OPENING))
    }) else {
        return Ok(());
    };
    let text = documentation_text(&attributes[opening]).unwrap_or_default();
    if text.contains(SCHEMA_DOCUMENT_CLOSING) {
        let description = text[..text.find(SCHEMA_DOCUMENT_OPENING).unwrap_or(0)].trim_end();
        if description.is_empty() {
            attributes.remove(opening);
        } else {
            attributes[opening] = syn::parse_quote!(#[doc = #description]);
        }
    } else {
        let Some(closing) = attributes[opening..].iter().position(|attribute| {
            documentation_text(attribute).is_some_and(|text| text.trim() == SCHEMA_DOCUMENT_CLOSING)
        }) else {
            bail!("typify's schema quote has no closing `{SCHEMA_DOCUMENT_CLOSING}`");
        };
        attributes.drain(opening..=opening + closing);
    }
    let mut end = opening;
    while end > 0
        && documentation_text(&attributes[end - 1]).is_some_and(|text| text.trim().is_empty())
    {
        attributes.remove(end - 1);
        end -= 1;
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/module_plan.rs"]
mod tests;
