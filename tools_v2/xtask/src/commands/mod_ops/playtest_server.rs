//! `cargo xtask mod playtest`: the local dedicated-server lane.
//!
//! Start a JOINABLE, mod-loaded, admin-capable dedicated server. The bash header's institutional
//! record is carried over below, because every paragraph of it is a measured fact that cost a wave
//! to learn, and a port that summarises them away throws that away.
//!
//! ── WHY THIS EXISTS ──────────────────────────────────────────────────────────────────
//!
//! Nothing in this repo started a server two people could join with the LOCAL mod loaded.
//! The staging deploy builds two ExecStarts and each one breaks a different half:
//!
//! ```text
//!   :1155  -addonsDir + -addons + -server   loads the local mod, registers NO backend room
//!   :1153  -config (no -addonsDir)          registers a room, cannot resolve the local mod
//! ```
//!
//! ── WHAT IS ACTUALLY TRUE (measured 2026-07-31, engine 1.7.0.54, this file's boot) ────────────
//!
//! `-addonsDir <dir>` **plus** `-config <json>` does BOTH at once. Verbatim, one boot:
//!
//! ```text
//!   ENGINE : FileSystem: Adding relative directory '<checkout>/apps/mod/tbd-framework'
//!            to filesystem under name TBD_Framework
//!   ENGINE : Loaded addons:
//!            gproj: '<addonsDir>/tbd-framework/addon.gproj' guid: 'B2C3D4E5F6A78901'
//!   NETWORK: Starting RPL server, listening on address 0.0.0.0:2001, fastValidation=true
//!   BACKEND: Server registered with address: 192.168.0.117:2001
//!   BACKEND: Direct Join Code: 0207990185
//! ```
//!
//! So the room DOES register with the local addon loaded.
//! `documentation_v2/runbooks/game_server_staging/README.md` said this was impossible without a
//! Workshop publish; that was measured on `-addons`, never on `-addonsDir`.
//!
//! ── THE TRAP THIS PROGRAM EXISTS TO CLOSE ────────────────────────────────────────────────────
//!
//! `tbd-framework` IS published to the Workshop, unlisted, under the SAME id as the local gproj
//! GUID (`B2C3D4E5F6A78901`), at a stale **version 1.0.1**. So `-config` on its own does not fail
//! loudly — the engine quietly downloads that June build and runs it. A `-config`-only boot
//! therefore looks completely healthy — it registers a room, it reaches LOBBY — while running
//! months-old script. The difference is the log FORMAT, not any one line: June emits flat
//! `[TBD] ...` with no subsystem tag, the current build tags every line `[TBD][Subsystem] ...`.
//! That is this codebase's signature defect wearing the engine's clothes, so
//! `boot::assert_local_addon_won` is a HARD GATE, not a warning: if the packed profile copy wins,
//! this program kills the server and exits non-zero.
//!
//! COUNT THE FORMAT, NOT THE LINES. The bash comment once asserted **109** tagged lines and
//! `documentation_v2/runbooks/game_server_staging/README.md` asserted **108** for the same
//! claim. Neither was a typo. Measured
//! on this checkout 2026-07-31 with `mod world-boot --keep-logs`: slot-loadout-coverage (7 slots)
//! -> 147 `[TBD][` lines, bridgehead-at-levie (18 slots) -> 155. The number rots even with the
//! mission held fixed, and is not monotonic in slot count either. The stable discriminator is the
//! discontinuity at ZERO: stale 1.0.1 emits zero `[TBD][` lines, any current build emits many.
//! Do not "correct" these numbers upward.
//!
//! ── EXIT CODES (same contract as world-boot / compile) ───────────────────────────────────────
//!
//! ```text
//!   0  server booted, local addon won, backend room registered — join details printed
//!   1  CODE/CONFIG: the server died, refused the config, or loaded the WRONG addon copy
//!   2  usage
//!   3  ENVIRONMENT: this machine cannot run the gate at all (no host bridge, no game installed)
//! ```
//!
//! A `1` can ALSO mean "this program could not confirm the server died" — see the STRAY SERVER
//! block in [`lifecycle`]. That block names the process group and the exact command to run.
//!
//! ── FILE LAYOUT (production modules have a 500-line ceiling) ────────────────────────────────
//!
//! | file | owns |
//! |---|---|
//! | `playtest_server.rs` | the header record, CLI parsing, `usage_fail`/`env_fail`, preflight, orchestration |
//! | `playtest_server/host.rs` | re-export of [`crate::core::host_execution`] — container detection, `distrobox-host-exec`/`host-spawn` |
//! | `playtest_server/lifecycle.rs` | the tri-state liveness probe, `kill_run`, the run lock, `assert_no_live_server`, `--selftest` |
//! | `playtest_server/render.rs` | the three former `python3` sites — backend config patch, admin list, `server.json` |
//! | `playtest_server/platform_deployment` | the deployment the server runs: provision, confirm, release; or the offline artifact |
//! | `playtest_server/logread.rs` | every `grep` against `server.out` — boot phase, the addon hard gate, the error dump |
//! | `playtest_server/boot.rs` | launching the engine, the wait loop, the join banner, Ctrl-C and shutdown |
//!
//! ── THE THREE `python3` SITES ARE GONE ───────────────────────────────────────────────────────
//!
//! `scripts/python-inventory.txt` listed this script for "backend cfg + admin list JSON". All three
//! heredocs are now `serde_json` in [`render`], which removes the last reason this launcher needed
//! an interpreter at all. `serde_json` is built with `preserve_order`, so a round-trip through the
//! dev config keeps the operator's key order exactly as `json.load` / `json.dump` did — see
//! [`render`] for the two formatting details that had to be reproduced by hand (`ensure_ascii` and
//! the `, ` item separator).

