use super::*;

#[test]
fn live_vocab_file_is_green() {
    let errs = check_as_errors(&worktree_root());
    assert!(
        errs.is_empty(),
        "committed scope-vocab.toml must pass its own gate; got:\n{}",
        errs.join("\n")
    );
}

/// Acceptance: counted shape READ FROM THE FILE at run time. D is pinned to 5
/// (domains are a closed set); L/C/F are asserted populated and printed, never hardcoded.
#[test]
fn counted_shape_from_live_file() {
    let text = fs::read_to_string(vocab_path(&worktree_root())).expect("live vocab");
    let value: toml::Value = text.parse().expect("live vocab parses");
    let domains = value.as_table().expect("top-level table");
    let (mut layers, mut components, mut surfaces) = (0usize, 0usize, 0usize);
    for dv in domains.values() {
        let lt = dv.as_table().expect("domain table");
        layers += lt.len();
        for lv in lt.values() {
            let ct = lv.as_table().expect("layer table");
            components += ct.len();
            for cv in ct.values() {
                surfaces += cv.as_array().expect("surface array").len();
            }
        }
    }
    println!(
        "{} domains, {layers} layers, {components} components, {surfaces} surfaces",
        domains.len()
    );
    assert_eq!(domains.len(), 5, "domains are a closed set of five");
    assert!(layers > 0, "vocabulary must carry layers");
    assert!(components > 0, "vocabulary must carry components");
    assert!(surfaces > 0, "vocabulary must carry surfaces");
}

