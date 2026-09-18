use super::*;

/// `^tbd-target-(T-[0-9]+)(-.*)?$` — note this one is uppercase-`T`-only and requires the dash.
pub(super) fn adhoc_token(base: &str) -> Option<String> {
    let rest = base.strip_prefix("tbd-target-T-")?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    let tail = &rest[digits.len()..];
    if !tail.is_empty() && !tail.starts_with('-') {
        return None;
    }
    Some(format!("T-{digits}"))
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
    let out = std::process::Command::new("df")
        .arg("-h")
        .arg(root)
        .output();
    let Ok(o) = out else { return String::new() };
    let body = String::from_utf8_lossy(&o.stdout).into_owned();
    body.lines()
        .next_back()
        .and_then(|l| l.split_whitespace().nth(3))
        .unwrap_or("")
        .to_string()
}
