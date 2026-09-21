use super::*;

#[test]
fn frozen_27_matches_live_corpus() {
    let root = repo_root();
    if crate::registry::typed_projection::tree_is_phase2(&root) {
        return;
    }
    let v = load_toml_tree(&root).expect("load");
    let got = union_ticket_keys(&v);
    let expect: BTreeSet<String> = FROZEN_27.iter().map(|s| (*s).to_string()).collect();
    assert_eq!(got, expect, "corpus keys drifted from FROZEN_27");
}

/// T-913.1 key governance — this is the gate `frozen_27_matches_live_corpus` cannot be
/// on a phase-2 tree (it early-returns above). Globs EVERY on-disk `.ai/tickets/T-*.toml`
/// — children included, no registry loader in the way — and demands each top-level key
/// be a mapped encoding-C key or a deliberate [`ALLOWED_NEW`] entry. The NEXT key someone
/// invents is red here until they consciously widen `ALLOWED_NEW` + `TicketFile` +
/// `.ai/tickets/schema.json` in a commit that says so.
#[test]
fn on_disk_keys_are_mapped_or_allowed_new() {
    let root = repo_root();
    let dir = tickets_dir(&root);
    let legal: BTreeSet<&str> = ENCODING_C_KEYS
        .iter()
        .chain(ALLOWED_NEW.iter())
        .copied()
        .collect();
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read tickets dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name().is_some_and(|n| {
                let n = n.to_string_lossy();
                n.starts_with("T-") && n.ends_with(".toml")
            })
        })
        .collect();
    paths.sort();
    let mut offenders = Vec::new();
    for path in &paths {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let text = fs::read_to_string(path).expect("read ticket file");
        let parsed: toml::Value = text
            .parse()
            .unwrap_or_else(|e| panic!("{name}: not TOML: {e}"));
        let table = parsed
            .as_table()
            .unwrap_or_else(|| panic!("{name}: root is not a table"));
        for key in table.keys() {
            if !legal.contains(key.as_str()) {
                offenders.push(format!("{name}: unmapped key `{key}`"));
            }
        }
    }
    assert!(
        paths.len() > 800,
        "governance scan must actually see the corpus (saw {} files)",
        paths.len()
    );
    assert!(
        offenders.is_empty(),
        "on-disk ticket keys outside ENCODING_C_KEYS ∪ ALLOWED_NEW — widen deliberately \
             (ALLOWED_NEW + TicketFile + schema.json, one commit) or remove the key:\n{}",
        offenders.join("\n")
    );
}

/// Pins the consts to the real `TicketFile`: a TOML doc carrying EXACTLY
/// `ENCODING_C_KEYS ∪ ALLOWED_NEW` minus the alias-history keys must deserialize
/// and re-serialize to that same key set. A const key `TicketFile` does not know
/// is silently dropped on parse (no `deny_unknown_fields` — the save path
/// tolerates `slice_plan`), so it would vanish from the output set and fail here.
///
/// T-920.1: `user_story` is the first FROZEN key that is also a serde ALIAS of a
/// live key (`main_goal`) — the two cannot co-occur in one doc (serde refuses the
/// duplicate), and no output ever contains the dead spelling, so the maximal doc
/// carries `main_goal` and the expected set subtracts `user_story`. The alias
/// parse itself is pinned separately (`user_story_alias_maps_to_main_goal`).
#[test]
fn ticket_file_key_set_matches_consts() {
    let legal: BTreeSet<String> = ENCODING_C_KEYS
        .iter()
        .chain(ALLOWED_NEW.iter())
        .filter(|k| **k != "user_story")
        .map(|s| (*s).to_string())
        .collect();
    let maximal = r#"
id = "T-001"
kind = "work"
title = "t"
summary = "s"
class = "chore"
status = "queued"
order = 1
spec = "docs/x.md"
plan = "docs/plans/T-001_plan.md"
executor = "claude-code"
notes = "n"
priority = 1
depends_on = ["T-002"]
unblocks = ["T-003"]
parent = "T-000"
children = ["T-001.1"]
active = "T-001.1"
main_goal = "u"
context = ["why"]
requirement = ["ask"]
current_state = ["today"]
approach = ["steps"]
verify = ["cargo test"]
acceptance = ["a"]
citations = ["docs/x.md"]
shipped_at = "abc123"
created_at = "2026-08-14T10:00:00Z"
completed_at = "2026-08-14T11:00:00Z"
estimated = ["tokens"]
estimate_note = "no receipts era"
migration_legacy = ["old wall"]
owns = ["docs/x.md"]
pack_last = true

[scope]
domain = "repo"
layer = "docs"
"#;
    let doc: toml::Value = maximal.parse().expect("maximal doc parses");
    let doc_keys: BTreeSet<String> = doc.as_table().unwrap().keys().cloned().collect();
    assert_eq!(
        doc_keys, legal,
        "maximal doc must exercise exactly ENCODING_C_KEYS ∪ ALLOWED_NEW minus the user_story alias"
    );
    let file: crate::TicketFile =
        toml::from_str(maximal).expect("maximal doc deserializes as TicketFile");
    let out = serde_json::to_value(&file).expect("TicketFile serializes");
    let out_keys: BTreeSet<String> = out.as_object().unwrap().keys().cloned().collect();
    assert_eq!(
        out_keys, legal,
        "TicketFile round-trip key set drifted from ENCODING_C_KEYS ∪ ALLOWED_NEW"
    );
}

