//! The capability matrix, and the UNTRIAGED gate behind it.
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
//! The verdict table (`documentation/mod/tbd-framework/capability_verdicts.tsv`) is
//! hand-authored and reviewed — it is product judgement. The aggregation is mechanical. Same
//! split as the rest of the oracle: humans decide, the tool measures.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use crate::{Error, Result};

/// One row of the verdict table.
#[derive(Debug, Clone)]
pub struct Rule {
    /// The framework path prefix the row covers; the longest matching prefix wins.
    pub prefix: String,
    /// The capability the covered files implement.
    pub capability: String,
    /// One of [`VERDICTS`].
    pub verdict: String,
    /// The free-text note, possibly empty.
    pub note: String,
}

/// The measured totals of one capability.
#[derive(Debug, Default)]
pub struct Agg {
    /// The verdict of the capability's rule.
    pub verdict: String,
    /// The note of the capability's rule.
    pub note: String,
    /// The framework files it covers.
    pub files: usize,
    /// Their lines.
    pub loc: usize,
    /// Their symbols.
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
    "DEFERRED", // out of scope BY OPERATOR WORD (see documentation/mod/tbd-framework/mod_design.md §Deferrals)
];

/// Read the verdict table at `path`, longest prefix first.
pub fn load_rules(path: &Path) -> Result<Vec<Rule>> {
    let text = std::fs::read_to_string(path).map_err(|cause| Error::ReadVerdictTable {
        path: path.to_path_buf(),
        cause,
    })?;
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim_end();
        // The header may sit below a comment block, so detect it by content, not position.
        if line.is_empty() || line.starts_with('#') || line.starts_with("prefix\t") {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 3 {
            return Err(Error::VerdictRowShape {
                path: path.to_path_buf(),
                line: i + 1,
            });
        }
        let verdict = f[2].trim().to_string();
        if !VERDICTS.contains(&verdict.as_str()) {
            return Err(Error::UnknownVerdict {
                path: path.to_path_buf(),
                line: i + 1,
                verdict,
                expected: VERDICTS.join(", "),
            });
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

/// The capability matrix of one `enf capability` run.
pub struct Report {
    /// The matrix as TSV, heaviest capability first.
    pub matrix_tsv: String,
    /// The framework files no rule covers.
    pub untriaged: Vec<String>,
    /// The capabilities in the matrix.
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
