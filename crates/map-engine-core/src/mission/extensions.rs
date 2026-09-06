//! T-936 — AUTHORED_BLOCKS: the one passthrough the editor's authored top-level blocks travel on.
//!
//! ══ Why a list, and not a copy line per block ════════════════════════════════════════════════
//! The T-936 program adds seven authored blocks (`winConditions`, `tasks`, `radioPlan`,
//! `weatherTimeline`, `audio`, `spawnModules`, `tacticalGraphics`), each in its own slice. Every
//! one has to be copied out of the editor document by `compile.rs` and read back by `flatten.rs` —
//! two files five OTHER open tickets also own. Seven slices each hand-editing both is seven merge
//! conflicts and seven chances to add the copy and forget the read.
//!
//! So both files get ONE call site each — [`copy_authored_blocks`] and [`ExtensionBlocks::from_payload`]
//! (plus [`AuthoredBlocks::parse`] for the blocks the document models with a typed field) — and a
//! later slice adds a block by adding a ROW to [`AUTHORED_BLOCKS`] and a validator module. Nothing
//! in `compile.rs`, nothing in `flatten.rs`.
//!
//! ══ This is a LIST, not an open passthrough ══════════════════════════════════════════════════
//! A key with no row here does not reach the compiled document. It stays exactly where T-219 put
//! it — parked in `payloadExtras`, re-emitted onto the saved editor payload, invisible to
//! `flatten_to_mod_document`. That matters because `mission.schema.json`'s top level is
//! `additionalProperties: false`: an open passthrough would let one stray key 500 the whole
//! `/compiled` route. Every row also carries a `validate`, so a listed key that is MALFORMED is
//! refused with a sentence rather than carried to a schema failure the author cannot read.
//!
//! ══ Where the blocks live in the editor document ═════════════════════════════════════════════
//! In `meta.environment` — the editor's per-mission settings BAG, not the compiled document's
//! `environment` block of the same name. That distinction is `panels/env.rs`'s own ("the bag is
//! the transport, the table is the contract"), and the T-224 `flow` keys already ride it: they are
//! written into `meta.environment` and land at `flow.timeLimitSeconds` on the wire, nowhere near
//! the compiled `environment`.
//!
//! It is the bag rather than a `meta.winConditions` sibling for a mechanical reason, and the
//! mechanism is what makes it correct: `meta.environment` is the only part of `meta` with a
//! read/write pair the editor can drive (`operations::read_env_value` / `update_environment`, one
//! merge patch, one undo step), and `MissionDocCore::hydrate` loads `payload.environment` back into
//! it VERBATIM. So an authored block survives Save → reload with no change to `doc/store.rs`, whose
//! parked-key machinery has its own owners and its own tickets.
//!
//! ══ Two destinations, and why a block has exactly one ════════════════════════════════════════
//! A block is either MODELLED by the compiled document (a typed field on `ModMissionDocument`) or
//! CARRIED by [`ExtensionBlocks`], never both — [`DOCUMENT_OWNED_BLOCKS`] is the list that decides,
//! and it is what stops the same key being emitted twice.
//!
//! `winConditions` is modelled: `mission.schema.json` lists it in the top-level `required`, so it
//! is never absent, it needs a derivation when unauthored, and its emitted shape is a typed struct
//! the emitter builds field by field. Everything the program adds after it is optional and rides
//! [`ExtensionBlocks`] verbatim.
//!
//! ══ Byte parity ══════════════════════════════════════════════════════════════════════════════
//! [`ExtensionBlocks`] is `#[serde(flatten)]`ed onto the compiled document and serialises to
//! NOTHING when it holds no block — not to an empty object. So a mission that authors no optional
//! block emits exactly the bytes it emitted before this module existed, which is what
//! `compiler_shaped_golden_is_a_fresh_emitter_output` pins and what
//! [`tests::an_empty_carrier_adds_nothing_to_the_document`] states directly.

use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};
use serde_json::{Map, Value};

