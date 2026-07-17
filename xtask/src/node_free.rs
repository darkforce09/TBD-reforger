//! T-165.10 — the Node-eradication closure set: the SIZE file-length gate (port of
//! `scripts/website/verify-file-length.mjs`), the Spleen font-table generator (port of
//! `scripts/website/gen-text-font-table.mjs`), and the `verify no-node` hard gate (the
//! T-162 verify-no-python pattern for Node: zero tracked .mjs/.cjs outside apps/mod and no
//! node/npx invocations outside the enfusion-mcp floor).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

fn repo_root() -> Result<PathBuf> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()?;
    Ok(PathBuf::from(String::from_utf8_lossy(&out.stdout).trim()))
}

/* ─────────────────────────── verify file-length (SIZE-1/3) ─────────────────────────── */

pub fn verify_file_length() -> Result<u8> {
    let root = repo_root()?;
    let al = std::fs::read_to_string(root.join(".coding-standards-allowlist.yaml"))
        .context(".coding-standards-allowlist.yaml")?;
    let mut size2: Vec<String> = Vec::new();
    let mut size3: Vec<String> = Vec::new();
    let mut rule: Option<&str> = None;
    for line in al.lines() {
        if let Some(r) = line.split("rule:").nth(1) {
            let r = r.trim();
            if r.starts_with("SIZE-") {
                rule = Some(if r.starts_with("SIZE-2") {
                    "SIZE-2"
                } else if r.starts_with("SIZE-3") {
                    "SIZE-3"
                } else {
                    "other"
                });
                continue;
            }
        }
        if let Some(p) = line.split("path:").nth(1) {
            let p = p.split_whitespace().next().unwrap_or("").to_string();
            if p.is_empty() {
                continue;
            }
            match rule {
                Some("SIZE-2") => size2.push(p),
                Some("SIZE-3") => size3.push(p),
                _ => {}
            }
        }
    }
    let is_size2 = |rel: &str| {
        size2.iter().any(|g| {
            let prefix = g.split("/**").next().unwrap_or(g);
            rel.starts_with(prefix)
        })
    };
    let excl = ["node_modules", "dist", "build", ".git", "coverage"];
    fn walk(dir: &Path, excl: &[&str], acc: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.filter_map(|e| e.ok()) {
            let p = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if p.is_dir() {
                if !excl.contains(&name.as_str()) {
                    walk(&p, excl, acc);
                }
            } else if name.ends_with(".go") || name.ends_with(".ts") || name.ends_with(".tsx") {
                acc.push(p);
            }
        }
    }
    let mut files = Vec::new();
    walk(&root.join("apps/website"), &excl, &mut files);
    let mut warns = 0u64;
    let mut fails = 0u64;
    for f in files {
        let rel = f
            .strip_prefix(&root)
            .unwrap_or(&f)
            .to_string_lossy()
            .into_owned();
        if is_size2(&rel) {
            continue;
        }
        let n = std::fs::read_to_string(&f)
            .map(|s| s.split('\n').count())
            .unwrap_or(0);
        if n > 1000 {
            if !size3.contains(&rel) {
                eprintln!("SIZE-3: {rel} is {n} lines (>1000, not allowlisted)");
                fails += 1;
            }
        } else if n > 600 {
            eprintln!("SIZE-1 warn: {rel} is {n} lines (>600)");
            warns += 1;
        }
    }
    println!("file-length: {warns} warning(s), {fails} violation(s).");
    Ok(u8::from(fails > 0))
}

/* ─────────────────────────── gen font-table (T-152.13) ─────────────────────────── */

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

/* ─────────────────────────── verify no-node (T-165.10 hard gate) ─────────────────────────── */

/// The closure gate: (1) zero tracked `.mjs`/`.cjs` outside `apps/mod`; (2) no `node `/`npx `
/// invocations in the Makefile, scripts/, or workflows outside the enfusion-mcp floor
/// (mcp-call.sh's `.js` runner tiers); (3) zero `actions/setup-node` in CI.
pub fn verify_no_node() -> Result<u8> {
    let root = repo_root()?;
    let mut fails = 0u64;

    println!("==> git ls-files '*.mjs' '*.cjs' (excl apps/mod)");
    let out = std::process::Command::new("git")
        .args(["ls-files", "*.mjs", "*.cjs"])
        .current_dir(&root)
        .output()?;
    let tracked: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.starts_with("apps/mod/"))
        .map(str::to_string)
        .collect();
    if tracked.is_empty() {
        println!("  OK (none)");
    } else {
        println!("FAIL: tracked Node scripts remain:");
        for t in &tracked {
            println!("  {t}");
        }
        fails += 1;
    }

    println!(
        "==> node/npx invocations in Makefile + scripts/ + .github/ (allowlist: enfusion-mcp floor)"
    );
    // Files allowed to invoke node/npx: the enfusion-mcp runner tiers only.
    let allow_files = ["scripts/mod/mcp-call.sh"];
    let mut offenders: Vec<String> = Vec::new();
    let mut scan = |path: &Path| {
        let Ok(text) = std::fs::read_to_string(path) else {
            return;
        };
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned();
        if allow_files.contains(&rel.as_str()) {
            return;
        }
        for (i, line) in text.lines().enumerate() {
            let t = line.trim_start();
            if t.starts_with('#') || t.starts_with("//") {
                continue; // comments may reference the floor
            }
            // invocation shapes only: `node <arg>` / `npx <arg>` in command position
            // (drop inline `##` help text first — Makefile target docs may NAME the ban).
            let code = line.split("##").next().unwrap_or(line);
            let hit = code.split(&['|', ';', '&', '(', ')'][..]).any(|seg| {
                let seg = seg.trim_start();
                seg.starts_with("node ") || seg.starts_with("npx ")
            });
            if hit {
                offenders.push(format!("{rel}:{} {}", i + 1, line.trim()));
            }
        }
    };
    scan(&root.join("Makefile"));
    fn walk_scripts(dir: &Path, acc: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.filter_map(|e| e.ok()) {
            let p = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if p.is_dir() {
                if name != "node_modules" && name != "__pycache__" {
                    walk_scripts(&p, acc);
                }
            } else if name.ends_with(".sh") || name.ends_with(".yml") || name.ends_with(".yaml") {
                acc.push(p);
            }
        }
    }
    let mut targets = Vec::new();
    walk_scripts(&root.join("scripts"), &mut targets);
    walk_scripts(&root.join(".github"), &mut targets);
    for t in &targets {
        scan(t);
    }
    if offenders.is_empty() {
        println!("  OK (none)");
    } else {
        println!("FAIL: node/npx invocations outside the enfusion-mcp floor:");
        for o in &offenders {
            println!("  {o}");
        }
        fails += 1;
    }

    println!("==> actions/setup-node in workflows");
    let mut setup_node = Vec::new();
    for t in &targets {
        if t.to_string_lossy().contains(".github")
            && std::fs::read_to_string(t).is_ok_and(|s| s.contains("actions/setup-node"))
        {
            setup_node.push(
                t.strip_prefix(&root)
                    .unwrap_or(t)
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
    if setup_node.is_empty() {
        println!("  OK (none)");
    } else {
        println!("FAIL: setup-node steps remain: {}", setup_node.join(", "));
        fails += 1;
    }

    if fails > 0 {
        eprintln!("\nverify-no-node: FAIL ({fails})");
        return Ok(1);
    }
    println!("\nverify-no-node: OK — Node exists solely as the enfusion-mcp runtime");
    Ok(0)
}
