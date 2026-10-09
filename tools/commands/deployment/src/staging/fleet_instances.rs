//! The fleet of dedicated game server instances on the staging host: how many there are, their
//! ports, names, visibility, folders and units, the relay that fronts one host agent, and the API
//! origin they all call.
//!
//! **Role:** reads the API origin `TBD_BACKEND_URL` ([`backend_url`]), parses the `TBD_FLEET_*`
//! settings and `TBD_HOST_AGENT_API_URL` into [`FleetSettings`], checks the port rules
//! ([`FleetSettings::check_port_rules`]), derives one [`FleetInstance`] per instance number, and
//! names an instance's files ([`InstanceFolder`]) and units.
//!
//! **Position:** fed by [`deploy_settings`]; consumed by `super::config::Env` and,
//! through it, the server config render, the remote payloads, the unit install, the boot verdicts
//! and the dry-run plan; and by `cargo xtask staging` and the instance-aware debug commands, which
//! all address an instance and the API only through these values.
//!
//! **Signals & state:** none; pure values built once per run.
//!
//! **Invariants:** the API origin has one reading, [`backend_url`], without a trailing `/`;
//! instance N (1-based, at most [`MAXIMUM_FLEET_INSTANCES`]) uses game port
//! `TBD_FLEET_GAME_PORT_BASE + N`, A2S port `TBD_FLEET_A2S_PORT_BASE + N` and RCON port
//! `TBD_FLEET_RCON_PORT_BASE + N`; every port of the fleet, the relay port and the relay's upstream
//! port are distinct; only instance 1 is listed in the server browser; instance N's files sit under
//! `~/tbd/fleet/instance-N/` (its `profile/`, `secrets/` and `server.config.json`) and its units
//! are `tbd-reforger@N`, `game_server_host_agent@N` and, for the relay instance,
//! `acknowledgement-dropping-relay@N`.

use std::collections::BTreeMap;
use std::fmt;

use deploy_settings::{DeployEnvironment, SettingError};

/// The fleet's folder under the deploy user's home; `%h/tbd/fleet` in the units.
pub const FLEET_ROOT_UNDER_HOME: &str = "tbd/fleet";
/// The file under the fleet root holding the join password every instance requires.
pub const JOIN_PASSWORD_FILE: &str = "join-password";
/// The instance's `mod_runtime` machine credential, under its `secrets/` folder.
pub const MOD_RUNTIME_CREDENTIAL_FILE: &str = "mod-runtime-credential";
/// The instance's `host_agent` machine credential, under its `secrets/` folder.
pub const HOST_AGENT_CREDENTIAL_FILE: &str = "host-agent-credential";
/// The instance's RCON password, generated on the host, under its `secrets/` folder.
pub const RCON_PASSWORD_FILE: &str = "rcon-password";
/// The size the staging host is provisioned for; a larger count is a typo, not a plan.
pub const MAXIMUM_FLEET_INSTANCES: u16 = 5;

/// The API origin on the staging host: every instance's profile names it as its `backendUrl`, and
/// every host agent polls it unless [`AGENT_API_URL_KEY`] names another origin.
const BACKEND_URL_KEY: &str = "TBD_BACKEND_URL";
/// [`BACKEND_URL_KEY`] when the settings leave it out: the API on the host's loopback.
const DEFAULT_BACKEND_URL: &str = "http://127.0.0.1:8080";
const INSTANCES_KEY: &str = "TBD_FLEET_INSTANCES";
const GAME_PORT_BASE_KEY: &str = "TBD_FLEET_GAME_PORT_BASE";
const A2S_PORT_BASE_KEY: &str = "TBD_FLEET_A2S_PORT_BASE";
const RCON_PORT_BASE_KEY: &str = "TBD_FLEET_RCON_PORT_BASE";
const RELAY_INSTANCE_KEY: &str = "TBD_FLEET_RELAY_INSTANCE";
const RELAY_PORT_KEY: &str = "TBD_FLEET_RELAY_PORT";
const AGENT_API_URL_KEY: &str = "TBD_HOST_AGENT_API_URL";

