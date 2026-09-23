use super::*;
use std::collections::BTreeSet;

/// Every [`documentation`] item that names locations a checkout must hold, by name, with each
/// path it holds.
fn required_documentation_locations() -> Vec<(&'static str, Vec<&'static str>)> {
    vec![
        ("TREE_DIR", vec![documentation::TREE_DIR]),
        ("PLANS_DIR", vec![documentation::PLANS_DIR]),
        ("PLAN_TEMPLATE", vec![documentation::PLAN_TEMPLATE]),
        ("SPECS_DIR", vec![documentation::SPECS_DIR]),
        ("ROADMAP", vec![documentation::ROADMAP]),
        ("GAP_ANALYSIS", vec![documentation::GAP_ANALYSIS]),
        (
            "TOKEN_ESTIMATE_FACTOR_DOC",
            vec![documentation::TOKEN_ESTIMATE_FACTOR_DOC],
        ),
        (
            "ARCHIVED_WAVE_PLAN_READERS",
            documentation::ARCHIVED_WAVE_PLAN_READERS
                .iter()
                .map(|(path, _)| *path)
                .collect(),
        ),
        (
            "STALE_TICKET_ID_SCAN_ROOTS",
            documentation::STALE_TICKET_ID_SCAN_ROOTS.to_vec(),
        ),
    ]
}

/// [`documentation`] items that name no location a checkout must hold, each with the reason.
const EXEMPT_DOCUMENTATION_ITEMS: [(&str, &str); 5] = [
    (
        "plan_path",
        "derives one ticket's plan under PLANS_DIR; mark-ready refuses while that file is absent",
    ),
    (
        "RETIRED_QUEUE_VIEW_PREFIX",
        "a prefix of retired files, matched against the paths historical numstat carries",
    ),
    (
        "SCAN_EXEMPT_PREFIXES",
        "prefixes the stale-identifier scan skips; a skipped tree need not exist",
    ),
    (
        "NUMSTAT_EXCLUDED_PREFIXES",
        "path prefixes the token estimator drops from a commit's changed-line count",
    ),
    (
        "ARCHIVED_WAVE_PLANS",
        "historical and deleted by design; read at past revisions through git show",
    ),
];

/// The sparse-checkout sets are keyed by the target vocabulary a ticket's `[scope]` uses, so a
/// target the validator accepts always resolves to a set of paths. Divergence would make
/// `ticket sparse-paths` silently emit a checkout missing the tree the work happens in.
#[test]
fn every_ticket_target_has_a_sparse_checkout_set() {
    let keys: Vec<&str> = SPARSE_CHECKOUT_SETS.iter().map(|(name, _)| *name).collect();
    assert_eq!(keys, crate::validation::constants::VALID_TARGETS);
}

/// A root slice checks out the tooling tree, the registry, the artifacts it writes and the
/// documentation it edits — and nothing that stopped existing.
#[test]
fn the_root_sparse_set_carries_the_task_surface() {
    let (_, root_set) = SPARSE_CHECKOUT_SETS
        .iter()
        .find(|(name, _)| *name == "root")
        .expect("a root target");
    for expected in ["tools_v2", ".cargo", TICKETS_DIR, ARTIFACTS_DIR] {
        assert!(root_set.contains(&expected), "root set lacks {expected}");
    }
}

/// A ticket's plan path is derived, not stored: lowercase id with dots as underscores, under the
/// plans directory.
#[test]
fn a_plan_path_is_the_lowercased_id_under_the_plans_directory() {
    assert_eq!(
        documentation::plan_path("T-917.6"),
        "docs/plans/t-917_6_plan.md"
    );
    assert!(documentation::plan_path("T-090.4").starts_with(documentation::PLANS_DIR));
}

/// A handoff document belongs to the artifact tree, which a gate never reads as input.
#[test]
fn a_handoff_document_lands_in_the_artifact_tree() {
    let path = handoff_doc("t090_1");
    assert!(path.starts_with(ARTIFACTS_DIR), "{path}");
    assert!(path.ends_with("_claude_code_handoff.md"), "{path}");
}

