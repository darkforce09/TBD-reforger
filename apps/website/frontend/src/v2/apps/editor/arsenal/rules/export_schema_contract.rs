//! Supported loadout export schema and fail-closed schema audit.

use super::export_schema_validation::{check_schema_node, parse_anchored_pattern};
use super::*;

/* ═════ checking a document against the shipped export schema ═════ */

/// The shipped export schema, embedded so browser validation uses the same bytes.
pub const LOADOUT_EXPORT_SCHEMA_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../contracts_v2/definitions/loadout-export.schema.json"
));

/// Keywords this checker evaluates or treats as annotations.
/// An unknown keyword causes a refusal before the document is checked.
pub(super) const SUPPORTED_SCHEMA_KEYWORDS: &[&str] = &[
    // annotations — no effect on validity
    "$schema",
    "$id",
    "title",
    "description",
    "$defs",
    // implemented assertions
    "$ref",
    "oneOf",
    "type",
    "const",
    "enum",
    "required",
    "properties",
    "patternProperties",
    "additionalProperties",
    "items",
    "minLength",
    "minimum",
];

/// Keywords whose named values are subschemas; the names are not assertions.
const SCHEMA_NAMED_SUBSCHEMAS: &[&str] = &["properties", "patternProperties", "$defs"];

/// The keywords whose value is a single subschema (where it is not a boolean).
const SCHEMA_SUBSCHEMA_KEYWORDS: &[&str] = &["items", "additionalProperties"];

/// How many refusals to carry back. A malformed document can fail every key it has; the author
/// needs the first handful to act, not a wall.
const MAX_SCHEMA_FAULTS: usize = 12;

/// Document faults and unsupported-schema refusals kept separate.
/// A refusal survives branch selection because the checker cannot claim to validate an unread rule.
#[derive(Default)]
pub(super) struct SchemaFaults {
    /// The document broke a rule this build implements.
    pub(super) faults: Vec<String>,
    /// This build could not read part of the rule set. Survives branch selection.
    pub(super) refusals: Vec<String>,
}

impl SchemaFaults {
    /// Records a document value that fails an implemented schema rule.
    pub(super) fn fault(&mut self, msg: String) {
        self.faults.push(msg);
    }

    /// Deduplicated: one `$defs` node reached from both `oneOf` branches is one refusal, not two.
    pub(super) fn refuse(&mut self, msg: String) {
        if !self.refusals.contains(&msg) {
            self.refusals.push(msg);
        }
    }

    /// Nothing to report — neither a rule broken nor a rule unread.
    pub(super) fn clean(&self) -> bool {
        self.faults.is_empty() && self.refusals.is_empty()
    }

    /// Refusals first: "this build cannot check X" outranks "your document got Y wrong", because
    /// an author can fix every Y and still be looking at an X nobody checked.
    fn into_messages(self) -> Vec<String> {
        let mut all = self.refusals;
        all.extend(self.faults);
        all
    }
}

/// Trim a verdict to [`MAX_SCHEMA_FAULTS`], saying how many were dropped.
fn cap_schema_messages(mut msgs: Vec<String>) -> Vec<String> {
    let total = msgs.len();
    if total > MAX_SCHEMA_FAULTS {
        msgs.truncate(MAX_SCHEMA_FAULTS);
        msgs.push(format!(
            "…and {} more schema fault(s).",
            total - MAX_SCHEMA_FAULTS
        ));
    }
    msgs
}

/// Checks a document against the embedded export schema.
/// Unsupported rules and malformed references cause refusals, never a partial validity claim.
pub fn validate_against_loadout_export_schema(doc: &serde_json::Value) -> Result<(), Vec<String>> {
    let root: serde_json::Value = match serde_json::from_str(LOADOUT_EXPORT_SCHEMA_JSON) {
        Ok(v) => v,
        Err(e) => {
            return Err(vec![format!(
                "the shipped loadout-export schema did not parse ({e}) — refusing to import against a rule set this build cannot read"
            )])
        }
    };
    validate_against_schema(&root, doc)
}

