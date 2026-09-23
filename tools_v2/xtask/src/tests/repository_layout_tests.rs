use super::*;
use crate::core::repository_root::test_repo_root;
use std::{collections::BTreeSet, path::Path};

/// Every [`documentation`] item that names locations a checkout must hold, by name, with every
/// location it names: one for a path constant or a re-export, each element for a list.
const REQUIRED_DOCUMENTATION_LOCATIONS: [(&str, &[&str]); 21] = [
    ("FACTORY_PACK_WAVE", &[documentation::FACTORY_PACK_WAVE]),
    ("LAYOUT_TARGET_DIR", &[documentation::LAYOUT_TARGET_DIR]),
    ("HOME_SERVER_RUNBOOK", &[documentation::HOME_SERVER_RUNBOOK]),
    (
        "STAGING_SERVER_RUNBOOK",
        &[documentation::STAGING_SERVER_RUNBOOK],
    ),
    (
        "SLICE_WORKFLOW_RUNBOOK",
        &[documentation::SLICE_WORKFLOW_RUNBOOK],
    ),
    (
        "PLATFORM_FACTORY_RUNBOOK",
        &[documentation::PLATFORM_FACTORY_RUNBOOK],
    ),
    ("MOD_DESIGN", &[documentation::MOD_DESIGN]),
    (
        "SPAWN_DETERMINISM_RUNBOOK",
        &[documentation::SPAWN_DETERMINISM_RUNBOOK],
    ),
    (
        "API_READINESS_EVIDENCE_PREFIX",
        &[documentation::API_READINESS_EVIDENCE_PREFIX],
    ),
    (
        "API_READINESS_REGISTER",
        &[documentation::API_READINESS_REGISTER],
    ),
    ("CODE_TREES", documentation::CODE_TREES),
    ("DOCUMENTATION_ROOT", &[documentation::DOCUMENTATION_ROOT]),
    ("ARCHIVE_DIR", &[documentation::ARCHIVE_DIR]),
    (
        "TICKET_DOCUMENTS_DIR",
        &[documentation::TICKET_DOCUMENTS_DIR],
    ),
    ("CURSOR_RULE_DIRS", documentation::CURSOR_RULE_DIRS),
    (
        "PROJECT_INSTRUCTIONS",
        &[documentation::PROJECT_INSTRUCTIONS],
    ),
    ("TREE_DIR", &[documentation::TREE_DIR]),
    ("SPECS_DIR", &[documentation::SPECS_DIR]),
    ("PLANS_DIR", &[documentation::PLANS_DIR]),
    ("ROADMAP", &[documentation::ROADMAP]),
    ("GAP_ANALYSIS", &[documentation::GAP_ANALYSIS]),
];

/// [`documentation`] items that name no location a checkout must hold, each with the reason.
const EXEMPT_DOCUMENTATION_ITEMS: [(&str, &str); 4] = [
    (
        "PENDING_MERGE_DIR",
        "holds merge sources only while a merge is pending, and is absent otherwise",
    ),
    (
        "PROGRAM_RECORDS_PREFIX",
        "a file-name prefix that matches the program records at the documentation root, not a \
         location",
    ),
    (
        "RETIRED_DOCS_ROOT",
        "names a folder that must hold no tracked file; markdown-placement fails while it does",
    ),
    (
        "PERMALINK_BASE",
        "a GitHub URL prefix, not a location in the checkout",
    ),
];

/// Every committed location must exist in a real checkout.
///
/// A constant that names nothing is worse than a literal: a command joins it onto the root and
/// reads a missing file, which surfaces as "not configured" rather than as a broken path.
#[test]
fn every_committed_location_exists_in_the_checkout() {
    let root = test_repo_root();

    for directory in [
        DEPLOY_DIR,
        SYSTEMD_UNITS_DIR,
        DEDICATED_SERVER_PROFILES_DIR,
        MCP_TRANSCRIPT_FIXTURES_DIR,
    ] {
        assert!(
            root.join(directory).is_dir(),
            "not a directory: {directory}"
        );
    }

    for file in [
        DEPLOY_ENV_EXAMPLE,
        CADDYFILE,
        WEBSITE_API_UNIT,
        DEV_SERVER_PROFILE,
    ] {
        assert!(root.join(file).is_file(), "not a file: {file}");
    }
}

/// The host secrets file is never committed, so it is pinned by shape: it sits beside the example
/// an operator copies, and the deploy's own exclude list is built from this constant.
#[test]
fn the_deploy_secrets_file_sits_beside_its_example() {
    assert_eq!(
        DEPLOY_ENV_EXAMPLE,
        format!("{DEPLOY_ENV}.example"),
        "the example must be the secrets path plus `.example`"
    );
    assert!(DEPLOY_ENV.starts_with(&format!("{DEPLOY_DIR}/")));
}

/// Each file constant names something inside the directory constant that describes its kind, so
/// moving a directory cannot leave a file behind.
#[test]
fn every_file_sits_inside_the_directory_that_describes_it() {
    for (file, directory) in [
        (DEPLOY_ENV_EXAMPLE, DEPLOY_DIR),
        (CADDYFILE, DEPLOY_DIR),
        (WEBSITE_API_UNIT, SYSTEMD_UNITS_DIR),
        (DEV_SERVER_PROFILE, DEDICATED_SERVER_PROFILES_DIR),
    ] {
        assert!(
            file.starts_with(&format!("{directory}/")),
            "{file} is not inside {directory}"
        );
    }
}

