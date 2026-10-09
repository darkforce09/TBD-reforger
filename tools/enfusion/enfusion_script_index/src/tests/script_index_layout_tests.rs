use super::*;

/// The committed verdict table exists in the checkout.
///
/// A relocation of the documentation tree rewrites this value, and a value left behind fails
/// quietly: `enf capability` joins against a missing table and reports every framework file as
/// untriaged.
#[test]
fn the_capability_verdict_table_exists_in_the_checkout() {
    let root = ::repository_root::find_repository_root().expect("active checkout");
    assert!(
        root.join(CAPABILITY_VERDICTS).is_file(),
        "missing: {CAPABILITY_VERDICTS}"
    );
}

/// The index folder is pipeline output inside the agent artifact tree, and the upstream table lies
/// inside the index folder.
#[test]
fn the_symbol_index_lies_inside_the_agent_artifact_tree() {
    assert!(ENF_INDEX_DIR.starts_with(&format!("{}/", ::repository_layout::ARTIFACTS_DIR)));
    assert!(CRF_SYMBOL_TABLE.starts_with(&format!("{ENF_INDEX_DIR}/")));
}
