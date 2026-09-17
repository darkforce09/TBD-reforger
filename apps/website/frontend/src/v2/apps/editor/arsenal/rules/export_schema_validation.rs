//! Document checks and bounded schema pattern matching.

use super::export_schema_contract::{
    schema_at, schema_deref, schema_type_matches, schema_type_of, SchemaFaults,
    SUPPORTED_SCHEMA_KEYWORDS,
};
use super::*;

/// Checks one document value against a schema node, including referenced rules.
pub(super) fn check_schema_node(
    root: &serde_json::Value,
    node: &serde_json::Value,
    doc: &serde_json::Value,
    path: &str,
    out: &mut SchemaFaults,
) {
    // A 2020-12 `$ref` applies *alongside* its siblings. This checker replaces the node with its
    // target, so a `$ref` carrying a real assertion beside it would have that assertion dropped —
    // an unexamined constraint, i.e. the failure mode this whole module is built to refuse.
    if let Some(map) = node.as_object() {
        if map.contains_key("$ref") {
            let extra: Vec<&str> = map
                .keys()
                .map(String::as_str)
                .filter(|k| !matches!(*k, "$ref" | "description" | "title"))
                .collect();
            if !extra.is_empty() {
                out.refuse(format!(
                    "{}: the schema puts {} beside a $ref, which this importer would drop — refusing rather than skipping the check",
                    schema_at(path),
                    extra.join(", ")
                ));
                return;
            }
        }
    }
    let Some(node) = schema_deref(root, node) else {
        out.refuse(format!(
            "{}: the schema uses a $ref this importer cannot resolve — refusing rather than skipping the check",
            schema_at(path)
        ));
        return;
    };
    // This checker implements only object subschemas; any other form must be refused.
    let Some(map) = node.as_object() else {
        out.refuse(format!(
            "{}: the schema puts {} where a subschema belongs — this importer implements only object subschemas, so it refuses rather than skipping the check",
            schema_at(path),
            schema_type_of(node)
        ));
        return;
    };

    for key in map.keys() {
        if !SUPPORTED_SCHEMA_KEYWORDS.contains(&key.as_str()) {
            out.refuse(format!(
                "{}: the shipped schema uses `{key}`, which this importer does not implement — refusing rather than accepting a document it only partly checked",
                schema_at(path)
            ));
        }
    }

    // A type mismatch makes every other keyword at this node noise, so it short-circuits.
    if let Some(t) = map.get("type") {
        let names: Vec<&str> = match t {
            serde_json::Value::String(s) => vec![s.as_str()],
            serde_json::Value::Array(a) => a.iter().filter_map(serde_json::Value::as_str).collect(),
            _ => Vec::new(),
        };
        if !names.is_empty() && !names.iter().any(|n| schema_type_matches(n, doc)) {
            out.fault(format!(
                "{}: expected {}, found {}",
                schema_at(path),
                names.join(" or "),
                schema_type_of(doc)
            ));
            return;
        }
    }
    if let Some(c) = map.get("const") {
        if doc != c {
            out.fault(format!("{}: must be {c}, found {doc}", schema_at(path)));
        }
    }
    if let Some(e) = map.get("enum").and_then(serde_json::Value::as_array) {
        if !e.contains(doc) {
            out.fault(format!(
                "{}: {doc} is outside the closed vocabulary {}",
                schema_at(path),
                serde_json::Value::Array(e.clone())
            ));
        }
    }
    if let (Some(m), Some(s)) = (
        map.get("minLength").and_then(serde_json::Value::as_u64),
        doc.as_str(),
    ) {
        if (s.chars().count() as u64) < m {
            out.fault(format!(
                "{}: must be at least {m} character(s), found {}",
                schema_at(path),
                s.chars().count()
            ));
        }
    }
    if let (Some(m), Some(n)) = (
        map.get("minimum").and_then(serde_json::Value::as_f64),
        doc.as_f64(),
    ) {
        if n < m {
            out.fault(format!(
                "{}: must be at least {m}, found {n}",
                schema_at(path)
            ));
        }
    }
    if let Some(branches) = map.get("oneOf").and_then(serde_json::Value::as_array) {
        check_schema_one_of(root, branches, doc, path, out);
    }
    if let Some(items) = map.get("items") {
        // The single-subschema form is the only one implemented. The tuple form is refused at the
        // KEYWORD, not per element: a fix that fired inside the loop would still be silent over the
        // empty array, i.e. over the document that exercises the constraint least.
        if items.is_object() {
            for (i, v) in doc.as_array().into_iter().flatten().enumerate() {
                check_schema_node(root, items, v, &format!("{path}/{i}"), out);
            }
        } else {
            out.refuse(format!(
                "{}: `items` is given as {} where this importer implements only a single subschema (the tuple form is not implemented) — refusing rather than skipping the check",
                schema_at(path),
                schema_type_of(items)
            ));
        }
    }
    if let Some(fields) = doc.as_object() {
        check_schema_object(root, map, fields, path, out);
    }
}