/// Audits an arbitrary schema, then checks the document against understood rules.
/// The audit covers every schema position even when the document would not visit it.
pub(super) fn validate_against_schema(
    root: &serde_json::Value,
    doc: &serde_json::Value,
) -> Result<(), Vec<String>> {
    let mut support = SchemaFaults::default();
    audit_schema_support(root, root, "#", &mut Vec::new(), &mut support);
    if !support.clean() {
        // No document verdict at all. "Your `qty` is wrong" alongside "and I could not read four
        // other rules" reads as a normal validation failure, and the author fixes `qty` and ships.
        return Err(cap_schema_messages(support.into_messages()));
    }
    let mut out = SchemaFaults::default();
    check_schema_node(root, root, doc, "", &mut out);
    if out.clean() {
        return Ok(());
    }
    Err(cap_schema_messages(out.into_messages()))
}

/// Walks every subschema position and records constructs this checker cannot evaluate.
/// Reference tracking bounds recursion; a chained reference is refused because the document walk follows one hop.
fn audit_schema_support(
    root: &serde_json::Value,
    node: &serde_json::Value,
    path: &str,
    visited: &mut Vec<String>,
    out: &mut SchemaFaults,
) {
    let Some(map) = node.as_object() else {
        out.refuse(format!(
            "{path}: the schema puts {} where a subschema belongs — this importer implements only object subschemas, so it refuses rather than skipping the check",
            schema_type_of(node)
        ));
        return;
    };

    for key in map.keys() {
        if !SUPPORTED_SCHEMA_KEYWORDS.contains(&key.as_str()) {
            out.refuse(format!(
                "{path}: the shipped schema uses `{key}`, which this importer does not implement — refusing rather than accepting a document it only partly checked"
            ));
        }
    }

    // A 2020-12 `$ref` applies *alongside* its siblings; this checker replaces the node with its
    // target, so a sibling assertion would be dropped. Refuse, and stop — the node IS its target.
    if map.contains_key("$ref") {
        let extra: Vec<&str> = map
            .keys()
            .map(String::as_str)
            .filter(|k| !matches!(*k, "$ref" | "description" | "title"))
            .collect();
        if !extra.is_empty() {
            out.refuse(format!(
                "{path}: the schema puts {} beside a $ref, which this importer would drop — refusing rather than skipping the check",
                extra.join(", ")
            ));
            return;
        }
        match (map["$ref"].as_str(), schema_deref(root, node)) {
            (Some(r), Some(target)) => {
                // The document walk follows one reference hop. A target that is another
                // reference leaves its assertions unread, so both chains and cycles are refused.
                if target.get("$ref").is_some() {
                    out.refuse(format!(
                        "{path}: `$ref` points at `{r}`, which is itself a $ref — this importer follows exactly one hop, so it refuses rather than skipping every assertion behind the chain"
                    ));
                    return;
                }
                if !visited.iter().any(|seen| seen == r) {
                    visited.push(r.to_string());
                    audit_schema_support(root, target, r, visited, out);
                }
            }
            _ => out.refuse(format!(
                "{path}: the schema uses a $ref this importer cannot resolve — refusing rather than skipping the check"
            )),
        }
        return;
    }

    // KEYWORD FORMS. Every one of these is a shape the document walk would read as `None` and skip
    // in silence, which is a skipped check wearing the costume of a satisfied one.
    for (key, want, ok) in [
        (
            "type",
            "a type name or a list of type names",
            map.get("type").is_none_or(|v| match v {
                serde_json::Value::String(_) => true,
                serde_json::Value::Array(a) => !a.is_empty() && a.iter().all(|n| n.is_string()),
                _ => false,
            }),
        ),
        (
            "required",
            "a list of key names",
            map.get("required").is_none_or(|v| {
                v.as_array()
                    .is_some_and(|a| a.iter().all(|k| k.is_string()))
            }),
        ),
        (
            "enum",
            "a non-empty list of allowed values",
            map.get("enum")
                .is_none_or(|v| v.as_array().is_some_and(|a| !a.is_empty())),
        ),
        (
            "oneOf",
            "a non-empty list of branch schemas",
            map.get("oneOf")
                .is_none_or(|v| v.as_array().is_some_and(|a| !a.is_empty())),
        ),
        (
            "items",
            "a single subschema (the tuple form is not implemented)",
            map.get("items").is_none_or(serde_json::Value::is_object),
        ),
        (
            "additionalProperties",
            "a boolean or a subschema",
            map.get("additionalProperties")
                .is_none_or(|v| v.is_boolean() || v.is_object()),
        ),
        (
            "minLength",
            "a number",
            map.get("minLength")
                .is_none_or(serde_json::Value::is_number),
        ),
        (
            "minimum",
            "a number",
            map.get("minimum").is_none_or(serde_json::Value::is_number),
        ),
    ] {
        if !ok {
            out.refuse(format!(
                "{path}: `{key}` is given as {} where this importer implements only {want} — refusing rather than skipping the check",
                schema_type_of(&map[key])
            ));
        }
    }
    for named in SCHEMA_NAMED_SUBSCHEMAS {
        if map.get(*named).is_some_and(|v| !v.is_object()) {
            out.refuse(format!(
                "{path}: `{named}` is given as {} where this importer implements only a map of names to subschemas — refusing rather than skipping the check",
                schema_type_of(&map[*named])
            ));
        }
    }
    // A pattern the matcher cannot parse is a refusal HERE, against the schema alone. Raised only
    // from the document walk it needed a document key to land on, so an empty `wear: {}` met an
    // unevaluable pattern with silence.
    for pattern in map
        .get("patternProperties")
        .and_then(serde_json::Value::as_object)
        .into_iter()
        .flatten()
        .map(|(p, _)| p)
    {
        if parse_anchored_pattern(pattern).is_none() {
            out.refuse(format!(
                "{path}: the schema's key pattern `{pattern}` uses a construct this importer cannot evaluate — refusing rather than skipping the check"
            ));
        }
    }

    for named in SCHEMA_NAMED_SUBSCHEMAS {
        let children = map.get(*named).and_then(serde_json::Value::as_object);
        for (name, child) in children.into_iter().flatten() {
            audit_schema_support(root, child, &format!("{path}/{named}/{name}"), visited, out);
        }
    }
    let branches = map.get("oneOf").and_then(serde_json::Value::as_array);
    for (i, branch) in branches.into_iter().flatten().enumerate() {
        audit_schema_support(root, branch, &format!("{path}/oneOf/{i}"), visited, out);
    }
    for single in SCHEMA_SUBSCHEMA_KEYWORDS {
        // Only objects descend, and the two keywords get there differently. `additionalProperties`
        // accepts the boolean schemas `true`/`false`, so its form check passes them and the
        // DOCUMENT WALK implements them ("admits anything" / "closes the object"); they are simply
        // not subschemas to recurse into. `items` accepts no boolean at all — its form check above
        // already REFUSED one, along with the tuple form. So a non-object here is either handled
        // elsewhere or already refused; neither is a check skipped in silence.
        if let Some(child) = map.get(*single).filter(|v| v.is_object()) {
            audit_schema_support(root, child, &format!("{path}/{single}"), visited, out);
        }
    }
}

