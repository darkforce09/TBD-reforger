//! `cargo xtask mod playtest`: the local dedicated-server lane.
//!
//! **Role:** starts a JOINABLE, mod-loaded, admin-capable dedicated server. The staging deploy's
//! two ExecStarts each break a different half:
//!
//! ```text
//!   :1155  -addonsDir + -addons + -server   loads the local mod, registers NO backend room
//!   :1153  -config (no -addonsDir)          registers a room, cannot resolve the local mod
//! ```
//!
//! `-addonsDir <dir>` **plus** `-config <json>` does BOTH at once (measured on engine 1.7.0.54):
//!
//! ```text
//!   ENGINE : FileSystem: Adding relative directory '<checkout>/apps/mod/tbd-framework'
//!            to filesystem under name TBD_Framework
//!   ENGINE : Loaded addons:
//!            gproj: '<addonsDir>/tbd-framework/addon.gproj' guid: 'B2C3D4E5F6A78901'
//!   NETWORK: Starting RPL server, listening on address 0.0.0.0:2001, fastValidation=true
//!   BACKEND: Server registered with address: <LAN address>:2001
//!   BACKEND: Direct Join Code: 0207990185
//! ```
//!
//! So the room registers with the local addon loaded, without a Workshop publish.
//!
//! **Position:** called by [`crate::mod_dispatch`] for `mod playtest` and through
//! [`crate::development_server`] for `mod dev-server`. This file holds the help text and the
//! options; the submodules own the rest:
//!
//! | file | owns |
//! |---|---|
//! | `playtest_server/usage_fail.rs` | flag parsing, `usage_fail`/`env_fail`, preflight, the run order |
//! | `playtest_server/host.rs` | the container↔host bridge, [`process_runner::host_execution`] |
//! | `playtest_server/lifecycle.rs` | the tri-state liveness probe, `kill_run`, the run lock, `assert_no_live_server`, `--selftest` |
//! | `playtest_server/render.rs` | the backend config patch, the admin list, `server.json` |
//! | `playtest_server/platform_deployment.rs` | the deployment the server runs: provision, confirm, release; or the offline artifact |
//! | `playtest_server/telemetry_check.rs` | the runtime's telemetry queue reading, the seen matches' events, the release verdict |
//! | `playtest_server/logread.rs` | every search of `server.out`: boot phase, the addon hard gate, the error dump |
//! | `playtest_server/boot.rs` | launching the engine, the wait loop, the join banner, Ctrl-C and shutdown |
//!
//! **Signals & state:** the run lock and the run folder (`$HOME/tbd-playtest` by default) for the
//! run; the server runs in a process group of its own.
//!
//! **Invariants:** `tbd-framework` is also published to the Workshop, unlisted, under the SAME id
//! as the local gproj GUID (`B2C3D4E5F6A78901`), at a stale **version 1.0.1**, so `-config` on its
//! own does not fail loudly: the engine downloads that build and runs it, registers a room and
//! reaches LOBBY while running months-old script. The difference is the log FORMAT, not any one
//! line: 1.0.1 emits flat `[TBD] ...` with no subsystem tag, the current build tags every line
//! `[TBD][Subsystem] ...`. `boot::assert_local_addon_won` is therefore a HARD GATE: if the packed
//! profile copy wins, the server is killed and the run exits non-zero. Count the format, not the
//! lines: the number of `[TBD][` lines varies between boots of the same mission and is not
//! monotonic in slot count (147 for a 7-slot mission, 155 for an 18-slot one); the stable
//! discriminator is zero (1.0.1) against many (current). Exit codes, the same contract as
//! `world-boot` and `compile`:
//!
//! ```text
//!   0  server booted, local addon won, backend room registered — join details printed
//!   1  CODE/CONFIG: the server died, refused the config, or loaded the WRONG addon copy;
//!      or, with --require-telemetry, the telemetry check failed
//!   2  usage
//!   3  ENVIRONMENT: this machine cannot run the gate at all (no host bridge, no game installed)
//! ```
//!
//! A `1` can also mean "the server's death could not be confirmed": see the STRAY SERVER block in
//! [`lifecycle`], which names the process group and the exact command to run. The rendered JSON
//! keeps the operator's key order (`serde_json` with `preserve_order`); see [`render`].

