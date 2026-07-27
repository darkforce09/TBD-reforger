//! T-181.2 — walk a source tree, emit the committed TSV index.
//!
//! Output is TSV on purpose: `rg '^CRF_EGamemodeState\t' crf_symbols.tsv` is instant and
//! needs no parser, and the files diff cleanly in review.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

use super::symbols::{self, Kind};

pub struct Stats {
    pub files: usize,
    pub loc: usize,
    pub symbols: usize,
    pub classes: usize,
    pub rpl_props: usize,
}

/// Recursively collect `*.c` under `root`, sorted for deterministic output.
pub fn collect_sources(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let rd = match std::fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(_) => continue, // unreadable dirs are skipped, not fatal
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                // `.git` and node_modules carry no Enfusion source.
                let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if name == ".git" || name == "node_modules" {
                    continue;
                }
                stack.push(p);
            } else if p.extension().and_then(|s| s.to_str()) == Some("c") {
                out.push(p);
            }
        }
    }
    out.sort();
    Ok(out)
}

fn sha256_file(p: &Path) -> String {
    match std::fs::read(p) {
        Ok(b) => {
            let mut h = Sha256::new();
            h.update(&b);
            // sha2 0.11 returns `Array`, which does not implement LowerHex — format bytes.
            h.finalize().iter().map(|b| format!("{b:02x}")).collect()
        }
        Err(_) => String::new(),
    }
}

/// Build the index for `root` and write `<prefix>_*.tsv` into `out_dir`.
pub fn build(root: &Path, out_dir: &Path, prefix: &str) -> Result<Stats> {
    let files =
        collect_sources(root).with_context(|| format!("collecting .c under {}", root.display()))?;

    let mut symbols_tsv = String::from("symbol\tkind\tfile\tline\tend_line\tloc\tparent\tbase\n");
    let mut files_tsv = String::from("file\tloc\tclasses\tmethods\tsha256\n");
    let mut modded_tsv = String::from("modded_class\tvanilla_base\tfile\tline\n");
    let mut rpl_tsv = String::from("class\tprop\ton_rpl_name\tfile\tline\n");

    let mut st = Stats {
        files: 0,
        loc: 0,
        symbols: 0,
        classes: 0,
        rpl_props: 0,
    };

    for p in &files {
        let rel = p
            .strip_prefix(root)
            .unwrap_or(p)
            .to_string_lossy()
            .replace('\\', "/");
        let scan = match symbols::scan(p, &rel) {
            Ok(s) => s,
            Err(_) => continue, // non-UTF8 carve fragments are skipped, never faked
        };

        let mut n_class = 0usize;
        let mut n_method = 0usize;
        for s in &scan.symbols {
            match s.kind {
                Kind::Method => n_method += 1,
                _ => n_class += 1,
            }
            let _ = writeln!(
                symbols_tsv,
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                s.name,
                s.kind.as_str(),
                s.file,
                s.line,
                s.end_line,
                s.end_line.saturating_sub(s.line) + 1,
                s.parent,
                s.base
            );
            if s.kind == Kind::ModdedClass {
                let _ = writeln!(modded_tsv, "{}\t{}\t{}\t{}", s.name, s.base, s.file, s.line);
            }
        }
        for r in &scan.rpl_props {
            let _ = writeln!(
                rpl_tsv,
                "{}\t{}\t{}\t{}\t{}",
                r.class, r.prop, r.on_rpl_name, r.file, r.line
            );
        }

        let _ = writeln!(
            files_tsv,
            "{}\t{}\t{}\t{}\t{}",
            rel,
            scan.loc,
            n_class,
            n_method,
            sha256_file(p)
        );

        st.files += 1;
        st.loc += scan.loc;
        st.symbols += scan.symbols.len();
        st.classes += n_class;
        st.rpl_props += scan.rpl_props.len();
    }

    std::fs::create_dir_all(out_dir)?;
    std::fs::write(out_dir.join(format!("{prefix}_symbols.tsv")), symbols_tsv)?;
    std::fs::write(out_dir.join(format!("{prefix}_files.tsv")), files_tsv)?;
    std::fs::write(out_dir.join(format!("{prefix}_modded.tsv")), modded_tsv)?;
    std::fs::write(out_dir.join(format!("{prefix}_rplprops.tsv")), rpl_tsv)?;

    Ok(st)
}

/// Resolve a citation `symbol` against an emitted index — the primitive `verify-oracle` uses.
/// Returns matching `file:line` strings.
pub fn lookup(index_tsv: &Path, symbol: &str) -> Result<Vec<String>> {
    let text = std::fs::read_to_string(index_tsv)
        .with_context(|| format!("reading {}", index_tsv.display()))?;
    let mut hits = Vec::new();
    for line in text.lines().skip(1) {
        let mut f = line.split('\t');
        let (Some(name), Some(_kind), Some(file), Some(lineno)) =
            (f.next(), f.next(), f.next(), f.next())
        else {
            continue;
        };
        if name == symbol {
            hits.push(format!("{file}:{lineno}"));
        }
    }
    Ok(hits)
}

/// Count symbols per top-level directory — the backbone of the capability matrix.
pub fn dir_histogram(index_tsv: &Path, depth: usize) -> Result<BTreeMap<String, usize>> {
    let text = std::fs::read_to_string(index_tsv)?;
    let mut out: BTreeMap<String, usize> = BTreeMap::new();
    for line in text.lines().skip(1) {
        let mut f = line.split('\t');
        let (_n, _k, Some(file)) = (f.next(), f.next(), f.next()) else {
            continue;
        };
        let key: Vec<&str> = file.split('/').take(depth).collect();
        *out.entry(key.join("/")).or_insert(0) += 1;
    }
    Ok(out)
}
