use super::*;

pub(super) fn repo_root() -> Result<PathBuf> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()?;
    Ok(PathBuf::from(String::from_utf8_lossy(&out.stdout).trim()))
}

pub fn verify_file_length() -> Result<u8> {
    Ok(verify_file_length_in(&repo_root()?))
}

pub(super) fn verify_file_length_in(root: &Path) -> u8 {
    match verify_file_length_inner(root) {
        Ok(code) => code,
        Err(cause) => refuse_file_length(cause),
    }
}

pub(super) fn refuse_file_length(cause: NotRun) -> u8 {
    let v = Verdict::did_not_run("file-length could not scan the Rust tree", Kind::Ban, cause);
    println!("{v}");
    println!("file-length: FAIL (did not run)");
    2
}

pub(super) fn verify_file_length_inner(root: &Path) -> std::result::Result<u8, NotRun> {
    let al_path = root.join(".coding-standards-allowlist.yaml");
    let al = match std::fs::read_to_string(&al_path) {
        Ok(s) => s,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return Err(NotRun::TargetMissing(al_path));
        }
        Err(source) => {
            return Err(NotRun::Unreadable {
                path: al_path,
                source,
            });
        }
    };
    let entries = parse_allowlist(&al);
    let today = today_ymd()?;

    let files = walk_rust_sources(root)?;
    if files.is_empty() {
        println!("FAIL: file-length walked 0 .rs files — refusing a vacuous pass (T-899)");
        return Ok(1);
    }

    let mut fails = 0u64;
    let scanned: std::collections::HashSet<String> =
        files.iter().map(|f| rel_posix(root, f)).collect();
    for entry in &entries {
        if !allowlist_path_is_scanned(entry, &scanned) {
            eprintln!(
                "SIZE-3: orphan allowlist row for {} ({})",
                entry.path, entry.rule
            );
            fails += 1;
        }
        if entry.rule == "SIZE-3" && entry.expires == "MC-perf" {
            eprintln!(
                "SIZE-3: {} uses MC-perf instead of a dated expiry",
                entry.path
            );
            fails += 1;
        }
    }
    for f in &files {
        let rel = rel_posix(root, f);
        let n = match std::fs::read_to_string(f) {
            Ok(s) => s.lines().count(),
            Err(source) => {
                return Err(NotRun::Unreadable {
                    path: f.clone(),
                    source,
                });
            }
        };
        if is_size2(&rel, &entries, &today) {
            continue;
        }
        let max_lines = if is_test_file(&rel) {
            SIZE_3_TEST_MAX_LINES
        } else {
            SIZE_3_PRODUCTION_MAX_LINES
        };
        if n > max_lines {
            if !is_size3_exempt(&rel, &entries, &today) {
                eprintln!("SIZE-3: {rel} is {n} lines (>{max_lines}, not allowlisted)");
                fails += 1;
            }
        }
    }
    println!(
        "file-length: scanned {} .rs file(s), {fails} violation(s).",
        files.len()
    );
    Ok(u8::from(fails > 0))
}

pub(super) fn walk_rust_sources(root: &Path) -> std::result::Result<Vec<PathBuf>, NotRun> {
    let roots = file_length_roots(root)?;
    let refs: Vec<&Path> = roots.iter().map(PathBuf::as_path).collect();
    scan::walk_files(&refs, |path| {
        path.extension().and_then(|ext| ext.to_str()) == Some("rs")
            // The contract generator owns this tree; source splits cannot maintain it.
            && !path.starts_with(root.join("apps/website/api/src/contract/generated"))
    })
}

pub(super) fn file_length_roots(root: &Path) -> std::result::Result<Vec<PathBuf>, NotRun> {
    let mut out: Vec<PathBuf> = FILE_LENGTH_PINS.iter().map(|r| root.join(r)).collect();
    let website = root.join("apps/website");
    if website.is_dir() {
        let rd = std::fs::read_dir(&website).map_err(|source| NotRun::Unreadable {
            path: website.clone(),
            source,
        })?;
        for ent in rd {
            let ent = ent.map_err(|source| NotRun::Unreadable {
                path: website.clone(),
                source,
            })?;
            let src = ent.path().join("src");
            if src.is_dir() && !out.iter().any(|p| p == &src) {
                out.push(src);
            }
            let tests = ent.path().join("tests");
            if tests.is_dir() {
                out.push(tests);
            }
        }
    }
    Ok(out)
}

pub(super) fn rel_posix(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .to_string_lossy()
        .replace('\\', "/")
}

