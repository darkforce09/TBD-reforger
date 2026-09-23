//! The agent's configuration file (TOML): where the platform API is, which file holds this
//! host's machine credential, which systemd user unit runs the game server, where the game
//! server's own JSON config is, and how to reach its RCON port. Loading validates every key and
//! reads both secret files, so a misconfigured agent stops at startup with a named
//! [`ConfigurationError`] instead of failing its first command.

mod configuration_error;
mod configuration_file;
mod secret_files;

use std::fs::{self, OpenOptions};
use std::net::{IpAddr, SocketAddr};
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::time::Duration;

use reqwest::Url;

pub use configuration_error::{ConfigurationError, SecretFileProblem, ServerConfigFileProblem};
use configuration_file::ConfigurationFile;
use secret_files::{machine_credential_format, rcon_password_format, read_secret_file};

use crate::dedicated_server_config::DedicatedServerConfig;
use crate::process_control::{ProcessControlSettings, SystemdUnitName};
use crate::rcon::{RconSettings, RconTimings};
use crate::secret_text::SecretText;

const POLL_INTERVAL_SECONDS: RangeInclusive<u64> = 1..=300;
/// Capped so that the verb timeout, the dwell and two state reads fit the ledger's 180 s
/// execution window for start and restart.
const START_DWELL_SECONDS: RangeInclusive<u64> = 0..=30;

/// A validated configuration with both secrets loaded.
#[derive(Debug, Clone)]
pub struct AgentConfiguration {
    /// The API origin, ending in `/`.
    pub api_base_url: Url,
    pub machine_credential: SecretText,
    pub poll_interval: Duration,
    pub process_control: ProcessControlSettings,
    /// The dedicated server's JSON config, which restart_with_mission rewrites.
    pub server_config: DedicatedServerConfig,
    pub rcon: RconSettings,
}

impl AgentConfiguration {
    pub fn load(path: &Path) -> Result<Self, ConfigurationError> {
        let text =
            fs::read_to_string(path).map_err(|source| ConfigurationError::FileUnreadable {
                path: path.to_owned(),
                source,
            })?;
        Self::parse(&text, path)
    }

    /// Validates `text`, the contents of the configuration file at `path`, and reads the secret
    /// files it names.
    pub fn parse(text: &str, path: &Path) -> Result<Self, ConfigurationError> {
        let file: ConfigurationFile =
            toml::from_str(text).map_err(|error| ConfigurationError::FileMalformed {
                path: path.to_owned(),
                message: error.to_string(),
            })?;
        let game_server = file.game_server;
        let rcon = file.rcon;
        Ok(Self {
            api_base_url: api_base_url(&file.api_base_url)?,
            machine_credential: read_secret_file(
                "credential_file",
                &file.credential_file,
                machine_credential_format,
            )?,
            poll_interval: seconds_within(
                file.poll_interval_seconds,
                POLL_INTERVAL_SECONDS,
                ConfigurationError::PollIntervalOutOfRange,
            )?,
            process_control: ProcessControlSettings {
                systemctl_program: absolute_program(game_server.systemctl_program)?,
                unit: SystemdUnitName::parse(&game_server.systemd_user_unit).map_err(
                    |problem| ConfigurationError::UnitNameInvalid {
                        value: game_server.systemd_user_unit.clone(),
                        problem,
                    },
                )?,
                start_dwell: seconds_within(
                    game_server.start_dwell_seconds,
                    START_DWELL_SECONDS,
                    ConfigurationError::StartDwellOutOfRange,
                )?,
                verb_timeout: ProcessControlSettings::VERB_TIMEOUT,
                state_read_timeout: ProcessControlSettings::STATE_READ_TIMEOUT,
            },
            server_config: DedicatedServerConfig::new(server_config_file(
                game_server.server_config_path,
            )?),
            rcon: RconSettings {
                server: rcon_server(&rcon.address, rcon.port)?,
                password: read_secret_file(
                    "rcon.password_file",
                    &rcon.password_file,
                    rcon_password_format,
                )?,
                timings: RconTimings::default(),
            },
        })
    }
}

/// An absolute http(s) origin without credentials, query or fragment, normalised to end in
/// `/`. Plain http is accepted only for a loopback host.
fn api_base_url(raw: &str) -> Result<Url, ConfigurationError> {
    let invalid = |problem| ConfigurationError::ApiBaseUrlInvalid {
        value: raw.to_owned(),
        problem,
    };
    let mut url = Url::parse(raw).map_err(|_| invalid("is not an absolute URL"))?;
    let loopback = url.host_str().is_some_and(is_loopback_host);
    match url.scheme() {
        "https" => {}
        "http" if loopback => {}
        _ => {
            return Err(invalid(
                "must use https (plain http is accepted only for a loopback host)",
            ));
        }
    }
    if url.host_str().is_none_or(str::is_empty) {
        return Err(invalid("names no host"));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(invalid("must not carry credentials"));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(invalid("must not carry a query or fragment"));
    }
    if !url.path().ends_with('/') {
        let path = format!("{}/", url.path());
        url.set_path(&path);
    }
    Ok(url)
}

/// `localhost`, or an IPv4 or IPv6 loopback address (IPv6 hosts arrive in brackets).
fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

fn seconds_within(
    seconds: u64,
    range: RangeInclusive<u64>,
    out_of_range: fn(u64) -> ConfigurationError,
) -> Result<Duration, ConfigurationError> {
    if range.contains(&seconds) {
        Ok(Duration::from_secs(seconds))
    } else {
        Err(out_of_range(seconds))
    }
}

fn absolute_program(path: PathBuf) -> Result<PathBuf, ConfigurationError> {
    if path.is_absolute() {
        Ok(path)
    } else {
        Err(ConfigurationError::SystemctlProgramNotAbsolute(path))
    }
}

/// An absolute path to a regular file the agent can open for writing. Opening it neither
/// truncates nor changes it.
fn server_config_file(path: PathBuf) -> Result<PathBuf, ConfigurationError> {
    let rejected = |problem| ConfigurationError::ServerConfigFileRejected {
        path: path.clone(),
        problem,
    };
    if !path.is_absolute() {
        return Err(rejected(ServerConfigFileProblem::RelativePath));
    }
    let metadata = fs::metadata(&path)
        .map_err(|error| rejected(ServerConfigFileProblem::Unreadable(error)))?;
    if !metadata.is_file() {
        return Err(rejected(ServerConfigFileProblem::NotRegularFile));
    }
    OpenOptions::new()
        .write(true)
        .open(&path)
        .map_err(|error| rejected(ServerConfigFileProblem::NotWritable(error)))?;
    Ok(path)
}

fn rcon_server(address: &str, port: u16) -> Result<SocketAddr, ConfigurationError> {
    let address: IpAddr = address
        .parse()
        .map_err(|_| ConfigurationError::RconAddressInvalid(address.to_owned()))?;
    if port == 0 {
        return Err(ConfigurationError::RconPortInvalid);
    }
    Ok(SocketAddr::new(address, port))
}

#[cfg(test)]
#[path = "tests/agent_configuration.rs"]
mod tests;
