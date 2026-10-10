//! Small readers the reclaim sweep shares.
//!
//! **Role:** extracts the slice from an ad-hoc build folder name (`tbd-target-T-<n>…` or
//! `tbd-target-<slug>`), measures a folder's age in whole days and reads the free space of the
//! disk holding the root.
//!
//! **Position:** called by the sibling `reclaim_command.rs`.
//!
//! **Signals & state:** none; reads folder metadata and spawns `df`.
//!
//! **Invariants:** only an uppercase `T-` id followed by digits, or a name that is exactly a ticket
//! slug, names a slice, so a folder whose name holds any other character (`_`, an uppercase
//! letter) is never attributed to one; ages use integer division of whole seconds.

use super::*;

/// `^tbd-target-(T-[0-9]+)(-.*)?$` — uppercase-`T`-only and requires the dash — names the legacy
/// ticket; otherwise `tbd-target-<slug>` names that slug when the whole rest is a ticket
/// reference.
pub(super) fn adhoc_token(base: &str) -> Option<String> {
    let rest = base.strip_prefix("tbd-target-")?;
    if let Some(legacy) = rest.strip_prefix("T-") {
        let digits: String = legacy.chars().take_while(char::is_ascii_digit).collect();
        if !digits.is_empty() {
            let tail = &legacy[digits.len()..];
            if !tail.is_empty() && !tail.starts_with('-') {
                return None;
            }
            return Some(format!("T-{digits}"));
        }
    }
    ticket_manager_client::is_ticket_reference(rest).then(|| rest.to_string())
}

/// `(( $(date +%s) - $(stat -c %Y "$d") ) / 86400)` — integer division, as the bash did it.
pub(super) fn dir_age_days(d: &Path) -> i64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let mtime = std::fs::metadata(d)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    (now - mtime) / 86400
}

/// `df -h "$ROOT" | tail -1 | awk '{print $4}'`.
pub(super) fn df_avail(root: &Path) -> String {
    let out = process_runner::Run::new("df").arg("-h").arg(root).output();
    let Ok(o) = out else { return String::new() };
    let body = o.stdout;
    body.lines()
        .next_back()
        .and_then(|l| l.split_whitespace().nth(3))
        .unwrap_or("")
        .to_string()
}