/// The acknowledgement-dropping relay between one instance's host agent and the API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelaySettings {
    /// `TBD_FLEET_RELAY_INSTANCE`: the instance whose agent polls through the relay.
    pub instance: u16,
    /// `TBD_FLEET_RELAY_PORT`: the loopback port the relay listens on.
    pub port: u16,
    /// The API origin the relay forwards to: the agents' API origin without a trailing slash.
    pub upstream: String,
}

/// Every fleet-wide setting, after defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetSettings {
    /// `TBD_FLEET_INSTANCES` (default 5): instances 1 to this number run.
    pub instance_count: u16,
    /// `TBD_FLEET_GAME_PORT_BASE` (default 2000).
    pub game_port_base: u16,
    /// `TBD_FLEET_A2S_PORT_BASE` (default 17776).
    pub a2s_port_base: u16,
    /// `TBD_FLEET_RCON_PORT_BASE` (default 19998).
    pub rcon_port_base: u16,
    /// `TBD_HOST_AGENT_API_URL`, else [`backend_url`]: the origin the agents poll.
    pub agent_api_url: String,
    /// The relay, when `TBD_FLEET_RELAY_INSTANCE` is set.
    pub relay: Option<RelaySettings>,
}

/// One instance of the fleet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetInstance {
    /// 1-based instance number, the `%i` of its units.
    pub number: u16,
    /// The UDP game port players join on.
    pub game_port: u16,
    /// The UDP Steam query (A2S) port.
    pub a2s_port: u16,
    /// The loopback RCON port its host agent logs in on.
    pub rcon_port: u16,
    /// The origin its host agent polls: the relay for the relay instance, else the API.
    pub agent_api_url: String,
    /// The relay's port when this instance's agent polls through the relay.
    pub relay_port: Option<u16>,
}

/// Instance N's folder under a fleet root, and the files the fleet keeps in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceFolder {
    /// `<fleet root>/instance-N`.
    path: String,
}

impl InstanceFolder {
    /// Instance `number`'s folder under `fleet_root`: `<fleet_root>/instance-N`. The root is the
    /// fleet's folder as its reader addresses it: an absolute path on the host, `$HOME/tbd/fleet`
    /// in a script, or [`FLEET_ROOT_UNDER_HOME`] relative to the deploy user's home.
    pub fn under(fleet_root: &str, number: u16) -> InstanceFolder {
        InstanceFolder {
            path: format!("{fleet_root}/instance-{number}"),
        }
    }

    /// The folder itself.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// `<folder>/profile`: the game server unit's `-profile` folder, under which the engine writes
    /// its `logs/` and `setup server-profile` the mod's `profile/TBD_BackendConfig.json`.
    pub fn profile(&self) -> String {
        format!("{}/profile", self.path)
    }

    /// `<folder>/secrets`: the instance's [`MOD_RUNTIME_CREDENTIAL_FILE`],
    /// [`HOST_AGENT_CREDENTIAL_FILE`] and [`RCON_PASSWORD_FILE`].
    pub fn secrets(&self) -> String {
        format!("{}/secrets", self.path)
    }

    /// `<folder>/server.config.json`: the config the game server unit starts with `-config`.
    pub fn server_config(&self) -> String {
        format!("{}/server.config.json", self.path)
    }
}

/// The API origin on the staging host: `TBD_BACKEND_URL`, else `http://127.0.0.1:8080`, without a
/// trailing `/`. The one reading of that setting: the deploy writes it into every instance's
/// profile as `backendUrl` and checks `/healthz` under it, the host agents poll it unless
/// `TBD_HOST_AGENT_API_URL` names another origin, and `cargo xtask staging` reads the API and
/// rewrites a profile through it, so no two of them differ by a slash.
pub fn backend_url(environment: &DeployEnvironment) -> String {
    environment
        .value_or(BACKEND_URL_KEY, DEFAULT_BACKEND_URL)
        .trim_end_matches('/')
        .to_string()
}

