//! The Scope v2 vocabulary tree, resolved at [`crate::Corpus::load`].
//!
//! `.ai/tickets/scope-vocab.toml` is the 4-level domain → layer → component → surface
//! word list. This module reads it LENIENTLY — tables of tables of string
//! arrays — because the file's own shape gate (sortedness, closed domain set, no
//! duplicates) lives in `crate::validation::vocabulary` and runs in `ticket check`; here the
//! tree only has to answer legality questions: is this ticket's
//! domain/layer/component/surface a word the vocabulary knows?
//!
//! Fail-closed: a missing or unparseable vocabulary refuses the corpus load naming the
//! path — a legality gate that cannot resolve must not wave tickets through. Scratch
//! corpora built with [`crate::Corpus::new`] never consult the vocabulary (they load
//! nothing); scratch TREES that go through `Corpus::load` must carry a minimal file.

use crate::ScopeV2;
use crate::repository::SCOPE_VOCAB;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

type LayerTable = BTreeMap<String, Vec<String>>;

/// domain → layer → component → surfaces. A component-free layer is an empty
/// component map (the bare `[domain.layer]` header encoding).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScopeVocab {
    tree: BTreeMap<String, BTreeMap<String, LayerTable>>,
}

impl ScopeVocab {
    /// Read `.ai/tickets/scope-vocab.toml` under `root`. Missing file is an error
    /// naming the path (fail-closed — the cutover made scope legality
    /// load-bearing for every corpus load).
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

    /// Lenient tree read: every level must be a table (or a surface string array at
    /// the component level); anything else refuses with the path into the tree.
    pub fn parse(text: &str) -> Result<Self, String> {
        let value: toml::Value = text
            .parse()
            .map_err(|e: toml::de::Error| format!("TOML parse: {}", e.message()))?;
        let Some(domains) = value.as_table() else {
            return Err("top level must be a table of domains".into());
        };
        let mut tree = BTreeMap::new();
        for (domain, dv) in domains {
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

    /// Legality of one work ticket's scope: domain/layer must be a tree node,
    /// `component` (when present) a key under that layer, every `surface` a member of
    /// that component's array. Errors name ticket + offending pair.
    pub fn check_scope(&self, id: &str, scope: &ScopeV2) -> Result<(), String> {
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

    /// Surfaces the vocabulary offers for a (domain, layer, component) position —
    /// `None` when the position itself is unknown. Read-only helper for reporting
    /// (`ticket scope-histogram`) and future gates.
    pub fn surfaces_of(&self, domain: &str, layer: &str, component: &str) -> Option<&[String]> {
        self.tree
            .get(domain)?
            .get(layer)?
            .get(component)
            .map(Vec::as_slice)
    }
}

#[cfg(test)]
#[path = "tests/vocab/mod.rs"]
mod tests;
