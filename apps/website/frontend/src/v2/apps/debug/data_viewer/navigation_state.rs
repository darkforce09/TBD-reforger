//! Shareable viewer locations; native node identifiers remain opaque.
use std::collections::BTreeMap;
/// A data viewer location: the page's query parameters as sorted key/value pairs. Every view
/// state lives here, so any location can be shared as a link; node identifiers stay opaque
/// strings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Navigation(pub BTreeMap<String, String>);
impl Navigation {
    /// Parses a URL query string, with or without its leading `?`. On the Resources tab a
    /// `capability` parameter becomes `catalog_capability`, so a capability link filters the
    /// resource catalog there.
    pub fn parse(query: &str) -> Self {
        let mut navigation = Self(
            url::form_urlencoded::parse(query.trim_start_matches('?').as_bytes())
                .into_owned()
                .collect(),
        );
        if navigation.tab() == "resources" {
            if let Some(capability) = navigation.0.remove("capability") {
                navigation
                    .0
                    .entry("catalog_capability".into())
                    .or_insert(capability);
            }
        }
        navigation
    }
    /// The value of the query parameter `key`, or an empty string when it is absent.
    pub fn get(&self, key: &str) -> &str {
        self.0.get(key).map(String::as_str).unwrap_or("")
    }
    /// The selected dataset: `diagnostic` when the location asks for it, otherwise `gameplay`.
    pub fn dataset(&self) -> &str {
        match self.get("dataset") {
            "diagnostic" => "diagnostic",
            _ => "gameplay",
        }
    }
    /// The pinned export generation, or `latest` when the location follows the newest export.
    pub fn generation(&self) -> &str {
        let v = self.get("generation");
        if v.is_empty() {
            "latest"
        } else {
            v
        }
    }
    /// The active top-level tab, `overview` when none is set.
    pub fn tab(&self) -> &str {
        let v = self.get("tab");
        if v.is_empty() {
            "overview"
        } else {
            v
        }
    }
    /// The active section of the selected resource. An unset section, and the `systems` and
    /// `source` sections that field and relationship links name, resolve to `data`, the surface
    /// that shows every container's source fields.
    pub fn section(&self) -> &str {
        let v = self.get("section");
        if v.is_empty() {
            "data"
        } else {
            if matches!(v, "systems" | "source") {
                "data"
            } else {
                v
            }
        }
    }
    /// This location with `values` applied, where an empty value removes its key, after clearing
    /// every parameter the change makes stale. Switching dataset drops the generation and all
    /// selection state and leaves the document and selection tabs; changing the resource, node,
    /// property or document drops the value pointer, value cursor and property search; changing
    /// the resource also drops the node selection; changing the node drops the property; changing
    /// the cursor, direction, kind or view drops the selected relationship.
    pub fn changed(&self, values: &[(&str, &str)]) -> Self {
        let mut next = self.clone();
        let changes = |key: &str| values.iter().any(|(k, v)| *k == key && self.get(key) != *v);
        if changes("dataset") {
            for key in [
                "generation",
                "node",
                "property",
                "pointer",
                "field",
                "relation",
                "view",
                "container_cursor",
                "value_cursor",
                "document",
                "cursor",
                "kind",
                "direction",
                "section",
            ] {
                next.0.remove(key);
            }
            if matches!(self.tab(), "document" | "selection") {
                next.0.insert("tab".into(), "overview".into());
            }
        }
        let source_changed =
            changes("resource") || changes("node") || changes("property") || changes("document");
        if source_changed {
            for key in ["pointer", "value_cursor", "property_q"] {
                next.0.remove(key);
            }
        }
        if changes("resource") {
            for key in ["node", "parent", "property", "container_cursor", "relation"] {
                next.0.remove(key);
            }
        }
        if changes("node") {
            next.0.remove("property");
        }
        if ["cursor", "direction", "kind", "view"]
            .iter()
            .any(|key| changes(key))
        {
            next.0.remove("relation");
        }
        for (k, v) in values {
            if v.is_empty() {
                next.0.remove(*k);
            } else {
                next.0.insert((*k).into(), (*v).into());
            }
        }
        next
    }
    /// The viewer route `/debug/data-viewer` at the location [`Self::changed`] gives for
    /// `values`.
    pub fn href(&self, values: &[(&str, &str)]) -> String {
        format!("/debug/data-viewer?{}", self.changed(values).query())
    }
    /// This location as a URL query string, keys in sorted order.
    pub fn query(&self) -> String {
        url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(&self.0)
            .finish()
    }
    /// The API path, relative to `/api/v1`, of `endpoint` under `/debug/equipment-data/` for
    /// `generation`. It carries the dataset and each set location parameter under its API name
    /// (`resource` as `resource_id`, `node` as `node_id`, `field` as `field_id`, `class` as
    /// `class_name`); `extra` then overrides, an empty value removing the parameter.
    pub fn request(&self, endpoint: &str, generation: &str, extra: &[(&str, &str)]) -> String {
        let mut values = BTreeMap::from([
            ("generation".to_owned(), generation.to_owned()),
            ("dataset".to_owned(), self.dataset().to_owned()),
        ]);
        for (from, to) in [
            ("resource", "resource_id"),
            ("node", "node_id"),
            ("property", "property"),
            ("view", "view"),
            ("capability", "capability"),
            ("direction", "direction"),
            ("kind", "kind"),
            ("document", "document"),
            ("pointer", "pointer"),
            ("field", "field_id"),
            ("class", "class_name"),
        ] {
            if !self.get(from).is_empty() {
                values.insert(to.into(), self.get(from).into());
            }
        }
        for (k, v) in extra {
            if v.is_empty() {
                values.remove(*k);
            } else {
                values.insert((*k).into(), (*v).into());
            }
        }
        format!(
            "/debug/equipment-data/{endpoint}?{}",
            url::form_urlencoded::Serializer::new(String::new())
                .extend_pairs(values)
                .finish()
        )
    }
    /// The location to replace this one with when the followed generation changes from
    /// `previous` to `current`: the same tab and resource with the node, property, value, field
    /// and relationship selection cleared, since those may not exist in the new export. `None`
    /// while the location pins a generation or no change happened.
    pub fn after_publication(&self, previous: &str, current: &str) -> Option<String> {
        if previous.is_empty()
            || current.is_empty()
            || previous == current
            || self.generation() != "latest"
        {
            return None;
        }
        Some(self.href(&[
            ("node", ""),
            ("parent", ""),
            ("property", ""),
            ("pointer", ""),
            ("cursor", ""),
            ("container_cursor", ""),
            ("value_cursor", ""),
            ("field", ""),
            ("relation", ""),
        ]))
    }
    /// The `resources` request of the catalog list, built only from the catalog's own parameters
    /// (dataset, domain, search and `catalog_capability`), so selecting a resource never reloads
    /// the listing. With `selected` it asks whether the selected resource lies inside the current
    /// filter instead of for the current page.
    pub fn catalog_request(&self, generation: &str, selected: bool) -> String {
        let mut catalog = Self::default();
        for key in ["domain", "q", "dataset"] {
            if !self.get(key).is_empty() {
                catalog.0.insert(key.into(), self.get(key).into());
            }
        }
        catalog.request(
            "resources",
            generation,
            &[
                ("capability", self.get("catalog_capability")),
                ("domain", self.get("domain")),
                ("q", self.get("q")),
                (
                    "cursor",
                    if selected {
                        ""
                    } else {
                        self.get("resource_cursor")
                    },
                ),
                (
                    "resource_id",
                    if selected { self.get("resource") } else { "" },
                ),
            ],
        )
    }
    /// A request scoped to the selected resource: only the resource, the dataset and, except for
    /// the `resources` and `download` endpoints, the configuration view are carried, plus
    /// `extra`, so catalog filters and list cursors never reach per-resource reads.
    pub fn inspection_request(
        &self,
        endpoint: &str,
        generation: &str,
        extra: &[(&str, &str)],
    ) -> String {
        let mut inspection = Self::default();
        for key in ["resource", "view", "dataset"] {
            if key == "view" && matches!(endpoint, "resources" | "download") {
                continue;
            }
            if !self.get(key).is_empty() {
                inspection.0.insert(key.into(), self.get(key).into());
            }
        }
        inspection.request(endpoint, generation, extra)
    }
    /// The link that opens resource `id` on the Resources tab at its data section, clearing the
    /// previous resource's node, property, value and field selection and its capability and view
    /// filters.
    pub fn resource_link(&self, id: &str) -> String {
        self.href(&[
            ("tab", "resources"),
            ("resource", id),
            ("node", ""),
            ("parent", ""),
            ("property", ""),
            ("pointer", ""),
            ("cursor", ""),
            ("section", "data"),
            ("capability", ""),
            ("field", ""),
            ("view", ""),
        ])
    }
}

#[cfg(test)]
#[path = "tests/navigation.rs"]
mod tests;
