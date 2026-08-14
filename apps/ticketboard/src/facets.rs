//! Scope facet options (T-918.1) — the vocab tree + the narrowed per-level
//! dropdown values. Pure, unit-tested, no egui types.
//!
//! `.ai/tickets/scope-vocab.toml` feeds facet dropdown VALUES only — strictly a
//! DISPLAY input. The board never validates tickets against it (`ticket check`
//! is the sole legality authority). `tbd_tickets::ScopeVocab` is not reused here
//! because it answers point legality questions and exposes no enumeration — the
//! dropdowns need to LIST values, so this module re-reads the same file into an
//! enumerable tree. A missing or broken vocab file degrades to
//! values-present-in-corpus ([`VocabTree::load`] → `None`) — never a crash,
//! never a refusal.
//!
//! Each dropdown offers the union of vocabulary values and corpus-present values
//! (narrowed by the higher facet selections); corpus values the vocabulary does
//! not know are marked (`vocab_unknown`) so drift is visible without ever being
//! enforced here.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::filters::{RowFacts, ScopeFacets};

/// The vocabulary file, relative to the repo root — the same path constant as
/// `tbd_tickets::vocab::VOCAB_REL` (part of the T-917.1 contract).
pub const VOCAB_REL: &str = ".ai/tickets/scope-vocab.toml";

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
        let text = fs::read_to_string(root.join(VOCAB_REL)).ok()?;
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
mod tests {
    use super::*;
    use crate::filters::FilterIndex;
    use crate::testutil::{Scratch, corpus_of, program, work_scoped};

    /// Fixture mirror of the live vocab's shape: two domains, component-free
    /// layers, componented layers with and without surfaces.
    const FIXTURE: &str = r#"
[repo.docs]

[repo.xtask]
check = []
wave = []

[website.backend]
http_api = []

[website.frontend]
mission_creator = ["map_canvas", "toolbelt"]
shell = ["router"]

[website.shared]

[website.tests]
"#;

    fn fixture_vocab() -> VocabTree {
        VocabTree::parse(FIXTURE).expect("fixture parses")
    }

    /// Corpus with one editor ticket (one surface the vocab does NOT know), one
    /// backend ticket, one repo ticket in a layer the vocab does not know, and a
    /// program (contributes nothing).
    fn rows() -> FilterIndex {
        FilterIndex::build(&corpus_of(vec![
            work_scoped(
                "T-1",
                "domain = \"website\"\nlayer = \"frontend\"\ncomponent = \"mission_creator\"\nsurface = [\"map_canvas\", \"ghost_surface\"]",
                "",
            ),
            work_scoped(
                "T-2",
                "domain = \"website\"\nlayer = \"backend\"\ncomponent = \"http_api\"",
                "",
            ),
            work_scoped("T-3", "domain = \"repo\"\nlayer = \"attic\"", ""),
            program("T-9", "status = \"idea\"", &["T-9.1"]),
        ]))
    }

    fn values(options: &[FacetOption]) -> Vec<&str> {
        options.iter().map(|o| o.value.as_str()).collect()
    }

    fn unknown_values(options: &[FacetOption]) -> Vec<&str> {
        options
            .iter()
            .filter(|o| o.vocab_unknown)
            .map(|o| o.value.as_str())
            .collect()
    }

    /// T-918.1 acceptance fixture: selecting domain=website narrows the layer
    /// dropdown to the website layers (backend/frontend/shared/tests), then
    /// layer=frontend narrows components, then component narrows surfaces.
    #[test]
    fn vocab_narrowing_walks_down_the_tree() {
        let vocab = fixture_vocab();
        let idx = rows();
        let mut sel = ScopeFacets::default();

        // No selection: unions across the whole tree (+ corpus strays).
        let opts = compute(Some(&vocab), &idx.rows, &mut sel);
        assert_eq!(values(&opts.domains), vec!["repo", "website"]);
        assert_eq!(
            values(&opts.layers),
            vec![
                "attic", "backend", "docs", "frontend", "shared", "tests", "xtask"
            ]
        );

        // domain=website ⇒ layers narrow to the website subtree.
        sel.domain = Some("website".to_owned());
        let opts = compute(Some(&vocab), &idx.rows, &mut sel);
        assert_eq!(
            values(&opts.layers),
            vec!["backend", "frontend", "shared", "tests"]
        );
        assert_eq!(
            values(&opts.components),
            vec!["http_api", "mission_creator", "shell"]
        );

        // layer=frontend ⇒ components narrow; surfaces union both components.
        sel.layer = Some("frontend".to_owned());
        let opts = compute(Some(&vocab), &idx.rows, &mut sel);
        assert_eq!(values(&opts.components), vec!["mission_creator", "shell"]);
        assert_eq!(
            values(&opts.surfaces),
            vec!["ghost_surface", "map_canvas", "router", "toolbelt"]
        );

        // component=mission_creator ⇒ surfaces narrow to that component.
        sel.component = Some("mission_creator".to_owned());
        let opts = compute(Some(&vocab), &idx.rows, &mut sel);
        assert_eq!(
            values(&opts.surfaces),
            vec!["ghost_surface", "map_canvas", "toolbelt"]
        );
        // Selection untouched — everything picked is still offered.
        assert_eq!(sel.component.as_deref(), Some("mission_creator"));
    }

