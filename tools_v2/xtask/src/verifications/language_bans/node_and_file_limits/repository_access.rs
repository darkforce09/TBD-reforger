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
    let v = Verdict::did_not_run(
        "file-length could not scan the source trees",
        Kind::Ban,
        cause,
    );
    println!("{v}");
    println!("file-length: FAIL (did not run)");
    2
}

pub(super) fn verify_file_length_inner(root: &Path) -> std::result::Result<u8, NotRun> {
    let files = walk_length_gated_sources(root)?;
    if files.is_empty() {
        println!("FAIL: file-length walked 0 source files — refusing a vacuous pass.");
        return Ok(1);
    }

    let mut fails = 0u64;
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
        let max_lines = if is_test_file(&rel) {
            SIZE_3_TEST_MAX_LINES
        } else {
            SIZE_3_PRODUCTION_MAX_LINES
        };
        if n > max_lines {
            eprintln!(
                "SIZE-3: {rel} is {n} lines (>{max_lines}). Hard limit exceeded; decompose by responsibility. No exemptions permitted."
            );
            fails += 1;
        }
    }
    println!("{}", length_scan_summary(&files, fails));
    Ok(u8::from(fails > 0))
}

/// The gate's summary line: total files, the per-extension split, and the violation count.
pub(super) fn length_scan_summary(files: &[PathBuf], violations: u64) -> String {
    let with_extension = |wanted: &str| {
        files
            .iter()
            .filter(|path| path.extension().is_some_and(|ext| ext == wanted))
            .count()
    };
    format!(
        "file-length: scanned {} source file(s) ({} .rs, {} .c), {violations} violation(s).",
        files.len(),
        with_extension("rs"),
        with_extension("c")
    )
}

/// Every file under the length-gate roots whose extension is in [`LENGTH_GATED_EXTENSIONS`].
pub(super) fn walk_length_gated_sources(root: &Path) -> std::result::Result<Vec<PathBuf>, NotRun> {
    let roots = file_length_roots(root)?;
    let refs: Vec<&Path> = roots.iter().map(PathBuf::as_path).collect();
    scan::walk_files(&refs, |path| {
        path.extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| LENGTH_GATED_EXTENSIONS.contains(&ext))
    })
}

/// Compile-time guard: every `apps/mod/` entry of `pins` is exactly one of `script_roots`, so a
/// gitignored reference tree or a whole addon folder can never be pinned.
pub(super) const fn mod_pins_are_script_roots(pins: &[&str], script_roots: &[&str]) -> bool {
    let mut pin_index = 0;
    while pin_index < pins.len() {
        let pin = pins[pin_index].as_bytes();
        if bytes_start_with(pin, b"apps/mod/") || bytes_equal(pin, b"apps/mod") {
            let mut root_index = 0;
            let mut matched = false;
            while root_index < script_roots.len() {
                if bytes_equal(pin, script_roots[root_index].as_bytes()) {
                    matched = true;
                }
                root_index += 1;
            }
            if !matched {
                return false;
            }
        }
        pin_index += 1;
    }
    true
}

const fn bytes_start_with(bytes: &[u8], prefix: &[u8]) -> bool {
    if bytes.len() < prefix.len() {
        return false;
    }
    let mut index = 0;
    while index < prefix.len() {
        if bytes[index] != prefix[index] {
            return false;
        }
        index += 1;
    }
    true
}

const fn bytes_equal(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len() && bytes_start_with(left, right)
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
    let path = Path::new(rel);
    path.components().any(|part| part.as_os_str() == "tests")
        || (path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| LENGTH_GATED_EXTENSIONS.contains(&extension))
            && path
                .file_stem()
                .is_some_and(|stem| stem.to_string_lossy().ends_with("_tests")))
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