/// One authored top-level block: the JSON key, and the check that says whether the author's value
/// is one this compile can carry.
///
/// `validate` returns the refusal CLAUSE, not a bool — it reaches the author through the compile's
/// diagnostics, and "invalid winConditions" tells nobody which of five modes and three params was
/// wrong.
pub struct AuthoredBlock {
    /// The top-level key, spelled exactly as `mission.schema.json` declares it.
    pub key: &'static str,
    /// The typed validator for this block's value.
    pub validate: fn(&Value) -> Result<(), String>,
}

/// Every block the editor authors and the compile carries verbatim.
///
/// Order is `mission.schema.json`'s own property order, which is the order [`ExtensionBlocks`]
/// emits them in.
pub const AUTHORED_BLOCKS: &[AuthoredBlock] = &[
    AuthoredBlock {
        key: "radioPlan",
        validate: crate::mission::radio_plan::validate,
    },
    AuthoredBlock {
        key: "winConditions",
        validate: crate::mission::win_conditions::validate,
    },
    AuthoredBlock {
        key: "tasks",
        validate: crate::mission::tasks::validate,
    },
];

/// The authored blocks the compiled document MODELS with a typed field of its own, and which
/// [`ExtensionBlocks`] must therefore not carry — see the module header's "two destinations".
///
/// A block belongs here when `ModMissionDocument` MODELS it with a typed field (so emitting it
/// from the carrier too would duplicate the key). `winConditions` is also top-level-`required`;
/// `radioPlan` is optional but already typed (T-203 derivation). Everything else rides the carrier.
pub const DOCUMENT_OWNED_BLOCKS: &[&str] = &["radioPlan", "winConditions"];

/// Is `key` an authored block?
#[must_use]
pub fn is_authored_block(key: &str) -> bool {
    AUTHORED_BLOCKS.iter().any(|b| b.key == key)
}

/// Copy every authored block present in `env` onto the compiled payload root, verbatim.
///
/// **`compile.rs`'s ONE call site.** `env` is the editor document's `meta.environment` settings bag
/// (see the module header for why that bag and not a `meta` sibling); `dst` is the payload object
/// being built.
///
/// A block that is present but MALFORMED is copied anyway, deliberately. This function is on the
/// SAVE path: a saved payload is stored immutably, and silently dropping a block the author is
/// mid-way through editing would lose their work with nothing said. The refusal happens at the
/// COMPILE boundary instead ([`AuthoredBlocks::parse`] / [`ExtensionBlocks::from_payload`]), which
/// is where the author can see it and where a bad block cannot reach a game server;
/// `mission-editor-payload.schema.json` is what rejects a genuinely wrong-shaped block at the write
/// boundary.
///
/// Returns the keys it copied, so the caller can keep the T-219 `payloadExtras` re-emit from
/// racing it.
pub fn copy_authored_blocks(env: &Value, dst: &mut Map<String, Value>) -> Vec<&'static str> {
    let mut copied = Vec::new();
    for block in AUTHORED_BLOCKS {
        let Some(value) = env.get(block.key) else {
            continue;
        };
        // `null` is not authoring. It is what a cleared key looks like coming out of a yrs map, and
        // emitting `"winConditions": null` would fail the schema's own type check.
        if value.is_null() {
            continue;
        }
        dst.insert(block.key.to_string(), value.clone());
        copied.push(block.key);
    }
    copied
}

/// The DOCUMENT-MODELLED authored blocks a payload carried, parsed and validated —
/// `flatten.rs`'s call site for the typed half.
///
/// Every field is an `Option`; `None` means "the author wrote no such block", which is what the
/// emitter tests to decide between the authored value and the derivation it has always run.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct AuthoredBlocks {
    /// `radioPlan` (T-936.3). Document-modelled: flatten substitutes this for `derive_radio_plan`.
    pub radio_plan: Option<crate::mission::radio_plan::AuthoredRadioPlan>,
    /// `winConditions` (T-936.1).
    pub win_conditions: Option<crate::mission::win_conditions::AuthoredWinConditions>,
}

