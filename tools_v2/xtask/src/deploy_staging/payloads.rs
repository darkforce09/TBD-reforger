//! The remote-shell payloads (bash lines 1591–1849), split out of [`super::remote`] for SIZE-3.
//!
//! Every function here builds the exact text that goes to a remote `bash -s` on stdin. They are
//! pure and therefore the ONLY part of the remote surface that can be asserted on a machine with
//! no `deploy.env`: the tests below pin the bytes, which is the stand-in for live coverage this
//! port cannot have.
//!
//! ── HEREDOC QUOTING IS THE WHOLE SUBTLETY ────────────────────────────────────────────────────
//!
//! The bash used three different quoting regimes and mixing them up would silently change what
//! runs on the host:
//!
//! * `<<EOF` (UNQUOTED) — `$TBD_*` expanded on the DEV machine while writing the payload;
//!   `\$CFG`, `\$HOME`, `\$code` were escaped so they survive to the REMOTE shell.
//! * `<<'UNITEOF'` nested INSIDE an unquoted `<<EOF` — the systemd unit body: the outer heredoc
//!   substituted `${TBD_SERVER_MODE}` and `${EXECSTART}` locally, and the inner quoted delimiter
//!   then stopped the remote shell touching the result again.
//! * `<<'AGENTINSTALL'` (QUOTED) — verbatim, nothing expanded anywhere but on the host.
//!
//! Each function below notes which regime it reproduces.

use super::config::Env;

/// The `ssh_cmd bash -s <<EOF` payload that sets up the remote profile and the addon symlink.
///
/// The bash heredoc was UNQUOTED, so `$TBD_*` expanded locally while `\$CFG` stayed literal for the
/// remote shell. Both halves are reproduced exactly; the `\$CFG` occurrences below are plain `$CFG`
/// in the payload because that is what the remote must see.
pub fn profile_payload(env: &Env) -> String {
    format!(
        "set -euo pipefail\n\
         mkdir -p \"{addons}\" \"{profile}\"\n\
         ln -sfn \"{remote}/apps/mod/tbd-framework\" \"{addons}/tbd-framework\"\n\
         export GAME_SERVER_TOKEN='{token}'\n\
         (cd \"{remote}\" && cargo run -q -p xtask -- setup server-profile \"{profile}\")\n\
         CFG=\"{profile}/profile/TBD_BackendConfig.json\"\n\
         sed -i \"s|replace-with-GAME_SERVER_TOKENS-value|{token}|g\" \"$CFG\"\n\
         sed -i 's|\"backendUrl\": \"[^\"]*\"|\"backendUrl\": \"{backend}\"|' \"$CFG\"\n\
         sed -i 's|\"missionId\": \"[^\"]*\"|\"missionId\": \"{mission}\"|' \"$CFG\"\n\
         sed -i 's|\"eventId\": \"[^\"]*\"|\"eventId\": \"{event}\"|' \"$CFG\"\n",
        addons = env.addons_staging,
        profile = env.profile_dir,
        remote = env.remote_dir,
        token = env.game_server_token,
        backend = env.backend_url,
        mission = env.mission_id,
        event = env.event_id,
    )
}

/// The V2–V4 API smoke payload.
///
/// These hit the game-server REST routes (`/api/missions/:id/compiled`,
/// `/api/game/.../roster`). Those existed only in the Phase-0 REST spike backend, since removed —
/// the current backend serves `/api/v1` only, so these curls 404 and would abort the deploy.
/// BLOCKED on T-092. Skipped by default; `TBD_RUN_T092_SMOKE=1` forces the gate.
pub fn smoke_payload(env: &Env) -> String {
    format!(
        "set -euo pipefail\n\
         TOKEN='{token}'\n\
         MID='{mission}'\n\
         EID='{event}'\n\
         code=$(curl -sS -o /tmp/tbd-mission.json -w '%{{http_code}}' -H \"Authorization: Bearer $TOKEN\" \\\n\
         \x20 \"http://127.0.0.1:8080/api/missions/$MID/compiled\")\n\
         echo \"V2 mission compiled: HTTP $code\"\n\
         [ \"$code\" = \"200\" ] || exit 1\n\
         code=$(curl -sS -o /tmp/tbd-roster.json -w '%{{http_code}}' -H \"Authorization: Bearer $TOKEN\" \\\n\
         \x20 \"http://127.0.0.1:8080/api/game/events/$EID/roster\")\n\
         echo \"V3 roster: HTTP $code\"\n\
         [ \"$code\" = \"200\" ] || exit 1\n\
         code=$(curl -sS -o /dev/null -w '%{{http_code}}' \"http://127.0.0.1:8080/api/missions/$MID/compiled\")\n\
         echo \"V4 unauth: HTTP $code\"\n\
         [ \"$code\" = \"401\" ] || exit 1\n",
        token = env.game_server_token,
        mission = env.mission_id,
        event = env.event_id,
    )
}

