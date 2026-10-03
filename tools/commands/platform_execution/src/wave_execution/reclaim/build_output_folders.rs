//! The reclaim sweeps of the main checkout's build output folder and of the retired root-level
//! build folders beside it.
//!
//! **Role:** reports the permanent part of `<main checkout>/target/` (the shared cache, the
//! development API build, the continuous-integration scratch folder, the `mcpd` build, the
//! database selftest's compose project), sweeps the wave-gate subfolders `target/gate-*` on
//! request, and deletes the retired root-level build folders (`target-dev-api`, `target-ci`,
//! `target-dev-mcpd`, `target-mk-db-selftest`, `target-gate-*`, `dist-gate-*`) wherever it finds
//! them.
//!
//! **Position:** called by `cmd_reclaim` in the sibling `reclaim_command.rs`; the folder names come
//! from [`repository_layout::build_output`].
//!
//! **Signals & state:** none held; each sweep deletes folders under the root it is given and
//! returns the megabytes it freed.
//!
//! **Invariants:** nothing outside `<root>/target/gate-*` and the retired root-level names is ever
//! deleted here; the shared cache, the development API build and every non-gate subfolder are
//! measured and spared; the retired folders are deleted by default, because no tool writes them.

use super::reclaim_command::{du_mb, glob_dir, sz_or_q};
use super::*;
use repository_layout::build_output::{
    BUILD_OUTPUT_FOLDER, CONTINUOUS_INTEGRATION_SUBFOLDER, DATABASE_SELFTEST_SUBFOLDER,
    DEV_API_SUBFOLDER, GATE_SUBFOLDER_PREFIX, MCP_DAEMON_SUBFOLDER, PURPOSE_SUBFOLDERS,
    build_output_subfolder, is_retired_root_level_build_folder,
};

/// Delete every retired root-level build folder directly under `main_root`; returns the megabytes
/// freed. Prints a section only when it finds one.
pub(super) fn sweep_retired_root_level_folders(main_root: &Path) -> u64 {
    let root = main_root.display().to_string();
    let retired: Vec<PathBuf> = glob_dir(&root, is_retired_root_level_build_folder)
        .into_iter()
        .filter(|d| d.is_dir())
        .collect();
    if retired.is_empty() {
        return 0;
    }
    wprintln!(
        "retired root-level build folders at {root} (no tool writes them; build output lives in target/):"
    );
    let mut freed = 0;
    for d in retired {
        let sz = du_mb(&d);
        if std::fs::remove_dir_all(&d).is_ok() {
            freed += sz.unwrap_or(0);
            wprintln!("  removed {:<44} {} MB", d.display(), sz_or_q(sz));
        } else {
            wprintln!(
                "  FAILED  {:<44} {} MB  (could not delete)",
                d.display(),
                sz_or_q(sz)
            );
        }
    }
    freed
}

/// Print the spared part of `<main_root>/target/`: the whole folder's size, then each non-gate
/// purpose subfolder that exists (the development API build, the continuous-integration scratch
/// folder, the `mcpd` build, the database selftest's compose project).
pub(super) fn report_permanent_build_output(main_root: &Path) {
    let shared = main_root.join(BUILD_OUTPUT_FOLDER);
    wprintln!("build output at {}:", shared.display());
    wprintln!(
        "  spared  {:<44} {} MB  (shared CARGO_TARGET_DIR with every purpose subfolder — never reclaimed whole)",
        shared.display(),
        sz_or_q(du_mb(&shared))
    );
    for subfolder in PURPOSE_SUBFOLDERS
        .iter()
        .filter(|name| !name.starts_with(GATE_SUBFOLDER_PREFIX))
    {
        let folder = build_output_subfolder(main_root, subfolder);
        if !folder.is_dir() {
            continue;
        }
        let why = match *subfolder {
            DEV_API_SUBFOLDER => "development API build of `cargo xtask mk rust-api` — permanent",
            CONTINUOUS_INTEGRATION_SUBFOLDER => "`cargo xtask mk reclaim-target-ci` deletes it",
            MCP_DAEMON_SUBFOLDER => "`mcpd` build of `cargo xtask mcp daemon start` — permanent",
            DATABASE_SELFTEST_SUBFOLDER => {
                "compose project of `cargo xtask db selftest` — rewritten each run"
            }
            _ => "purpose subfolder",
        };
        wprintln!(
            "  spared  {:<44} {} MB  ({why})",
            folder.display(),
            sz_or_q(du_mb(&folder))
        );
    }
}

/// The wave-gate subfolders `<main_root>/target/gate-*`, in the order bash's glob would list them.
pub(super) fn gate_folders(main_root: &Path) -> Vec<PathBuf> {
    let target = main_root.join(BUILD_OUTPUT_FOLDER).display().to_string();
    glob_dir(&target, |n| n.starts_with(GATE_SUBFOLDER_PREFIX))
        .into_iter()
        .filter(|d| d.is_dir())
        .collect()
}

/// The gate set, opt-in: with `sweep` set, delete every gate folder (only those older than
/// `min_age_days` when that is positive); otherwise print their total size. Returns the megabytes
/// freed.
///
/// The gate folders are a warm cache every future wave gate hits (a cold slice gate measured
/// 23.4 s against 9.3 s warm), so deleting them bills work that has not happened yet; that is why
/// the sweep waits for `--gate-dirs`.
pub(super) fn sweep_gate_folders(main_root: &Path, sweep: bool, min_age_days: i64) -> u64 {
    let target = main_root.join(BUILD_OUTPUT_FOLDER);
    if !sweep {
        let total: u64 = gate_folders(main_root)
            .iter()
            .map(|d| du_mb(d).unwrap_or(0))
            .sum();
        if total > 0 {
            wprintln!(
                "gate dirs at {}: {total} MB not reclaimed (pass --gate-dirs to opt in)",
                target.display()
            );
        }
        return 0;
    }
    // A bare `--gate-dirs` prints ", min age 0d": the age filter is off at 0.
    wprintln!("gate dirs (--gate-dirs, min age {min_age_days}d):");
    let mut freed = 0;
    for d in gate_folders(main_root) {
        if min_age_days > 0 {
            let age_days = dir_age_days(&d);
            if age_days < min_age_days {
                wprintln!(
                    "  spared (age {age_days}d < {min_age_days}d) {}",
                    d.display()
                );
                continue;
            }
        }
        let sz = du_mb(&d);
        if std::fs::remove_dir_all(&d).is_ok() {
            freed += sz.unwrap_or(0);
            wprintln!("  removed {:<44} {} MB", d.display(), sz_or_q(sz));
        }
    }
    freed
}
