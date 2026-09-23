//! The TOML shape of the configuration file, before validation. Unknown keys are errors, so a
//! misspelt key is reported instead of silently taking its default.

use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ConfigurationFile {
    pub(super) api_base_url: String,
    pub(super) credential_file: PathBuf,
    #[serde(default = "default_poll_interval_seconds")]
    pub(super) poll_interval_seconds: u64,
    pub(super) game_server: GameServerSection,
    pub(super) rcon: RconSection,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GameServerSection {
    pub(super) systemd_user_unit: String,
    /// The dedicated server's JSON config, which restart_with_mission rewrites.
    pub(super) server_config_path: PathBuf,
    #[serde(default = "default_start_dwell_seconds")]
    pub(super) start_dwell_seconds: u64,
    #[serde(default = "default_systemctl_program")]
    pub(super) systemctl_program: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RconSection {
    pub(super) address: String,
    #[serde(default = "default_rcon_port")]
    pub(super) port: u16,
    pub(super) password_file: PathBuf,
}

fn default_poll_interval_seconds() -> u64 {
    5
}

/// A Reforger server that fails to start exits a few seconds after systemd reports the start.
fn default_start_dwell_seconds() -> u64 {
    8
}

fn default_systemctl_program() -> PathBuf {
    PathBuf::from("/usr/bin/systemctl")
}

/// Arma Reforger's default RCON port.
fn default_rcon_port() -> u16 {
    19_999
}
