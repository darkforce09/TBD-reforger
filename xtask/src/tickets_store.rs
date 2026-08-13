//! Phase-1 per-ticket TOML store.
//!
//! On-disk: `.ai/tickets/T-001.toml` (parents) plus one file per `slice_plan` /
//! `slices[]` child. In-memory: the same `serde_json::Value` shape the rest of
//! xtask already consumes (`{ next_id, tickets: [...] }`), with `next_id`
//! derived from max parent numeric id + 1.

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

const NULL_SENTINEL: &str = "__tbd_null__";
const ORD_KEY: &str = "__ord";
const KEYS_KEY: &str = "__keys";

pub fn tickets_dir(root: &Path) -> PathBuf {
    root.join(".ai/tickets")
}

pub fn root_marker_path(root: &Path) -> PathBuf {
    tickets_dir(root).join("ROOT")
}

pub fn parent_toml_path(root: &Path, id: &str) -> PathBuf {
    tickets_dir(root).join(format!("{id}.toml"))
}

pub fn is_parent_id(id: &str) -> bool {
    let rest = match id.strip_prefix("T-") {
        Some(r) => r,
        None => return false,
    };
    !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
}

pub fn parent_numeric_id(id: &str) -> Option<u64> {
    if !is_parent_id(id) {
        return None;
    }
    id.strip_prefix("T-")?.parse().ok()
}

pub fn derive_next_id(tickets: &[Value]) -> u64 {
    let max = tickets
        .iter()
        .filter_map(|t| t.get("id").and_then(Value::as_str))
        .filter_map(parent_numeric_id)
        .max()
        .unwrap_or(0);
    max + 1
}

fn encode_object(map: &Map<String, Value>) -> Result<toml::Value> {
    let mut table = toml::map::Map::new();
    let keys: Vec<toml::Value> = map.keys().map(|k| toml::Value::String(k.clone())).collect();
    table.insert(KEYS_KEY.into(), toml::Value::Array(keys));
    for (k, val) in map {
        table.insert(k.clone(), encode_json_value(val)?);
    }
    Ok(toml::Value::Table(table))
}

fn encode_json_value(v: &Value) -> Result<toml::Value> {
    Ok(match v {
        Value::Null => toml::Value::String(NULL_SENTINEL.into()),
        Value::Bool(b) => toml::Value::Boolean(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                toml::Value::Integer(i)
            } else if let Some(u) = n.as_u64() {
                if u > i64::MAX as u64 {
                    bail!("integer {u} does not fit toml i64");
                }
                toml::Value::Integer(u as i64)
            } else if let Some(f) = n.as_f64() {
                toml::Value::Float(f)
            } else {
                bail!("unhandled json number {n}");
            }
        }
        Value::String(s) => toml::Value::String(s.clone()),
        Value::Array(arr) => toml::Value::Array(
            arr.iter()
                .map(encode_json_value)
                .collect::<Result<Vec<_>>>()?,
        ),
        Value::Object(map) => encode_object(map)?,
    })
}

