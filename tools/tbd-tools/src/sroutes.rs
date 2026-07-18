//! T-165.5 — S-routes gate (port of `manifests/extract-leptos-routes.mjs`).
//!
//! Extracts the route table from `apps/website/frontend/src/router.rs` and diffs it against the
//! frozen React oracle manifest `manifests/routes.csv`. Robust to rustfmt line-wrapping: splits
//! on `RouteDef { … }` blocks and pulls each field by name.
//!
//! Exit 0 = the Leptos route table set/column-diffs equal to routes.csv; 1 = drift (printed).

use anyhow::{Context, Result};
use regex::Regex;
use serde_json::json;

use crate::serve::repo_root;

pub fn run() -> Result<u8> {
    let root = repo_root();
    let router = root.join("apps/website/frontend/src/router.rs");
    let oracle_path = root.join(".ai/artifacts/t159_gates/manifests/routes.csv");

    let src =
        std::fs::read_to_string(&router).with_context(|| format!("read {}", router.display()))?;
    // Isolate the ROUTES array body so the `struct RouteDef { … }` definition isn't parsed.
    let body_re = Regex::new(r"(?s)static ROUTES[^=]*=\s*&\[(.*)\];").unwrap();
    let body = body_re
        .captures(&src)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
        .unwrap_or("");

    let def_re = Regex::new(r"(?s)RouteDef\s*\{(.*?)\}").unwrap(); // no nested braces in a RouteDef
    let field = |c: &str, k: &str| -> String {
        Regex::new(&format!(r#"{k}:\s*"([^"]*)""#))
            .unwrap()
            .captures(c)
            .and_then(|m| m.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default()
    };
    let flag =
        |c: &str, k: &str| -> bool { Regex::new(&format!(r"{k}:\s*true")).unwrap().is_match(c) };

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
