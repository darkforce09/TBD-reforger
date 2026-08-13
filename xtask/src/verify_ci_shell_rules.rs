//! Allowlist + forbidden-token rules for one already-parsed `run:` script.
//!
//! Split from [`crate::verify_ci_shell`] at the parse/rules seam (SIZE-3). This module never
//! sees YAML — the caller hands it a string that serde already produced. Checking the *script*
//! with ordinary string tests is not "regex YAML"; regex YAML is the defect class this gate
//! exists to kill (counting `run:` with a grep that also hits `defaults.run`).

/// `uses:` values that may appear on a step. Anything else is a composite-action bypass.
///
/// `actions/setup-go@v6` is deliberately absent: T-901 ports the editorconfig job to
/// `cargo xtask ci verify-editorconfig`, so Go-on-the-runner is no longer a CI dependency.
pub const ALLOWED_USES: &[&str] = &[
    "actions/checkout@v7",
    "dtolnay/rust-toolchain@stable",
    "Swatinem/rust-cache@v2",
    "taiki-e/install-action@v2",
    "actions/upload-artifact@v4",
];

/// Join backslash-continued physical lines, drop blanks and comment-only lines.
pub fn logical_lines(script: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    for raw in script.lines() {
        let line = raw.trim_end();
        if let Some(without) = line.strip_suffix('\\') {
            buf.push_str(without.trim_end());
            buf.push(' ');
            continue;
        }
        buf.push_str(line);
        push_logical(&mut out, &buf);
        buf.clear();
    }
    if !buf.is_empty() {
        push_logical(&mut out, &buf);
    }
    out
}

fn push_logical(out: &mut Vec<String>, buf: &str) {
    let t = buf.trim();
    if t.is_empty() || t.starts_with('#') {
        return;
    }
    out.push(t.to_string());
}

/// Why this logical line is illegal, or `None` if it is `cargo xtask` / the pre-cargo allowlist.
pub fn line_reason(line: &str) -> Option<String> {
    if let Some(why) = forbidden_reason(line) {
        return Some(why);
    }
    if is_allowlisted(line) {
        return None;
    }
    Some("not `cargo xtask` and not on the pre-cargo allowlist".into())
}

pub fn uses_reason(uses: &str) -> Option<String> {
    let t = uses.trim();
    if ALLOWED_USES.contains(&t) {
        None
    } else {
        Some(format!("uses: `{t}` is not on the T-901 pin list"))
    }
}

pub fn shell_reason(shell: &str) -> Option<String> {
    let first = shell
        .split_whitespace()
        .next()
        .unwrap_or("")
        .rsplit('/')
        .next()
        .unwrap_or("");
    let base = first.split(['{', '(']).next().unwrap_or(first);
    match base {
        "bash" | "sh" | "python" | "python2" | "python3" => {
            Some(format!("shell: `{shell}` names bash/sh/python"))
        }
        _ => None,
    }
}

fn forbidden_reason(line: &str) -> Option<String> {
    let t = line.trim();
    if t.contains("<<") {
        return Some("heredoc".into());
    }
    if t.contains("$(") {
        return Some("`$(` command substitution".into());
    }
    if t.contains('`') {
        return Some("backticks".into());
    }
    if t.contains("&&") {
        return Some("`&&`".into());
    }
    if t.contains('&') {
        // A lone `&` backgrounds the preceding command (`cmd & echo pwned`). Checked AFTER
        // `&&` so `cargo xtask foo && true` still reports `&&`, not `&`.
        return Some("`&`".into());
    }
    if t.contains("||") {
        return Some("`||`".into());
    }
    if t.contains('|') {
        return Some("pipe".into());
    }
    if t.contains(';') {
        return Some("`;`".into());
    }
    if t.contains('>') || t.contains('<') {
        return Some("redirection".into());
    }
    if has_set_dash(t) {
        return Some("`set -`".into());
    }
    let word = first_word(t);
    if matches!(word, "if" | "for" | "while" | "case") {
        return Some(format!("control flow (`{word}`)"));
    }
    None
}

fn has_set_dash(s: &str) -> bool {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 4 <= bytes.len() {
        if bytes[i..].starts_with(b"set") {
            let before_ok = i == 0 || is_word_break(bytes[i - 1]);
            let rest = &bytes[i + 3..];
            let mut j = 0;
            while j < rest.len() && rest[j].is_ascii_whitespace() {
                j += 1;
            }
            if before_ok && j < rest.len() && rest[j] == b'-' {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn is_word_break(b: u8) -> bool {
    b.is_ascii_whitespace() || matches!(b, b';' | b'|' | b'&' | b'(')
}

fn first_word(s: &str) -> &str {
    s.split(|c: char| c.is_whitespace() || c == '[' || c == '(')
        .next()
        .unwrap_or("")
}

fn is_allowlisted(line: &str) -> bool {
    let toks: Vec<&str> = line.split_whitespace().collect();
    if toks.len() >= 2 && toks[0] == "cargo" && toks[1] == "xtask" {
        return true;
    }
    if toks.len() >= 3
        && toks[0] == "rustup"
        && matches!(toks[1], "target" | "component")
        && toks[2] == "add"
    {
        return true;
    }
    if toks.len() >= 2 && toks[0] == "cargo" && toks[1] == "install" {
        return toks.iter().any(|t| *t == "--version" || *t == "--vers");
    }
    if toks.len() >= 3 && toks[0] == "git" && toks[1] == "lfs" && toks[2] == "pull" {
        return true;
    }
    if toks.len() >= 2 && matches!(toks[0], "docker" | "podman") && toks[1] == "compose" {
        return true;
    }
    toks.first().is_some_and(|t| *t == "docker-compose")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_xtask_is_allowlisted() {
        assert_eq!(line_reason("cargo xtask verify no-shell"), None);
    }

    #[test]
    fn git_lfs_pull_is_allowlisted() {
        assert_eq!(
            line_reason("git lfs pull --include packages/map-assets/everon/**"),
            None
        );
    }

    #[test]
    fn unpinned_cargo_install_is_not_allowlisted() {
        let r = line_reason("cargo install editorconfig-checker").unwrap();
        assert!(r.contains("allowlist"), "{r}");
    }

    #[test]
    fn pipe_is_forbidden_even_on_xtask() {
        assert_eq!(
            line_reason("cargo xtask verify no-shell | tee log").as_deref(),
            Some("pipe")
        );
    }

    #[test]
    fn ampersand_background_is_red() {
        assert_eq!(
            line_reason("cargo xtask verify no-shell & echo pwned").as_deref(),
            Some("`&`")
        );
    }

    #[test]
    fn double_ampersand_still_reports_and_and() {
        assert_eq!(
            line_reason("cargo xtask verify no-shell && true").as_deref(),
            Some("`&&`")
        );
    }
}
