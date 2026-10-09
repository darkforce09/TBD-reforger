//! Source length and Node tooling gates.
//!
//! **Role:** the file-length gate prints the size law of
//! [`repository_laws::file_length`] — every `.rs` and Enfusion `.c` source
//! under the law roots at or under its ceiling (production 500, test 1000) — and exits with its
//! verdict. The Node gate refuses tracked scripts and invocations outside the Enfusion tooling
//! floor.
//! **Position:** `cargo xtask verify file-length` and `verify no-node`.
//! **Signals & state:** none; each entry reads the checkout and prints.
//! **Invariants:** the length roots, ceilings and test-file rule live in `verification_core`
//! alone, so this gate and the `engineering_laws` test binary judge the same tree the same way.

use std::path::{Path, PathBuf};

use crate::Result;
use verification_core::{Kind, NotRun, Verdict};

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
#[path = "tests/node_free_tests.rs"]
mod file_length_tests;

mod repository_access;
pub use repository_access::verify_file_length;

mod verify_no_node;
#[cfg(test)]
use verify_no_node::refused_node_scripts;
pub use verify_no_node::verify_no_node;

// The file-length tests reach the library's roots, ceilings and walk under the gate's own rule
// names (SIZE-3 is the rule id the gate prints).
#[cfg(test)]
use repository_access::verify_file_length_in;
#[cfg(test)]
use repository_laws::file_length::{
    PRODUCTION_MAX_LINES as SIZE_3_PRODUCTION_MAX_LINES, TEST_MAX_LINES as SIZE_3_TEST_MAX_LINES,
    length_scan_summary,
};
#[cfg(test)]
use repository_laws::source_roots::{
    MOD_SCRIPT_ROOTS, PINNED_SCRIPT_ROOTS, is_test_file, mod_pins_are_script_roots,
    walk_length_gated_sources,
};
