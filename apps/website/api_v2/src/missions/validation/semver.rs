//! SemVer 2.0.0 parsing for `mission_versions.semver`, plus the shared ASCII-digit component
//! reader the clock predicate also uses.

/// SemVer 2.0.0 core plus optional pre-release / build metadata.
///
/// `mission_versions.semver` is a plain `text` unique key; without a parse, `' 0.1.0 '` and
/// `'0.1.0'` are distinct btree values, so the duplicate-version 409 never fires and the padded
/// row becomes `current_version_id`. Trim-only would still admit `"1"`, `"1.2"`, `"banana"`.
///
/// This REJECTS; it does not trim or canonicalise. Live census before enforce (dev DB
/// `tbd_reforger`, 2026-07-27): **133** `mission_versions` rows, **0** fail this predicate
/// (nine distinct values, all `MAJOR.MINOR.PATCH` with no leading zeros / padding).
pub(crate) fn valid_semver(s: &str) -> bool {
    let core = match s.split_once('+') {
        Some((core, build)) => {
            if !semver_build(build) {
                return false;
            }
            core
        }
        None => s,
    };
    let (core, pre) = match core.split_once('-') {
        Some((core, pre)) => (core, Some(pre)),
        None => (core, None),
    };
    if let Some(pre) = pre
        && !semver_prerelease(pre)
    {
        return false;
    }
    let mut parts = core.split('.');
    let Some(maj) = parts.next() else {
        return false;
    };
    let Some(min) = parts.next() else {
        return false;
    };
    let Some(pat) = parts.next() else {
        return false;
    };
    if parts.next().is_some() {
        return false;
    }
    semver_numeric_id(maj) && semver_numeric_id(min) && semver_numeric_id(pat)
}

/// Numeric identifier: `0` or `[1-9][0-9]*` — no leading zeros (SemVer 2.0 §2).
fn semver_numeric_id(s: &str) -> bool {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    s == "0" || !s.starts_with('0')
}

/// Pre-release: dot-separated identifiers, each numeric (`0` / `[1-9][0-9]*`) or
/// alphanumeric-with-hyphen containing at least one non-digit (SemVer 2.0 §9).
fn semver_prerelease(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    s.split('.').all(|id| {
        if id.is_empty() {
            return false;
        }
        if id.bytes().all(|b| b.is_ascii_digit()) {
            return semver_numeric_id(id);
        }
        id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    })
}

/// Build metadata: dot-separated `[0-9a-zA-Z-]+` identifiers (SemVer 2.0 §10). Leading zeros OK.
fn semver_build(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    s.split('.')
        .all(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'))
}

#[cfg(test)]
#[path = "tests/semver.rs"]
mod tests;
