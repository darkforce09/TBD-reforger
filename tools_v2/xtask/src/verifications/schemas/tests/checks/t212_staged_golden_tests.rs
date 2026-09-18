use super::{read_json, repo_root, schema_root};
use serde_json::Value;

/// The reader's three struct bodies, concatenated. Scoped to those so an identifier that
/// happens to appear elsewhere in the file cannot satisfy the assertion by accident.
fn reader_struct_bodies() -> String {
    let src = std::fs::read_to_string(repo_root().expect("repo root").join(
        "apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/TBD_ObjectiveRegistry.c",
    ))
    .expect("read TBD_ObjectiveRegistry.c");

    let mut out = String::new();
    for name in [
        "class TBD_ObjectiveEntityStruct",
        "class TBD_ObjectiveFramingStruct",
        "class TBD_ObjectiveFramingSideStruct",
    ] {
        let at = src
            .find(name)
            .unwrap_or_else(|| panic!("{name} must exist in the reader"));
        let end = src[at..]
            .find("\n}")
            .unwrap_or_else(|| panic!("{name} must be closed"));
        out.push_str(&src[at..at + end]);
        out.push('\n');
    }
    out
}

/// Every key the staged golden's `objectives[]` rows author is a declared member of the reader.
#[test]
fn the_staged_1_3_golden_objectives_row_binds_to_the_reader() {
    let root = repo_root().expect("repo root");
    let golden = read_json(&schema_root(&root).join("golden-missions/schema-1_3-wire-fields.json"))
        .expect("staged 1.3 golden");

    let rows = golden
        .get("objectives")
        .and_then(Value::as_array)
        .expect("the staged golden must carry objectives[]");
    assert!(
        !rows.is_empty(),
        "an empty objectives[] would make this assertion vacuous"
    );

    // Collect the authored keys, one level of nesting deep (framing.attacker.title, ...).
    let mut keys: Vec<String> = Vec::new();
    let push = |k: &String, keys: &mut Vec<String>| {
        if !keys.contains(k) {
            keys.push(k.clone());
        }
    };
    for row in rows {
        let obj = row.as_object().expect("objectives[] rows are objects");
        for (k, v) in obj {
            push(k, &mut keys);
            if let Some(side_map) = v.as_object() {
                for (side_key, side_val) in side_map {
                    push(side_key, &mut keys);
                    if let Some(leaf) = side_val.as_object() {
                        for leaf_key in leaf.keys() {
                            push(leaf_key, &mut keys);
                        }
                    }
                }
            }
        }
    }

    // The staged row is only worth asserting against if it exercises the fields the ticket is
    // about; a golden that dropped them would turn this green for the wrong reason.
    for required in ["framing", "lock", "autoLose", "side", "zoneId"] {
        assert!(
            keys.iter().any(|k| k == required),
            "the staged golden no longer authors '{required}' — it is the only document that \
             reaches this reader, so dropping a field there silently retires the proof"
        );
    }

    let bodies = reader_struct_bodies();
    let unbound: Vec<&String> = keys
        .iter()
        .filter(|k| !bodies.contains(&format!(" {k};")))
        .collect();

    assert!(
        unbound.is_empty(),
        "the staged 1.3 golden authors objectives[] keys the reader declares no member for: \
         {unbound:?}\nJsonLoadContext binds by member name, so those keys are invisible at \
         runtime — the document says one thing and the round does another."
    );
}