/// T-920.1 governance companion: the frozen `user_story` spelling still PARSES
/// (serde alias) and lands in `main_goal`; serialization never emits it. This is
/// what keeps every pre-rename git revision readable while the on-disk subset
/// rule reports the key as legally vanished.
#[test]
fn user_story_alias_maps_to_main_goal() {
    let legacy = "id = \"T-001\"\nkind = \"work\"\ntitle = \"t\"\nsummary = \"s\"\nstatus = \"idea\"\nuser_story = \"old spelling\"\n\n[scope]\ndomain = \"repo\"\nlayer = \"docs\"\n";
    let file: crate::TicketFile = toml::from_str(legacy).expect("user_story alias parses");
    assert_eq!(file.main_goal.as_deref(), Some("old spelling"));
    let out = serde_json::to_value(&file).expect("serialize");
    let obj = out.as_object().unwrap();
    assert!(obj.contains_key("main_goal"));
    assert!(
        !obj.contains_key("user_story"),
        "the dead spelling must never serialize"
    );
}

#[test]
fn toml_roundtrip_is_byte_identical_to_the_registry_document() {
    let root = repo_root();
    let (parsed, gold) = canonical_registry_document(&root);
    let n = parsed
        .get("tickets")
        .and_then(Value::as_array)
        .map(|a| a.len())
        .expect("tickets[]");
    assert!(n > 0, "the registry document has no tickets");

    let tmp = root.join("target").join("ticket-file-roundtrip");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(tmp.join(crate::repository::TICKETS_DIR)).unwrap();
    save_toml_tree(&tmp, &parsed).expect("save toml tree");
    let reloaded = load_toml_tree(&tmp).expect("load toml tree");
    let emitted = format_json_unicode_preserve(&reloaded).expect("emit");
    assert_eq!(
        reloaded.get("next_id"),
        parsed.get("next_id"),
        "derived next_id must equal stored next_id"
    );
    if emitted != gold {
        let minl = emitted.len().min(gold.len());
        let mut i = 0;
        while i < minl && emitted.as_bytes()[i] == gold.as_bytes()[i] {
            i += 1;
        }
        let lo = i.saturating_sub(80);
        let hi_e = (i + 120).min(emitted.len());
        let hi_o = (i + 120).min(gold.len());
        panic!(
            "mismatch at byte {i} (N={n} parents)\nemitted: {:?}\ngold: {:?}",
            &emitted[lo..hi_e],
            &gold[lo..hi_o]
        );
    }
}

#[test]
fn no_ticket_lost_set_equality() {
    let root = repo_root();
    let v = load_toml_tree(&root).unwrap();
    let (parents, all) = corpus_ids(&v);
    let n = parents.len();
    assert_eq!(
        v.get("tickets").and_then(Value::as_array).map(|a| a.len()),
        Some(n),
        "tickets[].len must equal parent-id set size (measured, not hardcoded)"
    );
    let disk: BTreeSet<String> = on_disk_ids(&root).unwrap().into_iter().collect();
    assert_eq!(
        disk, all,
        "on-disk ids must equal the union of parents, slice plans and slices"
    );
}

#[test]
fn derive_next_id_is_max_plus_one() {
    let t = vec![
        serde_json::json!({"id": "T-001"}),
        serde_json::json!({"id": "T-910"}),
        serde_json::json!({"id": "T-090.6"}),
    ];
    assert_eq!(derive_next_id(&t), 911);
    let planted = vec![serde_json::json!({"id": "T-950"})];
    assert_eq!(derive_next_id(&planted), 951);
}

#[test]
fn perturb_summary_makes_cmp_red() {
    let ticket = serde_json::json!({
        "id": "T-001",
        "title": "x",
        "summary": "hello",
        "status": "shipped",
    });
    let s = ticket_to_toml_string(&ticket, 0).unwrap();
    let (_, back) = ticket_from_toml_str(&s).unwrap();
    assert_eq!(back.get("summary").and_then(Value::as_str), Some("hello"));
    let flipped = s.replace("hello", "hallo");
    let (_, bad) = ticket_from_toml_str(&flipped).unwrap();
    assert_ne!(
        format_json_unicode_preserve(&ticket).unwrap(),
        format_json_unicode_preserve(&bad).unwrap()
    );
}
