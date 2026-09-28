//! The `//! @contract` tag grammar of the mod scripts and the schema view each tag cites.
//!
//! **Role:** parses `@contract <schema>#<pointer>[ (<sub-path>)][ partial]` banner lines and
//! resolves each against `contracts_v2/definitions` into a [`SchemaView`]: the property names a
//! wire key may take, the properties the cited object requires, its enum values, and every
//! property name reachable below it.
//!
//! **Position:** part of `enfscript_source_support`; the contract parity suite asks it what a
//! tagged class, body builder or constant group may carry.
//!
//! **Signals & state:** none; pure functions over the schema files read at call time.
//!
//! **Invariants:** the grammar is closed: a tag with any other shape is an error, never skipped.
//! `<pointer>` is RFC 6901 (`#` and `#/` are the whole document); a pointer that lands on an
//! array of schemas (a `oneOf` list) cites those alternatives. `<sub-path>` walks nested
//! properties from there, dot-separated, `name[]` stepping into the array's items. `partial`
//! declares a projection that reads a subset of the object and so need not carry every required
//! property. Only local `$ref`s are followed; a remote one panics. Alternatives (`oneOf`,
//! `anyOf`) contribute every branch's properties and the properties every branch requires;
//! `allOf` branches contribute theirs; `if`/`then`/`else` contribute properties, never
//! requirements.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde_json::{Value, json};

/// One parsed `@contract` tag.
pub struct ContractTag {
    /// The schema file name in `contracts_v2/definitions`.
    pub schema_file: String,
    /// The RFC 6901 pointer, with its leading `#`.
    pub pointer: String,
    /// Nested property steps below the pointer; `true` steps into the array's items.
    pub sub_path: Vec<(String, bool)>,
    /// The class reads a subset of the cited object.
    pub partial: bool,
    /// The tag as written after `@contract`.
    pub text: String,
}

/// What a cited schema node admits.
#[derive(Default)]
pub struct SchemaView {
    pub properties: BTreeSet<String>,
    /// `patternProperties` regular expressions.
    pub patterns: Vec<String>,
    /// The object declares neither properties nor patterns and admits additional ones.
    pub open: bool,
    pub required: BTreeSet<String>,
    /// The `enum` or `const` string values of the node or of its array items.
    pub values: BTreeSet<String>,
    /// Every property name declared at or below the node, through local `$ref`s.
    pub reachable: BTreeSet<String>,
}

impl SchemaView {
    /// True when `key` is a property of the object, matches one of its patterns, or the object is
    /// open.
    pub fn admits(&self, key: &str) -> bool {
        self.open
            || self.properties.contains(key)
            || self.patterns.iter().any(|pattern| {
                jsonschema::validator_for(&json!({"type": "string", "pattern": pattern}))
                    .unwrap_or_else(|error| panic!("pattern {pattern}: {error}"))
                    .is_valid(&json!(key))
            })
    }
}

/// The `@contract` tags of a banner, in order.
///
/// Returns every malformed tag as an error message.
pub fn contract_tags(banner: &[String]) -> Result<Vec<ContractTag>, String> {
    banner
        .iter()
        .filter_map(|line| line.strip_prefix("@contract"))
        .map(|rest| parse_tag(rest.trim()))
        .collect()
}

fn parse_tag(text: &str) -> Result<ContractTag, String> {
    let malformed = |why: &str| format!("malformed `@contract {text}`: {why}");
    let mut words = text.split_whitespace();
    let citation = words.next().ok_or_else(|| malformed("no citation"))?;
    let (schema_file, pointer) = citation
        .split_once('#')
        .ok_or_else(|| malformed("no `#` pointer"))?;
    if !schema_file.ends_with(".schema.json") || !(pointer.is_empty() || pointer.starts_with('/')) {
        return Err(malformed("expected <file>.schema.json#<pointer>"));
    }
    let mut tag = ContractTag {
        schema_file: schema_file.to_owned(),
        pointer: format!("#{pointer}"),
        sub_path: Vec::new(),
        partial: false,
        text: text.to_owned(),
    };
    for word in words {
        if let Some(path) = word.strip_prefix('(').and_then(|w| w.strip_suffix(')')) {
            if !tag.sub_path.is_empty() || tag.partial {
                return Err(malformed("the sub-path comes once, before `partial`"));
            }
            for step in path.split('.') {
                let (name, items) = match step.strip_suffix("[]") {
                    Some(name) => (name, true),
                    None => (step, false),
                };
                if name.is_empty() {
                    return Err(malformed("empty sub-path step"));
                }
                tag.sub_path.push((name.to_owned(), items));
            }
        } else if word == "partial" && !tag.partial {
            tag.partial = true;
        } else {
            return Err(malformed(&format!("unexpected `{word}`")));
        }
    }
    Ok(tag)
}

fn definitions_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../contracts_v2/definitions")
}

/// The schema document `file`.
pub fn schema_document(file: &str) -> Value {
    let path = definitions_dir().join(file);
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    serde_json::from_str(&raw).unwrap_or_else(|error| panic!("{file} is not JSON: {error}"))
}

