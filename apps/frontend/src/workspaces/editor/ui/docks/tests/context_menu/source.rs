//! Assembles the production source inspected by adjacent context menu tests.

pub(super) fn raw_context_menu() -> &'static str {
    static SOURCE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SOURCE
        .get_or_init(|| {
            let mut source = String::new();
            source.push_str(include_str!("../../context_menu/connection_types.rs"));
            source.push_str(include_str!("../../context_menu/menu_entries.rs"));
            source.push_str(include_str!("../../context_menu/menu_state.rs"));
            source.push_str(include_str!("../../context_menu/menu_geometry.rs"));
            source.push_str(include_str!("../../context_menu/menu_dispatch.rs"));
            source.push_str(include_str!("../../context_menu/menu_overlay.rs"));
            source.push_str(include_str!("../../context_menu.rs"));
            source
        })
        .as_str()
}
