//! Assembles the production source inspected by adjacent toolbelt tests.

pub(super) fn raw_toolbelt() -> &'static str {
    static SOURCE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SOURCE
        .get_or_init(|| {
            let mut source = String::new();
            source.push_str(include_str!("../../toolbelt/scale_math.rs"));
            source.push_str(include_str!("../../toolbelt/grid_reference.rs"));
            source.push_str(include_str!("../../toolbelt/toolbar_and_status.rs"));
            source.push_str(include_str!("../../toolbelt/map_furniture.rs"));
            source.push_str(include_str!("../../toolbelt/bottom_toolbelt.rs"));
            source.push_str(include_str!("../../toolbelt.rs"));
            source
        })
        .as_str()
}
