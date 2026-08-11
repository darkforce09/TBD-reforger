//! SQL-shape gates for the Axum API (T-853 port of `scripts/website/verify-no-select-star.sh`).
//!
//! ── WHAT THE GATE IS FOR (T-145) ─────────────────────────────────────────────────────────────
//!
//! A bare `SELECT *` / `RETURNING *` against a table with nullable columns re-introduces the
//! Go→Rust null-tolerance 500 hazard: Go/GORM read NULL as the zero value, sqlx rejects it. Model
//! reads must list columns explicitly and `COALESCE` the nullable non-`Option` ones. Only tables
//! with ZERO nullable columns may use `*`.
//!
//! ── WHAT THE PORT FIXES ──────────────────────────────────────────────────────────────────────
//!
//! The bash version fed its loop from
//!
//! ```text
//! done < <(grep -rnE 'SELECT \* FROM [a-z_]+' "$ROOT/src/handlers" "$ROOT/src/services" 2>/dev/null || true)
//! ```
//!
//! `2>/dev/null` hid "no such directory" and `|| true` turned the failure into an empty result
//! set, which the loop read as *zero violations*. Renaming `src/handlers` — or running the script
//! from a tree where `apps/website/api` had moved — printed `no-select-star: clean` over source it
//! never opened. That is the signature defect, and here a missing root is a `DidNotRun`.
//!
//! Output is byte-identical to the script otherwise, including the absolute paths that
//! `grep -rn "$ROOT/..."` produced, so the port is accepted by diffing stdout.

use std::path::Path;

use anyhow::Result;
use tbd_gate::scan::{self, Hit};
use tbd_gate::{NotRun, Pattern};

/// Tables verified to have no nullable columns, so `*` is safe on them.
///
/// The bash `ALLOW='modpack_mods|orbat_reservations'`, kept as a list rather than an alternation
/// so adding one is not an exercise in regex quoting.
const ALLOW: &[&str] = &["modpack_mods", "orbat_reservations"];

/// Directories searched, relative to `apps/website/api`.
const ROOTS: &[&str] = &["src/handlers", "src/services"];

pub fn verify_no_select_star(repo_root: &Path) -> Result<u8> {
    let api = repo_root.join("apps/website/api");
    let roots: Vec<_> = ROOTS.iter().map(|r| api.join(r)).collect();
    let root_refs: Vec<&Path> = roots.iter().map(|p| p.as_path()).collect();

    // A missing root is "the check did not run", never "clean". See the module docs.
    let files = match scan::walk_files(&root_refs, |_| true) {
        Ok(f) => f,
        Err(cause) => return Ok(report_did_not_run(cause)),
    };

    let select_star = Pattern::regex(r"SELECT \* FROM [a-z_]+")?;
    let returning_star = Pattern::regex(r"RETURNING \*")?;
    let table_of = Pattern::regex(r"SELECT \* FROM ([a-z_]+)")?;

    let mut bad = false;

    for hit in fetch(&select_star, &files)? {
        // bash: `grep -oE 'SELECT \* FROM [a-z_]+' | awk '{print $NF}'`, then an exact-match test
        // against the allow list. A line with several matches fails if ANY table is disallowed.
        let disallowed = tables_in(&table_of, &hit.line)
            .into_iter()
            .any(|t| !ALLOW.contains(&t.as_str()));
        if disallowed {
            println!("  SELECT-* on nullable-column table — list columns + COALESCE:");
            println!("    {}", hit.rendered());
            bad = true;
        }
    }

    for hit in fetch(&returning_star, &files)? {
        // bash tested the WHOLE LINE against the allow alternation here, not the table name.
        // Preserved deliberately: changing it would change which lines the gate reports, and that
        // is a behaviour change to argue for separately, not to smuggle into a port.
        if !ALLOW.iter().any(|a| hit.line.contains(a)) {
            println!("  RETURNING-* — list columns + COALESCE nullable ones:");
            println!("    {}", hit.rendered());
            bad = true;
        }
    }

    if bad {
        println!("no-select-star: FAIL");
        Ok(1)
    } else {
        println!("no-select-star: clean");
        Ok(0)
    }
}

fn fetch(pattern: &Pattern, files: &[std::path::PathBuf]) -> Result<Vec<Hit>> {
    match scan::grep_lines(pattern, files) {
        Ok(hits) => Ok(hits),
        Err(cause) => {
            // Reuse the same refusal shape rather than inventing a second one.
            report_did_not_run(cause);
            anyhow::bail!("no-select-star: the scan could not run");
        }
    }
}

fn tables_in(table_of: &Pattern, line: &str) -> Vec<String> {
    // `Pattern` deliberately exposes only `is_match`, so the capture is done here with the same
    // source text. Splitting on the literal keeps this independent of the regex engine's API.
    let mut out = Vec::new();
    if !table_of.is_match(line) {
        return out;
    }
    for part in line.split("SELECT * FROM ").skip(1) {
        let name: String = part
            .chars()
            .take_while(|c| c.is_ascii_lowercase() || *c == '_')
            .collect();
        if !name.is_empty() {
            out.push(name);
        }
    }
    out
}

fn report_did_not_run(cause: NotRun) -> u8 {
    let v = tbd_gate::Verdict::did_not_run(
        "no-select-star could not scan the API sources",
        tbd_gate::Kind::Ban,
        cause,
    );
    println!("{v}");
    println!("no-select-star: FAIL");
    // 2, not 1: "the tree is dirty" and "I never read the tree" are different operator actions.
    2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_the_table_name() {
        let p = Pattern::regex(r"SELECT \* FROM ([a-z_]+)").unwrap();
        assert_eq!(
            tables_in(&p, "  q(\"SELECT * FROM users WHERE id = $1\")"),
            vec!["users"]
        );
        assert_eq!(
            tables_in(&p, "SELECT * FROM modpack_mods; SELECT * FROM events"),
            vec!["modpack_mods", "events"]
        );
        assert!(tables_in(&p, "no match here").is_empty());
    }

    #[test]
    fn allowlisted_tables_are_exempt() {
        assert!(ALLOW.contains(&"modpack_mods"));
        assert!(ALLOW.contains(&"orbat_reservations"));
        assert!(!ALLOW.contains(&"users"));
    }

    #[test]
    fn a_missing_api_tree_does_not_read_as_clean() {
        // The bash `2>/dev/null || true` behaviour, inverted.
        let code = verify_no_select_star(Path::new("/nonexistent/tbd-gate/repo")).unwrap();
        assert_eq!(code, 2, "a scan that never ran must not exit 0");
    }
}
