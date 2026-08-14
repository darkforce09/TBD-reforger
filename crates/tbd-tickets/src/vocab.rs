//! T-917.2 — the Scope v2 vocabulary tree, resolved at [`crate::Corpus::load`].
//!
//! `.ai/tickets/scope-vocab.toml` is the 4-level domain → layer → component → surface
//! word list (T-917.1). This module reads it LENIENTLY — tables of tables of string
//! arrays — because the file's own shape gate (sortedness, closed domain set, no
//! duplicates) lives in `xtask/src/vocab_check.rs` and runs in `ticket check`; here the
//! tree only has to answer legality questions: is this ticket's
//! domain/layer/component/surface a word the vocabulary knows?
//!
//! Fail-closed: a missing or unparseable vocabulary refuses the corpus load naming the
//! path — a legality gate that cannot resolve must not wave tickets through. Scratch
//! corpora built with [`crate::Corpus::new`] never consult the vocabulary (they load
//! nothing); scratch TREES that go through `Corpus::load` must carry a minimal file.

use crate::ScopeV2;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// The vocabulary file, relative to the repo root (same constant as
/// `xtask::vocab_check::VOCAB_REL` — the path is part of the T-917.1 contract).
pub const VOCAB_REL: &str = ".ai/tickets/scope-vocab.toml";

type LayerTable = BTreeMap<String, Vec<String>>;

/// domain → layer → component → surfaces. A component-free layer is an empty
/// component map (the bare `[domain.layer]` header encoding).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScopeVocab {
    tree: BTreeMap<String, BTreeMap<String, LayerTable>>,
}

impl ScopeVocab {
    /// Read `.ai/tickets/scope-vocab.toml` under `root`. Missing file is an error
    /// naming the path (fail-closed — the T-917.2 cutover made scope legality
    /// load-bearing for every corpus load).
    pub fn load(root: &Path) -> Result<Self, String> {
        let path = root.join(VOCAB_REL);
        if !path.is_file() {
            return Err(format!(
                "missing scope vocabulary (required for every corpus load since T-917.2): {}",
                path.display()
            ));
        }
        let text = fs::read_to_string(&path).map_err(|e| format!("read {VOCAB_REL}: {e}"))?;
        Self::parse(&text).map_err(|e| format!("{VOCAB_REL}: {e}"))
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
                "{id}: scope domain \"{domain}\" has no table in {VOCAB_REL}"
            ));
        };
        let Some(components) = layers.get(&scope.layer) else {
            return Err(format!(
                "{id}: scope layer \"{domain}.{}\" is not in {VOCAB_REL}",
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
                "{id}: scope component \"{domain}.{}.{component}\" is not in {VOCAB_REL}",
                scope.layer
            ));
        };
        for s in &scope.surface {
            if !surfaces.iter().any(|v| v == s) {
                return Err(format!(
                    "{id}: scope surface \"{s}\" is not under \"{domain}.{}.{component}\" in {VOCAB_REL}",
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
mod tests {
    use super::*;
    use crate::Domain;

    const MINI: &str =
        "[repo.docs]\n\n[website.frontend]\nmission_creator = [\"map_canvas\", \"toolbelt\"]\n";

    fn scope(domain: Domain, layer: &str, component: Option<&str>, surface: &[&str]) -> ScopeV2 {
        ScopeV2 {
            domain,
            layer: layer.into(),
            component: component.map(str::to_string),
            surface: surface.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    #[test]
    fn legality_walks_the_tree() {
        let v = ScopeVocab::parse(MINI).unwrap();
        v.check_scope("T-1", &scope(Domain::Repo, "docs", None, &[]))
            .expect("component-free layer");
        v.check_scope(
            "T-1",
            &scope(
                Domain::Website,
                "frontend",
                Some("mission_creator"),
                &["toolbelt"],
            ),
        )
        .expect("known surface");

        let err = v
            .check_scope("T-2", &scope(Domain::Repo, "nope", None, &[]))
            .unwrap_err();
        assert!(err.contains("T-2") && err.contains("repo.nope"), "{err}");
        let err = v
            .check_scope(
                "T-3",
                &scope(Domain::Website, "frontend", Some("ghost"), &[]),
            )
            .unwrap_err();
        assert!(
            err.contains("T-3") && err.contains("website.frontend.ghost"),
            "{err}"
        );
        let err = v
            .check_scope(
                "T-4",
                &scope(
                    Domain::Website,
                    "frontend",
                    Some("mission_creator"),
                    &["dock_left"],
                ),
            )
            .unwrap_err();
        assert!(
            err.contains("T-4") && err.contains("\"dock_left\""),
            "{err}"
        );
        let err = v
            .check_scope("T-5", &scope(Domain::Engine, "core", None, &[]))
            .unwrap_err();
        assert!(err.contains("domain \"engine\""), "{err}");
    }

    #[test]
    fn missing_file_refuses_naming_path() {
        let dir = std::env::temp_dir().join(format!("t917-vocab-lib-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".ai/tickets")).unwrap();
        let err = ScopeVocab::load(&dir).unwrap_err();
        assert!(err.contains("scope-vocab.toml"), "{err}");
        std::fs::write(dir.join(VOCAB_REL), MINI).unwrap();
        let v = ScopeVocab::load(&dir).expect("present file loads");
        assert_eq!(
            v.surfaces_of("website", "frontend", "mission_creator"),
            Some(&["map_canvas".to_string(), "toolbelt".to_string()][..])
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
