//! Unit tests for [`crate::local_database::recipe_execution`]: the registry import's argv and its
//! echoed line, anchored on the repository root.

use super::*;

#[test]
fn the_registry_import_names_both_envelopes_from_the_repository_root() {
    let root = Path::new("/checkout");
    assert_eq!(
        registry_import_arguments(root),
        [
            "cargo",
            "run",
            "--bin",
            "import-item-registry",
            "--",
            "--items",
            "/checkout/contracts/catalogs/registry-items.workbench.json",
            "--compat",
            "/checkout/contracts/catalogs/registry-compat.workbench.json",
        ]
    );
}

#[test]
fn the_registry_import_echo_continues_before_each_envelope_flag() {
    let argv = registry_import_arguments(Path::new("/checkout"));
    assert_eq!(
        registry_import_echo(&argv),
        "cargo run --bin import-item-registry -- \\\n\t--items \
         /checkout/contracts/catalogs/registry-items.workbench.json \\\n\t--compat \
         /checkout/contracts/catalogs/registry-compat.workbench.json"
    );
}

/// The live checkout: both envelopes resolve to committed files, however deep the API crate
/// folder the importer runs in sits, and no argument climbs out of a folder.
#[test]
fn this_checkout_resolves_both_envelopes_and_no_argument_climbs() {
    let root = tool_test_support::test_repo_root();
    let argv = registry_import_arguments(&root);
    for envelope in [&argv[6], &argv[8]] {
        assert!(Path::new(envelope).is_file(), "{envelope} is no file");
    }
    assert!(
        argv.iter().all(|argument| !argument.contains("..")),
        "{argv:?}"
    );
    assert!(
        web_under(&root).abs.join("Cargo.toml").is_file(),
        "the importer runs in the API crate folder"
    );
}
