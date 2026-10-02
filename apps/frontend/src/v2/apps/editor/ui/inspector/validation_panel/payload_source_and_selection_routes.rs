//! Validation panel payload source and selection routes.

use super::*;

/// Compiled payload and asset catalog inputs for validation.
#[derive(Clone, Debug)]
pub struct PayloadSource {
    pub payload: serde_json::Value,
    pub known_asset_ids: Option<std::collections::HashSet<String>>,
}

type PayloadSourceGetter = std::rc::Rc<dyn Fn() -> Option<PayloadSource>>;

thread_local! {
    static PAYLOAD_SOURCE: std::cell::RefCell<Option<PayloadSourceGetter>> =
        const { std::cell::RefCell::new(None) };
}

/// Installs the live payload getter.
pub fn register_payload_source(f: PayloadSourceGetter) {
    install_seam(&PAYLOAD_SOURCE, f);
}

/// Reads the currently registered payload, if available.
#[must_use]
pub fn read_payload_source() -> Option<PayloadSource> {
    PAYLOAD_SOURCE.with(|c| c.borrow().as_ref().and_then(|f| f()))
}

type SelectByIdRouter = std::rc::Rc<dyn Fn(&str) -> bool>;

thread_local! {
    static SELECT_BY_ID: std::cell::RefCell<Option<SelectByIdRouter>> =
        const { std::cell::RefCell::new(None) };
}

/// Installs the finding click selection router.
pub fn register_select_by_id(f: SelectByIdRouter) {
    install_seam(&SELECT_BY_ID, f);
}

/// Selects a finding subject through the registered router.
pub fn route_select_by_subject_id(subject_id: &str) -> bool {
    SELECT_BY_ID.with(|c| c.borrow().as_ref().is_some_and(|f| f(subject_id)))
}

type RouteProbe = std::rc::Rc<dyn Fn(&str) -> bool>;

thread_local! {
    static ROUTE_PROBE: std::cell::RefCell<Option<RouteProbe>> =
        const { std::cell::RefCell::new(None) };
}

/// Installs the subject route availability probe.
pub fn register_route_probe(f: RouteProbe) {
    install_seam(&ROUTE_PROBE, f);
}

/// Reports whether the current router resolves a subject.
#[must_use]
pub fn subject_id_routes(subject_id: &str) -> bool {
    !subject_id.is_empty()
        && ROUTE_PROBE.with(|c| c.borrow().as_ref().is_some_and(|f| f(subject_id)))
}

/// Reports whether a finding links to a selectable subject.
#[must_use]
pub fn finding_is_routable(f: &PanelFinding) -> bool {
    f.subject_id.as_deref().is_some_and(subject_id_routes)
}

/// Builds the asset identifiers recognized by validation.
#[must_use]
pub fn known_asset_ids_from_registry(
    items: &[crate::v2::core::api::dto::RegistryItem],
) -> std::collections::HashSet<String> {
    let mut set = std::collections::HashSet::with_capacity(items.len() * 2);
    for item in items {
        set.insert(item.resource_name.clone());
        if matches!(item.kind.as_str(), "crate" | "other") {
            set.insert(
                crate::v2::apps::editor::arsenal::asset_catalog::derive_object_alias(
                    &item.resource_name,
                    &item.display_name,
                ),
            );
        }
    }
    set
}