pub(super) fn is_test_file(rel: &str) -> bool {
    rel.split('/').any(|component| component == "tests") || rel.ends_with("_tests.rs")
}

pub(super) fn allowlist_path_is_scanned(
    entry: &AllowEntry,
    scanned: &std::collections::HashSet<String>,
) -> bool {
    if entry.rule == "SIZE-2" {
        let prefix = entry.path.split("/**").next().unwrap_or(&entry.path);
        scanned
            .iter()
            .any(|rel| rel == &entry.path || rel.starts_with(prefix))
    } else {
        scanned.contains(&entry.path)
    }
}

pub(super) fn parse_allowlist(text: &str) -> Vec<AllowEntry> {
    let mut out = Vec::new();
    let mut cur: Option<AllowEntry> = None;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('#') || t.is_empty() {
            continue;
        }
        if let Some(r) = t.strip_prefix("- rule:") {
            if let Some(e) = cur.take() {
                if !e.path.is_empty() {
                    out.push(e);
                }
            }
            cur = Some(AllowEntry {
                rule: yaml_scalar(r),
                path: String::new(),
                reason: String::new(),
                expires: String::new(),
            });
            continue;
        }
        let Some(e) = cur.as_mut() else {
            continue;
        };
        if let Some(v) = t.strip_prefix("path:") {
            e.path = yaml_scalar(v);
        } else if let Some(v) = t.strip_prefix("reason:") {
            e.reason = yaml_scalar(v);
        } else if let Some(v) = t.strip_prefix("expires:") {
            e.expires = yaml_scalar(v);
        }
    }
    if let Some(e) = cur.take() {
        if !e.path.is_empty() {
            out.push(e);
        }
    }
    out
}

/// Strip one layer of surrounding YAML quotes and treat whitespace / `""` as empty.
pub(super) fn yaml_scalar(raw: &str) -> String {
    let t = raw.trim();
    let inner = if t.len() >= 2 {
        let b = t.as_bytes();
        let dq = b[0] == b'"' && *b.last().unwrap() == b'"';
        let sq = b[0] == b'\'' && *b.last().unwrap() == b'\'';
        if dq || sq {
            t[1..t.len() - 1].trim()
        } else {
            t
        }
    } else {
        t
    };
    inner.to_string()
}

pub(super) fn exemption_fields_ok(e: &AllowEntry, today: &str) -> bool {
    !e.reason.is_empty() && expires_ok(&e.expires, today)
}

pub(super) fn is_size2(rel: &str, entries: &[AllowEntry], today: &str) -> bool {
    entries.iter().any(|e| {
        if e.rule != "SIZE-2" || !exemption_fields_ok(e, today) {
            return false;
        }
        let prefix = e.path.split("/**").next().unwrap_or(&e.path);
        rel == e.path || rel.starts_with(prefix)
    })
}

pub(super) fn is_size3_exempt(rel: &str, entries: &[AllowEntry], today: &str) -> bool {
    entries.iter().any(|e| {
        e.rule == "SIZE-3"
            && e.path == rel
            && e.expires != "MC-perf"
            && exemption_fields_ok(e, today)
    })
}

pub(super) fn expires_ok(expires: &str, today: &str) -> bool {
    if expires == "MC-perf" {
        return true;
    }
    let b = expires.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                true
            } else {
                c.is_ascii_digit()
            }
        })
        && expires >= today
}

pub(super) fn today_ymd() -> std::result::Result<String, NotRun> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|source| NotRun::ToolError {
            tool: "clock".into(),
            status: -1,
            stderr: source.to_string(),
        })?
        .as_secs()
        / 86400;
    Ok(civil_ymd(days))
}

