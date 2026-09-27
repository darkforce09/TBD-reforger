//! Shareable viewer locations; native node identifiers remain opaque.
use std::collections::BTreeMap;
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Navigation(pub BTreeMap<String, String>);
impl Navigation {
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
    pub fn get(&self, key: &str) -> &str {
        self.0.get(key).map(String::as_str).unwrap_or("")
    }
    pub fn dataset(&self) -> &str {
        match self.get("dataset") {
            "diagnostic" => "diagnostic",
            _ => "gameplay",
        }
    }
    pub fn generation(&self) -> &str {
        let v = self.get("generation");
        if v.is_empty() { "latest" } else { v }
    }
    pub fn tab(&self) -> &str {
        let v = self.get("tab");
        if v.is_empty() { "overview" } else { v }
    }
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
    pub fn href(&self, values: &[(&str, &str)]) -> String {
        format!("/debug/data-viewer?{}", self.changed(values).query())
    }
    pub fn query(&self) -> String {
        url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(&self.0)
            .finish()
    }
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