    /// Corpus values the vocabulary does not know are offered AND marked; vocab
    /// values are never marked.
    #[test]
    fn corpus_strays_are_offered_and_marked() {
        let vocab = fixture_vocab();
        let idx = rows();
        let mut sel = ScopeFacets::default();
        let opts = compute(Some(&vocab), &idx.rows, &mut sel);
        // repo.attic exists only in the corpus: offered, marked.
        assert_eq!(unknown_values(&opts.layers), vec!["attic"]);
        // ghost_surface exists only in the corpus: offered, marked.
        assert_eq!(unknown_values(&opts.surfaces), vec!["ghost_surface"]);
        assert_eq!(unknown_values(&opts.domains), Vec::<&str>::new());
        assert_eq!(unknown_values(&opts.components), Vec::<&str>::new());
    }

    /// No vocabulary ⇒ every dropdown is exactly the corpus-present values,
    /// narrowing still works (corpus-driven), and nothing is marked unknown.
    #[test]
    fn missing_vocab_falls_back_to_corpus_values() {
        let idx = rows();
        let mut sel = ScopeFacets::default();
        let opts = compute(None, &idx.rows, &mut sel);
        assert_eq!(values(&opts.domains), vec!["repo", "website"]);
        assert_eq!(values(&opts.layers), vec!["attic", "backend", "frontend"]);
        assert_eq!(unknown_values(&opts.layers), Vec::<&str>::new());

        sel.domain = Some("website".to_owned());
        let opts = compute(None, &idx.rows, &mut sel);
        assert_eq!(values(&opts.layers), vec!["backend", "frontend"]);
        sel.layer = Some("frontend".to_owned());
        let opts = compute(None, &idx.rows, &mut sel);
        assert_eq!(values(&opts.components), vec!["mission_creator"]);
        assert_eq!(values(&opts.surfaces), vec!["ghost_surface", "map_canvas"]);
    }

    /// Changing a higher facet clears lower selections its narrowed dropdowns no
    /// longer offer — never a stale invisible constraint.
    #[test]
    fn stale_lower_selections_are_cleared_top_down() {
        let vocab = fixture_vocab();
        let idx = rows();
        let mut sel = ScopeFacets {
            domain: Some("website".to_owned()),
            layer: Some("frontend".to_owned()),
            component: Some("mission_creator".to_owned()),
            surface: Some("toolbelt".to_owned()),
        };
        // Same selection recomputes as a no-op.
        compute(Some(&vocab), &idx.rows, &mut sel);
        assert_eq!(sel.surface.as_deref(), Some("toolbelt"));

        // Domain flips to repo: frontend/mission_creator/toolbelt all vanish.
        sel.domain = Some("repo".to_owned());
        let opts = compute(Some(&vocab), &idx.rows, &mut sel);
        assert_eq!(sel.layer, None);
        assert_eq!(sel.component, None);
        assert_eq!(sel.surface, None);
        assert_eq!(values(&opts.layers), vec!["attic", "docs", "xtask"]);

        // A surviving lower selection is kept: shell → mission_creator swap
        // keeps layer=frontend.
        let mut sel = ScopeFacets {
            domain: Some("website".to_owned()),
            layer: Some("frontend".to_owned()),
            component: Some("shell".to_owned()),
            surface: Some("router".to_owned()),
        };
        sel.component = Some("mission_creator".to_owned());
        compute(Some(&vocab), &idx.rows, &mut sel);
        assert_eq!(sel.layer.as_deref(), Some("frontend"));
        assert_eq!(sel.surface, None, "router is not under mission_creator");
    }

    /// Broken vocab shapes refuse the parse (⇒ `load` → `None` ⇒ corpus
    /// fallback); a missing file is `None` without touching the parse.
    #[test]
    fn broken_or_missing_vocab_is_none_never_a_crash() {
        assert!(VocabTree::parse("domains = 3").is_err());
        assert!(VocabTree::parse("[repo]\ndocs = 5").is_err());
        assert!(VocabTree::parse("[website.frontend]\nmission_creator = [1]").is_err());
        assert!(VocabTree::parse("not toml [").is_err());

        let s = Scratch::new("facets-no-vocab");
        assert_eq!(VocabTree::load(s.path()), None);
        // Present-but-broken file: still None.
        std::fs::create_dir_all(s.path().join(".ai/tickets")).unwrap();
        std::fs::write(s.path().join(VOCAB_REL), "domains = 3").unwrap();
        assert_eq!(VocabTree::load(s.path()), None);
        // Fixed on disk: loads.
        std::fs::write(s.path().join(VOCAB_REL), FIXTURE).unwrap();
        assert_eq!(VocabTree::load(s.path()), Some(fixture_vocab()));
    }
}