mod boot;
mod host;
mod lifecycle;
mod logread;
mod platform_deployment;
mod render;
mod telemetry_check;

use std::path::{Path, PathBuf};

use crate::Result;

use host::Host;
use repository_root::find_repository_root;

/// Printed verbatim by `-h` / `--help`.
///
/// `help_text_matches_the_options_we_parse` pins every flag listed here against the parser, so
/// the help cannot advertise an option the command does not accept.
const HELP: &str = "\
Usage:
  cargo xtask mod playtest --mission=<uuid> [options]
  cargo xtask mod playtest --artifact-file=<p> --admin=<identityId> --dry-run
  cargo xtask mod playtest --selftest

Options:
  --mission=<uuid>      deploy this mission's approved artifact (submits + approves if needed)
  --event-mission=<id>  deploy it for this event mission (its seats bind to the artifact)
  --server=<uuid>       platform server to deploy on (default: the TBD Playtest server row)
  --artifact-file=<p>   boot this compiled mission document offline (no API, no credential)
  --backend-url=<url>   default http://127.0.0.1:8080
  --admin=<id>          identityId (UUID) or 17-digit SteamID; repeatable
  --name=<s>            server browser name
  --scenario=<id>       scenarioId override (default: from tbd-dev-server.config.json)
  --port=<n>            game port, default 2001
  --a2s-port=<n>        A2S port, default 17777 (MUST differ from --port)
  --max-players=<n>     default 8
  --run-dir=<dir>       staging root, default $HOME/tbd-playtest
  --timeout=<sec>       stop the server after <sec> (default: run until Ctrl-C)
  --require-telemetry   exit 1 when the runtime's telemetry does not reach the platform
  --dry-run             render + validate everything, print the command line, boot nothing
  --selftest            prove kill_run + the run lock actually work; boots no game server
";

/// The one-line refusal printed when a required flag is missing.
const USAGE_LINE: &str = "Usage: cargo xtask mod playtest --mission=<uuid> | --artifact-file=<p> [--admin=<id>] [--dry-run]";

/// Everything the flag loop can set.
#[derive(Debug, Clone)]
pub(crate) struct Opts {
    pub mission: String,
    pub event_mission: String,
    pub server: String,
    pub artifact_file: String,
    pub backend_url: String,
    pub server_name: String,
    pub scenario: String,
    pub game_port: String,
    pub a2s_port: String,
    pub max_players: String,
    pub run_dir: String,
    pub run_timeout: String,
    pub dry_run: bool,
    pub selftest: bool,
    /// `--require-telemetry`: a failed telemetry check fails the run.
    pub require_telemetry: bool,
    pub admins: Vec<String>,
}

impl Opts {
    fn defaults(home: &str) -> Opts {
        Opts {
            mission: String::new(),
            event_mission: String::new(),
            server: String::new(),
            artifact_file: String::new(),
            backend_url: "http://127.0.0.1:8080".into(),
            server_name: String::new(),
            scenario: String::new(),
            // Ports and counts stay strings until they are needed as numbers: `--port=abc`
            // reaches the JSON renderer, where the failure is legible, and gets its message there.
            game_port: "2001".into(),
            a2s_port: "17777".into(),
            max_players: "8".into(),
            run_dir: format!("{home}/tbd-playtest"),
            run_timeout: String::new(),
            dry_run: false,
            selftest: false,
            require_telemetry: false,
            admins: Vec::new(),
        }
    }
}

/// What the flag loop decided.
enum Parsed {
    Opts(Box<Opts>),
    /// `-h` / `--help` was reached — print and stop, rc 0.
    Help,
    /// An unrecognised token — `usage_fail`, rc 2.
    Unknown(String),
}

#[cfg(test)]
#[path = "tests/playtest_server/tests.rs"]
mod tests;

mod usage_fail;
use usage_fail::env_fail;
use usage_fail::grep_o;
pub(crate) use usage_fail::run;

#[cfg(test)]
use usage_fail::{admin_id_is_valid, parse, read_addon_guid, read_scenario};
