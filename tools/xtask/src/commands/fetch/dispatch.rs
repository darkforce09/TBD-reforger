use super::cli::FetchCmd;
use anyhow::Result;
use repository_layout::prelude::find_repository_root;
use std::path::PathBuf;

pub(crate) fn run(cmd: FetchCmd) -> Result<u8> {
    match cmd {
        FetchCmd::VanillaSource { args } => {
            // TBD_FETCH_ROOT: throwaway fixture roots for the comparison arms.
            // Production callers leave it unset → find_repository_root().
            let root = match std::env::var_os("TBD_FETCH_ROOT") {
                Some(p) => PathBuf::from(p),
                None => find_repository_root()?,
            };
            Ok(enfusion_script_index::vanilla_page_fetch::vanilla_source::run(&root, &args)?)
        }
        FetchCmd::VanillaApi { args } => {
            // Prefer $PWD (logical path) so cache: lines match bash `cd … && pwd`
            // on dual-homed hosts (/home vs /var/home). TBD_FETCH_ROOT wins for fixtures.
            let root = match std::env::var_os("TBD_FETCH_ROOT") {
                Some(p) => PathBuf::from(p),
                None => match std::env::var_os("PWD") {
                    Some(pwd) => {
                        let p = PathBuf::from(pwd);
                        if repository_layout::prelude::is_repository_root(&p) {
                            p
                        } else {
                            find_repository_root()?
                        }
                    }
                    None => find_repository_root()?,
                },
            };
            Ok(enfusion_script_index::vanilla_page_fetch::vanilla_api::run(
                &root, &args,
            )?)
        }
    }
}
