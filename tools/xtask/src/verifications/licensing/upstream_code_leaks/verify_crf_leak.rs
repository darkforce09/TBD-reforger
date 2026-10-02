//! The gate body of `cargo xtask verify no-crf-leak`: lane admission, the identifier step, and the
//! grep-compatible file reading both steps share.
//!
//! **Role:** Admits both reference lanes, runs the identifier step for each lane prefix, hands the
//! asset-GUID step to [`super::asset_guid_reuse`], and prints the epilogue with exit 1 or the PASS
//! line with exit 0.
//! **Position:** Called through [`verify_crf_leak`] by the `verify` dispatch and the mod wave gate;
//! the tests call [`run`] with fixture [`Lanes`].
//! **Signals & state:** None; every printed line goes through [`Log`].
//! **Invariants:** A lane that cannot be compared, an addon tree that cannot be walked, or a file
//! that cannot be read is exit 2 before any verdict; a pattern constant that does not compile is
//! exit 2, never "no hits".

use super::asset_guid_reuse::{asset_dirs, check_guid_leaks};
use super::*;

/// Runs the gate over the checkout at `repo_root` and returns its exit code (0 clean, 1 findings,
/// 2 did not run), printing the transcript as it goes.
pub fn verify_crf_leak(repo_root: &Path) -> Result<u8> {
    let mut log = Log {
        lines: Vec::new(),
        echo: true,
    };
    Ok(run(&Lanes::from_env(repo_root), &mut log))
}

/// The gate body: the identifier step per prefix, the asset-GUID step over both lanes, then the
/// epilogue-and-exit-1 or the PASS line. A lane that is absent, unreadable or holds nothing to
/// compare is exit 2 before any step runs.
pub(super) fn run(lanes: &Lanes, log: &mut Log) -> u8 {
    for (label, oracle) in lanes.references() {
        if let Err(cause) = require_lane(oracle) {
            return refuse_lane(log, label, oracle, cause);
        }
    }
    let mut fail = false;

    for (label, prefix) in IDENT_LANES {
        let ours = [lanes.mod_dir.as_path(), lanes.export_dir.as_path()];
        match check_identifier_leak(log, &ours, label, prefix) {
            Ok(hit) => fail |= hit,
            Err(cause) => return refuse(log, cause),
        }
    }
    match check_guid_leaks(log, lanes) {
        Ok(hit) => fail |= hit,
        Err(cause) => return refuse(log, cause),
    }

    if fail {
        log.say("");
        log.say("Oracles are reference-only. Design-mirror them; never copy them.");
        log.say("  CRF              — Arma Public License; read, cite, do not vendor.");
        log.say(EPILOGUE_PS);
        log.say(format!(
            "See {} §2 and {} §Oracle lanes.",
            crate::core::repository_layout::documentation::MOD_DESIGN,
            crate::core::repository_layout::documentation::SLICE_WORKFLOW_RUNBOOK
        ));
        return 1;
    }
    log.say("no-oracle-leak: PASS (CRF + PlayableSelector)");
    0
}

/// Exit **2**: "the tree is dirty" (1) and "the tree was never read" (2) are different operator
/// actions. The wave gate treats any nonzero code as FAIL.
pub(super) fn refuse(log: &mut Log, cause: NotRun) -> u8 {
    let msg = "no-oracle-leak could not examine the trees it was pointed at";
    log.say(Verdict::did_not_run(msg, Kind::Ban, cause).to_string());
    2
}

/// A lane the GUID step can compare: a folder holding at least one `UI/` or `Prefabs/` folder.
pub(super) fn require_lane(oracle: &Path) -> Result<(), NotRun> {
    // `is_dir` follows symlinks, as a slice worktree's lanes are.
    if !oracle.is_dir() {
        return Err(NotRun::TargetMissing(oracle.to_path_buf()));
    }
    if asset_dirs(oracle)?.is_empty() {
        let source = std::io::Error::other("the lane holds no UI/ or Prefabs/ folder");
        return Err(NotRun::Unreadable {
            path: oracle.to_path_buf(),
            source,
        });
    }
    Ok(())
}