/// The game server unit of instance `instance`, an instance of `tbd-reforger@.service`:
/// `tbd-reforger@<instance>.service`, where `instance` is a number, or `N` in a text about any
/// instance.
pub fn game_server_unit_of(instance: impl fmt::Display) -> String {
    format!("tbd-reforger@{instance}.service")
}

/// A whole-number setting, `default` when unset.
fn number_setting(
    environment: &DeployEnvironment,
    key: &str,
    default: Option<u16>,
) -> Result<Option<u16>, SettingError> {
    match environment.value(key) {
        None => Ok(default),
        Some(raw) => raw.parse::<u16>().map(Some).map_err(|_| {
            environment.invalid(
                key,
                format!("`{raw}` is not a whole number from 0 to 65535"),
            )
        }),
    }
}

/// An origin a host agent accepts: https, or http on a loopback host.
pub fn is_agent_origin(url: &str) -> bool {
    url.starts_with("https://") || is_loopback_http(url)
}

/// http on `127.0.0.1` or `localhost`.
pub fn is_loopback_http(url: &str) -> bool {
    url.strip_prefix("http://")
        .and_then(|rest| rest.split(['/', ':']).next())
        .is_some_and(|host| host == "127.0.0.1" || host == "localhost")
}

/// The explicit port of an http origin, else 80.
fn http_port(url: &str) -> Option<u16> {
    let authority = url.strip_prefix("http://")?.split('/').next()?;
    match authority.rsplit_once(':') {
        Some((_, port)) => port.parse().ok(),
        None => Some(80),
    }
}

impl FleetSettings {
    /// Reads the fleet settings; the agents' API origin is `TBD_HOST_AGENT_API_URL`, else
    /// [`backend_url`].
    pub fn from_environment(
        environment: &DeployEnvironment,
    ) -> Result<FleetSettings, SettingError> {
        let instance_count = number_setting(environment, INSTANCES_KEY, Some(5))?.unwrap_or(5);
        if !(1..=MAXIMUM_FLEET_INSTANCES).contains(&instance_count) {
            return Err(environment.invalid(
                INSTANCES_KEY,
                format!("must be 1 to {MAXIMUM_FLEET_INSTANCES}"),
            ));
        }
        let base = |key: &str, default: u16| -> Result<u16, SettingError> {
            Ok(number_setting(environment, key, Some(default))?.unwrap_or(default))
        };
        let api_origin = backend_url(environment);
        let agent_api_url = environment
            .value_or(AGENT_API_URL_KEY, &api_origin)
            .to_string();
        if !is_agent_origin(&agent_api_url) {
            let key = if environment.value(AGENT_API_URL_KEY).is_some() {
                AGENT_API_URL_KEY
            } else {
                BACKEND_URL_KEY
            };
            return Err(environment.invalid(
                key,
                "the host agents need https, or http on a loopback host",
            ));
        }
        let relay = match number_setting(environment, RELAY_INSTANCE_KEY, None)? {
            None => None,
            Some(instance) => {
                if !(1..=instance_count).contains(&instance) {
                    return Err(environment.invalid(
                        RELAY_INSTANCE_KEY,
                        format!("names no instance of 1 to {instance_count}"),
                    ));
                }
                if !is_loopback_http(&agent_api_url) {
                    return Err(environment.invalid(
                        RELAY_INSTANCE_KEY,
                        format!(
                            "the relay forwards to {agent_api_url}, and forwards only to http on a \
                             loopback host"
                        ),
                    ));
                }
                let port = number_setting(environment, RELAY_PORT_KEY, None)?
                    .ok_or_else(|| environment.missing(RELAY_PORT_KEY))?;
                Some(RelaySettings {
                    instance,
                    port,
                    upstream: agent_api_url.trim_end_matches('/').to_string(),
                })
            }
        };
        Ok(FleetSettings {
            instance_count,
            game_port_base: base(GAME_PORT_BASE_KEY, 2000)?,
            a2s_port_base: base(A2S_PORT_BASE_KEY, 17776)?,
            rcon_port_base: base(RCON_PORT_BASE_KEY, 19998)?,
            agent_api_url,
            relay,
        })
    }

