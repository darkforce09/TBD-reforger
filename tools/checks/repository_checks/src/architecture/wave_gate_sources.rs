//! Linkage checks for source audits of the platform wave gate implementations.
//!
//! The wave gate lives in the private `wave_execution` module of the `platform_execution` library
//! crate, so its facade `gate.rs` re-exports `gate_slice` and `cmd_gate` to the crate
//! (`pub(crate) use`), and the implementations declare themselves `pub(crate) fn`. A crate-wide
//! export is what the gate's callers (the wave command table, `land`) need; a `pub use` there
//! would name items the crate never exports. The audits that read these sources share the
//! spellings below rather than each pinning its own copy.

/// The two implementation modules of the platform wave gate facade, each with the function the
/// facade re-exports: `checkrun` with `gate_slice`, `gate_dispatch` with `cmd_gate`.
pub const WAVE_CHILDREN: [(&str, &str); 2] =
    [("checkrun", "gate_slice"), ("gate_dispatch", "cmd_gate")];

/// The visibility the facade gives each re-export: the whole library crate.
pub const WAVE_EXPORT_VISIBILITY: &str = "pub(crate)";

/// The two facade statements that link one implementation module, as `gate.rs` spells them: the
/// module declaration and the crate-wide re-export of its function.
pub fn wave_child_link_statements(module_name: &str, function_name: &str) -> [String; 2] {
    [
        format!("mod {module_name};"),
        format!("{WAVE_EXPORT_VISIBILITY} use {module_name}::{function_name};"),
    ]
}

/// The multi-line regex source that finds the line opening the named implementation function:
/// `fn <name>(` at the start of a line, private, `pub` or `pub(crate)`.
pub fn wave_function_opener_pattern(function_name: &str) -> String {
    format!(
        r"(?m)^(?:pub(?:\(crate\))?\s+)?fn {}\s*\(",
        regex::escape(function_name)
    )
}

/// Requires unconditional declarations and crate-wide exports (`pub` or `pub(crate)`; a private,
/// `pub(self)`, `pub(super)` or `pub(in …)` export does not count) of the inspected source owners.
pub fn wave_children_are_linked(source: &str) -> bool {
    let Ok(file) = syn::parse_file(source) else {
        return false;
    };
    WAVE_CHILDREN.iter().all(|(module_name, function_name)| {
        let declared = file.items.iter().any(|item| {
            matches!(item, syn::Item::Mod(module)
                if module.ident == *module_name && module.content.is_none() && module.attrs.is_empty())
        });
        let exported = file.items.iter().any(|item| {
            let syn::Item::Use(export) = item else { return false };
            let syn::UseTree::Path(path) = &export.tree else { return false };
            matches!(&*path.tree, syn::UseTree::Name(name)
                if path.ident == *module_name && name.ident == *function_name)
                && is_crate_wide(&export.vis)
                && export.attrs.is_empty()
        });
        declared && exported
    })
}

/// `pub` or `pub(crate)`: visible to every caller in the crate.
fn is_crate_wide(visibility: &syn::Visibility) -> bool {
    match visibility {
        syn::Visibility::Public(_) => true,
        syn::Visibility::Restricted(restricted) => {
            restricted.in_token.is_none() && restricted.path.is_ident("crate")
        }
        syn::Visibility::Inherited => false,
    }
}
