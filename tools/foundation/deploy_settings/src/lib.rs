//! The deploy settings file, `deploy/deploy.env`, read the same way by every command that reaches
//! a host.
//!
//! **Role:** [`DeployEnvironment`] loads the file and answers each setting under one precedence
//! rule (the file decides every key it assigns, the process environment fills the rest);
//! [`DeployHost`] parses `TBD_SSH_HOST`, [`DeployHostFolder`] resolves the remote folders that
//! default under the deploy user's home, and [`DeployEnvironment::ssh_base`] chooses the ssh
//! transport.
//! **Position:** tier 2 of `tools/foundation`, above `repository_layout` (the file's checkout
//! location) and `process_runner` (the ssh transport). The `deploy`, `setup`, `debug`, `mod` and
//! `staging` command groups of `xtask` and the API readiness checks read their settings through it.
//! **Signals & state:** a [`DeployEnvironment`] is an immutable snapshot of the file and of the
//! process environment, taken when it is loaded; nothing here writes either.
//! **Invariants:** the file is parsed and never executed; a stale exported variable never
//! overrides a key the file assigns; a present but unusable value is reported as `<path>:<line>`,
//! a missing one names the file, and no message echoes a value.

mod assignment_syntax;
mod deploy_environment;
mod deploy_host;
mod deploy_host_folders;
mod error;
pub mod prelude;

pub use deploy_environment::{
    DEPLOY_ENV_OVERRIDE_VARIABLE, DEPLOY_HOST_KEY, DeployEnvironment, SSH_IDENTITY_FILE_KEY,
    SSH_PASSWORD_KEY, deploy_environment_path, resolve_deploy_environment_path,
};
pub use deploy_host::{DeployHost, first_ipv4_address};
pub use deploy_host_folders::DeployHostFolder;
pub use error::{Error, Result, SettingError, SettingOrigin};
