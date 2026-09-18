use super::*;

// T-876: setup-mcp-game-root is in-process (no shell script to stub). Failure arms for
// the port live in `gate_setup_mcp_game_root` tests. Bootstrap still covers port_open.

#[test]
fn port_open_rejects_non_numeric_needle() {
    // Needle ":not-a-port " cannot appear in ss/netstat listen tables.
    assert!(!port_open("not-a-port"));
}