#[test]
fn missing_file_is_red_naming_path() {
    let dir = std::env::temp_dir().join(format!("t917-vocab-missing-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(crate::repository::TICKETS_DIR)).unwrap();
    let errs = check_as_errors(&dir);
    assert_eq!(errs.len(), 1, "exactly one missing-file error: {errs:?}");
    assert!(
        errs[0].contains("missing") && errs[0].contains(SCOPE_VOCAB),
        "must name the required path: {}",
        errs[0]
    );
    fs::remove_dir_all(&dir).unwrap();
}

/// The acceptance red: a planted duplicate surface under one parent names file, parent
/// path and value; restoring the file restores green.
#[test]
fn duplicate_surface_red_names_file_parent_value() {
    let dir = scratch("dup", GREEN);
    assert!(
        check_as_errors(&dir).is_empty(),
        "base fixture must be green"
    );

    let planted = GREEN.replace("\"map_canvas\", \"toolbelt\"", "\"toolbelt\", \"toolbelt\"");
    fs::write(dir.join(SCOPE_VOCAB), &planted).unwrap();
    let errs = check_as_errors(&dir);
    assert_eq!(errs.len(), 1, "one duplicate-surface error: {errs:?}");
    assert!(
        errs[0].contains(SCOPE_VOCAB)
            && errs[0].contains("website.frontend.mission_creator")
            && errs[0].contains("duplicate surface \"toolbelt\""),
        "must name file + parent + value: {}",
        errs[0]
    );

    fs::write(dir.join(SCOPE_VOCAB), GREEN).unwrap();
    assert!(check_as_errors(&dir).is_empty(), "restore must be green");
    fs::remove_dir_all(&dir).unwrap();
}

/// Unsorted keys go red at every level — domain, layer, and component tier each carry
/// a planted inversion here. This test doubles as the `preserve_order` pin: with a
/// sorted-map `toml` the inversions would vanish at parse and the rule could never fire.
#[test]
fn unsorted_keys_are_red_per_level() {
    let dir = scratch(
        "unsorted-components",
        "[website.frontend]\nsite_pages = []\nmission_creator = []\n",
    );
    let errs = check_as_errors(&dir);
    assert_eq!(errs.len(), 1, "one component-order error: {errs:?}");
    assert!(
        errs[0].contains("website.frontend")
            && errs[0].contains("not sorted")
            && errs[0].contains("site_pages"),
        "must name level path and offending pair: {}",
        errs[0]
    );

    fs::write(
        dir.join(SCOPE_VOCAB),
        "[website.shell]\nnav = []\n\n[website.frontend]\nmission_creator = []\n",
    )
    .unwrap();
    let errs = check_as_errors(&dir);
    assert!(
        errs.len() == 1 && errs[0].contains("website:") && errs[0].contains("shell"),
        "layer inversion must be red naming the domain level: {errs:?}"
    );

    fs::write(
        dir.join(SCOPE_VOCAB),
        "[website.frontend]\nmission_creator = []\n\n[engine.core]\n",
    )
    .unwrap();
    let errs = check_as_errors(&dir);
    assert!(
        errs.len() == 1 && errs[0].contains("top level") && errs[0].contains("website"),
        "domain inversion must be red at the top level: {errs:?}"
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn unknown_domain_is_red() {
    let dir = scratch("domain", "[frontend.mission_creator]\nmap_canvas = []\n");
    let errs = check_as_errors(&dir);
    assert_eq!(errs.len(), 1, "one unknown-domain error: {errs:?}");
    assert!(
        errs[0].contains("unknown domain \"frontend\"") && errs[0].contains("website"),
        "must name the stray domain and the closed set: {}",
        errs[0]
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn empty_values_are_red() {
    let dir = scratch(
        "empty-surface",
        "[website.frontend]\nmission_creator = [\"map_canvas\", \"\"]\n",
    );
    let errs = check_as_errors(&dir);
    assert!(
        errs.len() == 1
            && errs[0].contains("website.frontend.mission_creator")
            && errs[0].contains("empty surface"),
        "empty surface string must be red: {errs:?}"
    );

    fs::write(dir.join(SCOPE_VOCAB), "[website.frontend]\n\"\" = []\n").unwrap();
    let errs = check_as_errors(&dir);
    assert!(
        errs.iter()
            .any(|e| e.contains("website.frontend") && e.contains("empty key")),
        "empty component key must be red: {errs:?}"
    );
    fs::remove_dir_all(&dir).unwrap();
}

/// "No duplicate component names within a layer" arrives with the TOML parse itself
/// (redefining a key is a parse error) — proved red end-to-end, named by file.
#[test]
fn duplicate_component_key_is_parse_red() {
    let dir = scratch(
        "dup-component",
        "[website.frontend]\nmission_creator = []\nmission_creator = []\n",
    );
    let errs = check_as_errors(&dir);
    assert_eq!(errs.len(), 1, "one parse error: {errs:?}");
    assert!(
        errs[0].contains(SCOPE_VOCAB) && errs[0].contains("TOML parse"),
        "duplicate key must surface as a parse red naming the file: {}",
        errs[0]
    );
    fs::remove_dir_all(&dir).unwrap();
}

/// The encoding's non-table/non-array degenerate shapes each get a named red — the
/// serde-unambiguity claim in the file header, held by the walk.
#[test]
fn wrong_value_shapes_are_red() {
    let dir = scratch("shape-layer", "[website]\nfrontend = [\"map_canvas\"]\n");
    let errs = check_as_errors(&dir);
    assert!(
        errs.len() == 1
            && errs[0].contains("website.frontend")
            && errs[0].contains("layer must be a table"),
        "array-valued layer must be red (the OTHER encoding): {errs:?}"
    );

    fs::write(
        dir.join(SCOPE_VOCAB),
        "[website.frontend]\nmission_creator = \"map_canvas\"\n",
    )
    .unwrap();
    let errs = check_as_errors(&dir);
    assert!(
        errs.len() == 1
            && errs[0].contains("website.frontend.mission_creator")
            && errs[0].contains("must be an array"),
        "string-valued component must be red: {errs:?}"
    );

    fs::write(
        dir.join(SCOPE_VOCAB),
        "[website.frontend]\nmission_creator = [7]\n",
    )
    .unwrap();
    let errs = check_as_errors(&dir);
    assert!(
        errs.len() == 1 && errs[0].contains("surface entries must be strings"),
        "non-string surface must be red: {errs:?}"
    );
    fs::remove_dir_all(&dir).unwrap();
}
