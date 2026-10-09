//! Block goldens: nested lists, tables and the formatting guide seed, each checked against the
//! `WikiBlock` definition of `contracts/definitions/wiki-page.schema.json`.

use serde_json::{Value, json};

use super::read_markup;

/// The wiki page contract, read from the repository's contract definitions.
fn wiki_page_schema() -> Value {
    let root = repository_root::find_repository_root_from(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
    .expect("the repository root above the crate");
    let path = root.join("contracts/definitions/wiki-page.schema.json");
    let raw = std::fs::read_to_string(&path).expect("read wiki-page.schema.json");
    serde_json::from_str(&raw).expect("wiki-page.schema.json is JSON")
}

/// Asserts `value` is valid against `#/definitions/{definition}` of the wiki page contract,
/// naming every violation.
pub(super) fn assert_matches_definition(definition: &str, value: &Value) {
    let document = wiki_page_schema();
    let schema = json!({
        "$schema": document["$schema"],
        "$ref": format!("#/definitions/{definition}"),
        "definitions": document["definitions"],
    });
    let validator = jsonschema::options()
        .build(&schema)
        .expect("wiki-page.schema.json compiles");
    let errors: Vec<String> = validator
        .iter_errors(value)
        .map(|error| format!("{error} at {}", error.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "#/definitions/{definition} rejects the value:\n  {}\n{value:#}",
        errors.join("\n  ")
    );
}

/// Reads `markdown`, asserts it yields no findings and returns its blocks as JSON, each block
/// already checked against the contract.
pub(super) fn golden_blocks(markdown: &str) -> Value {
    let reading = read_markup(markdown);
    assert!(
        reading.findings.is_empty(),
        "unexpected findings for {markdown:?}: {:?}",
        reading.findings
    );
    let blocks = serde_json::to_value(&reading.blocks).expect("blocks serialize");
    for block in blocks.as_array().expect("blocks are an array") {
        assert_matches_definition("WikiBlock", block);
    }
    blocks
}

/// A paragraph holding one text run.
fn paragraph(text: &str) -> Value {
    json!({ "type": "paragraph", "inlines": [{ "type": "text", "text": text }] })
}

#[test]
fn wiki_markup_golden_nested_list_sits_in_its_item() {
    let blocks = golden_blocks("- outer\n  - inner\n");
    assert_eq!(
        blocks,
        json!([{
            "type": "list",
            "ordered": false,
            "items": [{
                "blocks": [
                    paragraph("outer"),
                    {
                        "type": "list",
                        "ordered": false,
                        "items": [{ "blocks": [paragraph("inner")] }],
                    },
                ],
            }],
        }])
    );
}

#[test]
fn wiki_markup_golden_table_with_every_alignment() {
    let markdown = "| Role | Kit | Count | Notes |\n\
                    | :--- | :-: | ---: | --- |\n\
                    | Rifleman | **M16A2** | 4 | `standard` |\n\
                    | Medic |\n";
    let blocks = golden_blocks(markdown);
    let text = |text: &str| json!([{ "type": "text", "text": text }]);
    assert_eq!(
        blocks,
        json!([{
            "type": "table",
            "alignments": ["left", "center", "right", "none"],
            "header": [text("Role"), text("Kit"), text("Count"), text("Notes")],
            "rows": [
                [
                    text("Rifleman"),
                    [{ "type": "strong", "children": [{ "type": "text", "text": "M16A2" }] }],
                    text("4"),
                    [{ "type": "code", "text": "standard" }],
                ],
                [text("Medic"), [], [], []],
            ],
        }])
    );
}

/// The markdown of the `wiki-formatting-guide` page in
/// `crates/api/api_database/seeds/wiki_pages.sql`, decoded from its Postgres `E'…'` literal
/// (`\n`, `\\` and doubled quotes).
fn formatting_guide_markdown() -> String {
    let root = repository_root::find_repository_root_from(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
    .expect("the repository root above the crate");
    let path = root.join("crates/api/api_database/seeds/wiki_pages.sql");
    let seed =
        std::fs::read_to_string(&path).expect("read crates/api/api_database/seeds/wiki_pages.sql");
    let row = seed
        .find("'wiki-formatting-guide'")
        .expect("the seed holds the wiki-formatting-guide page");
    let literal_start = row
        + seed[row..]
            .find("E'")
            .expect("the page body is an E'' literal")
        + 2;
    let mut markdown = String::new();
    let mut characters = seed[literal_start..].chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '\\' => match characters.next() {
                Some('n') => markdown.push('\n'),
                Some(escaped) => markdown.push(escaped),
                None => break,
            },
            '\'' if characters.peek() == Some(&'\'') => {
                characters.next();
                markdown.push('\'');
            }
            '\'' => return markdown,
            other => markdown.push(other),
        }
    }
    panic!("the wiki-formatting-guide body literal is not closed");
}

/// Every object of `value`, depth first, in document order.
fn objects(value: &Value) -> Vec<&Value> {
    let mut found = Vec::new();
    let mut pending = vec![value];
    while let Some(current) = pending.pop() {
        match current {
            Value::Array(items) => pending.extend(items.iter().rev()),
            Value::Object(fields) => {
                found.push(current);
                pending.extend(fields.values().rev());
            }
            _ => {}
        }
    }
    found
}

#[test]
fn wiki_markup_formatting_guide_seed_shows_every_construct_and_saves_clean() {
    let blocks = golden_blocks(&formatting_guide_markdown());
    let nodes = objects(&blocks);
    let of_type = |kind: &str| -> Vec<&Value> {
        nodes
            .iter()
            .copied()
            .filter(|node| node["type"] == kind)
            .collect()
    };

    let levels: std::collections::BTreeSet<i64> = of_type("heading")
        .iter()
        .filter_map(|heading| heading["level"].as_i64())
        .collect();
    assert_eq!(levels, (1..=6).collect());
    for kind in [
        "paragraph",
        "list",
        "table",
        "callout",
        "quote",
        "code",
        "rule",
        "strong",
        "emphasis",
        "strikethrough",
        "image",
        "line_break",
    ] {
        assert!(!of_type(kind).is_empty(), "the guide shows no {kind}");
    }
    let callout_kinds: std::collections::BTreeSet<&str> = of_type("callout")
        .iter()
        .filter_map(|callout| callout["kind"].as_str())
        .collect();
    assert_eq!(
        callout_kinds,
        [
            "caution",
            "critical",
            "important",
            "info",
            "note",
            "tip",
            "warning"
        ]
        .into_iter()
        .collect()
    );
    let table = of_type("table")[0];
    assert_eq!(
        table["alignments"],
        json!(["left", "center", "right", "none"])
    );
    assert!(
        of_type("list").iter().any(|list| list["items"]
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item["checked"] == true)
                && items.iter().any(|item| item["checked"] == false))),
        "the guide shows no checklist with ticked and empty boxes"
    );
    let links: Vec<(&str, bool)> = of_type("link")
        .iter()
        .filter_map(|link| Some((link["href"].as_str()?, link["external"].as_bool()?)))
        .collect();
    for (href, external) in [
        ("/wiki/field-manual", false),
        ("https://reforger.armaplatform.com", true),
        ("#contents", false),
        ("mailto:staff@example.com", true),
    ] {
        assert!(
            links.contains(&(href, external)),
            "no link to {href}: {links:?}"
        );
    }
    let image = of_type("image")[0];
    assert!(
        image["src"]
            .as_str()
            .is_some_and(|src| src.starts_with("/uploads/")),
        "{image}"
    );
}