/// `required` + `properties` + `patternProperties` + the `additionalProperties` applicator.
fn check_schema_object(
    root: &serde_json::Value,
    schema: &serde_json::Map<String, serde_json::Value>,
    doc: &serde_json::Map<String, serde_json::Value>,
    path: &str,
    out: &mut SchemaFaults,
) {
    for req in schema
        .get("required")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(k) = req.as_str() else { continue };
        if !doc.contains_key(k) {
            out.fault(format!("{}: missing required key `{k}`", schema_at(path)));
        }
    }
    let props = schema
        .get("properties")
        .and_then(serde_json::Value::as_object);
    let patterns = schema
        .get("patternProperties")
        .and_then(serde_json::Value::as_object);
    // `additionalProperties` is a subschema; `false` is the form that rejects every extra key.
    let additional = schema.get("additionalProperties");
    for (k, v) in doc {
        let child = format!("{path}/{k}");
        let mut matched = false;
        if let Some(spec) = props.and_then(|p| p.get(k)) {
            matched = true;
            check_schema_node(root, spec, v, &child, out);
        }
        for (pattern, spec) in patterns.into_iter().flatten() {
            match anchored_pattern_matches(pattern, k) {
                Some(true) => {
                    matched = true;
                    check_schema_node(root, spec, v, &child, out);
                }
                Some(false) => {}
                None => out.refuse(format!(
                    "{child}: the schema's key pattern `{pattern}` uses a construct this importer cannot evaluate — refusing rather than skipping the check"
                )),
            }
        }
        if matched {
            continue;
        }
        match additional {
            // Absent, or the `true` boolean schema: every remaining key is allowed.
            None | Some(serde_json::Value::Bool(true)) => {}
            Some(serde_json::Value::Bool(false)) => out.fault(format!(
                "{}: `{k}` is not in the schema and additionalProperties is false",
                schema_at(path)
            )),
            Some(sub) if sub.is_object() => check_schema_node(root, sub, v, &child, out),
            Some(other) => out.refuse(format!(
                "{}: `additionalProperties` is given as {} where this importer implements only a boolean or a subschema — refusing rather than skipping the check",
                schema_at(path),
                schema_type_of(other)
            )),
        }
    }
}

/// `oneOf` with the schema's own `loadoutVersion` const as the discriminator.
///
/// A generic `oneOf` failure ("none of 2 branches matched") is useless to an author, because it
/// hands back both branches' complaints and half of them are about the version they did not write.
/// So when every branch fails, this reports the branch whose `loadoutVersion` const the document
/// actually claims — and when it claims none of them, says exactly that instead.
fn check_schema_one_of(
    root: &serde_json::Value,
    branches: &[serde_json::Value],
    doc: &serde_json::Value,
    path: &str,
    out: &mut SchemaFaults,
) {
    let per_branch: Vec<SchemaFaults> = branches
        .iter()
        .map(|b| {
            let mut errs = SchemaFaults::default();
            check_schema_node(root, b, doc, path, &mut errs);
            errs
        })
        .collect();
    // Refusals survive branch selection because an unread rule applies regardless of
    // which branch the document matches. Faults from losing branches do not.
    for branch in &per_branch {
        for refusal in &branch.refusals {
            out.refuse(refusal.clone());
        }
    }
    // A branch this build could not fully evaluate has not "passed" — it is unexamined, and
    // counting it as a match is the same error one level down.
    let passing = per_branch.iter().filter(|e| e.clean()).count();
    if passing == 1 {
        return;
    }
    if passing > 1 {
        // Not reachable with the shipped schema (the version const separates the branches), but a
        // schema edit could make it so, and "matched two mutually exclusive shapes" is a real fault.
        out.fault(format!(
            "{}: the document satisfies {passing} mutually exclusive schema branches",
            schema_at(path)
        ));
        return;
    }
    let claimed = doc.get("loadoutVersion");
    let hit = branches.iter().position(|b| {
        claimed.is_some() && b.pointer("/properties/loadoutVersion/const") == claimed
    });
    if let Some(i) = hit {
        for f in &per_branch[i].faults {
            out.fault(f.clone());
        }
        return;
    }
    let versions: Vec<String> = branches
        .iter()
        .filter_map(|b| b.pointer("/properties/loadoutVersion/const"))
        .map(|v| v.to_string())
        .collect();
    if versions.is_empty() {
        // The schema stopped discriminating on version — report everything rather than nothing.
        for f in per_branch.into_iter().flat_map(|b| b.faults) {
            out.fault(f);
        }
        return;
    }
    out.fault(format!(
        "{}: `loadoutVersion` must be one of {} — this is not a loadout-export document",
        schema_at(path),
        versions.join(" / ")
    ));
}

/* ───── the tiny anchored-pattern matcher `patternProperties` needs ───── */

/// One term of a parsed pattern: a character class with a repetition range.
pub(super) struct PatTerm {
    negated: bool,
    ranges: Vec<(char, char)>,
    min: usize,
    max: usize,
}

/// Terms beyond this, and the backtracking below stops being obviously cheap. The shipped schema
/// uses two; a pattern needing nine is a pattern this matcher should refuse rather than run.
const MAX_PATTERN_TERMS: usize = 8;

