//! `cargo xtask mod world-boot`: boot the dedicated server headless and read its verdict.
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

use crate::commands::mod_ops::website_api_client::{
    ApiClient, CurlTransport, StagedArtifact, artifact_document, create_mission, delete_mission,
    development_login, mission, own_missions_titled, pending_review_artifact, stage_artifact_cache,
    submit_mission,
};
use crate::commands::mod_ops::world_boot_verdict::MissionCtx;
use crate::core::repository_root::find_repo_root;

const FIXTURE_TITLE: &str = "compiled-boot fixture";
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
    /// The mission maker's development token, when the compiled lane logged in: the sweep
    /// deletes the fixture missions it created.
    api_token: Option<String>,
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
        sweep_fixture_missions(&self.run_dir, &self.api_base, self.api_token.as_deref());
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
const EXPECTED_EQUIP_OK: usize = 4; // perturb to 3 → `mod world-boot --selftest` RED

mod execution;
use execution::api_doc_fail;
use execution::api_env_fail;
pub use execution::run;

mod compiled_lane;
use compiled_lane::assert_four_weapon_equip;
use compiled_lane::compiled_lane;
use compiled_lane::four_weapon_equip_selftest;
use compiled_lane::kill_run;
use compiled_lane::poll_for_log;
use compiled_lane::spawn_server;
use compiled_lane::sweep_fixture_missions;
use compiled_lane::write_server_json;

mod boot_environment;
use boot_environment::env_u64;
use boot_environment::host_command;
use boot_environment::is_executable;
use boot_environment::read_addon_guid;
use boot_environment::read_scenario_id;
use boot_environment::require_host;
use boot_environment::tempfile_dir;
