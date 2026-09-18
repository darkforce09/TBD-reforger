//! T-892 — port of `scripts/mod/world-boot.sh` → `cargo xtask mod world-boot`.
//!
//! Exit: **0** PASS · **1** CODE · **2** usage · **3** ENVIRONMENT.
//! Verdict / `--selftest` → [`crate::commands::mod_ops::world_boot_verdict`] (SIZE split).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use regex::Regex;
use serde_json::{Value, json};

use crate::commands::mod_ops::world_boot_verdict::MissionCtx;
use crate::core::repository_root::find_repo_root;

const FIXTURE_TITLE: &str = "T-186 compiled-boot fixture";
const SERVER_REL: &str = ".local/share/Steam/steamapps/common/Arma Reforger Server";

struct Opts {
    keep_logs: bool,
    selftest: bool,
    mission: Option<String>,
    compiled: bool,
    compiled_uuid: Option<String>,
}

struct GateExit(u8);

struct RunState {
    run_dir: PathBuf,
    keep_logs: bool,
    cleaned: AtomicBool,
    svc_token: Option<String>,
    dev_access_token: Option<String>,
    api_base: String,
    child: Option<Child>,
}

impl RunState {
    fn cleanup(&self) {
        if self
            .cleaned
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return;
        }
        kill_run(&self.run_dir.join("server.pid"));
        sweep_fixture_missions(
            &self.run_dir,
            &self.api_base,
            self.svc_token.as_deref(),
            self.dev_access_token.as_deref(),
        );
        if self.keep_logs {
            println!("run dir kept: {}", self.run_dir.display());
        } else {
            let _ = fs::remove_dir_all(&self.run_dir);
        }
    }
}

impl Drop for RunState {
    fn drop(&mut self) {
        self.cleanup();
    }
}

#[rustfmt::skip]
const T302_EQUIP_OK: usize = 4; // perturb to 3 → `mod world-boot --selftest` RED

mod execution;
use execution::api_doc_fail;
use execution::api_env_fail;
use execution::api_http_fail;
pub use execution::run;

mod compiled_lane;
use compiled_lane::compiled_lane;
use compiled_lane::kill_run;
use compiled_lane::poll_for_log;
use compiled_lane::spawn_server;
use compiled_lane::sweep_fixture_missions;
use compiled_lane::t302_assert;
use compiled_lane::t302_selftest;
use compiled_lane::write_server_json;

mod resolve_service_token;
use resolve_service_token::curl_http;
use resolve_service_token::dev_login_token;
use resolve_service_token::env_u64;
use resolve_service_token::host_command;
use resolve_service_token::is_executable;
use resolve_service_token::read_addon_guid;
use resolve_service_token::read_scenario_id;
use resolve_service_token::require_host;
use resolve_service_token::resolve_service_token;
use resolve_service_token::tempfile_dir;