fn pointer_target<'a>(document: &'a Value, pointer: &str, context: &str) -> &'a Value {
    let path = pointer.strip_prefix('#').unwrap_or(pointer);
    if path.is_empty() || path == "/" {
        return document;
    }
    document
        .pointer(path)
        .unwrap_or_else(|| panic!("{context}: {pointer} resolves to nothing"))
}

fn dereference<'a>(document: &'a Value, mut node: &'a Value, context: &str) -> &'a Value {
    let mut hops = 0;
    while let Some(target) = node.get("$ref").and_then(Value::as_str) {
        assert!(
            target.starts_with('#'),
            "{context}: remote $ref {target} is not followed"
        );
        hops += 1;
        assert!(hops < 32, "{context}: $ref cycle at {target}");
        node = pointer_target(document, target, context);
    }
    node
}

/// Resolves `tag` into the view of the node it cites.
pub fn resolve(tag: &ContractTag) -> SchemaView {
    let document = schema_document(&tag.schema_file);
    let context = format!("{}{}", tag.schema_file, tag.pointer);
    let mut node = pointer_target(&document, &tag.pointer, &context);
    for (name, items) in &tag.sub_path {
        let object = dereference(&document, node, &context);
        node = object
            .get("properties")
            .and_then(|properties| properties.get(name))
            .unwrap_or_else(|| panic!("{context}: sub-path step `{name}` is not a property"));
        if *items {
            node = dereference(&document, node, &context)
                .get("items")
                .unwrap_or_else(|| panic!("{context}: `{name}[]` is not an array"));
        }
    }
    view_of(&document, node, &context)
}

fn view_of(document: &Value, node: &Value, context: &str) -> SchemaView {
    let mut node = dereference(document, node, context);
    let mut view = SchemaView::default();
    if let Value::Array(alternatives) = node {
        add_alternatives(document, alternatives, &mut view, true, context);
    } else {
        if let Some(items) = node
            .get("items")
            .filter(|_| node.get("properties").is_none())
        {
            node = dereference(document, items, context);
        }
        collect(document, node, &mut view, true, context);
        view.open = view.properties.is_empty()
            && view.patterns.is_empty()
            && node.get("additionalProperties") != Some(&Value::Bool(false));
        let values = node.get("enum").and_then(Value::as_array).cloned();
        let constant = node.get("const").map(|value| vec![value.clone()]);
        for value in values.or(constant).unwrap_or_default() {
            if let Some(text) = value.as_str() {
                view.values.insert(text.to_owned());
            }
        }
    }
    let mut seen = BTreeSet::new();
    reach(document, node, &mut view.reachable, &mut seen, context);
    view
}

fn collect(document: &Value, node: &Value, view: &mut SchemaView, required: bool, context: &str) {
    let node = dereference(document, node, context);
    if let Some(properties) = node.get("properties").and_then(Value::as_object) {
        view.properties.extend(properties.keys().cloned());
    }
    if let Some(patterns) = node.get("patternProperties").and_then(Value::as_object) {
        view.patterns.extend(patterns.keys().cloned());
    }
    if required {
        let names = node.get("required").and_then(Value::as_array);
        for name in names.into_iter().flatten().filter_map(Value::as_str) {
            view.required.insert(name.to_owned());
        }
    }
    for branch in node
        .get("allOf")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        collect(document, branch, view, required, context);
    }
    for keyword in ["then", "else"] {
        if let Some(branch) = node.get(keyword) {
            collect(document, branch, view, false, context);
        }
    }
    for keyword in ["oneOf", "anyOf"] {
        if let Some(alternatives) = node.get(keyword).and_then(Value::as_array) {
            add_alternatives(document, alternatives, view, required, context);
        }
    }
}

fn add_alternatives(
    document: &Value,
    alternatives: &[Value],
    view: &mut SchemaView,
    required: bool,
    context: &str,
) {
    let mut common: Option<BTreeSet<String>> = None;
    for alternative in alternatives {
        let mut branch = SchemaView::default();
        collect(document, alternative, &mut branch, true, context);
        view.properties.extend(branch.properties);
        view.patterns.extend(branch.patterns);
        common = Some(match common {
            None => branch.required,
            Some(names) => names.intersection(&branch.required).cloned().collect(),
        });
    }
    if required {
        view.required.extend(common.unwrap_or_default());
    }
}

fn reach(
    document: &Value,
    node: &Value,
    out: &mut BTreeSet<String>,
    seen: &mut BTreeSet<String>,
    context: &str,
) {
    match node {
        Value::Object(map) => {
            let target = map.get("$ref").and_then(Value::as_str);
            if target.is_some_and(|target| seen.insert(target.to_owned())) {
                let next = dereference(document, node, context);
                reach(document, next, out, seen, context);
            }
            if let Some(properties) = map.get("properties").and_then(Value::as_object) {
                out.extend(properties.keys().cloned());
            }
            for (key, value) in map {
                if key != "$ref" {
                    reach(document, value, out, seen, context);
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                reach(document, value, out, seen, context);
            }
        }
        _ => {}
    }
}
