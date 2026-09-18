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
