//! Hierarchical scope choices from vocabulary and corpus values.
//!
//! Parent selections narrow child choices and clear stale selections. Missing or broken
//! vocabulary falls back to observed values; unknown values stay selectable and marked.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::ticket_browser::services::filtering::{RowFacts, ScopeFacets};

type Components = BTreeMap<String, Vec<String>>;
type Layers = BTreeMap<String, Components>;

/// domain → layer → component → surfaces — the checker's tree shape, re-read at
/// display tier.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VocabTree {
    tree: BTreeMap<String, Layers>,
}

impl VocabTree {
    /// Display-tier read: ANY failure (missing file, IO error, bad shape) is
    /// `None`, and the facets fall back to corpus-present values.
    pub fn load(root: &Path) -> Option<Self> {
        let text = fs::read_to_string(root.join(ticket_engine::repository::SCOPE_VOCAB)).ok()?;
        Self::parse(&text).ok()
    }

    /// Tables of tables of string arrays — the `ScopeVocab::parse` shape, kept
    /// all-or-nothing so a half-broken file cannot silently truncate dropdowns
    /// (broken ⇒ corpus fallback, a visible degradation).
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
            let mut layer_map: Layers = BTreeMap::new();
            for (layer, lv) in layers {
                let Some(components) = lv.as_table() else {
                    return Err(format!("{domain}.{layer}: layer must be a table"));
                };
                let mut component_map: Components = BTreeMap::new();
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
                        list.push(s.to_owned());
                    }
                    component_map.insert(component.clone(), list);
                }
                layer_map.insert(layer.clone(), component_map);
            }
            tree.insert(domain.clone(), layer_map);
        }
        Ok(Self { tree })
    }
}

/// One dropdown value. `vocab_unknown` marks a corpus-present value the loaded
/// vocabulary does not offer at this (narrowed) position — display-only marking,
/// never enforcement. With NO vocabulary loaded every value is corpus-derived
/// and unmarked (there is no authority to disagree with).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FacetOption {
    pub value: String,
    pub vocab_unknown: bool,
}

/// The four dropdown option lists, sorted, deduplicated, narrowed by the current
/// higher-level selections.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FacetOptions {
    pub domains: Vec<FacetOption>,
    pub layers: Vec<FacetOption>,
    pub components: Vec<FacetOption>,
    pub surfaces: Vec<FacetOption>,
}

/// Compute the narrowed option lists AND normalize the selection: after a higher
/// facet changes, any lower selection its dropdown no longer offers is cleared
/// (a stale invisible selection would silently pin the board to zero matches).
/// Runs on filter change / reload only — never per frame.
pub fn compute(
    vocab: Option<&VocabTree>,
    rows: &[RowFacts],
    sel: &mut ScopeFacets,
) -> FacetOptions {
    let domains = domain_options(vocab, rows);
    if stale(&sel.domain, &domains) {
        sel.domain = None;
        sel.layer = None;
        sel.component = None;
        sel.surface = None;
    }
    let layers = layer_options(vocab, rows, sel);
    if stale(&sel.layer, &layers) {
        sel.layer = None;
        sel.component = None;
        sel.surface = None;
    }
    let components = component_options(vocab, rows, sel);
    if stale(&sel.component, &components) {
        sel.component = None;
        sel.surface = None;
    }
    let surfaces = surface_options(vocab, rows, sel);
    if stale(&sel.surface, &surfaces) {
        sel.surface = None;
    }
    FacetOptions {
        domains,
        layers,
        components,
        surfaces,
    }
}

fn stale(sel: &Option<String>, options: &[FacetOption]) -> bool {
    sel.as_ref()
        .is_some_and(|v| !options.iter().any(|o| &o.value == v))
}

/// value → vocab_unknown accumulator: vocab inserts known (false); corpus values
/// only mark unknown when a vocabulary is loaded to disagree with.
fn to_options(set: BTreeMap<String, bool>) -> Vec<FacetOption> {
    set.into_iter()
        .map(|(value, vocab_unknown)| FacetOption {
            value,
            vocab_unknown,
        })
        .collect()
}

fn domain_options(vocab: Option<&VocabTree>, rows: &[RowFacts]) -> Vec<FacetOption> {
    let mut set: BTreeMap<String, bool> = BTreeMap::new();
    if let Some(v) = vocab {
        for domain in v.tree.keys() {
            set.insert(domain.clone(), false);
        }
    }
    for r in rows {
        if let Some(d) = &r.domain {
            set.entry(d.clone()).or_insert(vocab.is_some());
        }
    }
    to_options(set)
}

fn layer_options(
    vocab: Option<&VocabTree>,
    rows: &[RowFacts],
    sel: &ScopeFacets,
) -> Vec<FacetOption> {
    let mut set: BTreeMap<String, bool> = BTreeMap::new();
    if let Some(v) = vocab {
        for (domain, layers) in &v.tree {
            if sel.domain.as_deref().is_none_or(|d| d == domain) {
                for layer in layers.keys() {
                    set.insert(layer.clone(), false);
                }
            }
        }
    }
    for r in rows {
        if sel
            .domain
            .as_deref()
            .is_none_or(|d| r.domain.as_deref() == Some(d))
            && let Some(l) = &r.layer
        {
            set.entry(l.clone()).or_insert(vocab.is_some());
        }
    }
    to_options(set)
}

fn component_options(
    vocab: Option<&VocabTree>,
    rows: &[RowFacts],
    sel: &ScopeFacets,
) -> Vec<FacetOption> {
    let mut set: BTreeMap<String, bool> = BTreeMap::new();
    if let Some(v) = vocab {
        for (domain, layers) in &v.tree {
            if sel.domain.as_deref().is_none_or(|d| d == domain) {
                for (layer, components) in layers {
                    if sel.layer.as_deref().is_none_or(|l| l == layer) {
                        for component in components.keys() {
                            set.insert(component.clone(), false);
                        }
                    }
                }
            }
        }
    }
    for r in rows {
        if sel
            .domain
            .as_deref()
            .is_none_or(|d| r.domain.as_deref() == Some(d))
            && sel
                .layer
                .as_deref()
                .is_none_or(|l| r.layer.as_deref() == Some(l))
            && let Some(c) = &r.component
        {
            set.entry(c.clone()).or_insert(vocab.is_some());
        }
    }
    to_options(set)
}

fn surface_options(
    vocab: Option<&VocabTree>,
    rows: &[RowFacts],
    sel: &ScopeFacets,
) -> Vec<FacetOption> {
    let mut set: BTreeMap<String, bool> = BTreeMap::new();
    if let Some(v) = vocab {
        for (domain, layers) in &v.tree {
            if sel.domain.as_deref().is_none_or(|d| d == domain) {
                for (layer, components) in layers {
                    if sel.layer.as_deref().is_none_or(|l| l == layer) {
                        for (component, surfaces) in components {
                            if sel.component.as_deref().is_none_or(|c| c == component) {
                                for surface in surfaces {
                                    set.insert(surface.clone(), false);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    for r in rows {
        if sel
            .domain
            .as_deref()
            .is_none_or(|d| r.domain.as_deref() == Some(d))
            && sel
                .layer
                .as_deref()
                .is_none_or(|l| r.layer.as_deref() == Some(l))
            && sel
                .component
                .as_deref()
                .is_none_or(|c| r.component.as_deref() == Some(c))
        {
            for surface in &r.surfaces {
                set.entry(surface.clone()).or_insert(vocab.is_some());
            }
        }
    }
    to_options(set)
}

#[cfg(test)]
#[path = "tests/scope_facets.rs"]
mod tests;