impl AuthoredBlocks {
    /// Parse every document-modelled block out of a payload root.
    ///
    /// **A malformed block is REPORTED, not fatal.** `payload` is stored bytes: a saved mission is
    /// immutable and re-compiled on every `/compiled` fetch, so refusing the whole compile over one
    /// bad block would turn a mission that loads today into a permanent HTTP 500 — the exact
    /// failure mode `EditorPayload::environment` is a bare `Value` to avoid. The refusal clause
    /// comes back in `refusals` for the caller to record as a diagnostic, and the block falls back
    /// to the derivation as if it had not been authored at all.
    ///
    /// `refusals` is `(key, clause)` in [`AUTHORED_BLOCKS`] order.
    #[must_use]
    pub fn parse(payload: &Value) -> (Self, Vec<(&'static str, String)>) {
        let mut out = Self::default();
        let mut refusals = Vec::new();

        for block in AUTHORED_BLOCKS {
            if !DOCUMENT_OWNED_BLOCKS.contains(&block.key) {
                continue;
            }
            let Some(value) = payload.get(block.key) else {
                continue;
            };
            if value.is_null() {
                continue;
            }
            if let Err(clause) = (block.validate)(value) {
                refusals.push((block.key, clause));
                continue;
            }
            // One `if` rather than a `match` because [`DOCUMENT_OWNED_BLOCKS`] has exactly one
            // entry; a second document-modelled block turns this into a match. Either way a key
            // listed there with no branch here would be validated and then silently DROPPED, which
            // is why `every_document_owned_block_has_a_parse_arm` exists rather than a comment
            // asking the next slice to remember.
            //
            // Validated one line above, so this cannot fail; `ok()` rather than `expect` because a
            // compile must never panic on stored bytes.
            match block.key {
                "winConditions" => {
                    out.win_conditions = crate::mission::win_conditions::parse(value).ok();
                }
                "radioPlan" => {
                    out.radio_plan = crate::mission::radio_plan::parse(value).ok();
                }
                _ => {}
            }
        }

        (out, refusals)
    }
}

/// The optional authored blocks as they reach the compiled document — `#[serde(flatten)]`ed onto
/// `ModMissionDocument` so each emits at the document ROOT, in [`AUTHORED_BLOCKS`] order.
///
/// **Deliberately untyped.** The blocks it carries are passed through VERBATIM after their row's
/// validator has accepted them, so a typed field per block would buy nothing the validator has not
/// already bought and would cost every later slice an edit here. Keeping the values as `Value`
/// is what lets T-936.2…T-936.7 land a block with a row plus a validator and no edit to this file,
/// `compile.rs` or `flatten.rs` — which is the whole reason this module exists.
///
/// T-936.1's `winConditions` is document-modelled; T-936.2's `tasks` rides this carrier
/// ([`DOCUMENT_OWNED_BLOCKS`]). The mechanism is not speculative: [`Self::from_payload`] runs on
/// every compile and its withholding of `winConditions` is what stops a duplicate key, and
/// [`tests::a_carried_block_reaches_the_document_root`] drives a populated carrier through the
/// exact `#[serde(flatten)]` shape `ModMissionDocument` has.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ExtensionBlocks {
    /// `(key, value)` in [`AUTHORED_BLOCKS`] order. A `Vec` rather than a map because the order is
    /// the schema's and must be reproducible — `/compiled` is re-fetched by the game server and
    /// must not change under it.
    blocks: Vec<(&'static str, Value)>,
}

impl ExtensionBlocks {
    /// Read the carrier's blocks out of a payload root — `flatten.rs`'s ONE call site.
    ///
    /// Skips anything in [`DOCUMENT_OWNED_BLOCKS`] (that block has a typed field and emitting it
    /// here too would produce a duplicate key), anything absent or `null`, and anything its row's
    /// validator refuses. Refusals come back as `(key, clause)` for the caller to record as a
    /// diagnostic — the same not-fatal rule [`AuthoredBlocks::parse`] states and for the same
    /// reason.
    #[must_use]
    pub fn from_payload(payload: &Value) -> (Self, Vec<(&'static str, String)>) {
        let mut out = Self::default();
        let mut refusals = Vec::new();

        for block in AUTHORED_BLOCKS {
            if DOCUMENT_OWNED_BLOCKS.contains(&block.key) {
                continue;
            }
            let Some(value) = payload.get(block.key) else {
                continue;
            };
            if value.is_null() {
                continue;
            }
            match (block.validate)(value) {
                Ok(()) => out.set(block.key, value.clone()),
                Err(clause) => refusals.push((block.key, clause)),
            }
        }

        (out, refusals)
    }

    /// Place one block. Replaces an existing value for the same key rather than appending a second
    /// one, so the carrier cannot emit a duplicate key however it is driven.
    pub fn set(&mut self, key: &'static str, value: Value) {
        if let Some(slot) = self.blocks.iter_mut().find(|(k, _)| *k == key) {
            slot.1 = value;
        } else {
            self.blocks.push((key, value));
        }
    }

    /// The value carried for `key`, if any.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.blocks.iter().find(|(k, _)| *k == key).map(|(_, v)| v)
    }