/// Resolves one local, unescaped JSON pointer hop.
/// Unsupported or ambiguous pointer syntax returns `None` so callers refuse it.
pub(super) fn schema_deref<'a>(
    root: &'a serde_json::Value,
    node: &'a serde_json::Value,
) -> Option<&'a serde_json::Value> {
    let Some(raw) = node.get("$ref") else {
        return Some(node);
    };
    let mut cur = root;
    for seg in raw.as_str()?.strip_prefix("#/")?.split('/') {
        if seg.contains('~') {
            return None;
        }
        cur = cur.get(seg)?;
    }
    Some(cur)
}

/// The path label a fault is reported under — `""` is the document itself.
pub(super) fn schema_at(path: &str) -> &str {
    if path.is_empty() {
        "document"
    } else {
        path
    }
}

/// The JSON type name of a value, in the schema's own vocabulary.
pub(super) fn schema_type_of(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                "integer"
            } else {
                "number"
            }
        }
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

/// Does `doc` satisfy a single `type` name? `number` admits integers (JSON Schema's rule); the
/// reverse is deliberately NOT true — `qty: 1.5` is not an integer here and must not pass.
pub(super) fn schema_type_matches(name: &str, doc: &serde_json::Value) -> bool {
    match name {
        "number" => doc.is_number(),
        other => schema_type_of(doc) == other,
    }
}
