//! The remote folders a deploy writes, each set in `deploy.env` or defaulted under the deploy
//! user's home.
//!
//! **Role:** names each folder's setting and resolves its value.
//!
//! **Position:** read by `deploy website` (the checkout), `deploy staging` (all four),
//! `mod bootstrap-staging` (the three `tbd/` folders), `mod remote-logs` and `debug direct-join`
//! (the profile).
//!
//! **Signals & state:** none; pure functions over a [`DeployEnvironment`].
//!
//! **Invariants:** an explicit value always wins; the defaults are
//! `/home/<user>/tbd/{repo,profile,addons-staging}` and `/home/<user>/steam/arma-reforger-server`;
//! with no user in `TBD_SSH_HOST` there is no default and the setting is required.

use super::{DeployEnvironment, DeployHost, SettingError};

/// A folder on the deploy host that the commands write or read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeployHostFolder {
    /// `TBD_REMOTE_DIR`: the rsynced checkout.
    Checkout,
    /// `TBD_PROFILE_DIR`: the dedicated server's `-profile` folder.
    Profile,
    /// `TBD_ADDONS_STAGING`: the `-addonsDir` folder holding the addon link.
    AddonsStaging,
    /// `TBD_SERVER_DIR`: the dedicated server install.
    ServerInstall,
}

impl DeployHostFolder {
    /// The setting that holds the folder.
    pub fn key(self) -> &'static str {
        match self {
            Self::Checkout => "TBD_REMOTE_DIR",
            Self::Profile => "TBD_PROFILE_DIR",
            Self::AddonsStaging => "TBD_ADDONS_STAGING",
            Self::ServerInstall => "TBD_SERVER_DIR",
        }
    }

    /// The default's place under the deploy user's home.
    fn path_under_home(self) -> &'static str {
        match self {
            Self::Checkout => "tbd/repo",
            Self::Profile => "tbd/profile",
            Self::AddonsStaging => "tbd/addons-staging",
            Self::ServerInstall => "steam/arma-reforger-server",
        }
    }

    /// The explicit value, else the default under `/home/<user>`, else
    /// [`SettingError::Missing`] when the host names no user.
    pub fn resolve(
        self,
        environment: &DeployEnvironment,
        host: &DeployHost,
    ) -> Result<String, SettingError> {
        if let Some(explicit) = environment.value(self.key()) {
            return Ok(explicit.to_string());
        }
        host.home_directory()
            .map(|home| format!("{home}/{}", self.path_under_home()))
            .ok_or_else(|| environment.missing(self.key()))
    }
}

#[cfg(test)]
#[path = "tests/deploy_host_folders/tests.rs"]
mod tests;
