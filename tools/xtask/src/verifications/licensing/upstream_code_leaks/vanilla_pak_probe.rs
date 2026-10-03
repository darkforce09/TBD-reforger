//! The vanilla probe of the leak gate: which of a set of bare GUIDs any vanilla `.pak` holds.
//!
//! **Role:** Answers, for every wanted GUID at once, what `grep -qla <guid> <folder>/*.pak` answers
//! for one: whether its sixteen hex digits occur anywhere in any pak.
//! **Position:** Called once per gate run by [`super::asset_guid_reuse::check_guid_leaks`] with the
//! union of both lanes' shared GUIDs; the paks are the local game install's (~25 GB).
//! **Signals & state:** One scoped thread per `.pak`, each waiting on one `grep` child that streams
//! the pak once; nothing survives the call.
//! **Invariants:**
//! - Every occurrence of a GUID lies inside a maximal run of uppercase hex digits, so the paks are
//!   reduced to their runs of at least sixteen such digits ([`HEX_RUN`], C locale, binary read as
//!   text) and a GUID is found when it is a substring of any run. This is exact, overlapping
//!   GUIDs included, and reads each pak once whatever the number of GUIDs.
//! - A pak that `grep` cannot read is `NotRun` naming it, never "not in vanilla"; an absent `grep`
//!   is `NotRun` too.
//! - A folder with no `.pak` (no game install) answers "none found".
//! - Paks are the folder's entries named `*.pak` that do not start with `.`, as the shell glob
//!   `<folder>/*.pak` lists them.

use process_runner::Run;

use super::*;

/// The digits of a bare GUID.
const GUID_DIGITS: usize = 16;
/// A maximal run of uppercase hex digits long enough to hold a bare GUID.
const HEX_RUN: &str = "[0-9A-F]{16,}";

/// The GUIDs of `wanted` (bare, sixteen uppercase hex digits) that any `.pak` in `dir` holds.
pub(super) fn present_in_paks(
    dir: &Path,
    wanted: &BTreeSet<String>,
) -> Result<BTreeSet<String>, NotRun> {
    let paks = paks(dir);
    if paks.is_empty() || wanted.is_empty() {
        return Ok(BTreeSet::new());
    }
    let runs: Vec<Result<String, NotRun>> = std::thread::scope(|scope| {
        let workers: Vec<_> = paks
            .iter()
            .map(|pak| scope.spawn(move || hex_runs(pak)))
            .collect();
        workers
            .into_iter()
            .map(|w| {
                w.join()
                    .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
            })
            .collect()
    });

    let mut found = BTreeSet::new();
    for pak_runs in runs {
        for run in pak_runs?.lines() {
            for window in run.as_bytes().windows(GUID_DIGITS) {
                if let Some(hit) = std::str::from_utf8(window).ok().and_then(|w| wanted.get(w)) {
                    found.insert(hit.clone());
                }
            }
        }
    }
    Ok(found)
}

/// Every maximal run of at least [`GUID_DIGITS`] uppercase hex digits in `pak`, one per line.
fn hex_runs(pak: &Path) -> Result<String, NotRun> {
    let out = Run::new("grep")
        .env("LC_ALL", "C")
        .args([
            "--binary-files=text",
            "--only-matching",
            "--extended-regexp",
        ])
        .arg(HEX_RUN)
        .arg(pak)
        .output()?;
    match out.code {
        0 => Ok(out.stdout),
        1 => Ok(String::new()),
        status => Err(NotRun::ToolError {
            tool: format!("grep {}", pak.display()),
            status,
            stderr: out.stderr,
        }),
    }
}

/// The entries of `dir` the shell glob `"$dir"/*.pak` lists, sorted. No folder, no paks.
pub(super) fn paks(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let glob = |n: &str| n.ends_with(".pak") && !n.starts_with('.');
    let named = |p: &PathBuf| p.file_name().and_then(|n| n.to_str()).is_some_and(glob);
    let mut out: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(named).collect();
    out.sort();
    out
}
