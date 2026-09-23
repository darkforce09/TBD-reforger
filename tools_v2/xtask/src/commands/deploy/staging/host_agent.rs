//! The host agent (`apps/fleet_host_agent`) on the staging host: its deploy.env settings, the
//! loopback `rcon` block the server config gains for it, and the remote install payload. The
//! agent runs as a user service of the account that runs `tbd-reforger.service`, because it
//! restarts that unit and rewrites its server config's `scenarioId` for mission restarts; it
//! polls the platform with its `host_agent` machine credential and reads the game over RCON.

/// The user unit, installed byte for byte from the committed template.
const UNIT_TEMPLATE: &str = include_str!("../../../../deploy/systemd/fleet-host-agent.service");

/// What deploy.env says about the host agent when `TBD_INSTALL_HOST_AGENT=1`.
#[derive(Debug, Clone)]
pub struct HostAgentSettings {
    /// `TBD_HOST_AGENT_CREDENTIAL`: the agent's `host_agent` machine credential.
    pub credential: String,
    /// `TBD_RCON_PASSWORD`: the server config's `rcon.password`, which the agent logs in with.
    pub rcon_password: String,
    /// `TBD_RCON_PORT`: the loopback RCON port.
    pub rcon_port: String,
    /// `TBD_HOST_AGENT_API_URL`: the platform origin the agent polls.
    pub api_base_url: String,
}

/// A machine credential as the platform issues it: `tbdm_<32 hex>_<64 hex>`, lowercase.
pub fn validate_machine_credential(key: &str, value: &str) -> Result<(), u8> {
    let lower_hex = |text: &str, length: usize| {
        text.len() == length
            && text
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    };
    let valid = value
        .strip_prefix("tbdm_")
        .and_then(|rest| rest.split_once('_'))
        .is_some_and(|(id, secret)| lower_hex(id, 32) && lower_hex(secret, 64));
    if !valid {
        eprintln!(
            "{key} is not a machine credential (tbdm_<32 hex>_<64 hex>); issue one for this \
             server in Server Control -> credentials."
        );
        return Err(1);
    }
    Ok(())
}

impl HostAgentSettings {
    /// Refuse settings the agent or the engine would refuse, before anything is deployed.
    pub fn validate(&self, server_mode: &str) -> Result<(), u8> {
        validate_machine_credential("TBD_HOST_AGENT_CREDENTIAL", &self.credential)?;
        if server_mode != "config" {
            eprintln!(
                "TBD_INSTALL_HOST_AGENT=1 needs TBD_SERVER_MODE=config: the agent restarts missions \
                 by rewriting the server config's scenarioId."
            );
            return Err(1);
        }
        let password = &self.rcon_password;
        if password.len() < 3
            || password.len() > 256
            || password
                .chars()
                .any(|c| c.is_whitespace() || c.is_control() || matches!(c, '"' | '\'' | '\\'))
        {
            eprintln!(
                "TBD_RCON_PASSWORD must have 3 to 256 bytes with no whitespace, quotes or \
                 backslashes (Reforger refuses spaces, and the config and payload carry it raw)."
            );
            return Err(1);
        }
        if !self.rcon_port.parse::<u16>().is_ok_and(|port| port != 0) {
            eprintln!(
                "TBD_RCON_PORT='{}' is not a port (1..65535).",
                self.rcon_port
            );
            return Err(1);
        }
        if !(self.api_base_url.starts_with("https://") || is_loopback_http(&self.api_base_url)) {
            eprintln!(
                "TBD_HOST_AGENT_API_URL='{}' must be https, or http on a loopback host — the \
                 agent refuses anything else.",
                self.api_base_url
            );
            return Err(1);
        }
        Ok(())
    }

