//! `cargo xtask mod compile`: the headless Enfusion compile gate.
//! Exit: **0** clean · **1** CODE · **2** no verdict · **3** ENV. `--selftest` must exit **1**.

use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use regex::Regex;

use crate::commands::mod_ops::compile_host::{
    Session, hostrun, is_executable, kill_run, mktemp_dir, require_host,
};
use crate::core::repository_root::find_repo_root;

/// bash `sed -n '2,44p' "$0"`.
const HELP: &str = include_str!("compile_help.txt");

const SELFTEST_GPROJ: &str = "\
GameProject {\n\
 ID \"TBD_CompileSelfTest\"\n\
 GUID \"C0FFEE0000000001\"\n\
 TITLE \"TBD Compile Self Test\"\n\
 Dependencies {\n\
  \"58D0FB3206B6F859\"\n\
 }\n\
 Configurations {\n\
  GameProjectConfig PC {\n\
  }\n\
  GameProjectConfig HEADLESS {\n\
  }\n\
 }\n\
}\n";

const SELFTEST_C: &str = "\
// Deliberately broken — proves the gate still detects compile errors.\n\
// NOTE: must be an undefined symbol; malformed punctuation compiles clean in Enfusion.\n\
class TBD_CompileSelfTest\n\
{\n\
\tvoid Broken()\n\
\t{\n\
\t\tTBD_ThisSymbolDoesNotExist_SelfTest();\n\
\t}\n\
}\n";

const PROBE_GPROJ: &str = "\
GameProject {\n\
 ID \"TBD_ApiProbe\"\n\
 GUID \"C0FFEE0000000002\"\n\
 TITLE \"TBD API Probe\"\n\
 Dependencies {\n\
  \"58D0FB3206B6F859\"\n\
 }\n\
 Configurations {\n\
  GameProjectConfig PC {\n\
  }\n\
  GameProjectConfig HEADLESS {\n\
  }\n\
 }\n\
}\n";

const LAUNCH_SH: &str = r#"
  echo $$ > "$1/server.pid"
  exec timeout "$2" ./ArmaReforgerServer \
    -addonsDir "$1/addons" -addons "$3" -profile "$1/profile" -maxFPS 15
"#;

const CAL_SH: &str = r#"
    echo $$ > "$1/server.pid"
    exec timeout 120 ./ArmaReforgerServer -addonsDir "$1/addons" -profile "$1/profile" -maxFPS 15
"#;

#[derive(Debug, Default)]
pub struct Opts {
    pub selftest: bool,
    pub keep_logs: bool,
    pub probe_dir: Option<PathBuf>,
}

enum Parse {
    Help,
    Run(Opts),
}

#[cfg(test)]
#[path = "tests/compile/tests.rs"]
mod tests;

mod execution;
use execution::count_game_scripts;
pub use execution::run;
pub use execution::run_preflight;
pub use execution::run_selftest;

mod report_compile_errors;
use report_compile_errors::count_tbd_warnings;
use report_compile_errors::env_fail;
use report_compile_errors::file_contains;
use report_compile_errors::last_re;
use report_compile_errors::latest_logs_dir;
use report_compile_errors::load_count_guard;
use report_compile_errors::parse_args;
use report_compile_errors::report_compile_errors;

#[cfg(test)]
use execution::{run_with_root, workbench_tooling_guard};