mod boot;
mod host;
mod lifecycle;
mod logread;
mod platform_deployment;
mod render;

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::core::repository_root::find_repo_root;
use host::Host;

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
  --token=<tok>         SERVICE_TOKEN; default read from apps/website/api_v2/.env
  --admin=<id>          identityId (UUID) or 17-digit SteamID; repeatable
  --name=<s>            server browser name
  --scenario=<id>       scenarioId override (default: from tbd-dev-server.config.json)
  --port=<n>            game port, default 2001
  --a2s-port=<n>        A2S port, default 17777 (MUST differ from --port)
  --max-players=<n>     default 8
  --run-dir=<dir>       staging root, default $HOME/tbd-playtest
  --timeout=<sec>       stop the server after <sec> (default: run until Ctrl-C)
  --dry-run             render + validate everything, print the command line, boot nothing
  --selftest            prove kill_run + the run lock actually work; boots no game server
";

/// The one-line refusal printed when a required flag is missing.
const USAGE_LINE: &str = "Usage: cargo xtask mod playtest --mission=<uuid> | --artifact-file=<p> [--admin=<id>] [--dry-run]";

/// Everything the flag loop can set.
#[derive(Debug, Clone)]
pub struct Opts {
    pub mission: String,
    pub event_mission: String,
    pub server: String,
    pub artifact_file: String,
    pub backend_url: String,
    pub token: String,
    pub server_name: String,
    pub scenario: String,
    pub game_port: String,
    pub a2s_port: String,
    pub max_players: String,
    pub run_dir: String,
    pub run_timeout: String,
    pub dry_run: bool,
    pub selftest: bool,
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
            token: String::new(),
            server_name: String::new(),
            scenario: String::new(),
            // PORTS AND COUNTS STAY STRINGS until the moment they are needed as numbers. bash never
            // validated them either, and `--port=abc` has to reach the same place it always did:
            // the JSON renderer, which is where the failure is legible. Parsing here would invent a
            // new error message that no baseline covers.
            game_port: "2001".into(),
            a2s_port: "17777".into(),
            max_players: "8".into(),
            run_dir: format!("{home}/tbd-playtest"),
            run_timeout: String::new(),
            dry_run: false,
            selftest: false,
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
pub use usage_fail::run;

#[cfg(test)]
use usage_fail::{admin_id_is_valid, parse, read_addon_guid, read_scenario};
