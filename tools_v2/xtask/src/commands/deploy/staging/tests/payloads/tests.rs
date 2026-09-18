use super::*;
use crate::commands::deploy::staging::config::tests::base;
use crate::commands::deploy::staging::remote::exec_start;

#[test]
fn profile_payload_leaves_cfg_for_the_remote_shell() {
    let p = profile_payload(&base());
    assert!(p.starts_with("set -euo pipefail\n"));
    // Locally expanded (the unquoted-heredoc half).
    assert!(p.contains("mkdir -p \"/home/sam/tbd/addons\" \"/home/sam/tbd/profile\""));
    assert!(p.contains("export GAME_SERVER_TOKEN='tok'"));
    assert!(p.contains(
        "(cd \"/home/sam/tbd/repo\" && cargo run -q -p xtask -- setup server-profile \"/home/sam/tbd/profile\")"
    ));
    // Left for the REMOTE shell — `\$CFG` in the heredoc.
    assert!(p.contains("sed -i \"s|replace-with-GAME_SERVER_TOKENS-value|tok|g\" \"$CFG\""));
    assert!(p.contains("\"backendUrl\": \"http://127.0.0.1:8080\""));
    assert!(p.contains("\"missionId\": \"msn_8f3a2c\""));
    assert!(p.contains("\"eventId\": \"b0000000-0000-4000-8000-000000000001\""));
    // The symlink is what makes -addonsDir point at the checkout this deploy just rsynced.
    assert!(p.contains(
        "ln -sfn \"/home/sam/tbd/repo/apps/mod/tbd-framework\" \"/home/sam/tbd/addons/tbd-framework\""
    ));
}

#[test]
fn unit_payload_nests_a_quoted_heredoc() {
    let e = base();
    let p = unit_payload(&e, &exec_start(&e));
    // The inner heredoc is QUOTED, so the remote performs no expansion on the unit body.
    assert!(p.contains("cat > \"$UNIT\" <<'UNITEOF'\n"));
    assert!(
        p.contains("Description=TBD Arma Reforger dedicated server (TBD_Dev_POC, mode=config)")
    );
    assert!(p.contains("WorkingDirectory=/home/sam/steam/arma-reforger-server\n"));
    assert!(
        p.contains("ExecStart=/home/sam/steam/arma-reforger-server/ArmaReforgerServer -addonsDir")
    );
    assert!(p.contains("\nUNITEOF\nsystemctl --user daemon-reload\n"));
    // `enable` is allowed to fail (the unit may already be enabled); `restart` falls back to
    // `start` for a unit that has never run. Both `||` forms are load-bearing.
    assert!(p.contains("systemctl --user enable tbd-reforger.service 2>/dev/null || true\n"));
    assert!(p.contains(
        "systemctl --user restart tbd-reforger.service 2>/dev/null || systemctl --user start tbd-reforger.service\n"
    ));
    // $HOME and $UNIT are the remote's to expand.
    assert!(p.contains("UNIT=\"$HOME/.config/systemd/user/tbd-reforger.service\""));
    // Restart=on-failure is NOT enough on its own — see the a2sPort note in boot.rs — but it
    // is what the bash installed and removing it would change the host's behaviour.
    assert!(p.contains("Restart=on-failure\nRestartSec=10\n"));
}

#[test]
fn smoke_payload_asserts_all_three_status_codes() {
    let p = smoke_payload(&base());
    assert!(p.contains("TOKEN='tok'"));
    assert!(p.contains("-w '%{http_code}'"), "{p}");
    // V4 is the one that must be 401 — an UNAUTHENTICATED read of the same route. A smoke test
    // that only checked the happy path would pass against a backend with auth switched off.
    assert!(p.contains("[ \"$code\" = \"401\" ] || exit 1"));
    assert_eq!(p.matches("[ \"$code\" = \"200\" ] || exit 1").count(), 2);
    // The remote shell expands these, not us.
    assert!(p.contains("Authorization: Bearer $TOKEN"));
    assert!(p.contains("/api/missions/$MID/compiled"));
}

#[test]
fn agent_install_payload_rereads_the_socket_state() {
    // The agent's own rule, applied to the agent's installation: do not trust the enable.
    assert!(
        AGENT_INSTALL_PAYLOAD.contains("systemctl --user enable --now tbd-reforger-agent.socket")
    );
    assert!(
        AGENT_INSTALL_PAYLOAD.contains("show -p ActiveState --value tbd-reforger-agent.socket")
    );
    assert!(AGENT_INSTALL_PAYLOAD.contains("!= \"active\" ] && [ \"$state\" != \"listening\" ]"));
    assert!(AGENT_INSTALL_PAYLOAD.contains("exit 1"));
}
