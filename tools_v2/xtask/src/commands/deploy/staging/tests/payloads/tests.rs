use super::*;
use crate::commands::deploy::staging::config::tests::{RUNTIME_CREDENTIAL, base};
use crate::commands::deploy::staging::remote::exec_start;

#[test]
fn profile_payload_hands_the_token_and_the_credential_to_setup() {
    let p = profile_payload(&base());
    assert!(p.starts_with("set -euo pipefail\n"));
    assert!(p.contains("mkdir -p \"/home/sam/tbd/addons\" \"/home/sam/tbd/profile\""));
    // `setup server-profile` reads both from its environment.
    assert!(p.contains("export SERVICE_TOKEN='tok'"));
    assert!(p.contains(&format!(
        "export TBD_MACHINE_CREDENTIAL='{RUNTIME_CREDENTIAL}'"
    )));
    assert!(p.contains(
        "(cd \"/home/sam/tbd/repo\" && cargo run -q -p xtask -- setup server-profile \"/home/sam/tbd/profile\")"
    ));
    // Left for the REMOTE shell.
    assert!(p.contains("\"backendUrl\": \"http://127.0.0.1:8080\""));
    assert!(p.contains("\"$CFG\"\n"));
    // The mission comes from the server's deployment, never from the profile.
    assert!(!p.contains("missionId") && !p.contains("eventId"), "{p}");
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
fn smoke_payload_reads_the_deployment_with_and_without_the_credential() {
    let p = smoke_payload(&base());
    assert!(p.contains(&format!("CREDENTIAL='{RUNTIME_CREDENTIAL}'")));
    assert!(p.contains("API='http://127.0.0.1:8080/api/v1/game-runtime'"));
    assert!(p.contains("-H \"Authorization: Bearer $CREDENTIAL\" \"$API/deployment\""));
    // No deployment yet is a named refusal, not any 404.
    assert!(p.contains("grep -q '\"NO_DEPLOYMENT\"' /tmp/tbd-deployment.json || exit 1"));
    // A deployment's artifact must hash to its recorded SHA-256.
    assert!(p.contains("\"$API/artifacts/$ARTIFACT\""));
    assert!(p.contains("[ \"$ACTUAL\" = \"$EXPECTED\" ]"));
    // V4 is the one that must be 401 — an UNAUTHENTICATED read of the same route. A smoke test
    // that only checked the happy path would pass against a backend with auth switched off.
    assert!(p.contains("code=$(curl -sS -o /dev/null -w '%{http_code}' \"$API/deployment\")"));
    assert!(p.contains("[ \"$code\" = \"401\" ] || exit 1"));
    assert!(
        !p.contains("/compiled") && !p.contains("X-Service-Token"),
        "{p}"
    );
}
