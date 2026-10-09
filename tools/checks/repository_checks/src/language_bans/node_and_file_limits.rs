//! Source length and Node tooling gates.
//!
//! **Role:** the file-length gate prints the size advice of [`repository_laws::file_length`] —
//! every production `.rs` and Enfusion `.c` source under the law roots over 500 lines, as a
//! warning — and exits 0 once it has scanned. The Node gate refuses tracked scripts and
//! invocations outside the Enfusion tooling floor.
//! **Position:** `cargo xtask verify file-length` and `verify no-node`.
//! **Signals & state:** none; each entry reads the checkout and prints.
//! **Invariants:** the length roots, the ceiling and the test-file rule live in
//! `repository_laws` alone; the file-length gate never fails on a long file, but a scan that could
//! not run or read nothing is never a pass.

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

mod repository_access;
pub use repository_access::verify_file_length;

mod verify_no_node;
pub use verify_no_node::verify_no_node;
