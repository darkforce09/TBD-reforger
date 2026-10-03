//! The asset-GUID step of the leak gate: an asset GUID a reference lane declares, reused in our
//! addons, that no vanilla `.pak` holds.
//!
//! **Role:** Reads our addons once into a GUID → references map, reads each lane's declared GUIDs
//! once, asks [`super::vanilla_pak_probe`] once about the union of the shared GUIDs, and prints one
//! block per lane with every leaked GUID and the `path:line` of each of its references.
//! **Position:** Called by [`super::verify_crf_leak::run`] after the identifier step, with lanes
//! that [`super::verify_crf_leak::require_lane`] admitted.
//! **Signals & state:** None; pure reads of the lane folders, our addons and the vanilla paks.
//! **Invariants:** A GUID is a leak exactly when a lane declares it, our addons reference it and no
//! vanilla `.pak` holds its sixteen hex digits. Every read failure is `NotRun`, never "no GUIDs".

use std::collections::BTreeMap;

use super::vanilla_pak_probe::present_in_paks;
use super::verify_crf_leak::{broken_pattern, grep_visible, numbered, read};
use super::*;

/// Every GUID our addons reference, each with the `path:line` of every line that references it,
/// in path order, then line order.
pub(super) type GuidReferences = BTreeMap<String, Vec<String>>;

/// The asset-GUID step over both lanes. `Ok(true)` when any lane leaks a GUID.
pub(super) fn check_guid_leaks(log: &mut Log, lanes: &Lanes) -> Result<bool, NotRun> {
    let guid = Regex::new(GUID_RE).map_err(|e| broken_pattern(GUID_RE, e))?;
    let ours = guid_references(
        &guid,
        &[lanes.mod_dir.as_path(), lanes.export_dir.as_path()],
    )?;

    let mut declared: Vec<(&str, BTreeSet<String>)> = Vec::new();
    for (label, oracle) in lanes.references() {
        let dirs = asset_dirs(oracle)?;
        let roots: Vec<&Path> = dirs.iter().map(PathBuf::as_path).collect();
        declared.push((label, guids_under(&guid, &roots)?));
    }

    let shared: BTreeSet<String> = declared
        .iter()
        .flat_map(|(_, guids)| guids.iter())
        .filter(|g| ours.contains_key(*g))
        .map(|g| bare(g).to_string())
        .collect();
    let in_vanilla = present_in_paks(&lanes.vanilla, &shared)?;

    let mut fail = false;
    for (label, guids) in &declared {
        fail |= report_lane(log, label, guids, &ours, &in_vanilla);
    }
    Ok(fail)
}

/// One lane's block of the transcript. `true` when the lane leaks a GUID.
fn report_lane(
    log: &mut Log,
    label: &str,
    declared: &BTreeSet<String>,
    ours: &GuidReferences,
    in_vanilla: &BTreeSet<String>,
) -> bool {
    log.say(format!(
        "==> {label} layout/prefab GUIDs reused in tbd-framework or tbd-export"
    ));
    if declared.is_empty() || ours.is_empty() {
        log.say("  OK (nothing to compare)");
        return false;
    }
    // `BTreeSet` order is byte order, which for `{16 uppercase hex}` strings is sorted order.
    let leaks: Vec<&String> = declared
        .iter()
        .filter(|g| ours.contains_key(*g) && !in_vanilla.contains(bare(g)))
        .collect();
    if leaks.is_empty() {
        log.say("  OK (shared GUIDs are all vanilla engine facts)");
        return false;
    }
    log.say(format!(
        "FAIL: {label}-only asset GUIDs reused (not present in vanilla):"
    ));
    for g in leaks {
        log.say(format!("  {g}"));
        for at in &ours[g] {
            log.say(format!("    {at}"));
        }
    }
    true
}

/// The sixteen hex digits of a braced GUID, the form a `.pak` is searched for.
fn bare(guid: &str) -> &str {
    guid.trim_start_matches('{').trim_end_matches('}')
}

/// The `UI/` and `Prefabs/` folders of a lane, at depth 0 to 2, following symlinks at every level.
///
/// Depth 0 is the lane itself; depth 2 bounds a symlink cycle.
pub(super) fn asset_dirs(oracle: &Path) -> Result<Vec<PathBuf>, NotRun> {
    let mut out = Vec::new();
    if is_asset_dir(oracle) {
        out.push(oracle.to_path_buf());
    }
    for one in children(oracle)? {
        if !one.is_dir() {
            continue;
        }
        if is_asset_dir(&one) {
            out.push(one.clone());
        }
        for two in children(&one)? {
            if is_asset_dir(&two) {
                out.push(two);
            }
        }
    }
    out.sort();
    Ok(out)
}

/// A folder named `UI` or `Prefabs` (through a symlink too).
fn is_asset_dir(path: &Path) -> bool {
    let named = path
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| ASSET_DIR_NAMES.contains(&n));
    named && path.is_dir()
}

/// One directory level. A folder that cannot be listed is `NotRun`: a lane that could not be read
/// must not report "nothing to compare".
fn children(dir: &Path) -> Result<Vec<PathBuf>, NotRun> {
    let bad = |source: std::io::Error| NotRun::Unreadable {
        path: dir.to_path_buf(),
        source,
    };
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(bad)? {
        out.push(entry.map_err(bad)?.path());
    }
    Ok(out)
}

/// Every distinct GUID in the files under `roots`.
///
/// Whole-text matching equals line-by-line matching here: the pattern holds no `.` and no
/// newline, so a match never spans a line break.
pub(super) fn guids_under(guid: &Regex, roots: &[&Path]) -> Result<BTreeSet<String>, NotRun> {
    let mut out = BTreeSet::new();
    for file in scan::walk_files(roots, |_| true)? {
        let bytes = read(&file)?;
        for m in guid.find_iter(&String::from_utf8_lossy(grep_visible(&bytes))) {
            out.insert(m.as_str().to_string());
        }
    }
    Ok(out)
}

/// Every GUID in the files under `roots`, with the `path:line` of each line that references it
/// (a line naming a GUID twice is listed once).
pub(super) fn guid_references(guid: &Regex, roots: &[&Path]) -> Result<GuidReferences, NotRun> {
    let mut out = GuidReferences::new();
    for file in scan::walk_files(roots, |_| true)? {
        let bytes = read(&file)?;
        for (line_no, line) in numbered(grep_visible(&bytes)) {
            for m in guid.find_iter(&line) {
                let at = format!("{}:{line_no}", file.display());
                let refs = out.entry(m.as_str().to_string()).or_default();
                if refs.last() != Some(&at) {
                    refs.push(at);
                }
            }
        }
    }
    Ok(out)
}
