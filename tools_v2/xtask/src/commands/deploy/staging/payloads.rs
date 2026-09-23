//! The remote-shell payloads (bash lines 1591–1849), split out of [`super::remote`] for SIZE-3.
//!
//! Every function here builds the exact text that goes to a remote `bash -s` on stdin. They are
//! pure and therefore the ONLY part of the remote surface that can be asserted on a machine with
//! no `deploy.env`: the tests below pin the bytes, which is the stand-in for live coverage this
//! port cannot have.
//!
//! ── HEREDOC QUOTING IS THE WHOLE SUBTLETY ────────────────────────────────────────────────────
//!
//! Two quoting regimes are in use, and mixing them up would silently change what runs on the
//! host:
//!
//! * `<<EOF` (UNQUOTED) — `$TBD_*` expanded on the DEV machine while writing the payload;
//!   `\$CFG`, `\$HOME`, `\$code` were escaped so they survive to the REMOTE shell.
//! * `<<'UNITEOF'` nested INSIDE an unquoted `<<EOF` — the systemd unit body: the outer heredoc
//!   substituted `${TBD_SERVER_MODE}` and `${EXECSTART}` locally, and the inner quoted delimiter
//!   then stopped the remote shell touching the result again.
//!
//! Each function below notes which regime it reproduces.

use super::config::Env;

/// The `ssh_cmd bash -s <<EOF` payload that sets up the remote profile and the addon symlink.
///
/// `setup server-profile` writes `TBD_BackendConfig.json` from the committed example with the
/// service token and the runtime's machine credential taken from its environment; the payload
/// then points `backendUrl` at this deployment's API. Values are expanded here, locally; `$CFG`
/// is for the remote shell.
pub fn profile_payload(env: &Env) -> String {
    format!(
        "set -euo pipefail\n\
         mkdir -p \"{addons}\" \"{profile}\"\n\
         ln -sfn \"{remote}/apps/mod/tbd-framework\" \"{addons}/tbd-framework\"\n\
         export SERVICE_TOKEN='{token}'\n\
         export TBD_MACHINE_CREDENTIAL='{credential}'\n\
         (cd \"{remote}\" && cargo run -q -p xtask -- setup server-profile \"{profile}\")\n\
         CFG=\"{profile}/profile/TBD_BackendConfig.json\"\n\
         sed -i 's|\"backendUrl\": \"[^\"]*\"|\"backendUrl\": \"{backend}\"|' \"$CFG\"\n",
        addons = env.addons_staging,
        profile = env.profile_dir,
        remote = env.remote_dir,
        token = env.game_server_token,
        credential = env.mod_runtime_credential,
        backend = env.backend_url,
    )
}

/// The V2–V4 game-runtime smoke, run on the server against its own API with the runtime's
/// machine credential:
///
/// * V2 — `GET /api/v1/game-runtime/deployment` answers the deployment this server runs (200)
///   or `NO_DEPLOYMENT` (404): the credential is accepted and scoped to this server.
/// * V3 — with a deployment, `GET /api/v1/game-runtime/artifacts/{id}` answers the artifact's
///   bytes, and they hash to the deployment's `artifact_sha256`.
/// * V4 — the same deployment read without a credential answers 401.
pub fn smoke_payload(env: &Env) -> String {
    format!(
        "set -euo pipefail\n\
         CREDENTIAL='{credential}'\n\
         API='http://127.0.0.1:8080/api/v1/game-runtime'\n\
         code=$(curl -sS -o /tmp/tbd-deployment.json -w '%{{http_code}}' -H \"Authorization: Bearer $CREDENTIAL\" \"$API/deployment\")\n\
         echo \"V2 game-runtime deployment: HTTP $code\"\n\
         if [ \"$code\" = \"404\" ]; then\n\
         \x20 grep -q '\"NO_DEPLOYMENT\"' /tmp/tbd-deployment.json || exit 1\n\
         \x20 echo \"V3 artifact: no deployment yet — deploy an approved mission to this server\"\n\
         elif [ \"$code\" = \"200\" ]; then\n\
         \x20 ARTIFACT=$(sed -n 's/.*\"artifact_id\": *\"\\([^\"]*\\)\".*/\\1/p' /tmp/tbd-deployment.json)\n\
         \x20 EXPECTED=$(sed -n 's/.*\"artifact_sha256\": *\"\\([0-9a-f]*\\)\".*/\\1/p' /tmp/tbd-deployment.json)\n\
         \x20 code=$(curl -sS -o /tmp/tbd-artifact.json -w '%{{http_code}}' -H \"Authorization: Bearer $CREDENTIAL\" \"$API/artifacts/$ARTIFACT\")\n\
         \x20 ACTUAL=$(sha256sum /tmp/tbd-artifact.json | cut -d' ' -f1)\n\
         \x20 echo \"V3 artifact $ARTIFACT: HTTP $code, sha256 $ACTUAL\"\n\
         \x20 [ \"$code\" = \"200\" ] && [ -n \"$EXPECTED\" ] && [ \"$ACTUAL\" = \"$EXPECTED\" ] || exit 1\n\
         else\n\
         \x20 exit 1\n\
         fi\n\
         code=$(curl -sS -o /dev/null -w '%{{http_code}}' \"$API/deployment\")\n\
         echo \"V4 without a credential: HTTP $code\"\n\
         [ \"$code\" = \"401\" ] || exit 1\n",
        credential = env.mod_runtime_credential,
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

#[cfg(test)]
#[path = "tests/payloads/tests.rs"]
mod tests;
