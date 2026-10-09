//! The remote check both deploys run before their `--delete` rsync: the host's API `.env`.
//!
//! **Role:** the probe ([`probe_script`]) that asks the host whether the checkout's
//! [`API_ENVIRONMENT_FILE`] exists and is readable, its verdict ([`classify`]), the printed verdict
//! ([`report`]) and [`rsync_only_when_present`], which runs the rsync only after the file is proven
//! present.
//! **Position:** called by `cargo xtask deploy website` (`crate::website`), which sends the probe
//! as its `bash -lc` ssh word, and by `cargo xtask deploy staging`
//! (`tools/commands/deployment/src/staging/remote/fleet_deploy.rs`), which sends it as a `bash -s`
//! payload; both print the probe in their dry run.
//! **Signals & state:** none; pure functions over the probe's exit code.
//! **Invariants:** only exit 0 lets the rsync run; a missing or unreadable file, a missing
//! checkout, an ssh failure, a missing ssh program or any other exit refuses before the rsync,
//! with the operator's step printed.
//!
//! The `.env` lives on the host alone: no checkout tracks it and both rsyncs exclude
//! [`API_ENVIRONMENT_FILE`], so `--delete` leaves the host's copy alone only at that path. A host
//! whose file still sits where an earlier folder layout kept it holds it at a path no exclusion
//! names any more, and the rsync would delete it. Refusing while the file is absent at its current
//! path makes the operator move it first, and the deploy never runs against a host whose API
//! could not start for want of its secrets.

use crate::host_owned_paths::{API_ENVIRONMENT_FILE, API_SERVER_FOLDER};

/// The probe's exit code for "the file is missing or unreadable".
const MISSING: i32 = 20;

/// What the host's checkout holds at [`API_ENVIRONMENT_FILE`], as judged by the remote probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApiEnvironmentFile {
    /// A readable regular file: the deploy proceeds to the rsync.
    Present,
    /// No readable regular file at that path (or no checkout at all).
    Missing,
    /// The probe gave no verdict: the host was unreachable, ssh failed, or the shell exited with
    /// an unexpected status.
    Indeterminate(i32),
}

/// The host's absolute path of the file, in `remote_dir`'s checkout.
pub(crate) fn remote_path(remote_dir: &str) -> String {
    format!("{remote_dir}/{API_ENVIRONMENT_FILE}")
}

/// The remote shell that answers the question as an exit code: 0 when a readable regular file
/// sits at [`remote_path`], [`MISSING`] otherwise.
pub(crate) fn probe_script(remote_dir: &str) -> String {
    let file = remote_path(remote_dir);
    format!("if [ -f '{file}' ] && [ -r '{file}' ]; then exit 0; fi; exit {MISSING}")
}

/// The verdict of one probe exit code.
pub(crate) fn classify(code: i32) -> ApiEnvironmentFile {
    match code {
        0 => ApiEnvironmentFile::Present,
        MISSING => ApiEnvironmentFile::Missing,
        other => ApiEnvironmentFile::Indeterminate(other),
    }
}

/// The operator step the refusal of a missing file prints, one that works on any host. A host
/// whose `.env` still sits where the previous folder layout kept it moves that file (and the
/// API's `.tools` folder beside it) into the API server's folder. A fresh host holds no checkout
/// yet, so no template on the host either: the operator copies this checkout's
/// `.env.example` there over ssh, the command of `documentation/runbooks/website_deployment.md`
/// step 3, and fills it in.
pub(crate) fn missing_file_operator_step(remote_dir: &str) -> String {
    let folder = format!("{remote_dir}/{API_SERVER_FOLDER}");
    let file = remote_path(remote_dir);
    format!(
        "       Operator step: on a host that already holds the API's .env where the previous \
         folder layout kept it, move that file (and the .tools folder beside it) into {folder}/, \
         keeping the file's mode. On a fresh host, copy the template from this checkout, then \
         fill it in (documentation/runbooks/website_deployment.md, step 3):\n\
         \x20        ssh <TBD_SSH_HOST> 'mkdir -p {folder} && install -m 600 /dev/stdin {file}' \
         < {API_SERVER_FOLDER}/.env.example\n\
         \x20      Then rerun the deploy."
    )
}

/// Prints the verdict. `Ok(())` lets the deploy continue to the rsync; `Err(1)` stops it first.
pub(crate) fn report(verdict: ApiEnvironmentFile, remote_dir: &str) -> Result<(), u8> {
    let file = remote_path(remote_dir);
    match verdict {
        ApiEnvironmentFile::Present => {
            println!("    {file} present");
            Ok(())
        }
        ApiEnvironmentFile::Missing => {
            eprintln!("ERROR: the API's .env is missing or unreadable on the host: {file}");
            eprintln!(
                "       The rsync runs with --delete and never carries this file, so the deploy \
                 stops before it."
            );
            eprintln!("{}", missing_file_operator_step(remote_dir));
            Err(1)
        }
        ApiEnvironmentFile::Indeterminate(code) => {
            eprintln!(
                "ERROR: could not determine whether the host holds {file} (probe exit {code})."
            );
            eprintln!(
                "       Refusing to continue: the next step is an --delete rsync, and it is not \
                 safe to run that against a host whose API secrets could not be found."
            );
            Err(1)
        }
    }
}

/// Runs `rsync` only when `probe_exit`, the exit status of [`probe_script`] on the host, proves the
/// file present. A probe that could not run (`Err`, the code its runner reported) refuses with that
/// code, and every other verdict refuses through [`report`]; in both cases `rsync` never runs.
pub(crate) fn rsync_only_when_present(
    probe_exit: Result<i32, u8>,
    remote_dir: &str,
    rsync: impl FnOnce() -> Result<(), u8>,
) -> Result<(), u8> {
    let code = probe_exit?;
    report(classify(code), remote_dir)?;
    rsync()
}

#[cfg(test)]
#[path = "tests/api_environment_file_preflight/tests.rs"]
mod tests;