/// UTC YYYY-MM-DD from days since 1970-01-01 (Howard Hinnant `civil_from_days`).
pub(super) fn civil_ymd(unix_days: u64) -> String {
    let z = unix_days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

pub fn gen_font_table(bdf_path: &Path) -> Result<u8> {
    const GLYPH_W: usize = 16;
    const GLYPH_H: usize = 32;
    const FIRST: u32 = 32;
    const LAST: u32 = 126;
    let bdf = std::fs::read_to_string(bdf_path).with_context(|| bdf_path.display().to_string())?;
    let mut glyphs: std::collections::HashMap<u32, Vec<u16>> = std::collections::HashMap::new();
    for block in bdf.split("\nSTARTCHAR ").skip(1) {
        let field = |key: &str| -> Option<String> {
            block
                .lines()
                .find(|l| l.starts_with(key))
                .map(|l| l[key.len()..].trim().to_string())
        };
        let Some(enc) = field("ENCODING ").and_then(|v| v.parse::<i64>().ok()) else {
            continue;
        };
        if enc < i64::from(FIRST) || enc > i64::from(LAST) {
            continue;
        }
        let enc = enc as u32;
        let bbx = field("BBX ").unwrap_or_default();
        let nums: Vec<i64> = bbx
            .split_whitespace()
            .filter_map(|v| v.parse().ok())
            .collect();
        if nums.len() != 4 || nums[0] != GLYPH_W as i64 || nums[1] != GLYPH_H as i64 || nums[2] != 0
        {
            bail!("U+{enc:x}: BBX {bbx} — not a full 16x32 cell");
        }
        let bitmap_at = block
            .find("BITMAP")
            .ok_or_else(|| anyhow::anyhow!("U+{enc:x}: no BITMAP"))?;
        let rows: Vec<u16> = block[bitmap_at..]
            .lines()
            .skip(1)
            .take(GLYPH_H)
            .filter_map(|l| u16::from_str_radix(l.trim(), 16).ok())
            .collect();
        if rows.len() != GLYPH_H {
            bail!("U+{enc:x}: bad bitmap ({} rows)", rows.len());
        }
        glyphs.insert(enc, rows);
    }
    for c in FIRST..=LAST {
        if !glyphs.contains_key(&c) {
            bail!(
                "missing glyph U+{c:x} '{}'",
                char::from_u32(c).unwrap_or('?')
            );
        }
    }
    for ch in ['7', 'a', 'A', '-'] {
        let rows = &glyphs[&(ch as u32)];
        eprintln!("── '{ch}' ──");
        for r in rows {
            let line: String = (0..GLYPH_W)
                .map(|x| {
                    if (r >> (15 - x)) & 1 == 1 {
                        '█'
                    } else {
                        '·'
                    }
                })
                .collect();
            eprintln!("{line}");
        }
    }
    let mut out = Vec::new();
    out.push("//! GENERATED by `cargo xtask gen font-table` — DO NOT EDIT BY HAND.".to_string());
    out.push("//!".into());
    out.push("//! Glyph raster data extracted from **Spleen 16x32 v2.2.0**".into());
    out.push(
        "//! Copyright (c) 2018-2026, Frederic Cambus — BSD-2-Clause (SPDX: BSD-2-Clause).".into(),
    );
    out.push("//! <https://github.com/fcambus/spleen> · release tarball sha256".into());
    out.push("//! `ec42925c6b56d2138c862b2f97147c872e472f674bf03423417d827a08d69a89`.".into());
    out.push("//!".into());
    out.push(
        "//! Redistribution notice (BSD-2-Clause): redistributions of source code must retain"
            .into(),
    );
    out.push(
        "//! the above copyright notice; see the upstream `LICENSE` file for the full text.".into(),
    );
    out.push(String::new());
    out.push("/// Glyph ink width in pixels (half the 32 px atlas cell).".into());
    out.push(format!("pub const FONT_GLYPH_W: u32 = {GLYPH_W};"));
    out.push("/// Glyph height in pixels (fills the 32 px atlas cell).".into());
    out.push(format!("pub const FONT_GLYPH_H: u32 = {GLYPH_H};"));
    out.push(String::new());
    out.push("/// One u16 per pixel row, bit 15 = leftmost pixel. Index = ASCII − 32 for".into());
    out.push(
        "/// U+0020..=U+007E; index 95 is all-zero (the baker paints the tofu box there).".into(),
    );
    out.push("#[rustfmt::skip]".into());
    out.push(format!("pub const FONT_16X32: [[u16; {GLYPH_H}]; 96] = ["));
    let zero = vec![0u16; GLYPH_H];
    for c in FIRST..=LAST + 1 {
        let rows = if c <= LAST { &glyphs[&c] } else { &zero };
        let label = if c <= LAST {
            let ch = char::from_u32(c).unwrap();
            match ch {
                '\'' => "'\\''".to_string(),
                '\\' => "'\\\\'".to_string(),
                _ => format!("'{ch}'"),
            }
        } else {
            "tofu (baker-drawn)".to_string()
        };
        let hex: Vec<String> = rows.iter().map(|r| format!("0x{r:04x}")).collect();
        out.push(format!("    [{}], // {label}", hex.join(", ")));
    }
    out.push("];".into());
    println!("{}", out.join("\n"));
    Ok(0)
}
