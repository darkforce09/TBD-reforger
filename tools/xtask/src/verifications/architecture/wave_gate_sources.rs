//! Linkage checks for source audits of the platform wave gate implementations.

pub(crate) const WAVE_CHILDREN: [(&str, &str); 2] =
    [("checkrun", "gate_slice"), ("gate_dispatch", "cmd_gate")];

/// Requires unconditional declarations and public exports to the inspected source owners.
pub(crate) fn wave_children_are_linked(source: &str) -> bool {
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
                && matches!(export.vis, syn::Visibility::Public(_))
                && export.attrs.is_empty()
        });
        declared && exported
    })
}
