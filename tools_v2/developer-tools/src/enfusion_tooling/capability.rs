//! T-181.2.1 — the capability matrix, and the UNTRIAGED gate behind it.
//!
//! THE PROBLEM THIS SOLVES
//! -----------------------
//! Operator, verbatim: *"I feel like I'm only reaching 20% of what I need to say. I don't
//! know what I don't know."* Reforger ships no lobby, briefing, slotting, respawn, spectator
//! or admin tooling, so `tbd-framework` has to supply all of it — and the failure mode is
//! silently forgetting a whole subsystem until it blocks an event.
//!
//! CRF is a *working* framework covering that ground. So instead of trusting memory, every
//! CRF source file must map to an explicit TBD verdict. A file that matches no rule is
//! reported `UNTRIAGED` and the check FAILS. A forgotten capability becomes a build error.
//!
//! The verdict table (`docs/mod/capability_verdicts.tsv`) is hand-authored and reviewed —
//! it is product judgement. The aggregation is mechanical. Same split as the rest of T-181:
//! humans decide, the tool measures.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Rule {
    pub prefix: String,
    pub capability: String,
    pub verdict: String,
    pub note: String,
}

#[derive(Debug, Default)]
pub struct Agg {
    pub verdict: String,
    pub note: String,
    pub files: usize,
    pub loc: usize,
    pub symbols: usize,
}

/// Legal verdicts. Anything else in the table is a typo and fails loudly.
pub const VERDICTS: &[&str] = &[
    "BUILD",    // TBD must implement this
    "HAVE",     // already exists in tbd-framework
    "PARTIAL",  // partially covered, tracked elsewhere
    "REPLACE",  // CRF's mechanism swapped for TBD JSON
    "LATER",    // wanted, not on the critical path
    "SKIP",     // deliberately out of scope
    "DEFERRED", // out of scope BY OPERATOR WORD (see docs/mod/TBD_MOD_DESIGN.md §Deferrals)
];

pub fn load_rules(path: &Path) -> Result<Vec<Rule>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading verdict table {}", path.display()))?;
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim_end();
        // The header may sit below a comment block, so detect it by content, not position.
        if line.is_empty() || line.starts_with('#') || line.starts_with("prefix\t") {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 3 {
            anyhow::bail!(
                "{}:{}: need prefix\\tcapability\\tverdict[\\tnote]",
                path.display(),
                i + 1
            );
        }
        let verdict = f[2].trim().to_string();
        if !VERDICTS.contains(&verdict.as_str()) {
            anyhow::bail!(
                "{}:{}: unknown verdict '{}' (expected one of {})",
                path.display(),
                i + 1,
                verdict,
                VERDICTS.join(", ")
            );
        }
        out.push(Rule {
            prefix: f[0].trim().to_string(),
            capability: f[1].trim().to_string(),
            verdict,
            note: f.get(3).unwrap_or(&"").trim().to_string(),
        });
    }
    // Longest prefix wins, so a specific file can override its directory.
    // `Reverse` rather than a flipped `cmp`: identical ordering (Reverse's Ord IS `other.cmp(self)`)
    // and `sort_by`/`sort_by_key` are both stable, so equal-length prefixes keep file order.
    out.sort_by_key(|r| std::cmp::Reverse(r.prefix.len()));
    Ok(out)
}

pub struct Report {
    pub matrix_tsv: String,
    pub untriaged: Vec<String>,
    pub capabilities: usize,
}

/// Join the measured index against the verdict table.
pub fn build(files_tsv: &Path, symbols_tsv: &Path, rules: &[Rule]) -> Result<Report> {
    // symbol counts per file
    let sym_text = std::fs::read_to_string(symbols_tsv)?;
    let mut sym_per_file: BTreeMap<String, usize> = BTreeMap::new();
    for line in sym_text.lines().skip(1) {
        let mut f = line.split('\t');
        if let (_, _, Some(file)) = (f.next(), f.next(), f.next()) {
            *sym_per_file.entry(file.to_string()).or_insert(0) += 1;
        }
    }

    let files_text = std::fs::read_to_string(files_tsv)?;
    let mut agg: BTreeMap<String, Agg> = BTreeMap::new();
    let mut untriaged = Vec::new();

    for line in files_text.lines().skip(1) {
        let mut f = line.split('\t');
        let (Some(file), Some(loc)) = (f.next(), f.next()) else {
            continue;
        };
        let loc: usize = loc.parse().unwrap_or(0);

        match rules.iter().find(|r| file.starts_with(&r.prefix)) {
            Some(rule) => {
                let e = agg.entry(rule.capability.clone()).or_default();
                e.verdict = rule.verdict.clone();
                e.note = rule.note.clone();
                e.files += 1;
                e.loc += loc;
                e.symbols += sym_per_file.get(file).copied().unwrap_or(0);
            }
            None => untriaged.push(file.to_string()),
        }
    }

    let mut matrix = String::from("capability\tverdict\tfiles\tloc\tsymbols\tnote\n");
    // Heaviest capabilities first — that is the reading order that matters.
    // `Reverse`, same reasoning as the prefix sort above: same order, same stability.
    let mut rows: Vec<_> = agg.into_iter().collect();
    rows.sort_by_key(|r| std::cmp::Reverse(r.1.loc));
    for (cap, a) in &rows {
        let _ = writeln!(
            matrix,
            "{}\t{}\t{}\t{}\t{}\t{}",
            cap, a.verdict, a.files, a.loc, a.symbols, a.note
        );
    }

    Ok(Report {
        matrix_tsv: matrix,
        untriaged,
        capabilities: rows.len(),
    })
}
