//! T-181.4 — the `@idx` citation gate.
//!
//! Prose in `docs/mod/**` may assert things about CRF or vanilla only if the symbol it names
//! actually exists in a generated index. Markers look like:
//!
//! ```text
//! The slot claim entry point is `UpdateSlotPlayerID` @idx crf#UpdateSlotPlayerID
//! Deploy goes through `FromEntity` @idx api#SCR_PossessSpawnData
//! ```
//!
//! Lanes: `crf` (CRF symbols) · `vanilla` (carved vanilla symbols) · `api` (official BI Script
//! API class list). Resolution prints the real `file:line`, so **line numbers are never typed
//! by hand** — a doc cites a name, the tool supplies the coordinates.
//!
//! This exists because an agent summarising one CRF file invented four APIs that do not exist
//! (`RequestSlotChange`, `ReleaseSlot`, `GetInstance`, a wrong base class). Under this gate
//! that doc fails to build instead of misleading the next session.

use std::collections::HashSet;
use std::path::Path;

use anyhow::{Context, Result};

pub struct Citation {
    pub file: String,
    pub line: usize,
    pub lane: String,
    pub symbol: String,
}

pub struct CiteReport {
    pub checked: usize,
    pub unresolved: Vec<(Citation, String)>,
    pub docs: usize,
}

fn load_names(tsv: &Path, col: usize) -> HashSet<String> {
    let mut out = HashSet::new();
    if let Ok(text) = std::fs::read_to_string(tsv) {
        for line in text.lines().skip(1) {
            if let Some(v) = line.split('\t').nth(col) {
                if !v.is_empty() {
                    out.insert(v.to_string());
                }
            }
        }
    }
    out
}

/// Extract every `@idx lane#Symbol` marker from a markdown file.
pub fn extract(text: &str, file: &str) -> Vec<Citation> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let mut rest = line;
        while let Some(p) = rest.find("@idx ") {
            let after = &rest[p + 5..];
            let tok: String = after
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '#')
                .collect();
            if let Some((lane, symbol)) = tok.split_once('#') {
                if !lane.is_empty() && !symbol.is_empty() {
                    out.push(Citation {
                        file: file.to_string(),
                        line: i + 1,
                        lane: lane.to_string(),
                        symbol: symbol.to_string(),
                    });
                }
            }
            rest = &after[tok.len().min(after.len())..];
        }
    }
    out
}

/// Verify every citation under `docs_root` against the indexes in `index_dir`.
pub fn verify(docs_root: &Path, index_dir: &Path) -> Result<CiteReport> {
    let crf = load_names(&index_dir.join("crf_symbols.tsv"), 0);
    let vanilla = load_names(&index_dir.join("vanilla_symbols.tsv"), 0);
    let api = load_names(&index_dir.join("vanilla_api_classes.tsv"), 0);

    let mut rep = CiteReport {
        checked: 0,
        unresolved: Vec::new(),
        docs: 0,
    };

    let mut stack = vec![docs_root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            if p.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }
            let text = std::fs::read_to_string(&p)
                .with_context(|| format!("reading {}", p.display()))?;
            let rel = p.to_string_lossy().to_string();
            let cites = extract(&text, &rel);
            if !cites.is_empty() {
                rep.docs += 1;
            }
            for c in cites {
                rep.checked += 1;
                // Resolve the lane before touching `c` — pushing `c` into the report moves it,
                // which would conflict with a borrow of `c.lane` held by the match.
                let known: Option<&HashSet<String>> = match c.lane.as_str() {
                    "crf" => Some(&crf),
                    "vanilla" => Some(&vanilla),
                    "api" => Some(&api),
                    _ => None,
                };
                let Some(known) = known else {
                    let why = format!("unknown lane '{}' (expected crf|vanilla|api)", c.lane);
                    rep.unresolved.push((c, why));
                    continue;
                };
                if known.is_empty() {
                    rep.unresolved
                        .push((c, "index for this lane is empty — rebuild it".into()));
                } else if !known.contains(&c.symbol) {
                    rep.unresolved
                        .push((c, "symbol not found in the index".into()));
                }
            }
        }
    }
    Ok(rep)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_markers() {
        let c = extract("claim is `X` @idx crf#UpdateSlotPlayerID here\n@idx api#SCR_BaseGameMode", "d.md");
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].lane, "crf");
        assert_eq!(c[0].symbol, "UpdateSlotPlayerID");
        assert_eq!(c[0].line, 1);
        assert_eq!(c[1].lane, "api");
        assert_eq!(c[1].symbol, "SCR_BaseGameMode");
    }

    #[test]
    fn ignores_prose_without_markers() {
        assert!(extract("no citations here at all", "d.md").is_empty());
    }
}
