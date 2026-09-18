//! Rust source length, font-table generation, and Node tooling gates.
//!
//! The length gate uses raw lines and fails on source files above their kind-specific ceiling.
//! The Node gate refuses tracked scripts and invocations outside the Enfusion tooling floor.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use verification_core::scan;
use verification_core::{Kind, NotRun, Verdict};

/* ─────────────────────────── verify file-length (SIZE-3) ─────────────────────────── */

const SIZE_3_PRODUCTION_MAX_LINES: usize = 500;
const SIZE_3_TEST_MAX_LINES: usize = 1000;

/// Directories the SIZE gate must examine. A missing pin is [`NotRun::TargetMissing`], never
/// an empty pass (T-899). Extra `apps/website/<name>/src` trees are picked up if they exist.
const FILE_LENGTH_PINS: &[&str] = &[
    "tools_v2/xtask",
    "tools_v2/verification-core",
    "tools_v2/ticket-engine",
    "tools_v2/developer-tools",
    "apps/ticketboard/src",
    "apps/website/api/src",
    "apps/website/frontend/src",
];

struct AllowEntry {
    rule: String,
    path: String,
    reason: String,
    expires: String,
}

/* ─────────────────────────── gen font-table (T-152.13) ─────────────────────────── */

/* ─────────────────────────── verify no-node (T-165.10 hard gate) ─────────────────────────── */

/// Files this gate declares it scans, over and above the [`SCAN_DIRS`] walk. A declared path that
/// is MISSING is a FAILURE, never a silent narrowing — see [`verify_no_node()`].
///
/// `Makefile` sat here until T-897 deleted it. It is removed rather than left to fail, and the
/// fail-closed rule below is the price of that removal: the next deletion cannot quietly shrink
/// the gate's reach the way this one could have.
const SCAN_FILES: &[&str] = &[];

/// Directory roots walked for `.sh` / `.yml` / `.yaml`. Same rule: declared-but-absent FAILS.
const SCAN_DIRS: &[&str] = &["scripts", ".github"];

#[cfg(test)]
#[path = "../../tests/node_free_tests.rs"]
mod file_length_tests;

mod repository_access;
pub use repository_access::gen_font_table;
use repository_access::repo_root;
pub use repository_access::verify_file_length;

mod verify_no_node;
pub use verify_no_node::verify_no_node;

#[cfg(test)]
use repository_access::{civil_ymd, is_test_file, verify_file_length_in, walk_rust_sources};
