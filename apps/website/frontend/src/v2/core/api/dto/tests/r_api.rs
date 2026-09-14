//! The captured-response round-trip harness: every wire key is claimed by a named field, and
//! re-serialising a golden reproduces it byte for byte.

use super::*;
use crate::v2::core::test_support::fixtures::{golden, FIXTURE_DIR};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;

/// Recursively key-sort + renormalize a JSON string to a canonical form.
fn canon(s: &str) -> String {
    fn sort(v: Value) -> Value {
        match v {
            Value::Object(m) => {
                let mut keys: Vec<String> = m.keys().cloned().collect();
                keys.sort();
                let mut out = serde_json::Map::new();
                for k in keys {
                    let child = m.get(&k).cloned().unwrap();
                    out.insert(k, sort(child));
                }
                Value::Object(out)
            }
            Value::Array(a) => Value::Array(a.into_iter().map(sort).collect()),
            other => other,
        }
    }
    let v: Value = serde_json::from_str(s).expect("golden is valid JSON");
    serde_json::to_string(&sort(v)).unwrap()
}

/// Half one — the textual gate: `golden` must deserialize into `T` and re-serialize
/// canonical-equal to `golden`. This is the original `assert_golden`, unchanged.
///
/// **It cannot see a dropped field on any struct carrying `#[serde(flatten)]`** — see
/// [`assert_every_wire_key_is_claimed`] for the half that can, and
/// [`byte_equality_alone_cannot_see_a_dropped_field_under_flatten`] for the proof that it
/// can't.
fn assert_canonical_round_trip<T: Serialize + DeserializeOwned>(golden: &str) {
    let dto: T = serde_json::from_str(golden)
        .unwrap_or_else(|e| panic!("R-api: golden does not deserialize into the DTO: {e}"));
    let back = serde_json::to_string(&dto).expect("DTO re-serializes");
    assert_eq!(
        canon(golden),
        canon(&back),
        "R-api: DTO must re-serialize canonically byte-equal to the live-backend golden"
    );
}

/* ═══════════════════════ the structural half of the gate ═══════════════════════
   `assert_canonical_round_trip` compares two *strings*. A `#[serde(flatten)]` sibling can
   satisfy that comparison on behalf of a named field that no longer exists: delete
   `MissionCard::rejection_reason` and the key it used to own is simply collected by the
   `extra: Map<String, Value>` catch-all instead, then re-emitted verbatim. Same bytes out,
   gate green, field gone from the type.  proved it as a negative control — with
   `#[serde(skip)]` on `rejection_reason` the byte-equality passed and only its hand-written
   "is this a named field with a value" assertion failed.

   Hand-writing that assertion per field is not the fix: it has to be written again for every
   field of every struct, and the fields it is NOT written for stay invisible — the same
   "the gate can only see what someone remembered to look at" defect in a new costume.

   So this half asserts on the **deserialized type** instead of the text, and it needs no
   per-field code at all. For every position in the golden it replaces the value with a
   poison and asks serde whether that breaks the decode. A named field with a real type
   rejects at least one poison (a `String`/number/bool rejects both, a `Vec<_>` rejects the
   object, a struct/map rejects the array). A key that is only being swept into a flatten
   catch-all rejects neither — `Map<String, Value>` takes anything — and that is exactly the
   signal "no named field of this DTO is reading this key". A `Value`-typed field is
   indistinguishable from a catch-all here, correctly: it, too, reads nothing.

   Each test therefore declares the golden's *inventory of holes* and the set must match
   exactly, so the inventory cannot rot in either direction: a new entry means a named field
   was dropped or renamed, a missing entry means a key was promoted to a named field.
*/

/// One step down into the golden's JSON tree. Array indices render as `*`, so the inventory
/// is one line per **shape** rather than one per row.
#[derive(Clone, Copy)]
enum Step<'a> {
    Key(&'a str),
    Index(usize),
}

