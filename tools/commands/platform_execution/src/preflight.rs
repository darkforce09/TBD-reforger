//! `cargo xtask platform preflight`: is this machine set up to run the platform factory unattended?
//!
//! **Role:** runs the factory's machine assertions (host bridge, cargo, disk, memory, swap, the
//! shared `CARGO_TARGET_DIR`, stray per-worktree build folders, the run target's provenance stamp,
//! working tree, branch, remote, stale worktrees, the central ticket manager and its ticket check,
//! the wave plan and its agreement with the marker ledger, Postgres, the API, `trunk serve` and
//! stray Chrome) and prints one `✓` / `✗ BLOCK` / `! WARN` line per check
//! plus a summary.
//!
//! **Position:** reached through `platform_dispatch` from `cargo xtask platform preflight
//! [--warn]`; `execution.rs` holds the check sequence and `ok.rs` the probes it calls. The run
//! stamp it reads is the one `wave_execution` writes beside the run binaries.
//!
//! **Signals & state:** the block and warn `Counters` of one run; probes spawn read-only processes
//! (`git`, `df`, `pgrep`, `curl`, the host bridge) through [`process_runner::Run`].
//!
//! **Invariants:** any block makes the exit non-zero unless `--warn` is given; a run target whose
//! binaries carry no readable stamp, or a stamp naming another commit or checkout, blocks, because
//! unknown provenance is never green; disk and memory figures are wall-clock readings and not
//! reproducible; `TBD_PREFLIGHT_ROOT` overrides the checked repository root for fixtures.

use std::env;
use std::fs;
use std::io::{self, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::{Error, Result};

use process_runner::Run;
use repository_root::find_repository_root;
use ticket_manager_client::{TicketManager, WavePlan};

struct Counters {
    block: u32,
    warn: u32,
}

// ── THE RUN TARGET'S PROVENANCE ──────────────────────────────────────────────────────────────
//
// `stray_worktree_targets` above answers "did a worktree build into its own `target/`?" — a disk
// question. This answers the one that cost wave 1 a day: "is the binary a run lane is about to
// launch the code that is actually on main?" Cargo cannot answer it. Its `-C metadata` hash does
// not include the manifest path, so two checkouts of one package write the same artifact and the
// same uplifted `<profile>/<bin>`, and freshness is mtime-keyed, so the second build is satisfied
// by the first and prints `Finished` with no `Compiling` line. MEASURED 2026-09-06:
// a worktree built `UNMERGED-SLICE-CODE`, the main checkout's `cargo run` then printed it.
//
// So `cargo xtask platform wave run` writes `tbd-built-from` (`<sha> <checkout>`) beside the
// binaries, and this reads it back. THREE answers, and only one of them is green: agreement,
// disagreement, and NO STAMP — because binaries whose provenance is unknown are exactly the
// case the wave-1 incident presented as, and treating unknown as fine is the signature defect.

/// Cargo's two profile directories, in the order this check reports them.
const RUN_PROFILE_DIRS: &[&str] = &["debug", "release"];

/// What the run target's stamp says about the binaries sitting in it.
#[derive(Debug, PartialEq, Eq)]
enum RunTargetState {
    /// No run target on disk, or no binaries in it. Nothing can be stale.
    Empty,
    /// Stamped, and the stamp names this checkout at HEAD.
    Fresh { profile: String, bins: usize },
    /// Binaries with no readable `tbd-built-from` beside them.
    Unstamped { profile: String, bins: Vec<String> },
    /// Stamped, and the stamp disagrees with HEAD or with this checkout.
    Stale {
        profile: String,
        bins: Vec<String>,
        stamp: crate::wave_execution::RunStamp,
    },
}

mod ok;
use ok::api_listen_pid;
use ok::capture_stdout;
use ok::count_pgrep;
use ok::curl_http_code;
use ok::format_hhmm_epoch;
use ok::free_gb;
use ok::git_out;
use ok::git_status_porcelain;
use ok::hostrun;
use ok::is_git_young;
use ok::mem_available_mib;
use ok::nope;
use ok::ok;
use ok::orphan_cache_mb;
use ok::proc_start_epoch;
use ok::resolve_root;
use ok::run_target_detail;
use ok::run_target_state;
use ok::soft;
use ok::status_ok;
use ok::stray_worktree_targets;
use ok::swap_used_pct;
use ok::tcp_up;
use ok::wave_ledgers_agree;
use ok::worktree_paths;

mod execution;
pub(crate) use execution::run;