/// Every path a sparse-checkout set names exists in the checkout. `ticket sparse-paths` prints
/// them for a slice's sparse checkout, and a path that names nothing checks out nothing: the
/// slice lacks the tree its work needs, and no command says so.
#[test]
fn every_sparse_checkout_path_exists_in_the_checkout() {
    let root = find_repo_root().expect("repository root");
    let missing: Vec<String> = SPARSE_CHECKOUT_SETS
        .iter()
        .flat_map(|(target, paths)| paths.iter().map(move |path| (*target, *path)))
        .filter(|(_, path)| !present(&root, path))
        .map(|(target, path)| format!("{target}: {path}"))
        .collect();
    assert!(
        missing.is_empty(),
        "sparse-checkout paths missing from the checkout: {missing:#?}"
    );
}

/// Every required documentation location exists in the checkout.
///
/// A relocation of the documentation tree rewrites these values, and a value left behind fails
/// quietly: `ticket sync` skips the roadmap block and the gap-analysis column when their file is
/// missing, the stale-identifier scan walks past a missing root, and a stale archived-wave-plan
/// reader entry excuses nothing while it stays on the list.
#[test]
fn every_required_documentation_location_exists_in_the_checkout() {
    let root = find_repo_root().expect("repository root");
    let missing: Vec<String> = required_documentation_locations()
        .into_iter()
        .flat_map(|(name, paths)| paths.into_iter().map(move |path| (name, path)))
        .filter(|(_, path)| !present(&root, path))
        .map(|(name, path)| format!("{name}: {path}"))
        .collect();
    assert!(
        missing.is_empty(),
        "documentation locations missing from the checkout: {missing:#?}"
    );
}

/// Every item the [`documentation`] module declares is classified above, as a required location
/// or as an exemption with its reason.
///
/// The names are read from the module's own source, so an item added without a row here fails
/// this test instead of slipping past the existence check.
#[test]
fn every_documentation_item_is_classified() {
    let classified: Vec<&str> = required_documentation_locations()
        .into_iter()
        .map(|(name, _)| name)
        .chain(EXEMPT_DOCUMENTATION_ITEMS.iter().map(|(name, _)| *name))
        .collect();
    let unique: BTreeSet<&str> = classified.iter().copied().collect();
    assert_eq!(
        unique.len(),
        classified.len(),
        "an item is classified twice"
    );
    assert!(
        EXEMPT_DOCUMENTATION_ITEMS
            .iter()
            .all(|(_, reason)| !reason.trim().is_empty()),
        "every exemption carries its reason"
    );
    let declared = declared_documentation_items(include_str!("../repository.rs"));
    let unclassified: Vec<&str> = declared.difference(&unique).copied().collect();
    let undeclared: Vec<&str> = unique.difference(&declared).copied().collect();
    assert!(
        unclassified.is_empty() && undeclared.is_empty(),
        "classify every `documentation` item as required or exempt; unclassified: \
         {unclassified:?}; classified but not declared: {undeclared:?}"
    );
}

/// Whether `path` names something inside `root`. A path ending in `/` is a prefix, so it must
/// name a directory; any other path may name a file or a directory.
fn present(root: &Path, path: &str) -> bool {
    if path.ends_with('/') {
        root.join(path).is_dir()
    } else {
        root.join(path).exists()
    }
}

/// The names a layout module's `documentation` module declares, read from the layout module's
/// source: every `pub const`, `pub static` and `pub fn` at the module's own indentation.
fn declared_documentation_items(source: &str) -> BTreeSet<&str> {
    let (_, module) = source
        .split_once("\npub mod documentation {\n")
        .expect("the layout module declares `pub mod documentation`");
    let (module, _) = module
        .split_once("\n}\n")
        .expect("the documentation module closes at column zero");
    module
        .lines()
        .filter_map(|line| {
            ["    pub const ", "    pub static ", "    pub fn "]
                .into_iter()
                .find_map(|keyword| line.strip_prefix(keyword))
        })
        .filter_map(|declaration| {
            declaration
                .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .next()
        })
        .collect()
}
