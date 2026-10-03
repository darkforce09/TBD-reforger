use super::*;
use crate::find_repository_root;

/// The npm package folder is committed, so it exists in a real checkout.
#[test]
fn the_enfusion_mcp_node_package_exists_in_the_checkout() {
    let root = find_repository_root().expect("active checkout");
    let package = enfusion_mcp_node_package_dir(&root);
    assert!(package.is_dir(), "not a directory: {}", package.display());
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
