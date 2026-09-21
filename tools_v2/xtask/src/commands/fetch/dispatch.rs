use super::cli::FetchCmd;
use crate::core::repository_root::find_repo_root;
use anyhow::Result;
use std::path::PathBuf;

pub(crate) fn run(cmd: FetchCmd) -> Result<u8> {
    match cmd {
        FetchCmd::VanillaSource { args } => {
            // TBD_FETCH_ROOT: throwaway fixture roots for T-853 bash-vs-port arms.
            // Production callers leave it unset → find_repo_root().
            let root = match std::env::var_os("TBD_FETCH_ROOT") {
                Some(p) => PathBuf::from(p),
                None => find_repo_root()?,
            };
            crate::commands::fetch::vanilla_source::run(&root, &args)
        }
        FetchCmd::VanillaApi { args } => {
            // Prefer $PWD (logical path) so cache: lines match bash `cd … && pwd`
            // on dual-homed hosts (/home vs /var/home). TBD_FETCH_ROOT wins for fixtures.
            let root = match std::env::var_os("TBD_FETCH_ROOT") {
                Some(p) => PathBuf::from(p),
                None => match std::env::var_os("PWD") {
                    Some(pwd) => {
                        let p = PathBuf::from(pwd);
                        if ticket_engine::repository::is_repo_root(&p) {
                            p
                        } else {
                            find_repo_root()?
                        }
                    }
                    None => find_repo_root()?,
                },
            };
            crate::commands::fetch::vanilla_api::run(&root, &args)
        }
    }
}