    /// The `rcon` block of the server config: bound to loopback, monitor permission (every
    /// command the agent sends over RCON is a read), two clients.
    pub fn rcon_block(&self) -> String {
        format!(
            "\"rcon\": {{ \"address\": \"127.0.0.1\", \"port\": {port}, \"password\": \"{password}\", \
             \"permission\": \"monitor\", \"maxClients\": 2 }},",
            port = self.rcon_port,
            password = self.rcon_password,
        )
    }
}

fn is_loopback_http(url: &str) -> bool {
    url.strip_prefix("http://")
        .and_then(|rest| rest.split(['/', ':']).next())
        .is_some_and(|host| host == "127.0.0.1" || host == "localhost")
}

/// The remote install: build the agent from the synced checkout, install the binary, write its
/// secret files and configuration (mode 600 under a mode 700 directory), install the unit, let it
/// outlive the SSH session and start it — then read the unit's state back, because a unit that
/// did not come up must fail the deploy rather than be reported as installed.
///
/// The heredoc for `agent.toml` is unquoted so the REMOTE shell expands `$AGENT_DIR`; everything
/// else in it was fixed here. The unit heredoc is quoted: the template goes over verbatim.
pub fn install_payload(
    settings: &HostAgentSettings,
    remote_dir: &str,
    server_config_remote: &str,
) -> String {
    format!(
        "set -euo pipefail\n\
         umask 077\n\
         AGENT_DIR=\"$HOME/.config/fleet-host-agent\"\n\
         mkdir -p \"$AGENT_DIR\" \"$HOME/.local/bin\" \"$HOME/.config/systemd/user\"\n\
         chmod 700 \"$AGENT_DIR\"\n\
         (cd '{remote_dir}' && cargo build --release -q -p fleet-host-agent)\n\
         install -m 755 '{remote_dir}/target/release/fleet-host-agent' \"$HOME/.local/bin/fleet-host-agent\"\n\
         printf '%s' '{credential}' > \"$AGENT_DIR/machine-credential\"\n\
         printf '%s' '{password}' > \"$AGENT_DIR/rcon-password\"\n\
         chmod 600 \"$AGENT_DIR/machine-credential\" \"$AGENT_DIR/rcon-password\"\n\
         cat > \"$AGENT_DIR/agent.toml\" <<AGENTTOML\n\
         api_base_url = \"{api}\"\n\
         credential_file = \"$AGENT_DIR/machine-credential\"\n\
         poll_interval_seconds = 5\n\
         \n\
         [game_server]\n\
         systemd_user_unit = \"tbd-reforger.service\"\n\
         server_config_path = \"{config}\"\n\
         \n\
         [rcon]\n\
         address = \"127.0.0.1\"\n\
         port = {port}\n\
         password_file = \"$AGENT_DIR/rcon-password\"\n\
         AGENTTOML\n\
         chmod 600 \"$AGENT_DIR/agent.toml\"\n\
         cat > \"$HOME/.config/systemd/user/fleet-host-agent.service\" <<'UNITEOF'\n\
         {unit}UNITEOF\n\
         loginctl enable-linger \"$(id -un)\" 2>/dev/null || true\n\
         systemctl --user daemon-reload\n\
         systemctl --user enable fleet-host-agent.service\n\
         systemctl --user restart fleet-host-agent.service\n\
         sleep 3\n\
         state=\"$(systemctl --user show -p ActiveState --value fleet-host-agent.service 2>/dev/null || true)\"\n\
         if [ \"$state\" != \"active\" ]; then\n\
         \x20 echo \"FAIL: fleet-host-agent.service is '$state', not active.\" >&2\n\
         \x20 journalctl --user -u fleet-host-agent.service -n 20 --no-pager >&2 || true\n\
         \x20 exit 1\n\
         fi\n\
         echo \"  fleet-host-agent.service active, polling {api}\"\n",
        credential = settings.credential,
        password = settings.rcon_password,
        api = settings.api_base_url,
        config = server_config_remote,
        port = settings.rcon_port,
        unit = UNIT_TEMPLATE,
    )
}

#[cfg(test)]
#[path = "tests/host_agent/tests.rs"]
mod tests;