fn decode_table(table: &toml::map::Map<String, toml::Value>) -> Result<Value> {
    let order: Vec<String> = match table.get(KEYS_KEY).and_then(|v| v.as_array()) {
        Some(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        None => table
            .keys()
            .filter(|k| *k != ORD_KEY && *k != KEYS_KEY)
            .cloned()
            .collect(),
    };
    let mut map = Map::new();
    for k in order {
        if k == ORD_KEY || k == KEYS_KEY {
            continue;
        }
        let Some(val) = table.get(&k) else {
            continue;
        };
        map.insert(k, toml_to_json(val)?);
    }
    Ok(Value::Object(map))
}

fn toml_to_json(v: &toml::Value) -> Result<Value> {
    Ok(match v {
        toml::Value::String(s) if s == NULL_SENTINEL => Value::Null,
        toml::Value::String(s) => Value::String(s.clone()),
        toml::Value::Integer(i) => Value::Number((*i).into()),
        toml::Value::Float(f) => {
            let n = serde_json::Number::from_f64(*f)
                .with_context(|| format!("non-finite float {f}"))?;
            Value::Number(n)
        }
        toml::Value::Boolean(b) => Value::Bool(*b),
        toml::Value::Datetime(dt) => Value::String(dt.to_string()),
        toml::Value::Array(arr) => {
            Value::Array(arr.iter().map(toml_to_json).collect::<Result<Vec<_>>>()?)
        }
        toml::Value::Table(table) => decode_table(table)?,
    })
}

pub fn ticket_to_toml_string(ticket: &Value, ord: i64) -> Result<String> {
    let obj = ticket
        .as_object()
        .with_context(|| "ticket is not an object")?;
    let mut table = match encode_object(obj)? {
        toml::Value::Table(t) => t,
        _ => unreachable!(),
    };
    table.insert(ORD_KEY.into(), toml::Value::Integer(ord));
    toml::to_string_pretty(&toml::Value::Table(table)).context("serialize ticket toml")
}

pub fn ticket_from_toml_str(text: &str) -> Result<(i64, Value)> {
    let parsed: toml::Value = text.parse().context("parse ticket toml")?;
    let table = parsed
        .as_table()
        .with_context(|| "ticket toml root is not a table")?;
    let ord = table
        .get(ORD_KEY)
        .and_then(|v| v.as_integer())
        .unwrap_or(i64::MAX);
    Ok((ord, toml_to_json(&parsed)?))
}

fn child_ids_from_parent(ticket: &Value) -> Vec<String> {
    let mut ids = Vec::new();
    if let Some(plan) = ticket.get("slice_plan").and_then(Value::as_object) {
        for k in plan.keys() {
            if !ids.iter().any(|x| x == k) {
                ids.push(k.clone());
            }
        }
    }
    if let Some(slices) = ticket.get("slices").and_then(Value::as_array) {
        for s in slices {
            if let Some(id) = s.as_str() {
                if !ids.iter().any(|x| x == id) {
                    ids.push(id.to_string());
                }
            }
        }
    }
    if let Some(slices) = ticket.get("children").and_then(Value::as_array) {
        for s in slices {
            if let Some(id) = s.as_str() {
                if !ids.iter().any(|x| x == id) {
                    ids.push(id.to_string());
                }
            }
        }
    }
    ids
}

fn child_doc(parent_id: &str, child_id: &str, parent: &Value) -> Value {
    let mut map = Map::new();
    map.insert("id".into(), Value::String(child_id.into()));
    map.insert("parent".into(), Value::String(parent_id.into()));
    if let Some(entry) = parent
        .get("slice_plan")
        .and_then(Value::as_object)
        .and_then(|p| p.get(child_id))
    {
        if let Some(obj) = entry.as_object() {
            for (k, v) in obj {
                map.insert(k.clone(), v.clone());
            }
        }
    }
    Value::Object(map)
}

/// Write the TOML tree for `registry` (uses `tickets[]` only; `next_id` is not stored).
pub fn save_toml_tree(root: &Path, registry: &Value) -> Result<()> {
    let dir = tickets_dir(root);
    fs::create_dir_all(&dir).with_context(|| format!("mkdir {}", dir.display()))?;
    let marker = root_marker_path(root);
    if !marker.is_file() {
        fs::write(&marker, "# ticket-registry root marker\n")
            .with_context(|| format!("write {}", marker.display()))?;
    }

    let tickets = registry
        .get("tickets")
        .and_then(Value::as_array)
        .context("registry.tickets missing")?;

    let mut desired: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (i, ticket) in tickets.iter().enumerate() {
        let id = ticket
            .get("id")
            .and_then(Value::as_str)
            .with_context(|| format!("ticket[{i}] missing id"))?;
        let path = parent_toml_path(root, id);
        fs::write(&path, ticket_to_toml_string(ticket, i as i64)?)
            .with_context(|| format!("write {}", path.display()))?;
        desired.insert(format!("{id}.toml"));

        for cid in child_ids_from_parent(ticket) {
            let child = child_doc(id, &cid, ticket);
            let cpath = dir.join(format!("{cid}.toml"));
            fs::write(&cpath, ticket_to_toml_string(&child, 0)?)
                .with_context(|| format!("write {}", cpath.display()))?;
            desired.insert(format!("{cid}.toml"));
        }
    }

    for ent in fs::read_dir(&dir).with_context(|| format!("read {}", dir.display()))? {
        let ent = ent?;
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("T-") && name.ends_with(".toml") && !desired.contains(name.as_ref()) {
            fs::remove_file(ent.path())
                .with_context(|| format!("remove stale {}", ent.path().display()))?;
        }
    }
    Ok(())
}

fn is_parent_toml_name(name: &str) -> bool {
    name.starts_with("T-")
        && name.ends_with(".toml")
        && is_parent_id(name.trim_end_matches(".toml"))
}

pub fn load_toml_tree(root: &Path) -> Result<Value> {
    let dir = tickets_dir(root);
    let mut loaded: Vec<(i64, Value)> = Vec::new();
    let mut found = false;
    for ent in fs::read_dir(&dir).with_context(|| format!("read {}", dir.display()))? {
        let ent = ent?;
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if !is_parent_toml_name(&name) {
            continue;
        }
        found = true;
        let text = fs::read_to_string(ent.path())
            .with_context(|| format!("read {}", ent.path().display()))?;
        loaded.push(ticket_from_toml_str(&text)?);
    }
    if !found {
        bail!("no parent T-*.toml files in {}", dir.display());
    }
    loaded.sort_by_key(|(ord, _)| *ord);
    let tickets: Vec<Value> = loaded.into_iter().map(|(_, t)| t).collect();
    let next_id = derive_next_id(&tickets);
    let mut root_obj = Map::new();
    root_obj.insert("next_id".into(), Value::Number(next_id.into()));
    root_obj.insert("tickets".into(), Value::Array(tickets));
    Ok(Value::Object(root_obj))
}

/// All on-disk ticket ids (parents + children). Used by the no-ticket-lost proof.
#[allow(dead_code)]
pub fn on_disk_ids(root: &Path) -> Result<Vec<String>> {
    let dir = tickets_dir(root);
    let mut ids = Vec::new();
    for ent in fs::read_dir(&dir)? {
        let ent = ent?;
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("T-") && name.ends_with(".toml") {
            ids.push(name.trim_end_matches(".toml").to_string());
        }
    }
    ids.sort();
    Ok(ids)
}

#[allow(dead_code)]
pub fn corpus_ids(
    registry: &Value,
) -> (
    std::collections::BTreeSet<String>,
    std::collections::BTreeSet<String>,
) {
    use std::collections::BTreeSet;
    let mut parents = BTreeSet::new();
    let mut all = BTreeSet::new();
    if let Some(arr) = registry.get("tickets").and_then(Value::as_array) {
        for t in arr {
            if let Some(id) = t.get("id").and_then(Value::as_str) {
                parents.insert(id.to_string());
                all.insert(id.to_string());
                for cid in child_ids_from_parent(t) {
                    all.insert(cid);
                }
            }
        }
    }
    (parents, all)
}

#[allow(dead_code)]
pub const FROZEN_27: &[&str] = &[
    "id",
    "title",
    "summary",
    "program",
    "surfaces",
    "impact",
    "status",
    "executor",
    "targets",
    "stream",
    "order",
    "priority",
    "notes",
    "route",
    "spec",
    "depends_on",
    "shipped_at",
    "branch",
    "parallel_ok",
    "slices",
    "unblocks",
    "slice_plan",
    "implements",
    "active_slice",
    "user_story",
    "milestone",
    "acceptance",
];

#[allow(dead_code)]
pub fn union_ticket_keys(registry: &Value) -> std::collections::BTreeSet<String> {
    let mut keys = std::collections::BTreeSet::new();
    if let Some(arr) = registry.get("tickets").and_then(Value::as_array) {
        for t in arr {
            if let Some(obj) = t.as_object() {
                for k in obj.keys() {
                    keys.insert(k.clone());
                }
            }
        }
    }
    keys
}

/// Status map at a git rev: JSON monolith if that blob exists, else every `T-*.toml`.
/// Returns `None` when neither form exists (refuse — never an empty "all unshipped" map).
pub fn status_map_at_rev(
    repo: &Path,
    rev: &str,
) -> Option<std::collections::HashMap<String, String>> {
    use std::process::Command;
    let spec = format!("{rev}:.ai/tickets/registry.json");
    let json_exists = Command::new("git")
        .args(["cat-file", "-e", &spec])
        .current_dir(repo)
        .status()
        .ok()?
        .success();
    if json_exists {
        let blob = Command::new("git")
            .args(["show", &spec])
            .current_dir(repo)
            .output()
            .ok()?;
        if !blob.status.success() {
            return None;
        }
        let v: Value = serde_json::from_str(&String::from_utf8(blob.stdout).ok()?).ok()?;
        let mut by = std::collections::HashMap::new();
        if let Some(arr) = v.get("tickets").and_then(Value::as_array) {
            for t in arr {
                let id = t.get("id")?.as_str()?.to_string();
                let st = t
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                by.insert(id, st);
            }
        }
        return Some(by);
    }
    let listing = Command::new("git")
        .args(["ls-tree", "-r", "--name-only", rev, "--", ".ai/tickets/"])
        .current_dir(repo)
        .output()
        .ok()?;
    if !listing.status.success() {
        return None;
    }
    let mut by = std::collections::HashMap::new();
    let mut any = false;
    for line in String::from_utf8_lossy(&listing.stdout).lines() {
        let path = line.trim();
        if !path.starts_with(".ai/tickets/T-") || !path.ends_with(".toml") {
            continue;
        }
        let blob = Command::new("git")
            .args(["show", &format!("{rev}:{path}")])
            .current_dir(repo)
            .output()
            .ok()?;
        if !blob.status.success() {
            continue;
        }
        let text = String::from_utf8(blob.stdout).ok()?;
        let (_, v) = ticket_from_toml_str(&text).ok()?;
        let id = v.get("id")?.as_str()?.to_string();
        let st = v
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        by.insert(id, st);
        any = true;
    }
    if !any {
        return None;
    }
    Some(by)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{format_json_unicode_preserve, load_json_monolith};
    use std::collections::BTreeSet;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("repo root")
            .to_path_buf()
    }

    fn load_json_or_toml(root: &Path) -> Result<Value> {
        let json = tickets_dir(root).join("registry.json");
        if json.is_file() {
            load_json_monolith(&json)
        } else {
            load_toml_tree(root)
        }
    }

    /// Gold JSON is the unicode-preserving emit of the parsed monolith.
    /// The on-disk file still contains `\\u` escapes; parse+emit is the cmp target.
    fn canonical_monolith(root: &Path) -> (Value, String) {
        let json_path = tickets_dir(root).join("registry.json");
        let original = if json_path.is_file() {
            fs::read_to_string(&json_path).expect("read monolith")
        } else {
            // After the cutover commit deletes the blob, gold is the parent of that delete.
            let del = std::process::Command::new("git")
                .args([
                    "log",
                    "-1",
                    "--diff-filter=D",
                    "--pretty=%H",
                    "--",
                    ".ai/tickets/registry.json",
                ])
                .current_dir(root)
                .output()
                .expect("git log delete");
            let sha = String::from_utf8_lossy(&del.stdout).trim().to_string();
            assert!(
                !sha.is_empty(),
                "no deleting commit for .ai/tickets/registry.json"
            );
            let shown = std::process::Command::new("git")
                .args(["show", &format!("{sha}^:.ai/tickets/registry.json")])
                .current_dir(root)
                .output()
                .expect("git show cutover monolith");
            assert!(
                shown.status.success(),
                "git show {sha}^:.ai/tickets/registry.json failed"
            );
            String::from_utf8(shown.stdout).unwrap()
        };
        let parsed: Value = serde_json::from_str(&original).expect("parse monolith");
        let gold = format_json_unicode_preserve(&parsed).expect("emit gold");
        (parsed, gold)
    }

    #[test]
    fn frozen_27_matches_live_corpus() {
        let root = repo_root();
        if crate::phase2::tree_is_phase2(&root) {
            return;
        }
        let v = load_json_or_toml(&root).expect("load");
        let got = union_ticket_keys(&v);
        let expect: BTreeSet<String> = FROZEN_27.iter().map(|s| (*s).to_string()).collect();
        assert_eq!(got, expect, "corpus keys drifted from FROZEN_27");
    }

    #[test]
    fn toml_roundtrip_is_byte_identical_to_canonical_monolith() {
        let root = repo_root();
        let (parsed, gold) = canonical_monolith(&root);
        let n = parsed
            .get("tickets")
            .and_then(Value::as_array)
            .map(|a| a.len())
            .expect("tickets[]");
        assert!(n > 0, "cutover monolith has no tickets");

        let tmp = root.join("target").join("phase1-toml-roundtrip");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join(".ai/tickets")).unwrap();
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
        let json_path = tickets_dir(&root).join("registry.json");
        let v = if json_path.is_file() {
            load_json_monolith(&json_path).unwrap()
        } else {
            load_toml_tree(&root).unwrap()
        };
        let (parents, all) = corpus_ids(&v);
        let n = parents.len();
        assert_eq!(
            v.get("tickets").and_then(Value::as_array).map(|a| a.len()),
            Some(n),
            "tickets[].len must equal parent-id set size (measured, not hardcoded)"
        );
        if json_path.is_file() {
            let tmp = root.join("target").join("phase1-id-set");
            let _ = fs::remove_dir_all(&tmp);
            fs::create_dir_all(tmp.join(".ai/tickets")).unwrap();
            save_toml_tree(&tmp, &v).unwrap();
            let disk: BTreeSet<String> = on_disk_ids(&tmp).unwrap().into_iter().collect();
            assert_eq!(
                disk, all,
                "on-disk ids must equal parent∪slice_plan∪slices (N={n} parents)"
            );
        } else {
            let disk: BTreeSet<String> = on_disk_ids(&root).unwrap().into_iter().collect();
            assert_eq!(disk, all, "on-disk ids must equal parent∪slice_plan∪slices");
        }
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
    fn write_live_toml_tree() {
        if std::env::var("TBD_PHASE1_WRITE").ok().as_deref() != Some("1") {
            return;
        }
        let root = repo_root();
        let json_path = tickets_dir(&root).join("registry.json");
        if !json_path.is_file() {
            return;
        }
        let v = load_json_monolith(&json_path).unwrap();
        save_toml_tree(&root, &v).unwrap();
        let reloaded = load_toml_tree(&root).unwrap();
        let emitted = format_json_unicode_preserve(&reloaded).unwrap();
        let gold = format_json_unicode_preserve(&v).unwrap();
        assert_eq!(
            emitted, gold,
            "refusing to delete: emit is not byte-identical to canonical monolith"
        );
        fs::write(root_marker_path(&root), "# ticket-registry root marker\n").unwrap();
        fs::remove_file(&json_path).unwrap();
        assert!(!json_path.exists());
        assert!(root_marker_path(&root).is_file());
    }

    #[test]
    fn dual_read_json_then_toml() {
        use std::process::Command;
        let tmp = repo_root().join("target").join("phase1-dual-read");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join(".ai/tickets")).unwrap();
        let git = |args: &[&str]| {
            Command::new("git")
                .args(args)
                .current_dir(&tmp)
                .env("GIT_AUTHOR_NAME", "tbd")
                .env("GIT_AUTHOR_EMAIL", "tbd@test")
                .env("GIT_COMMITTER_NAME", "tbd")
                .env("GIT_COMMITTER_EMAIL", "tbd@test")
                .status()
                .unwrap()
        };
        git(&["init", "-q"]);
        git(&["checkout", "-q", "-b", "main"]);
        let json =
            r#"{"tickets":[{"id":"T-AAA","status":"shipped"},{"id":"T-BBB","status":"queued"}]}"#;
        fs::write(tmp.join(".ai/tickets/registry.json"), json).unwrap();
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "A json"]);
        let sha_a = String::from_utf8(
            Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(&tmp)
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_string();

        fs::remove_file(tmp.join(".ai/tickets/registry.json")).unwrap();
        fs::write(tmp.join(".ai/tickets/ROOT"), "#\n").unwrap();
        let aaa = serde_json::json!({"id":"T-AAA","status":"shipped"});
        let bbb = serde_json::json!({"id":"T-BBB","status":"shipped"});
        fs::write(
            tmp.join(".ai/tickets/T-AAA.toml"),
            ticket_to_toml_string(&aaa, 0).unwrap(),
        )
        .unwrap();
        fs::write(
            tmp.join(".ai/tickets/T-BBB.toml"),
            ticket_to_toml_string(&bbb, 1).unwrap(),
        )
        .unwrap();
        git(&["add", "-A"]);
        git(&["commit", "-q", "-m", "B toml"]);
        let sha_b = String::from_utf8(
            Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(&tmp)
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_string();

        let map_at = |rev: &str| -> Option<std::collections::HashMap<String, String>> {
            crate::tickets_store::status_map_at_rev(&tmp, rev)
        };

        let a = map_at(&sha_a).expect("json rev");
        assert_eq!(a.get("T-AAA").map(String::as_str), Some("shipped"));
        assert_eq!(a.get("T-BBB").map(String::as_str), Some("queued"));
        let ids = ["T-AAA", "T-BBB"];
        let open_a: Vec<_> = ids
            .iter()
            .copied()
            .filter(|t| {
                !matches!(
                    a.get(*t).map(String::as_str),
                    Some("shipped") | Some("cancelled")
                )
            })
            .collect();
        assert_eq!(open_a, ["T-BBB"]);

        let b = map_at(&sha_b).expect("toml rev");
        let open_b: Vec<_> = ids
            .iter()
            .copied()
            .filter(|t| {
                !matches!(
                    b.get(*t).map(String::as_str),
                    Some("shipped") | Some("cancelled")
                )
            })
            .collect();
        assert!(open_b.is_empty(), "both shipped at B");

        fs::remove_dir_all(tmp.join(".ai/tickets")).unwrap();
        git(&["add", "-A"]);
        git(&["commit", "-q", "-m", "C empty"]);
        let sha_c = String::from_utf8(
            Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(&tmp)
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_string();
        assert!(
            map_at(&sha_c).is_none(),
            "neither form must refuse, not empty map"
        );
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
}
