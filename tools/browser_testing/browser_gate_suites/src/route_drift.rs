//! S-routes gate: the Leptos router's routes against the committed manifest.
//!
//! Extracts the route table from `apps/frontend/src/foundation/route_table/mod.rs` and diffs it
//! against the frozen React oracle manifest `manifests/routes.csv`. Robust to rustfmt line-wrapping: splits
//! on `RouteDef { … }` blocks and pulls each field by name.
//!
//! Exit 0 = the Leptos route table set/column-diffs equal to routes.csv; 1 = drift (printed).
//!
//! **Role:** `gate s-routes`: the router's `ROUTES` table read from its source text against the
//! committed CSV.
//! **Position:** dispatched by the `gate` command line; reads
//! `apps/frontend/src/foundation/route_table/mod.rs` and
//! `fixtures/dom_oracle/manifests/routes.csv`.
//! **Signals & state:** none; one read and one comparison.
//! **Invariants:** rows compare sorted by path; every differing row is printed; no browser runs.

use crate::error::{Result, ResultExt};
use regex::Regex;
use serde_json::json;

use ::repository_layout::find_repository_root;

/// `gate s-routes`: compares the router's route table with the committed CSV; returns 0 when
/// they match and 1 after printing each differing row.
pub fn run() -> Result<u8> {
    let root = find_repository_root()?;
    let router = root.join("apps/frontend/src/foundation/route_table/mod.rs");
    let oracle_path = root
        .join("tools/browser_testing/browser_gate_suites/fixtures/dom_oracle/manifests/routes.csv");

    let src =
        std::fs::read_to_string(&router).with_context(|| format!("read {}", router.display()))?;
    // Isolate the ROUTES array body so the `struct RouteDef { … }` definition isn't parsed.
    let body_re = Regex::new(r"(?s)static ROUTES[^=]*=\s*&\[(.*)\];")?;
    let body = body_re
        .captures(&src)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
        .unwrap_or("");

    let def_re =
        Regex::new(r"(?s)RouteDef\s*\{(.*?)\}").expect("the pattern is a valid regular expression"); // no nested braces in a RouteDef
    let field = |c: &str, k: &str| -> String {
        Regex::new(&format!(r#"{k}:\s*"([^"]*)""#))
            .expect("the pattern is a valid regular expression")
            .captures(c)
            .and_then(|m| m.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default()
    };
    let flag = |c: &str, k: &str| -> bool {
        Regex::new(&format!(r"{k}:\s*true"))
            .expect("the pattern is a valid regular expression")
            .is_match(c)
    };

    let mut rows: Vec<[String; 5]> = Vec::new();
    for m in def_re.captures_iter(body) {
        let c = m.get(1).map(|x| x.as_str()).unwrap_or("");
        rows.push([
            field(c, "path"),
            field(c, "component"),
            flag(c, "full_bleed").to_string(),
            flag(c, "chromeless").to_string(),
            field(c, "auth"),
        ]);
    }
    rows.sort_by(|a, b| a[0].cmp(&b[0]));
    let leptos_csv = std::iter::once("path,component,fullBleed,chromeless,router_auth".to_string())
        .chain(rows.iter().map(|r| r.join(",")))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";

    let oracle = std::fs::read_to_string(&oracle_path)
        .with_context(|| format!("read {}", oracle_path.display()))?;

    if leptos_csv == oracle {
        println!(
            "{}",
            serde_json::to_string_pretty(
                &json!({ "gate": "S-routes", "pass": true, "routes": rows.len() })
            )?
        );
        return Ok(0);
    }

    // Report the row-level diff.
    let to_map = |csv: &str| -> Vec<(String, String)> {
        csv.trim()
            .lines()
            .skip(1)
            .map(|l| (l.split(',').next().unwrap_or("").to_string(), l.to_string()))
            .collect()
    };
    let lm = to_map(&leptos_csv);
    let om = to_map(&oracle);
    let lookup = |m: &[(String, String)], k: &str| -> Option<String> {
        m.iter().find(|(p, _)| p == k).map(|(_, l)| l.clone())
    };
    let mut diffs: Vec<(String, String, String)> = Vec::new();
    for (path, line) in &om {
        match lookup(&lm, path) {
            None => diffs.push((path.clone(), line.clone(), "(missing)".into())),
            Some(l) if &l != line => diffs.push((path.clone(), line.clone(), l)),
            _ => {}
        }
    }
    for (path, line) in &lm {
        if lookup(&om, path).is_none() {
            diffs.push((path.clone(), "(missing)".into(), line.clone()));
        }
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "gate": "S-routes", "pass": false,
            "oracle": om.len(), "leptos": lm.len(), "diffs": diffs.len(),
        }))?
    );
    for (path, o, l) in diffs.iter().take(40) {
        println!("  {path}\n    oracle: {o}\n    leptos: {l}");
    }
    Ok(1)
}
