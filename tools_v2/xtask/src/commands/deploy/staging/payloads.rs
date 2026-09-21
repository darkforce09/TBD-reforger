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
/// These curl the unversioned game-server REST routes (`/api/missions/:id/compiled`,
/// `/api/game/.../roster`). The backend serves `/api/v1` only, so both answer 404 and the payload
/// would abort the deploy at its first `|| exit 1`. It is therefore skipped by default;
/// `TBD_RUN_GAME_SERVER_REST_SMOKE=1` runs it anyway.
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
#[path = "tests/payloads/tests.rs"]
mod tests;
