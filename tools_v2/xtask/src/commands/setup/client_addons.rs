//! `cargo xtask setup client-addons`: link the framework into the client addon folder.
//!
//! **Role:** links `apps/mod/tbd-framework` into `$HOME/.local/share/tbd-server-addons`, prints
//! the Steam launch options that load it, and prints where to Direct Join.
//!
//! **Position:** called by [`crate::commands::setup::dispatch`]; the join hint reads the staging
//! host from `deploy.env` through [`crate::core::deploy_environment`].
//!
//! **Signals & state:** writes one symlink under `$HOME`; reads the deploy settings and, for the
//! hint, asks the resolver for the host's IPv4 address.
//!
//! **Invariants:** `mkdir -p` and `ln -sfn` run as the shell tools, so a failure reports their own
//! message and exit code; the link is made even when the framework folder is missing (a dangling
//! link, as `ln -sfn` makes it); the hint never fails the command: a host that is not configured
//! or does not resolve changes only the hint's words.

use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

use crate::core::deploy_environment::{DeployEnvironment, deploy_environment_path};
use crate::core::repository_root::find_repo_root;

/// The path pins, for an already-resolved monorepo root.
struct Paths {
    mod_root: PathBuf,
}

impl Paths {
    fn from_root(root: &Path) -> Self {
        Self {
            mod_root: root.join("apps/mod"),
        }
    }
}

/// Where the last line tells the player to Direct Join.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectJoinTarget {
    /// `TBD_SSH_HOST` names a host with an IPv4 address from here.
    Resolved { host: String, address: Ipv4Addr },
    /// `TBD_SSH_HOST` names a host this machine cannot resolve to IPv4.
    Unresolved { host: String, reason: String },
    /// No usable `TBD_SSH_HOST` in the deploy settings file at `path`.
    NotConfigured { path: PathBuf },
}

impl DirectJoinTarget {
    /// The target the deploy settings at `path` name.
    fn from_settings(path: &Path) -> DirectJoinTarget {
        let not_configured = || DirectJoinTarget::NotConfigured {
            path: path.to_path_buf(),
        };
        let Ok(environment) = DeployEnvironment::load_if_present(path) else {
            return not_configured();
        };
        let Ok(host) = environment.deploy_host() else {
            return not_configured();
        };
        match host.resolve_ipv4() {
            Ok(address) => DirectJoinTarget::Resolved {
                host: host.host().to_string(),
                address,
            },
            Err(reason) => DirectJoinTarget::Unresolved {
                host: host.host().to_string(),
                reason,
            },
        }
    }
}

/// The closing line: where to Direct Join, and what to fix when the address is unknown.
pub fn direct_join_hint(target: &DirectJoinTarget) -> String {
    let destination = match target {
        DirectJoinTarget::Resolved { host, address } => format!("{host} ({address}) port 2001"),
        DirectJoinTarget::Unresolved { host, reason } => {
            format!("{host} port 2001 (no IPv4 address from here: {reason})")
        }
        DirectJoinTarget::NotConfigured { path } => format!(
            "the staging host, port 2001 (set TBD_SSH_HOST in {} to print its address)",
            path.display()
        ),
    };
    format!("Restart the game, then Direct Join → {destination}")
}

/// Entry for `xtask setup client-addons`.
pub fn run() -> Result<u8> {
    let root = find_repo_root()?;
    run_in(&root)
}

/// `run` with the repo root injected: reads `$HOME` and the deploy settings from the process.
///
/// Split out so the `$HOME` test does not have to `set_current_dir` into a throwaway root to make
/// `find_repo_root` land there: that chdir is process-wide, and every other test thread walking
/// from the working directory at that instant would resolve the throwaway root, which carries a
/// `.ai/tickets/ROOT` marker.
pub fn run_in(root: &Path) -> Result<u8> {
    let home = std::env::var("HOME").context("HOME is unset")?;
    let target = DirectJoinTarget::from_settings(&deploy_environment_path(root));
    run_with_root(root, Path::new(&home), &target)
}

/// Testable entry that does not walk for the repo root or read `$HOME` from the process.
pub fn run_with_root(root: &Path, home: &Path, target: &DirectJoinTarget) -> Result<u8> {
    let paths = Paths::from_root(root);
    let staging = home.join(".local/share/tbd-server-addons");
    let framework = paths.mod_root.join("tbd-framework");
    let link = staging.join("tbd-framework");

    // `mkdir -p` as the shell tool, so a failure prints GNU mkdir's own message.
    let mkdir_status = Command::new("mkdir")
        .arg("-p")
        .arg(&staging)
        .status()
        .context("mkdir -p")?;
    if !mkdir_status.success() {
        return Ok(mkdir_status.code().unwrap_or(1) as u8);
    }

    // `ln -sfn` as the shell tool, for GNU ln's own message on a permission or nesting failure.
    let ln_status = Command::new("ln")
        .arg("-sfn")
        .arg(&framework)
        .arg(&link)
        .status()
        .context("ln -sfn")?;
    if !ln_status.success() {
        return Ok(ln_status.code().unwrap_or(1) as u8);
    }

    println!("Client addon staging: {}", link.display());
    println!();
    println!("Steam → Arma Reforger → Properties → Launch Options:");
    println!(
        "  -addonsDir \"{}\" -addons B2C3D4E5F6A78901",
        staging.display()
    );
    println!();
    println!("{}", direct_join_hint(target));

    Ok(0)
}

#[cfg(test)]
#[path = "tests/client_addons/tests.rs"]
mod tests;
