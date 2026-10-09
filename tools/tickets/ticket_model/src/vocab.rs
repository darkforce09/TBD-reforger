//! The scope vocabulary that a work ticket's scope is checked against.
//!
//! **Role:** [`ScopeVocab`], the domain → layer → component → surfaces tree read from
//! `.ai/tickets/scope-vocab.toml`, and the legality question it answers for one scope.
//! **Position:** read by [`crate::Corpus::load`] before any ticket file; `ticket_registry`'s
//! checks and reports read it too. The file's own shape rules (sorted keys, the closed domain
//! set, no duplicates) are `ticket_registry`'s vocabulary check in `ticket check`; this module
//! reads the file leniently, as tables of tables of string arrays.
//! **Signals & state:** none; a [`ScopeVocab`] is an immutable tree.
//! **Invariants:** fail-closed: a missing or unparseable vocabulary refuses the corpus load
//! naming the path, so a legality check that cannot resolve never passes a ticket. A corpus
//! built with [`crate::Corpus::new`] reads no vocabulary; a scratch tree loaded with
//! [`crate::Corpus::load`] carries a minimal vocabulary file.

use crate::ScopeV2;
use repository_layout::SCOPE_VOCAB;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

type LayerTable = BTreeMap<String, Vec<String>>;

/// The vocabulary tree: domain → layer → component → surfaces. A component-free layer is an
/// empty component map, which the file writes as a bare `[domain.layer]` header.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScopeVocab {
    tree: BTreeMap<String, BTreeMap<String, LayerTable>>,
}

impl ScopeVocab {
    /// Reads `.ai/tickets/scope-vocab.toml` under the checkout root `root`. A missing file is an
    /// error naming the path, because every corpus load depends on scope legality.
    pub fn load(root: &Path) -> Result<Self, String> {
        let path = root.join(SCOPE_VOCAB);
        if !path.is_file() {
            return Err(format!(
                "missing scope vocabulary (required for every corpus load): {}",
                path.display()
            ));
        }
        let text = fs::read_to_string(&path).map_err(|e| format!("read {SCOPE_VOCAB}: {e}"))?;
        Self::parse(&text).map_err(|e| format!("{SCOPE_VOCAB}: {e}"))
    }

    /// Parses vocabulary text leniently: each level must be a table, and each component a
    /// string array of surfaces; anything else refuses with the dotted path into the tree.
    pub fn parse(text: &str) -> Result<Self, String> {
        let domains: toml::Table = text
            .parse()
            .map_err(|e: toml::de::Error| format!("TOML parse: {}", e.message()))?;
        let mut tree = BTreeMap::new();
        for (domain, dv) in &domains {
            let Some(layers) = dv.as_table() else {
                return Err(format!("{domain}: domain must be a table of layers"));
            };
            let mut layer_map = BTreeMap::new();
            for (layer, lv) in layers {
                let Some(components) = lv.as_table() else {
                    return Err(format!("{domain}.{layer}: layer must be a table"));
                };
                let mut component_map: LayerTable = BTreeMap::new();
                for (component, cv) in components {
                    let Some(surfaces) = cv.as_array() else {
                        return Err(format!(
                            "{domain}.{layer}.{component}: component must be a surface array"
                        ));
                    };
                    let mut list = Vec::with_capacity(surfaces.len());
                    for s in surfaces {
                        let Some(s) = s.as_str() else {
                            return Err(format!(
                                "{domain}.{layer}.{component}: surface entries must be strings"
                            ));
                        };
                        list.push(s.to_string());
                    }
                    component_map.insert(component.clone(), list);
                }
                layer_map.insert(layer.clone(), component_map);
            }
            tree.insert(domain.clone(), layer_map);
        }
        Ok(Self { tree })
    }

    /// Checks one work ticket's scope: the domain and layer must be tree nodes, the `component`
    /// (when present) a key under that layer, and every `surface` a member of that component's
    /// array. The error names the ticket `id` and the offending word.
    pub fn check_scope(&self, id: &crate::TicketId, scope: &ScopeV2) -> Result<(), String> {
        let domain = scope.domain.as_str();
        let Some(layers) = self.tree.get(domain) else {
            return Err(format!(
                "{id}: scope domain \"{domain}\" has no table in {SCOPE_VOCAB}"
            ));
        };
        let Some(components) = layers.get(&scope.layer) else {
            return Err(format!(
                "{id}: scope layer \"{domain}.{}\" is not in {SCOPE_VOCAB}",
                scope.layer
            ));
        };
        let Some(component) = &scope.component else {
            // Component-free scope; the surface-requires-component shape rule in
            // `into_ticket` guarantees `surface` is empty here.
            return Ok(());
        };
        let Some(surfaces) = components.get(component) else {
            return Err(format!(
                "{id}: scope component \"{domain}.{}.{component}\" is not in {SCOPE_VOCAB}",
                scope.layer
            ));
        };
        for s in &scope.surface {
            if !surfaces.iter().any(|v| v == s) {
                return Err(format!(
                    "{id}: scope surface \"{s}\" is not under \"{domain}.{}.{component}\" in {SCOPE_VOCAB}",
                    scope.layer
                ));
            }
        }
        Ok(())
    }

    /// The whole tree, read-only: domain name → layer name → component name → that component's
    /// surfaces in file order. The maps iterate in name order, and a component-free layer is an
    /// empty component map. The ticketboard's scope facets list their choices from it; a
    /// legality question goes through [`ScopeVocab::check_scope`] instead.
    pub fn domains(&self) -> &BTreeMap<String, BTreeMap<String, LayerTable>> {
        &self.tree
    }

    /// The surfaces the vocabulary lists for a (domain, layer, component) position; `None` when
    /// the position is unknown. `ticket_registry`'s scope check reads it.
    pub fn surfaces_of(&self, domain: &str, layer: &str, component: &str) -> Option<&[String]> {
        self.tree
            .get(domain)?
            .get(layer)?
            .get(component)
            .map(Vec::as_slice)
    }
}