/// Keys longer than this are not evaluated at all. A 512-character JSON key is not loadout data,
/// and refusing beats spending unbounded backtracking on it.
pub(super) const MAX_PATTERN_INPUT: usize = 512;

/// Match `text` against an **anchored** regex from the subset `loadout-export.schema.json` uses:
/// `^`, `$`, character classes (`[a-zA-Z0-9_]`, `[^…]`), single literal characters, and the
/// quantifiers `{m,n}` / `{m,}` / `{m}` / `*` / `+` / `?`.
///
/// * `Some(true)` / `Some(false)` — the pattern was fully evaluated.
/// * **`None` — the pattern uses a construct this matcher does not implement, and the caller must
///   REFUSE.** That is the whole point of the third return value: a pattern we cannot evaluate is
///   not a pattern we may ignore. Alternation, groups, `.`, backslash escapes and unanchored
///   patterns all land here.
///
/// A full regex engine is not the right answer for one `patternProperties` entry, and pretending
/// the pattern is a rubber stamp is how mod-added wear keys would smuggle themselves past a closed
/// object. This is the honest middle: evaluate what it can, refuse what it cannot.
pub fn anchored_pattern_matches(pattern: &str, text: &str) -> Option<bool> {
    let terms = parse_anchored_pattern(pattern)?;
    let chars: Vec<char> = text.chars().collect();
    if chars.len() > MAX_PATTERN_INPUT {
        return None;
    }
    Some(match_pattern_terms(&terms, &chars))
}

/// Parses the bounded, anchored pattern subset accepted by the schema checker.
pub(super) fn parse_anchored_pattern(pattern: &str) -> Option<Vec<PatTerm>> {
    let cs: Vec<char> = pattern.chars().collect();
    if cs.first() != Some(&'^') || cs.last() != Some(&'$') || cs.len() < 2 {
        return None; // unanchored — this matcher makes no claim about partial matches.
    }
    let end = cs.len() - 1;
    let mut i = 1usize;
    let mut terms: Vec<PatTerm> = Vec::new();
    while i < end {
        let (negated, ranges) = match cs[i] {
            '[' => {
                let mut j = i + 1;
                let negated = cs.get(j) == Some(&'^');
                if negated {
                    j += 1;
                }
                let mut ranges: Vec<(char, char)> = Vec::new();
                loop {
                    let c = *cs.get(j)?; // unterminated class
                    if c == ']' {
                        break;
                    }
                    if c == '\\' {
                        return None; // escapes: not implemented
                    }
                    if cs.get(j + 1) == Some(&'-') && cs.get(j + 2).is_some_and(|e| *e != ']') {
                        let hi = *cs.get(j + 2)?;
                        if hi == '\\' {
                            return None;
                        }
                        ranges.push((c, hi));
                        j += 3;
                    } else {
                        ranges.push((c, c));
                        j += 1;
                    }
                }
                if ranges.is_empty() {
                    return None;
                }
                i = j + 1;
                (negated, ranges)
            }
            c if c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | ' ' | '/' | ':') => {
                i += 1;
                (false, vec![(c, c)])
            }
            // `.`, `(`, `|`, `\`, `^`, `$` mid-pattern, and anything else: not implemented.
            _ => return None,
        };
        let (min, max) = match cs.get(i) {
            Some('{') => {
                let mut j = i + 1;
                let mut inner = String::new();
                while j < end && cs[j] != '}' {
                    inner.push(cs[j]);
                    j += 1;
                }
                if cs.get(j) != Some(&'}') {
                    return None;
                }
                i = j + 1;
                match inner.split_once(',') {
                    None => {
                        let n: usize = inner.parse().ok()?;
                        (n, n)
                    }
                    Some((lo, "")) => (lo.parse().ok()?, usize::MAX),
                    Some((lo, hi)) => (lo.parse().ok()?, hi.parse().ok()?),
                }
            }
            Some('*') => {
                i += 1;
                (0, usize::MAX)
            }
            Some('+') => {
                i += 1;
                (1, usize::MAX)
            }
            Some('?') => {
                i += 1;
                (0, 1)
            }
            _ => (1, 1),
        };
        if min > max || terms.len() == MAX_PATTERN_TERMS {
            return None;
        }
        terms.push(PatTerm {
            negated,
            ranges,
            min,
            max,
        });
    }
    Some(terms)
}

fn pat_class_matches(t: &PatTerm, ch: char) -> bool {
    t.ranges.iter().any(|(lo, hi)| ch >= *lo && ch <= *hi) != t.negated
}

/// Greedy match with backtracking. Bounded by [`MAX_PATTERN_TERMS`] and [`MAX_PATTERN_INPUT`].
fn match_pattern_terms(terms: &[PatTerm], text: &[char]) -> bool {
    let Some((t, rest)) = terms.split_first() else {
        return text.is_empty();
    };
    let mut n = 0usize;
    while n < text.len() && n < t.max && pat_class_matches(t, text[n]) {
        n += 1;
    }
    loop {
        if n >= t.min && match_pattern_terms(rest, &text[n..]) {
            return true;
        }
        if n == 0 || n - 1 < t.min {
            return false;
        }
        n -= 1;
    }
}
