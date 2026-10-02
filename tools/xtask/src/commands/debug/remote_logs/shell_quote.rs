use super::*;

pub(super) fn shell_quote(s: &str) -> String {
    // bash printf '%q' for the FIND_LOG payload — enough for our fixed string (no single quotes).
    if s.bytes().all(|b| {
        b.is_ascii_alphanumeric()
            || matches!(
                b,
                b'/' | b'.' | b'_' | b'-' | b'=' | b':' | b'@' | b'+' | b','
            )
    }) {
        return s.to_string();
    }
    let mut out = String::from("'");
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

pub(super) fn tempfile_dir(prefix: &str) -> Result<PathBuf> {
    let mut p = std::env::temp_dir();
    p.push(format!("{prefix}.{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p)?;
    Ok(p)
}

pub(super) fn write_log(path: &Path, lines: &[&str]) {
    let mut f = fs::File::create(path).expect("create log");
    for line in lines {
        writeln!(f, "{line}").expect("write log");
    }
}

pub(super) fn append_line(path: &Path, line: &str) {
    let mut f = fs::OpenOptions::new()
        .append(true)
        .open(path)
        .expect("append log");
    writeln!(f, "{line}").expect("append");
}