    /// True when no block is carried — the state every mission is in today, and the state whose
    /// emitted bytes must be indistinguishable from a document compiled before this struct existed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// How many blocks are carried.
    #[must_use]
    pub fn len(&self) -> usize {
        self.blocks.len()
    }
}

/// Emits each carried block as a root-level key.
///
/// Hand-written rather than derived because the field set is dynamic. Under `#[serde(flatten)]`
/// serde hands this a `FlatMapSerializer`, which forwards `serialize_map` entries straight onto the
/// OUTER object — so an empty carrier writes no entries and therefore contributes not one byte,
/// which is the parity property the whole module rests on.
impl Serialize for ExtensionBlocks {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.blocks.len()))?;
        for (key, value) in &self.blocks {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The `ModMissionDocument` shape, in miniature: a required modelled block, then the flattened
    /// carrier. Every serialisation test below goes through this rather than through
    /// `ExtensionBlocks` alone, because `#[serde(flatten)]` is the thing under test — a carrier that
    /// serialises correctly on its own and wrongly when flattened is exactly the T-394 shape.
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Host {
        win_conditions: Value,
        #[serde(flatten)]
        extensions: ExtensionBlocks,
    }

    fn host(extensions: ExtensionBlocks) -> Value {
        let h = Host {
            win_conditions: json!({"mode": "attrition", "endOn": ["time_limit"]}),
            extensions,
        };
        serde_json::from_str(&serde_json::to_string(&h).expect("serialises")).expect("parses")
    }

    #[test]
    fn win_conditions_is_the_registered_block_and_the_document_models_it() {
        assert!(is_authored_block("winConditions"));
        assert!(DOCUMENT_OWNED_BLOCKS.contains(&"winConditions"));
        assert!(
            is_authored_block("tasks"),
            "T-936.2 registers tasks; a missing row is a silent drop at flatten"
        );
        assert!(
            is_authored_block("radioPlan"),
            "T-936.3 registers radioPlan; a missing row is a silent drop at flatten"
        );
        assert!(DOCUMENT_OWNED_BLOCKS.contains(&"radioPlan"));
        assert!(!is_authored_block("payloadExtras"));
        assert_eq!(AUTHORED_BLOCKS.len(), 3);
    }

    /// Every entry in [`DOCUMENT_OWNED_BLOCKS`] must be a registered block, or the withhold rule
    /// silently protects a key nothing carries.
    #[test]
    fn every_document_owned_block_is_registered() {
        for key in DOCUMENT_OWNED_BLOCKS {
            assert!(is_authored_block(key), "`{key}` is not in AUTHORED_BLOCKS");
        }
    }

    #[test]
    fn copy_carries_a_listed_key_verbatim_and_leaves_everything_else() {
        let env = json!({
            "weather": "clear",
            "timeLimitSeconds": 5400,
            "winConditions": {"mode": "vip", "endOn": ["faction_eliminated"], "vipSlotId": "s1"},
            "audio": {"emitters": []},
        });
        let mut dst = Map::new();
        let copied = copy_authored_blocks(&env, &mut dst);

        assert_eq!(copied, ["winConditions"]);
        assert_eq!(dst.len(), 1, "only the listed key travels: {dst:?}");
        assert_eq!(dst["winConditions"], env["winConditions"], "verbatim");
        assert!(
            !dst.contains_key("audio"),
            "an unlisted key stays parked in payloadExtras"
        );
        assert!(
            !dst.contains_key("weather"),
            "the bag's own keys stay in the bag"
        );
    }

    #[test]
    fn an_absent_or_null_block_copies_nothing() {
        let mut dst = Map::new();
        assert!(copy_authored_blocks(&json!({}), &mut dst).is_empty());
        assert!(dst.is_empty());

        let mut dst = Map::new();
        assert!(copy_authored_blocks(&json!({"winConditions": null}), &mut dst).is_empty());
        assert!(
            dst.is_empty(),
            "null is a cleared key, not an authored one: {dst:?}"
        );

        let mut dst = Map::new();
        assert!(copy_authored_blocks(&json!(null), &mut dst).is_empty());
        assert!(dst.is_empty());
    }

    /// The SAVE path carries a half-edited block; the COMPILE path refuses it. Both halves are
    /// asserted here because keeping them apart is the whole reason `copy_authored_blocks` does not
    /// validate.
    #[test]
    fn a_malformed_block_is_saved_but_refused_at_compile() {
        let bad = json!({"mode": "vip_hunt", "endOn": ["time_limit"]});

        let mut dst = Map::new();
        assert_eq!(
            copy_authored_blocks(&json!({"winConditions": bad}), &mut dst),
            ["winConditions"],
            "save must not lose the author's work in progress"
        );

        let (blocks, refusals) = AuthoredBlocks::parse(&json!({"winConditions": bad}));
        assert!(
            blocks.win_conditions.is_none(),
            "compile falls back to the derivation"
        );
        assert_eq!(refusals.len(), 1, "{refusals:?}");
        assert_eq!(refusals[0].0, "winConditions");
        assert!(refusals[0].1.contains("vip_hunt"), "{}", refusals[0].1);
    }

    #[test]
    fn parse_reads_a_good_block_and_ignores_an_absent_one() {
        let (blocks, refusals) = AuthoredBlocks::parse(&json!({
            "winConditions": {
                "mode": "extraction", "endOn": ["time_limit"], "extractionZoneId": "z1"
            }
        }));
        assert!(refusals.is_empty(), "{refusals:?}");
        let wc = blocks.win_conditions.expect("parsed");
        assert_eq!(wc.mode, "extraction");
        assert_eq!(wc.params.extraction_zone_id.as_deref(), Some("z1"));

        let (blocks, refusals) = AuthoredBlocks::parse(&json!({"schemaVersion": 1}));
        assert_eq!(blocks, AuthoredBlocks::default());
        assert!(refusals.is_empty());
    }

    /// Every document-modelled row must have a `parse` arm, or a slice adds a validator and a key
    /// and the block silently never lands. The `_ => {}` arm makes that failure SILENT, which is
    /// precisely why this is a test and not a comment asking the next slice to remember.
    #[test]
    fn every_document_owned_block_has_a_parse_arm() {
        for key in DOCUMENT_OWNED_BLOCKS {
            let sample = match *key {
                "winConditions" => json!({"mode": "attrition", "endOn": ["time_limit"]}),
                "radioPlan" => {
                    json!({"nets": [{"id": "net:blufor_cmd", "label": "Command", "freqMHz": 30.0}]})
                }
                other => panic!(
                    "DOCUMENT_OWNED_BLOCKS row `{other}` has no sample here — add one, and an arm \
                     in AuthoredBlocks::parse, or the block validates and is then dropped"
                ),
            };
            let mut payload = Map::new();
            payload.insert((*key).to_string(), sample);
            let (blocks, refusals) = AuthoredBlocks::parse(&Value::Object(payload));
            assert!(refusals.is_empty(), "{refusals:?}");
            let landed = match *key {
                "winConditions" => blocks.win_conditions.is_some(),
                "radioPlan" => blocks.radio_plan.is_some(),
                _ => false,
            };
            assert!(
                landed,
                "`{key}` validated but AuthoredBlocks::parse dropped it"
            );
        }
    }

    /// The byte-parity floor, stated where it can be read: an unauthored document emits no
    /// extension keys AT ALL, not an empty object, and the flattened carrier adds not one byte.
    #[test]
    fn an_empty_carrier_adds_nothing_to_the_document() {
        let empty = ExtensionBlocks::default();
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);

        let h = Host {
            win_conditions: json!({"mode": "attrition", "endOn": ["time_limit"]}),
            extensions: empty,
        };
        assert_eq!(
            serde_json::to_string(&h).expect("serialises"),
            r#"{"winConditions":{"mode":"attrition","endOn":["time_limit"]}}"#,
            "an empty carrier must add NOTHING — this is the parity claim, in bytes"
        );
    }

    /// `winConditions` is document-modelled, so the carrier must WITHHOLD it even though it is a
    /// registered authored block. This is the live behaviour that stops a duplicate root key, and
    /// it runs on every compile today.
    #[test]
    fn the_carrier_withholds_a_document_modelled_block() {
        let (carried, refusals) = ExtensionBlocks::from_payload(&json!({
            "winConditions": {"mode": "vip", "endOn": ["time_limit"], "vipSlotId": "s1"}
        }));
        assert!(refusals.is_empty(), "{refusals:?}");
        assert!(
            carried.is_empty(),
            "winConditions has a typed field; carrying it too would emit the key twice"
        );

        let wire = host(carried);
        assert_eq!(
            wire.as_object().expect("object").len(),
            1,
            "exactly one winConditions key on the wire: {wire}"
        );
    }

    /// Rule-17 proof that the carrier can FIRE — the mechanism is not a shell waiting for a slice
    /// that might never come. `tasks` is the shape T-936.2 lands; when it does, its row in
    /// `AUTHORED_BLOCKS` makes `from_payload` produce exactly this and nothing here changes.
    #[test]
    fn a_carried_block_reaches_the_document_root() {
        let mut carried = ExtensionBlocks::default();
        carried.set("tasks", json!([{"id": "t1", "tier": "primary"}]));
        assert!(!carried.is_empty());
        assert_eq!(carried.len(), 1);
        assert_eq!(
            carried.get("tasks"),
            Some(&json!([{"id": "t1", "tier": "primary"}]))
        );

        let wire = host(carried);
        assert_eq!(
            wire["tasks"],
            json!([{"id": "t1", "tier": "primary"}]),
            "a carried block must land at the ROOT, beside winConditions: {wire}"
        );
        assert_eq!(wire["winConditions"]["mode"], "attrition");
        assert_eq!(
            wire.as_object().expect("object").len(),
            2,
            "the modelled block plus the one carried block: {wire}"
        );
    }

    /// Two blocks emit in the order they were set, and setting one twice replaces rather than
    /// duplicates — `/compiled` is re-fetched by the game server and must not change under it.
    #[test]
    fn the_carrier_is_ordered_and_cannot_emit_a_key_twice() {
        let mut carried = ExtensionBlocks::default();
        carried.set("tasks", json!([1]));
        carried.set("audio", json!({"emitters": []}));
        carried.set("tasks", json!([2]));
        assert_eq!(carried.len(), 2, "a re-set replaces");

        let text = serde_json::to_string(&Host {
            win_conditions: json!({"mode": "attrition", "endOn": ["time_limit"]}),
            extensions: carried,
        })
        .expect("serialises");
        assert_eq!(
            text,
            r#"{"winConditions":{"mode":"attrition","endOn":["time_limit"]},"tasks":[2],"audio":{"emitters":[]}}"#
        );
    }
}
