//! Source length, font-table generation, and Node tooling gates.
//!
//! The length gate walks `.rs` and Enfusion `.c` sources, counts raw lines, and fails on any
//! file above its kind-specific ceiling (production 500, test 1000).
//! The Node gate refuses tracked scripts and invocations outside the Enfusion tooling floor.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use verification_core::scan;
use verification_core::{Kind, NotRun, Verdict};

/* ─────────────────────────── verify file-length (SIZE-3) ─────────────────────────── */

const SIZE_3_PRODUCTION_MAX_LINES: usize = 500;
const SIZE_3_TEST_MAX_LINES: usize = 1000;

/// Directories the SIZE gate must examine. A missing pin is [`NotRun::TargetMissing`], never
/// an empty pass. Extra `apps/website/<name>/src` trees are picked up if they exist.
const FILE_LENGTH_PINS: &[&str] = &[
    "tools_v2/xtask",
    "tools_v2/verification-core",
    "tools_v2/ticket-engine",
    "tools_v2/developer-tools",
    "apps/ticketboard/src",
    "apps/website/api_v2/src",
    "apps/website/frontend/src",
    "apps/fleet_host_agent/src",
    "apps/fleet_host_agent/tests",
    "apps/mod/tbd-framework/Scripts",
];

/// Enfusion script roots of the three shipped addons, the only `apps/mod` trees the length gate
/// may pin. `apps/mod/crf_framework` and `apps/mod/vanilla_reference` are gitignored upstream
/// references and never enter [`FILE_LENGTH_PINS`]. Each root joins the pins once its addon's
/// scripts sit at or under the ceilings.
const MOD_SCRIPT_ROOTS: &[&str] = &[
    "apps/mod/tbd-framework/Scripts",
    "apps/mod/tbd-export/Scripts",
    "apps/mod/tbd-emcp/Scripts",
];

/// File extensions the length gate counts: Rust sources and Enfusion scripts.
const LENGTH_GATED_EXTENSIONS: &[&str] = &["rs", "c"];

const _: () = assert!(
    repository_access::mod_pins_are_script_roots(FILE_LENGTH_PINS, MOD_SCRIPT_ROOTS),
    "an apps/mod pin in FILE_LENGTH_PINS must be one of MOD_SCRIPT_ROOTS"
);

/* ─────────────────────────── gen font-table ─────────────────────────── */

/* ─────────────────────────── verify no-node ─────────────────────────── */

/// Individual files this gate scans, over and above the [`SCAN_DIRS`] walk. A declared path that
/// is MISSING is a FAILURE, never a silent narrowing — see [`verify_no_node()`].
///
/// Empty today. The fail-closed rule is what keeps it honest: removing a scanned file means
/// removing its entry here deliberately, rather than letting a deletion quietly shrink the
/// gate's reach.
const SCAN_FILES: &[&str] = &[];

/// Directory roots walked for `.sh` / `.yml` / `.yaml`. Same rule: declared-but-absent FAILS.
const SCAN_DIRS: &[&str] = &[".github"];

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
use repository_access::{
    is_test_file, length_scan_summary, mod_pins_are_script_roots, verify_file_length_in,
    walk_length_gated_sources,
};
