//! Structural and documentation audit of every production file under `src/v2`.

use std::fs;
use std::path::{Path, PathBuf};

/// Longest a v2 production file is allowed to be.
const MAX_LINES: usize = 500;

/// Every `.rs` file under `dir` that is not inside a `tests/` subtree, sorted by path.
fn production_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(dir, &mut out);
    out.sort();
    out
}

/// Depth-first collector behind [`production_files`].
fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "tests") {
                continue;
            }
            walk(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// True when `line` is an attribute line, which may sit between a doc block and its item.
fn is_attribute(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("#[") || t.starts_with("#!")
}

/// True when `line` documents the item that follows it.
fn is_doc(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("///") || t.starts_with("#[doc")
}

/// True when `line` declares an item that the documentation rule covers: a visible
/// `fn`/`struct`/`enum`/`type`/`const`/`static`/`trait`, or a Leptos component.
fn needs_doc(line: &str) -> bool {
    let t = line.trim_start();
    if t.starts_with("#[component]") {
        return true;
    }
    let Some(rest) = strip_visibility(t) else {
        return false;
    };
    for kw in ["fn", "struct", "enum", "type", "const", "static", "trait"] {
        if let Some(tail) = rest.strip_prefix(kw) {
            if tail.starts_with(|c: char| !c.is_alphanumeric() && c != '_') {
                return true;
            }
        }
    }
    false
}

/// The text after a leading `pub`, `pub(crate)`, `pub(super)` … and its whitespace, if present.
fn strip_visibility(t: &str) -> Option<&str> {
    let rest = t.strip_prefix("pub")?;
    let rest = match rest.strip_prefix('(') {
        Some(inner) => {
            let close = inner.find(')')?;
            if !inner[..close].chars().all(|c| c.is_ascii_lowercase()) {
                return None;
            }
            &inner[close + 1..]
        }
        None => rest,
    };
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    Some(rest.trim_start())
}

/// True when `line` declares an inline module body (`mod foo {`), with any visibility.
fn is_inline_mod(line: &str) -> bool {
    let t = line.trim_start();
    let t = strip_visibility(t).unwrap_or(t);
    let Some(rest) = t.strip_prefix("mod ") else {
        return false;
    };
    let rest = rest.trim_start();
    let name_end = rest
        .find(|c: char| !c.is_alphanumeric() && c != '_')
        .unwrap_or(rest.len());
    !rest[..name_end].is_empty() && rest[name_end..].trim_start().starts_with('{')
}

/// True when a comment line names a ticket or a wave — the two kinds of internal tracking
/// reference that must not appear in v2 documentation.
fn names_ticket_or_wave(line: &str) -> bool {
    let t = line.trim_start();
    if !t.starts_with("//") {
        return false;
    }
    let chars: Vec<char> = t.to_ascii_lowercase().chars().collect();
    // The ticket spelling allows dots between digit runs; the wave spellings take a bare run of
    // digits, optionally separated from the keyword by one space or dash.
    for (kw, dotted) in [("t-", true), ("wave", false), ("w", false)] {
        let k: Vec<char> = kw.chars().collect();
        for start in 0..chars.len() {
            if start + k.len() > chars.len() || chars[start..start + k.len()] != k[..] {
                continue;
            }
            if start > 0 && is_word(chars[start - 1]) {
                continue;
            }
            let mut j = start + k.len();
            if !dotted && chars.get(j).is_some_and(|c| *c == ' ' || *c == '-') {
                j += 1;
            }
            if !chars.get(j).is_some_and(char::is_ascii_digit) {
                continue;
            }
            while chars
                .get(j)
                .is_some_and(|c| c.is_ascii_digit() || (dotted && *c == '.'))
            {
                j += 1;
            }
            return true;
        }
    }
    false
}

/// True when `c` can appear inside an identifier.
fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[test]
fn v2_production_files_meet_the_documentation_standard() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/v2");
    let mut bad: Vec<String> = Vec::new();
    for path in production_files(&root) {
        let text = fs::read_to_string(&path).expect("read v2 source");
        let rel = path.strip_prefix(&root).unwrap_or(&path).display();
        let lines: Vec<&str> = text.lines().collect();

        match lines.iter().find(|l| !l.trim().is_empty()) {
            Some(first) if first.trim_start().starts_with("//!") => {}
            _ => bad.push(format!("{rel}:1: file does not open with a `//!` header")),
        }
        if lines.len() > MAX_LINES {
            bad.push(format!(
                "{rel}:{}: {} lines exceeds the {MAX_LINES}-line limit",
                lines.len(),
                lines.len()
            ));
        }
        for (i, line) in lines.iter().enumerate() {
            if needs_doc(line) {
                let mut k = i;
                while k > 0 && is_attribute(lines[k - 1]) {
                    k -= 1;
                }
                if k == 0 || !is_doc(lines[k - 1]) {
                    bad.push(format!("{rel}:{}: item is undocumented", i + 1));
                }
            }
            if line.trim_start().starts_with("#[cfg(test)]") {
                if let Some(next) = lines[i + 1..].iter().find(|l| !l.trim().is_empty()) {
                    if is_inline_mod(next) {
                        bad.push(format!("{rel}:{}: inline test module", i + 1));
                    }
                }
            }
            if names_ticket_or_wave(line) {
                bad.push(format!("{rel}:{}: comment names a ticket or wave", i + 1));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "v2 documentation audit failed:\n{}",
        bad.join("\n")
    );
}