    /// The port rules: every derived port fits in 1..=65535, no two ports of the fleet are the
    /// same, and the relay port is neither a fleet port, nor 0, nor the upstream's port.
    pub fn check_port_rules(&self) -> Result<(), String> {
        let mut owners: BTreeMap<u16, String> = BTreeMap::new();
        for number in 1..=self.instance_count {
            for (kind, base, key) in [
                ("game", self.game_port_base, GAME_PORT_BASE_KEY),
                ("A2S", self.a2s_port_base, A2S_PORT_BASE_KEY),
                ("RCON", self.rcon_port_base, RCON_PORT_BASE_KEY),
            ] {
                let owner = format!("instance {number}'s {kind} port");
                let port = base
                    .checked_add(number)
                    .ok_or_else(|| format!("{owner} is {key} {base} + {number}, above 65535"))?;
                if let Some(previous) = owners.insert(port, owner.clone()) {
                    return Err(format!("port {port} is both {previous} and {owner}"));
                }
            }
        }
        if let Some(relay) = &self.relay {
            if relay.port == 0 {
                return Err(format!("{RELAY_PORT_KEY} must not be 0"));
            }
            if let Some(owner) = owners.get(&relay.port) {
                return Err(format!("{RELAY_PORT_KEY} {} is also {owner}", relay.port));
            }
            if http_port(&relay.upstream) == Some(relay.port) {
                return Err(format!(
                    "{RELAY_PORT_KEY} {} is the port of the API it forwards to",
                    relay.port
                ));
            }
        }
        Ok(())
    }

    /// Instances 1 to [`Self::instance_count`], in order.
    pub fn instances(&self) -> Vec<FleetInstance> {
        (1..=self.instance_count)
            .map(|number| {
                let relay_port = self
                    .relay
                    .as_ref()
                    .filter(|relay| relay.instance == number)
                    .map(|relay| relay.port);
                FleetInstance {
                    number,
                    game_port: self.game_port_base.saturating_add(number),
                    a2s_port: self.a2s_port_base.saturating_add(number),
                    rcon_port: self.rcon_port_base.saturating_add(number),
                    agent_api_url: match relay_port {
                        Some(port) => format!("http://127.0.0.1:{port}"),
                        None => self.agent_api_url.clone(),
                    },
                    relay_port,
                }
            })
            .collect()
    }
}

impl FleetInstance {
    /// The in-game name, which matches the server's registration: `TBD Staging N`.
    pub fn server_name(&self) -> String {
        format!("TBD Staging {}", self.number)
    }

    /// Only instance 1 appears in the server browser; the others take direct joins only.
    pub fn listed_in_server_browser(&self) -> bool {
        self.number == 1
    }

    /// The instance's folder relative to the deploy user's home: `tbd/fleet/instance-N`, its
    /// [`InstanceFolder::under`] [`FLEET_ROOT_UNDER_HOME`].
    pub fn home_relative_folder(&self) -> String {
        InstanceFolder::under(FLEET_ROOT_UNDER_HOME, self.number).path
    }

    /// The game server unit, an instance of `tbd-reforger@.service` ([`game_server_unit_of`]).
    pub fn game_server_unit(&self) -> String {
        game_server_unit_of(self.number)
    }

    /// The game server host agent unit, an instance of `game_server_host_agent@.service`.
    pub fn host_agent_unit(&self) -> String {
        format!(
            "{}@{}.service",
            super::host_agent::HOST_AGENT_PACKAGE,
            self.number
        )
    }

    /// The relay unit, an instance of `acknowledgement-dropping-relay@.service`, for the relay
    /// instance only.
    pub fn relay_unit(&self) -> Option<String> {
        self.relay_port
            .map(|_| format!("acknowledgement-dropping-relay@{}.service", self.number))
    }
}