/// Exit **2** for a lane the gate cannot compare against, naming the lane and how to fill it.
pub(super) fn refuse_lane(log: &mut Log, label: &str, oracle: &Path, cause: NotRun) -> u8 {
    let msg = format!("no-oracle-leak could not examine the {label} lane");
    log.say(Verdict::did_not_run(msg, Kind::Ban, cause).to_string());
    log.say(format!(
        "      Fill {} as {REFERENCES_DIR}/README.md describes, then run the gate again.",
        oracle.display()
    ));
    2
}

/// The identifier step for one lane prefix over `roots`. `Ok(true)` when it found a leak.
pub(super) fn check_identifier_leak(
    log: &mut Log,
    roots: &[&Path],
    label: &str,
    prefix: &str,
) -> Result<bool, NotRun> {
    log.say(format!(
        "==> {prefix} identifiers in tbd-framework + tbd-export code ({label})"
    ));
    let ident = pattern(&identifier_pattern(prefix))?;
    let comment = pattern(COMMENT_RE)?;

    let mut hits: Vec<String> = Vec::new();
    for file in scan::walk_files(roots, outside_excluded_dir)? {
        let bytes = read(&file)?;
        for (line_no, line) in numbered(grep_visible(&bytes)) {
            if !ident.is_match(&line) {
                continue;
            }
            // The comment filter is anchored on the rendered `path:line:text`, so render first.
            let rendered = format!("{}:{line_no}:{line}", file.display());
            if !comment.is_match(&rendered) {
                hits.push(rendered);
            }
        }
    }

    if hits.is_empty() {
        log.say("  OK (none)");
        return Ok(false);
    }
    log.say(format!(
        "FAIL: {prefix} symbols found in our mod trees (tbd-framework, tbd-export):"
    ));
    for hit in hits.iter().take(HEAD) {
        log.say(hit.clone());
    }
    Ok(true)
}

/// False for a file under an `EnfusionMCP` folder, which the identifier step skips.
pub(super) fn outside_excluded_dir(path: &Path) -> bool {
    !path.components().any(|c| c.as_os_str() == EXCLUDE_DIR)
}

/// The bytes of `path`, or `NotRun::Unreadable` naming it.
pub(super) fn read(path: &Path) -> Result<Vec<u8>, NotRun> {
    std::fs::read(path).map_err(|source| NotRun::Unreadable {
        path: path.to_path_buf(),
        source,
    })
}

/// The bytes GNU grep produces output from.
///
/// A NUL inside the first read buffer ([`GREP_BUF`]) flags the whole file binary and nothing of it
/// is visible; a NUL that arrives later leaves the bytes before it visible and hides the rest.
pub(super) fn grep_visible(bytes: &[u8]) -> &[u8] {
    if bytes[..bytes.len().min(GREP_BUF)].contains(&0) {
        return &[];
    }
    match bytes.iter().position(|b| *b == 0) {
        Some(cut) => &bytes[..cut],
        None => bytes,
    }
}

/// `grep -n`'s numbering: 1-based, split on `\n` only.
///
/// Not [`scan::matching_lines`], which uses `str::lines` and therefore strips a trailing `\r`;
/// grep keeps it, and `tbd-framework` holds CRLF files written by the Workbench bridge.
pub(super) fn numbered(bytes: &[u8]) -> Vec<(usize, String)> {
    let mut lines: Vec<&[u8]> = bytes.split(|b| *b == b'\n').collect();
    // `split` yields a trailing empty piece for a file that ends in a newline; grep does not
    // count that as a line. A file NOT ending in one still has its last partial line counted.
    if lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    let n = |(i, l): (usize, &&[u8])| (i + 1, String::from_utf8_lossy(l).into_owned());
    lines.iter().enumerate().map(n).collect()
}

/// A pattern that will not compile is a bug in this module; it must not read as "no hits".
pub(super) fn pattern(src: &str) -> Result<Pattern, NotRun> {
    Pattern::regex(src).map_err(|e| broken_pattern(src, e))
}

/// The `NotRun` for a pattern that does not compile.
pub(super) fn broken_pattern(src: &str, e: regex::Error) -> NotRun {
    NotRun::ToolError {
        tool: "regex".into(),
        status: 1,
        stderr: format!("{src}: {e}"),
    }
}
