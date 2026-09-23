use super::*;
use crate::repository_paths::find_repo_root;
use std::collections::BTreeSet;

/// Every [`documentation`] constant that names a location a checkout must hold, by name.
const REQUIRED_DOCUMENTATION_LOCATIONS: [(&str, &str); 3] = [
    ("MOD_DOCS_DIR", documentation::MOD_DOCS_DIR),
    ("CAPABILITY_VERDICTS", documentation::CAPABILITY_VERDICTS),
    ("EDITOR_GATE_RUNBOOK", documentation::EDITOR_GATE_RUNBOOK),
];

/// [`documentation`] items that name no location a checkout must hold, each with the reason.
const EXEMPT_DOCUMENTATION_ITEMS: [(&str, &str); 0] = [];

/// Every declared location must exist in a real checkout.
///
/// A path helper that silently resolves to nothing is worse than a literal: callers read it as a
/// guarantee, and a directory walk over a missing root reports "no findings" rather than failing.
/// This pins the whole surface against the live tree so a relocation cannot half-land.
#[test]
fn every_declared_location_exists_in_the_checkout() {
    let root = find_repo_root().expect("active checkout");

    for dir in [
        contracts_dir(&root),
        contract_definitions_dir(&root),
        contract_rules_dir(&root),
        contract_catalogs_dir(&root),
        contract_fixtures_dir(&root),
        mission_fixtures_valid_dir(&root),
        mission_fixtures_invalid_dir(&root),
        map_fixtures_dir(&root),
        density_fixtures_dir(&root),
        registry_fixtures_dir(&root),
        enfusion_sample_fixtures_dir(&root),
        bridge_sample_fixtures_dir(&root),
        terrain_assets_dir(&root),
        terrain_dir(&root, "everon"),
        glyph_assets_dir(&root),
        enfusion_mcp_node_package_dir(&root),
    ] {
        assert!(dir.is_dir(), "not a directory: {}", dir.display());
    }

    for file in [
        definition_path(&root, "mission.schema.json"),
        prefab_classify_path(&root),
        kit_aliases_path(&root),
        registry_items_catalog_path(&root),
        registry_compat_catalog_path(&root),
        terrain_registry_path(&root),
        terrain_manifest_path(&root, "everon"),
        glyph_manifest_path(&root),
    ] {
        assert!(file.is_file(), "not a file: {}", file.display());
    }
}

/// Export scratch is deliberately absent from a fresh clone, so it is pinned by shape rather than
/// existence: it is named for its island, and it sits OUTSIDE the terrain tree.
///
/// That separation is the invariant worth a test. The terrain tree is served wholesale at
/// `/map-assets`, so scratch nested inside it would publish gigabytes of uncommitted export
/// intermediates to every map client.
#[test]
fn export_scratch_is_named_for_its_island_and_sits_outside_the_served_tree() {
    let root = find_repo_root().expect("active checkout");
    let scratch = map_scratch_dir(&root, "everon");
    assert!(scratch.ends_with("everon"), "{}", scratch.display());
    assert!(
        !scratch.starts_with(terrain_assets_dir(&root)),
        "{}",
        scratch.display()
    );
}

/// The installed server module is absent from a fresh clone, so it is pinned by shape: it lives
/// inside the package directory whose manifest declares it.
///
/// The two constants are separate literals, and a relocation that moved one without the other
/// would leave `npm ci` installing into one directory while every runner looked in another — a
/// mismatch that shows up only as a silent fall-through to a network download.
#[test]
fn the_enfusion_mcp_entrypoint_sits_inside_its_npm_package_directory() {
    assert!(
        ENFUSION_MCP_ENTRYPOINT
            .starts_with(&format!("{ENFUSION_MCP_NODE_PACKAGE_DIR}/node_modules/")),
        "{ENFUSION_MCP_ENTRYPOINT} is not installed under {ENFUSION_MCP_NODE_PACKAGE_DIR}"
    );
    let root = Path::new("/tmp/checkout");
    assert!(enfusion_mcp_entrypoint(root).starts_with(enfusion_mcp_node_package_dir(root)));
}

/// Locations are resolved against the caller's root, never an ambient one.
#[test]
fn locations_resolve_against_the_given_root() {
    let root = Path::new("/tmp/checkout");
    for path in [
        contracts_dir(root),
        contract_definitions_dir(root),
        terrain_assets_dir(root),
        glyph_assets_dir(root),
        map_scratch_dir(root, "everon"),
    ] {
        assert!(
            path.starts_with(root),
            "escaped the root: {}",
            path.display()
        );
    }
}

/// Every required documentation location exists in the checkout.
///
/// A relocation of the documentation tree rewrites these values, and a value left behind fails
/// quietly: `enf citations` walks a missing documentation root, checks nothing and passes, and the
/// font-cache diagnostic points at a dead runbook.
#[test]
fn every_required_documentation_location_exists_in_the_checkout() {
    let root = find_repo_root().expect("active checkout");
    let missing: Vec<String> = REQUIRED_DOCUMENTATION_LOCATIONS
        .iter()
        .filter(|(_, path)| !present(&root, path))
        .map(|(name, path)| format!("{name} = {path}"))
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
    let classified: Vec<&str> = REQUIRED_DOCUMENTATION_LOCATIONS
        .iter()
        .map(|(name, _)| *name)
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
    let declared = declared_documentation_items(include_str!("../repository_layout.rs"));
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
