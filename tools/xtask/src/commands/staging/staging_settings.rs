//! The staging harness's settings, read from `deploy.env`.
//!
//! **Role:** answers where the staging host is, how ssh reaches it, where its checkout, server
//! install and fleet live, and which database, units, guilds and load addresses a procedure uses.
//!
//! **Position:** loaded once per command by `dispatch.rs` through
//! [`crate::core::deploy_environment`]; the fleet part and the API origin are the staging deploy's
//! own [`FleetSettings`] and [`backend_url`], so both commands address the same instances, ports,
//! units, folders and API.
//!
//! **Signals & state:** none; an immutable snapshot of the file.
//!
//! **Invariants:** the ssh password stays inside [`SshBase`] and is never rendered; every folder
//! is absolute on the host (defaulted under `/home/<user>`); a key a procedure needs and the file
//! lacks is reported when that procedure asks for it, never replaced by a guess.

use std::path::Path;

use anyhow::{Context, Result, anyhow};

use crate::commands::deploy::staging::fleet_instances::{
    FLEET_ROOT_UNDER_HOME, FleetSettings, backend_url,
};
use crate::core::deploy_environment::{
    DeployEnvironment, DeployHost, DeployHostFolder, deploy_environment_path,
};
use process_runner::secure_shell_transport::SshBase;

/// The database every staging read and backup names.
pub(crate) const STAGING_DATABASE: &str = "tbd_reforger";
/// The database role the staging Postgres container authenticates locally.
pub(crate) const STAGING_DATABASE_USER: &str = "tbd";

const DATABASE_CONTAINER_KEY: &str = "TBD_STAGING_DB_CONTAINER";
const API_UNIT_KEY: &str = "TBD_WEBSITE_SYSTEMD_UNIT";
const OPERATOR_KEY: &str = "TBD_STAGING_OPERATOR_DISCORD_ID";
const PARTNER_GUILD_KEY: &str = "TBD_STAGING_PARTNER_GUILD_ID";
const PARTNER_ROLE_KEY: &str = "TBD_STAGING_PARTNER_ROLE_ID";
const LOAD_ORIGIN_KEY: &str = "TBD_LOAD_TARGET_ORIGIN";
const LOAD_ADDRESSES_KEY: &str = "TBD_LOAD_SOURCE_ADDRESSES";
const PUBLIC_ADDRESS_KEY: &str = "TBD_PUBLIC_ADDRESS";

/// Everything the harness reads from `deploy.env`.
#[derive(Debug, Clone)]
pub(crate) struct StagingSettings {
    /// How ssh reaches the host.
    pub ssh: SshBase,
    /// `TBD_SSH_HOST`.
    pub host: DeployHost,
    /// `/home/<user>` on the host.
    pub home: String,
    /// The rsynced checkout the API and the host tools are built in (`TBD_REMOTE_DIR`).
    pub checkout: String,
    /// The dedicated server install the fleet units run (`TBD_SERVER_DIR`).
    pub server_install: String,
    /// The fleet: instance count, ports, relay.
    pub fleet: FleetSettings,
    /// The Postgres container (`TBD_STAGING_DB_CONTAINER`, default `tbd_staging_db`).
    pub database_container: String,
    /// The API's user unit (`TBD_WEBSITE_SYSTEMD_UNIT`, default `tbd-website-api.service`).
    pub api_unit: String,
    /// The API origin on the host, `TBD_BACKEND_URL` as the staging deploy reads it
    /// ([`backend_url`]), so the profile `backendUrl` W12 writes equals the deploy's.
    pub api_origin: String,
    /// `TBD_PUBLIC_ADDRESS`, the address the servers register with, when set.
    pub public_address: Option<String>,
    /// `TBD_STAGING_OPERATOR_DISCORD_ID`, when set.
    pub operator_discord_id: Option<String>,
    /// `TBD_STAGING_PARTNER_GUILD_ID`, when set.
    pub partner_guild_id: Option<String>,
    /// `TBD_STAGING_PARTNER_ROLE_ID`, when set.
    pub partner_role_id: Option<String>,
    /// `TBD_LOAD_TARGET_ORIGIN`, when set.
    pub load_target_origin: Option<String>,
    /// `TBD_LOAD_SOURCE_ADDRESSES`, comma-separated, possibly empty.
    pub load_source_addresses: Vec<String>,
}

