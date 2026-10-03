use super::*;

#[test]
fn usage_rejects_unknown_action() {
    assert_eq!(cmd(Some("bogus")), 2);
}

#[test]
fn status_stopped_when_no_socket() {
    let sock = format!("/tmp/tbd-mcp-t888-ut-status-{}.sock", std::process::id());
    let _ = fs::remove_file(&sock);
    let _ = fs::remove_file(format!("{sock}.pid"));
    // Isolate from ambient MCP_SOCK
    let code = status_at(&sock, true);
    assert_eq!(code, 1);
}

/// `mcpd` builds into its purpose subfolder of the checkout's build output folder, never into a
/// root-level folder beside it.
#[test]
fn mcp_daemon_builds_mcpd_under_the_build_output_folder() {
    let root = Path::new("/checkout");
    assert_eq!(
        default_mcpd_target_dir(root),
        Path::new("/checkout/target/dev-mcpd")
    );
}