fn render(path: &[Step<'_>]) -> String {
    path.iter()
        .map(|s| match s {
            Step::Key(k) => (*k).to_string(),
            Step::Index(_) => "*".to_string(),
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn node_at<'a>(root: &'a Value, path: &[Step<'_>]) -> &'a Value {
    let mut cur = root;
    for step in path {
        cur = match step {
            Step::Key(k) => &cur[*k],
            Step::Index(i) => &cur[*i],
        };
    }
    cur
}

/// `root` with the node at `path` replaced by `poison`.
fn poisoned(root: &Value, path: &[Step<'_>], poison: &Value) -> Value {
    let mut out = root.clone();
    let mut cur = &mut out;
    for step in path {
        cur = match step {
            Step::Key(k) => cur.get_mut(*k).expect("path was walked out of this tree"),
            Step::Index(i) => cur.get_mut(*i).expect("path was walked out of this tree"),
        };
    }
    *cur = poison.clone();
    out
}

/// Is the node at `path` **claimed** — i.e. does some named field of `T` actually read it?
///
/// Two poisons, because one is not enough: an object is rejected by every scalar and by
/// `Vec<_>`, an array is rejected by every scalar and by every struct/map. Claimed means at
/// least one of them breaks the decode.
fn claimed<T: DeserializeOwned>(root: &Value, path: &[Step<'_>]) -> bool {
    [
        json!({ "__t394_poison__": true }),
        json!(["__t394_poison__"]),
    ]
    .iter()
    .any(|p| serde_json::from_value::<T>(poisoned(root, path, p)).is_err())
}

/// Depth-first over the golden. An unclaimed position is recorded and **not** descended into:
/// everything under a `Value` sink is unclaimed by construction, and listing it would bury the
/// one line that matters.
fn walk<'a, T: DeserializeOwned>(root: &'a Value, path: &mut Vec<Step<'a>>, out: &mut Vec<String>) {
    let visit = |path: &mut Vec<Step<'a>>, out: &mut Vec<String>| {
        if claimed::<T>(root, path) {
            walk::<T>(root, path, out);
        } else {
            out.push(render(path));
        }
    };
    match node_at(root, path) {
        Value::Object(m) => {
            for k in m.keys() {
                path.push(Step::Key(k));
                visit(path, out);
                path.pop();
            }
        }
        Value::Array(a) => {
            for i in 0..a.len() {
                path.push(Step::Index(i));
                visit(path, out);
                path.pop();
            }
        }
        _ => {}
    }
}

/// Every key on the wire that no named field of `T` reads.
fn unclaimed_keys<T: DeserializeOwned>(golden: &str) -> Vec<String> {
    let root: Value = serde_json::from_str(golden).expect("golden is valid JSON");
    let mut out = Vec::new();
    walk::<T>(&root, &mut Vec::new(), &mut out);
    out.sort();
    out.dedup();
    out
}

/// Half two — the structural gate: the golden's unclaimed-key set must be exactly `expected`.
fn assert_every_wire_key_is_claimed<T: DeserializeOwned>(golden: &str, expected: &[&str]) {
    let got = unclaimed_keys::<T>(golden);
    let mut want: Vec<String> = expected.iter().map(|s| (*s).to_string()).collect();
    want.sort();
    want.dedup();
    if got == want {
        return;
    }
    let appeared: Vec<&String> = got.iter().filter(|k| !want.contains(k)).collect();
    let vanished: Vec<&String> = want.iter().filter(|k| !got.contains(k)).collect();
    panic!(
        "R-api : the set of wire keys no named DTO field claims has changed.\n\
         NEWLY UNCLAIMED {appeared:?}\n  \
         — a named field was dropped or renamed, and a `#[serde(flatten)]` catch-all is now\n  \
         swallowing its key. Byte-equality stays green through this; that is the whole point\n  \
         of this assertion.\n\
         NO LONGER UNCLAIMED {vanished:?}\n  \
         — the key is a named field now. Drop it from this test's list.\n\
         full unclaimed set: {got:?}"
    );
}

/// The gate: both halves. `unclaimed` is this golden's inventory of keys the DTO does not
/// read — `&[]` means the DTO claims every byte on the wire.
fn assert_golden<T: Serialize + DeserializeOwned>(golden: &str, unclaimed: &[&str]) {
    assert_canonical_round_trip::<T>(golden);
    assert_every_wire_key_is_claimed::<T>(golden, unclaimed);
}

/// **The proof that the structural half is not vacuous, frozen as a test.**
///
/// Two shapes of the same wire row. One names the field; the other skips it, so the flattened
/// catch-all takes the key instead. A purely textual comparison cannot tell them apart, because
/// the catch-all re-emits the key byte for byte. Asking the type whether anything reads the key
/// separates them, and this runs on every test run rather than living in a commit message.
#[derive(Serialize, Deserialize)]
struct Claimed {
    id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rejection_reason: Option<String>,
    #[serde(flatten)]
    extra: serde_json::Map<String, Value>,
}

#[derive(Serialize, Deserialize)]
struct Absorbed {
    id: String,
    #[serde(skip)]
    #[allow(dead_code)]
    rejection_reason: Option<String>,
    #[serde(flatten)]
    extra: serde_json::Map<String, Value>,
}

#[test]
fn byte_equality_alone_cannot_see_a_dropped_field_under_flatten() {
    const ROW: &str = r#"{"id":"m1","rejection_reason":"too many AI"}"#;
    // The defect: the textual gate is green either way. Deleting the named field changed
    // nothing it can measure, because `extra` re-emits the key byte-for-byte.
    assert_canonical_round_trip::<Claimed>(ROW);
    assert_canonical_round_trip::<Absorbed>(ROW);
    // The structural gate separates them by asking the *type*, not the text.
    assert_eq!(unclaimed_keys::<Claimed>(ROW), Vec::<String>::new());
    assert_eq!(
        unclaimed_keys::<Absorbed>(ROW),
        vec!["rejection_reason".to_string()],
        "with no named field reading it, the key is only being swept into `extra`"
    );
}

// The goldens are embedded at compile time from the crate-local fixture corpus.

/// The documented fixture directory and the base the macro embeds from agree. A rename would break
/// the embed anyway; this keeps the human-visible path honest.
#[test]
fn fixture_dir_constant_documented() {
    assert!(FIXTURE_DIR.ends_with("fixtures/api/"));
}

#[cfg(test)]
#[path = "r_api_auth.rs"]
mod auth;
#[cfg(test)]
#[path = "r_api_content.rs"]
mod content;
#[cfg(test)]
#[path = "r_api_events.rs"]
mod events;
#[cfg(test)]
#[path = "r_api_missions.rs"]
mod missions;
#[cfg(test)]
#[path = "r_api_registry.rs"]
mod registry;
#[cfg(test)]
#[path = "r_api_servers.rs"]
mod servers;
#[cfg(test)]
#[path = "r_api_telemetry.rs"]
mod telemetry;
