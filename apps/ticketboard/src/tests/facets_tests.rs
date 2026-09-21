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
    std::fs::write(
        s.path().join(ticket_engine::repository::SCOPE_VOCAB),
        "domains = 3",
    )
    .unwrap();
    assert_eq!(VocabTree::load(s.path()), None);
    // Fixed on disk: loads.
    std::fs::write(
        s.path().join(ticket_engine::repository::SCOPE_VOCAB),
        FIXTURE,
    )
    .unwrap();
    assert_eq!(VocabTree::load(s.path()), Some(fixture_vocab()));
}