/// The register sits inside the tree the api-readiness source fingerprint covers, so an edit to
/// the register invalidates recorded evidence. The prefix ends in `/` because the fingerprint
/// matches it with `starts_with`.
#[test]
fn the_api_readiness_register_sits_inside_the_fingerprinted_evidence_tree() {
    use documentation::{API_READINESS_EVIDENCE_PREFIX, API_READINESS_REGISTER};
    assert!(
        API_READINESS_EVIDENCE_PREFIX.ends_with('/'),
        "{API_READINESS_EVIDENCE_PREFIX} must end in `/`"
    );
    assert!(
        API_READINESS_REGISTER.starts_with(API_READINESS_EVIDENCE_PREFIX),
        "{API_READINESS_REGISTER} is not under {API_READINESS_EVIDENCE_PREFIX}"
    );
}

/// Every required documentation location exists in the checkout.
///
/// A relocation of the documentation tree rewrites these values, and most of them fail quietly
/// when left behind: the wave gate treats a missing factory marker as absent and falls back to the
/// ledger, and a refusal that names a runbook prints a dead path.
#[test]
fn every_required_documentation_location_exists_in_the_checkout() {
    let root = test_repo_root();
    let missing: Vec<String> = REQUIRED_DOCUMENTATION_LOCATIONS
        .iter()
        .flat_map(|(name, paths)| paths.iter().map(move |path| (*name, *path)))
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
    let declared = declared_documentation_items(include_str!("../core/repository_layout.rs"));
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
/// source: every `pub const`, `pub static` and `pub fn` at the module's own indentation, and every
/// name a `pub use` at that indentation re-exports.
fn declared_documentation_items(source: &str) -> BTreeSet<&str> {
    let (_, module) = source
        .split_once("\npub mod documentation {\n")
        .expect("the layout module declares `pub mod documentation`");
    let (module, _) = module
        .split_once("\n}\n")
        .expect("the documentation module closes at column zero");
    let mut declared: BTreeSet<&str> = module
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
        .collect();
    declared.extend(reexported_names(module));
    declared
}

/// The names every `pub use` at a module's own indentation brings in, whether the statement names
/// one item or a braced list, on one line or across several; `X as Y` contributes `Y`.
fn reexported_names(module: &str) -> Vec<&str> {
    let mut names = Vec::new();
    let mut lines = module.lines();
    while let Some(line) = lines.next() {
        let Some(head) = line.strip_prefix("    pub use ") else {
            continue;
        };
        let mut statement = vec![head];
        while !statement.last().is_some_and(|piece| piece.contains(';')) {
            statement.push(lines.next().expect("a `pub use` statement ends in `;`"));
        }
        let braced = statement.iter().any(|piece| piece.contains('{'));
        for piece in statement {
            let piece = if braced {
                piece.split_once('{').map_or(piece, |(_, list)| list)
            } else {
                piece.rsplit("::").next().unwrap_or(piece)
            };
            let piece = piece.split(['}', ';']).next().unwrap_or(piece);
            names.extend(
                piece
                    .split(',')
                    .map(|item| item.rsplit(" as ").next().unwrap_or(item).trim())
                    .filter(|name| !name.is_empty()),
            );
        }
    }
    names
}

/// The re-export reader sees every shape rustfmt writes a `pub use` in, so a re-exported name
/// cannot slip past the classification check by being wrapped differently.
#[test]
fn the_reexport_reader_sees_every_statement_shape() {
    let module = "    /// docs\n    pub use a::b::ONE;\n    pub use a::{TWO, THREE as FOUR};\n    \
                  #[allow(unused_imports)]\n    pub use a::b::{\n        FIVE, SIX,\n    };\n    \
                  pub const SEVEN: &str = \"x\";\n";
    assert_eq!(
        reexported_names(module),
        ["ONE", "TWO", "FOUR", "FIVE", "SIX"]
    );
}

/// The areas the documentation gates freeze, skip or read as records sit inside the
/// documentation root, so relocating the root cannot leave one of them behind.
#[test]
fn the_documentation_areas_sit_inside_the_documentation_root() {
    use documentation::{
        ARCHIVE_DIR, DOCUMENTATION_ROOT, PENDING_MERGE_DIR, PROGRAM_RECORDS_PREFIX,
        TICKET_DOCUMENTS_DIR,
    };
    for area in [
        ARCHIVE_DIR,
        TICKET_DOCUMENTS_DIR,
        PENDING_MERGE_DIR,
        PROGRAM_RECORDS_PREFIX,
    ] {
        assert!(
            area.starts_with(&format!("{DOCUMENTATION_ROOT}/")),
            "{area} is not inside {DOCUMENTATION_ROOT}"
        );
    }
}

/// The code trees, the documentation root and the retired documentation root are distinct
/// top-level folders: the documentation gates match them against the first path component.
#[test]
fn the_documentation_gate_roots_are_distinct_top_level_folders() {
    let mut roots: Vec<&str> = documentation::CODE_TREES.to_vec();
    roots.push(documentation::DOCUMENTATION_ROOT);
    roots.push(documentation::RETIRED_DOCS_ROOT);
    for root in &roots {
        assert!(
            !root.is_empty() && !root.contains('/'),
            "{root} is not a top-level folder name"
        );
    }
    let unique: BTreeSet<&str> = roots.iter().copied().collect();
    assert_eq!(unique.len(), roots.len(), "a gate root is listed twice");
}

/// A permalink is the prefix, a commit and a path, so the prefix is this repository's blob view
/// and ends in `/`.
#[test]
fn the_permalink_base_is_a_blob_url_prefix() {
    assert!(documentation::PERMALINK_BASE.starts_with("https://github.com/"));
    assert!(documentation::PERMALINK_BASE.ends_with("/blob/"));
}