/// The systemd-unit install payload.
///
/// The INNER heredoc was `<<'UNITEOF'` (quoted) nested inside the OUTER unquoted one, so
/// `${TBD_SERVER_MODE}` / `${EXECSTART}` were expanded by the LOCAL shell while writing the payload
/// and then passed through verbatim on the remote. `$HOME` and `$UNIT` are the reverse: escaped
/// locally, expanded remotely.
pub fn unit_payload(env: &Env, exec_start: &str) -> String {
    format!(
        "set -euo pipefail\n\
         UNIT=\"$HOME/.config/systemd/user/tbd-reforger.service\"\n\
         mkdir -p \"$HOME/.config/systemd/user\"\n\
         cat > \"$UNIT\" <<'UNITEOF'\n\
         [Unit]\n\
         Description=TBD Arma Reforger dedicated server (TBD_Dev_POC, mode={mode})\n\
         After=network-online.target\n\
         Wants=network-online.target\n\
         \n\
         [Service]\n\
         Type=simple\n\
         WorkingDirectory={server_dir}\n\
         ExecStart={exec_start}\n\
         Restart=on-failure\n\
         RestartSec=10\n\
         \n\
         [Install]\n\
         WantedBy=default.target\n\
         UNITEOF\n\
         systemctl --user daemon-reload\n\
         systemctl --user enable tbd-reforger.service 2>/dev/null || true\n\
         systemctl --user restart tbd-reforger.service 2>/dev/null || systemctl --user start tbd-reforger.service\n",
        mode = env.server_mode,
        server_dir = env.server_dir,
    )
}

/// The agent enable payload — `<<'AGENTINSTALL'`, quoted, so it is verbatim.
///
/// Same rule the agent itself follows: do not trust the enable, go look. A socket that did not come
/// up must fail the deploy rather than be reported as installed.
pub const AGENT_INSTALL_PAYLOAD: &str = "set -euo pipefail\n\
systemctl --user daemon-reload\n\
systemctl --user enable --now tbd-reforger-agent.socket\n\
# Same rule the agent itself follows: do not trust the enable, go look. A socket that\n\
# did not come up must fail the deploy rather than be reported as installed.\n\
state=\"$(systemctl --user show -p ActiveState --value tbd-reforger-agent.socket 2>/dev/null || true)\"\n\
if [ \"$state\" != \"active\" ] && [ \"$state\" != \"listening\" ]; then\n\
  echo \"FAIL: tbd-reforger-agent.socket is '$state', not listening.\" >&2\n\
  exit 1\n\
fi\n\
echo \"  agent socket listening at ${XDG_RUNTIME_DIR}/tbd-reforger-agent.sock\"\n";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deploy_staging::config::tests::base;
    use crate::deploy_staging::remote::exec_start;

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
        assert!(p.contains(
            "ExecStart=/home/sam/steam/arma-reforger-server/ArmaReforgerServer -addonsDir"
        ));
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
            AGENT_INSTALL_PAYLOAD
                .contains("systemctl --user enable --now tbd-reforger-agent.socket")
        );
        assert!(
            AGENT_INSTALL_PAYLOAD.contains("show -p ActiveState --value tbd-reforger-agent.socket")
        );
        assert!(
            AGENT_INSTALL_PAYLOAD.contains("!= \"active\" ] && [ \"$state\" != \"listening\" ]")
        );
        assert!(AGENT_INSTALL_PAYLOAD.contains("exit 1"));
    }
}