impl StagingSettings {
    /// Loads the deploy settings file this process reads; it must exist.
    pub(crate) fn load(repository_root: &Path) -> Result<Self> {
        let path = deploy_environment_path(repository_root);
        let environment =
            DeployEnvironment::load_required(&path).map_err(|error| anyhow!("{error}"))?;
        Self::from_environment(&environment)
    }

    /// Reads every setting from `environment`.
    pub(crate) fn from_environment(environment: &DeployEnvironment) -> Result<Self> {
        let host = environment
            .deploy_host()
            .map_err(|error| anyhow!("{error}"))?;
        let home = host
            .home_directory()
            .context("TBD_SSH_HOST names no user, so the fleet folder has no home to sit under")?;
        let folder = |folder: DeployHostFolder| {
            folder
                .resolve(environment, &host)
                .map_err(|error| anyhow!("{error}"))
        };
        let fleet =
            FleetSettings::from_environment(environment).map_err(|error| anyhow!("{error}"))?;
        let optional = |key: &str| environment.value(key).map(str::to_string);
        Ok(Self {
            ssh: environment.ssh_base(),
            checkout: folder(DeployHostFolder::Checkout)?,
            server_install: folder(DeployHostFolder::ServerInstall)?,
            fleet,
            database_container: environment
                .value_or(DATABASE_CONTAINER_KEY, "tbd_staging_db")
                .to_string(),
            api_unit: environment
                .value_or(API_UNIT_KEY, "tbd-website-api.service")
                .to_string(),
            api_origin: backend_url(environment),
            public_address: optional(PUBLIC_ADDRESS_KEY),
            operator_discord_id: optional(OPERATOR_KEY),
            partner_guild_id: optional(PARTNER_GUILD_KEY),
            partner_role_id: optional(PARTNER_ROLE_KEY),
            load_target_origin: optional(LOAD_ORIGIN_KEY),
            load_source_addresses: environment
                .value(LOAD_ADDRESSES_KEY)
                .map(|list| {
                    list.split(',')
                        .map(str::trim)
                        .filter(|address| !address.is_empty())
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default(),
            home,
            host,
        })
    }

    /// `<home>/tbd/fleet` ([`FLEET_ROOT_UNDER_HOME`]), the folder every instance's
    /// [`crate::commands::deploy::staging::fleet_instances::InstanceFolder`] sits under.
    pub(crate) fn fleet_root(&self) -> String {
        format!("{}/{FLEET_ROOT_UNDER_HOME}", self.home)
    }

    /// The API env file of the host checkout, the file the API unit loads.
    pub(crate) fn api_env_file(&self) -> String {
        format!("{}/apps/api/.env", self.checkout)
    }

    /// The address the fleet's servers register with: `TBD_PUBLIC_ADDRESS`, else the first IPv4
    /// address `TBD_SSH_HOST` resolves to, as `deploy staging` writes into each server config.
    pub(crate) fn server_address(&self) -> Result<String> {
        match &self.public_address {
            Some(address) => Ok(address.clone()),
            None => self
                .host
                .resolve_ipv4()
                .map(|address| address.to_string())
                .map_err(|problem| {
                    anyhow!(
                        "{PUBLIC_ADDRESS_KEY} is unset and {}: {problem}",
                        self.host.host()
                    )
                }),
        }
    }

    /// `TBD_STAGING_OPERATOR_DISCORD_ID`, or an error naming the key.
    pub(crate) fn operator(&self) -> Result<&str> {
        self.operator_discord_id
            .as_deref()
            .with_context(|| format!("{OPERATOR_KEY} is not set in deploy.env"))
    }

    /// The fleet's game server units, `tbd-reforger@1.service` onward.
    pub(crate) fn game_server_units(&self) -> Vec<String> {
        self.fleet
            .instances()
            .iter()
            .map(|instance| instance.game_server_unit())
            .collect()
    }

    /// The fleet's host agent units, `fleet_host_agent@1.service` onward.
    pub(crate) fn host_agent_units(&self) -> Vec<String> {
        self.fleet
            .instances()
            .iter()
            .map(|instance| instance.host_agent_unit())
            .collect()
    }

    /// The relay's unit and instance, when the fleet has a relay.
    pub(crate) fn relay_unit(&self) -> Option<(u16, String)> {
        self.fleet
            .instances()
            .into_iter()
            .find_map(|instance| instance.relay_unit().map(|unit| (instance.number, unit)))
    }
}

#[cfg(test)]
#[path = "tests/staging_settings.rs"]
mod tests;
