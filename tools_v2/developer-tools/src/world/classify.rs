//! T-165.8 — shared prefab classifier + raw-JSONL streaming (port of
//! `scripts/map-assets/lib/classify-prefab.mjs`). ONE source of truth wrapping
//! `packages/tbd-schema/rules/prefab-classify.json`: first rule whose
//! `match.resourceNameContains` substring appears wins (rule order = priority; substring
//! match case-sensitive); otherwise the file's `fallback` (kind=prop, class=unknown).

use std::collections::HashMap;
use std::io::BufRead as _;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::serve::repo_root;

pub struct Rules {
    pub doc: Value,
}

pub struct Classification {
    pub kind: String,
    pub class: String,
    pub matched: bool,
    /// Index into `rules.doc["rules"]`, or None = fallback.
    pub rule_idx: Option<usize>,
}

pub fn load_rules() -> Result<Rules> {
    let p = repo_root().join("packages/tbd-schema/rules/prefab-classify.json");
    let doc: Value = serde_json::from_str(
        &std::fs::read_to_string(&p).with_context(|| p.display().to_string())?,
    )?;
    Ok(Rules { doc })
}

impl Rules {
    pub fn rule(&self, idx: Option<usize>) -> &Value {
        match idx {
            Some(i) => &self.doc["rules"][i],
            None => &self.doc["fallback"],
        }
    }

    fn classify_uncached(&self, resource_name: &str) -> Classification {
        if let Some(rules) = self.doc["rules"].as_array() {
            for (i, rule) in rules.iter().enumerate() {
                let needles = rule["match"]["resourceNameContains"].as_array();
                let hit = needles.is_some_and(|ns| {
                    ns.iter().any(|n| {
                        n.as_str()
                            .is_some_and(|n| !n.is_empty() && resource_name.contains(n))
                    })
                });
                if hit {
                    return Classification {
                        kind: rule["kind"].as_str().unwrap_or_default().to_string(),
                        class: rule["class"].as_str().unwrap_or_default().to_string(),
                        matched: true,
                        rule_idx: Some(i),
                    };
                }
            }
        }
        let fb = &self.doc["fallback"];
        Classification {
            kind: fb["kind"].as_str().unwrap_or("prop").to_string(),
            class: fb["class"].as_str().unwrap_or("unknown").to_string(),
            matched: false,
            rule_idx: None,
        }
    }
}

/// Memoized classifier over a full-map stream (≈1M rows over a few-k unique names).
pub struct Classifier<'r> {
    rules: &'r Rules,
    memo: HashMap<String, (String, String, bool, Option<usize>)>,
}

impl<'r> Classifier<'r> {
    pub fn new(rules: &'r Rules) -> Self {
        Classifier {
            rules,
            memo: HashMap::new(),
        }
    }

    pub fn classify(&mut self, resource_name: &str) -> Classification {
        if let Some((kind, class, matched, idx)) = self.memo.get(resource_name) {
            return Classification {
                kind: kind.clone(),
                class: class.clone(),
                matched: *matched,
                rule_idx: *idx,
            };
        }
        let c = self.rules.classify_uncached(resource_name);
        self.memo.insert(
            resource_name.to_string(),
            (c.kind.clone(), c.class.clone(), c.matched, c.rule_idx),
        );
        c
    }
}

/// Stream a raw-entities.jsonl of any size. Parse errors are FATAL by design (the pipeline's
/// count gates are exact-integer identities; a truncated copy must never survive to a census).
/// Returns the non-empty line count.
pub fn stream_raw_entities(path: &Path, mut on_row: impl FnMut(&Value)) -> Result<u64> {
    let f = std::fs::File::open(path).with_context(|| path.display().to_string())?;
    let reader = std::io::BufReader::with_capacity(1 << 20, f);
    let mut line_number = 0u64;
    let mut line_count = 0u64;
    for line in reader.lines() {
        let line = line?;
        line_number += 1;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let row: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => bail!(
                "streamRawEntities: parse error at {}:{line_number} — {e}",
                path.display()
            ),
        };
        line_count += 1;
        on_row(&row);
    }
    Ok(line_count)
}
