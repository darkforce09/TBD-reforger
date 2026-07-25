//! Wire-safety scan of an authored editor payload (T-181.44).
//!
//! T-181.43 added `mission.schema.json#/$defs/wireSafeString` (`^[^\x00-\x1F\x7F]*$`) to the
//! authored strings of the COMPILED document, because a callsign containing a TAB is otherwise a
//! legal mission that shifts every column of the mod's tab-separated roster wire and makes a seat
//! unselectable. That kills the class — but only at `GET /missions/:id/compiled`, long after the
//! author pressed Save, and by then the only human in the loop is a server operator watching a
//! mission that will not load.
//!
//! `mission-editor-payload.schema.json` deliberately leaves `editor.slots[]` unconstrained (no
//! per-item subschema) so write-side validation stays O(1) on missions with hundreds of thousands
//! of slots. That constraint is real and this module respects it: the rule is expressed **in code,
//! not in the schema**, precisely because the schema form would cost a per-item object walk plus a
//! regex execution per slot. What is here instead is a single linear pass that touches two short
//! strings per slot with a **byte** scan — no regex engine, no subschema, no per-item allocation on
//! the clean path — riding the `serde_json::Value` the payload validator has *already* parsed. That
//! is a different cost class from the thing the O(1) note refuses, and the cost is paid where the
//! walk already exists.
//!
//! MEASURED (2026-07-25, release, 367k slots / 2k squads / 65.7 MB payload — the T-060.1.3 scale):
//! `serde_json` parse **615.6 ms**, this scan **34.5 ms** — **5.6%** of a parse that has to happen
//! anyway, on the largest mission the editor has ever saved. Re-measure by parsing a payload of
//! that shape and timing `scan_editor_payload` against the parse it follows; if the ratio ever
//! stops being a rounding error, the reachability shortcut below is the first thing to revisit.
//!
//! ## What is scanned, and why exactly these fields
//!
//! Only strings that [`crate::mission::flatten::flatten_to_mod_document`] copies into a
//! `wireSafeString` field of the compiled document — mirroring flatten's fallback chains exactly, so
//! a value the compile would have replaced is not reported:
//!
//! | editor field | compiled `wireSafeString` field |
//! |---|---|
//! | `editor.factions[].name` (non-empty only) | `factions[].displayName` |
//! | `editor.squads[].callsign`, else `.name`, else `.id` | `slots[].groupCallsign`, `orbat.*.groups[].callsign`, and `slots[].id` |
//! | `editor.slots[].role` (non-empty only) | `slots[].role`, `orbat.*.groups[].roles[].slot`, and `slots[].id` |
//! | `editor.slots[].id` | `slots[].uid` (carried verbatim) |
//!
//! An empty `factions[].name` / `slots[].role` / squad callsign is NOT scanned because flatten
//! substitutes for it (`slug_key(key)`, `ROLE_FALLBACK`, `CALLSIGN_FALLBACK`) and the substitute is
//! wire-safe by construction. That direction matters: every string that CAN reach the compiled
//! document is scanned, so a payload this pass accepts cannot produce a `/compiled` rejection for
//! this cause. That equivalence is pinned against the real compiler + the real schema — not against
//! a restatement of either — by `save_scan_agrees_with_the_compiled_schema` in
//! `apps/website/api/src/services/mission_compile.rs`, which is the only place both live.
//!
//! ## Reachability is deliberately NOT modelled
//!
//! Flatten only emits slots reachable through `faction.squadIds → squad.slotIds`; this pass scans
//! every `editor.slots[]` entry regardless. Two reasons. Filtering would mean building the
//! id→slot map flatten builds — an O(n) hash of 367k short strings at save, which is the cost this
//! module exists to avoid. And an unreachable slot carrying a control character is not a false
//! positive but an earlier one: it is authored data that becomes live the moment the author drags
//! that slot into a squad.
//!
//! ## No silent repair
//!
//! Nothing here rewrites a value. Substituting a squad id for a *blank* callsign is defensible
//! (a blank is unambiguous and the author expressed no intent); quietly deleting a character out of
//! a name somebody typed is not — they would ship a mission whose roster reads differently from
//! their editor and never be told. This pass reports and the caller rejects.

use std::collections::HashMap;

use serde_json::Value;

/// Distinct offending values reported before the list is truncated, matching the cap the
/// `/compiled` handler applies to schema findings. A systematic defect (a 10k-slot paste of one bad
/// role) collapses into ONE row by value dedup long before this bites; the cap is for the pathology
/// where every slot is separately bad.
pub const MAX_REPORTED: usize = 20;

/// True for the characters `wireSafeString` forbids: the C0 control block plus DEL.
///
/// Scanned as bytes, not chars. Every byte of a multi-byte UTF-8 sequence is `>= 0x80`, so a byte
/// scan is *exactly* the char scan over this set — without decoding — which is what keeps the pass
/// cheap enough to run on every save.
#[must_use]
pub const fn is_wire_unsafe(b: u8) -> bool {
    b <= 0x1F || b == 0x7F
}

/// First forbidden byte in `s`, or `None` when the string is wire-safe.
#[must_use]
pub fn first_unsafe_byte(s: &str) -> Option<u8> {
    s.bytes().find(|b| is_wire_unsafe(*b))
}

/// Name a control byte the way an author can act on: `TAB (U+0009)`, not `\u{9}`.
#[must_use]
pub fn describe(b: u8) -> String {
    let name = match b {
        0x00 => "NUL",
        0x07 => "BEL",
        0x08 => "BACKSPACE",
        0x09 => "TAB",
        0x0A => "LF (newline)",
        0x0B => "VT",
        0x0C => "FF",
        0x0D => "CR (carriage return)",
        0x1B => "ESC",
        0x7F => "DEL",
        _ => "control character",
    };
    format!("{name} (U+{b:04X})")
}

/// Render an offending value safely for a log line or an error body: control characters become
/// visible escapes (a raw TAB echoed into the message would be as invisible there as it was in the
/// editor), and a long value is elided.
#[must_use]
pub fn quote_value(s: &str) -> String {
    const MAX_CHARS: usize = 60;
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for (i, c) in s.chars().enumerate() {
        if i == MAX_CHARS {
            out.push('…');
            break;
        }
        match c {
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x80 && is_wire_unsafe(c as u8) => {
                out.push_str(&format!("\\u{{{:02x}}}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Where an authored string lands in the compiled document — the half of a finding that tells the
/// author why a field they thought was free text is not.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Dest {
    DisplayName,
    GroupCallsign,
    Role,
    Uid,
}

impl Dest {
    const fn compiled_field(self) -> &'static str {
        match self {
            Dest::DisplayName => "factions[].displayName",
            Dest::GroupCallsign => "slots[].groupCallsign (and slots[].id)",
            Dest::Role => "slots[].role (and slots[].id)",
            Dest::Uid => "slots[].uid",
        }
    }
}

#[derive(Default)]
struct Findings {
    /// (destination, offending value) → index into `rows`. Deduping on the VALUE is what keeps a
    /// bulk paste of one bad role from producing 10k identical lines.
    seen: HashMap<(Dest, String), usize>,
    rows: Vec<Row>,
    /// Distinct values dropped after [`MAX_REPORTED`].
    dropped: usize,
}

struct Row {
    location: String,
    value: String,
    byte: u8,
    dest: Dest,
    occurrences: usize,
}

impl Findings {
    /// Check one authored string. `location` is only formatted when there is something to report,
    /// so the clean path — every slot of a healthy mission — allocates nothing.
    fn check(&mut self, value: Option<&Value>, dest: Dest, location: impl FnOnce() -> String) {
        let Some(s) = value.and_then(Value::as_str) else {
            return;
        };
        let Some(byte) = first_unsafe_byte(s) else {
            return;
        };
        if let Some(&idx) = self.seen.get(&(dest, s.to_string())) {
            self.rows[idx].occurrences += 1;
            return;
        }
        if self.rows.len() >= MAX_REPORTED {
            self.dropped += 1;
            return;
        }
        self.seen.insert((dest, s.to_string()), self.rows.len());
        self.rows.push(Row {
            location: location(),
            value: s.to_string(),
            byte,
            dest,
            occurrences: 1,
        });
    }

    fn into_details(self) -> Vec<String> {
        let dropped = self.dropped;
        let mut out: Vec<String> = self
            .rows
            .into_iter()
            .map(|r| {
                let more = if r.occurrences > 1 {
                    format!(" (and {} more with the same value)", r.occurrences - 1)
                } else {
                    String::new()
                };
                // Kept to one readable line: this string is shown to the AUTHOR, in the editor's
                // Save dialog. Where, what, and which compiled field — the schema reference and
                // the roster-wire mechanics live in this module's docs, not in their face.
                format!(
                    "{}: {} contains {} — control characters break the in-game roster and are \
                     rejected when the mission compiles (reaches {}){}",
                    r.location,
                    quote_value(&r.value),
                    describe(r.byte),
                    r.dest.compiled_field(),
                    more,
                )
            })
            .collect();
        if dropped > 0 {
            out.push(format!(
                "/editor: {dropped} further distinct value(s) carry a forbidden control character \
                 — fix the ones above and save again to see the rest"
            ));
        }
        out
    }
}

/// Scan an already-parsed editor payload for authored strings that would violate
/// `wireSafeString` once compiled. Empty result = nothing to report.
///
/// Takes a `&Value` on purpose: the caller (`contract::validate`) has already parsed the payload to
/// run the JSON-Schema pass, so this adds a walk but not a parse. Re-parsing a 140 MB save payload
/// to run this separately would cost far more than the check.
#[must_use]
pub fn scan_editor_payload(payload: &Value) -> Vec<String> {
    let Some(editor) = payload.get("editor") else {
        return Vec::new();
    };
    let mut acc = Findings::default();

    if let Some(factions) = editor.get("factions").and_then(Value::as_array) {
        for (i, f) in factions.iter().enumerate() {
            // Only a NON-empty name reaches displayName — flatten falls back to slug_key(key),
            // which is `[a-z][a-z0-9_]*` by construction and cannot carry a control character.
            if non_empty(f.get("name")) {
                acc.check(f.get("name"), Dest::DisplayName, || {
                    format!("/editor/factions/{i}/name")
                });
            }
        }
    }

    if let Some(squads) = editor.get("squads").and_then(Value::as_array) {
        for (i, sq) in squads.iter().enumerate() {
            // flatten's exact chain: callsign → name → id → CALLSIGN_FALLBACK. Scanning the two
            // rungs the compile would never read would reject a payload that compiles clean.
            let (key, field) = if non_empty(sq.get("callsign")) {
                (sq.get("callsign"), "callsign")
            } else if non_empty(sq.get("name")) {
                (sq.get("name"), "name")
            } else if non_empty(sq.get("id")) {
                (sq.get("id"), "id")
            } else {
                (None, "")
            };
            acc.check(key, Dest::GroupCallsign, || {
                format!("/editor/squads/{i}/{field}")
            });
        }
    }

    if let Some(slots) = editor.get("slots").and_then(Value::as_array) {
        // The hot loop — this is the one that runs 367k times. Two `Map::get` lookups and two byte
        // scans over ~10-40 bytes each, with no allocation unless something is actually wrong.
        for (i, sl) in slots.iter().enumerate() {
            if non_empty(sl.get("role")) {
                acc.check(sl.get("role"), Dest::Role, || {
                    format!("/editor/slots/{i}/role")
                });
            }
            // The editor generates slot ids, but flatten copies this one VERBATIM into `uid`
            // (no fallback rung at all), so an imported or hand-edited payload reaches the wire
            // through it unfiltered.
            acc.check(sl.get("id"), Dest::Uid, || format!("/editor/slots/{i}/id"));
        }
    }

    acc.into_details()
}

fn non_empty(v: Option<&Value>) -> bool {
    v.and_then(Value::as_str).is_some_and(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn byte_scan_equals_char_scan_over_the_schema_pattern() {
        // The byte shortcut is only legal because no UTF-8 continuation byte can collide with the
        // C0/DEL set. Prove it on the boundaries and on real multi-byte text.
        for c in ['\u{0}', '\u{1f}', '\u{7f}', '\t', '\n', '\r'] {
            assert!(
                first_unsafe_byte(&c.to_string()).is_some(),
                "{c:?} must be caught"
            );
        }
        for s in ["ALPHA", "Ålpha — Brávo 中文 🎯", " ", "", "a.b~c:d"] {
            assert_eq!(first_unsafe_byte(s), None, "{s:?} must be wire-safe");
        }
    }

    #[test]
    fn clean_payload_reports_nothing() {
        let p = json!({"editor": {
            "factions": [{"key": "blufor", "name": "US Army"}],
            "squads": [{"id": "sq1", "callsign": "Alpha", "slotIds": ["s1"]}],
            "slots": [{"id": "s1", "role": "SL"}],
        }});
        assert!(scan_editor_payload(&p).is_empty());
    }

    #[test]
    fn tab_in_callsign_is_reported_with_location_and_cause() {
        let p = json!({"editor": {
            "squads": [{"id": "sq1", "callsign": "AL\tPHA"}],
        }});
        let d = scan_editor_payload(&p);
        assert_eq!(d.len(), 1, "{d:?}");
        assert!(d[0].starts_with("/editor/squads/0/callsign:"), "{d:?}");
        assert!(
            d[0].contains("\"AL\\tPHA\""),
            "value must be escaped: {d:?}"
        );
        assert!(d[0].contains("TAB (U+0009)"), "{d:?}");
        assert!(d[0].contains("slots[].groupCallsign"), "{d:?}");
    }

    #[test]
    fn substituted_blanks_are_not_reported_but_the_rung_that_is_read_is() {
        // callsign blank → flatten reads `name`; a bad `callsign` here is not read at all, so
        // reporting it would reject a payload that compiles clean.
        let clean =
            json!({"editor": {"squads": [{"id": "sq\t1", "callsign": "Alpha", "name": "n\tm"}]}});
        assert!(scan_editor_payload(&clean).is_empty());

        let dirty = json!({"editor": {"squads": [{"id": "sq1", "callsign": "", "name": "n\tm"}]}});
        let d = scan_editor_payload(&dirty);
        assert_eq!(d.len(), 1, "{d:?}");
        assert!(d[0].starts_with("/editor/squads/0/name:"), "{d:?}");

        // Blank role → ROLE_FALLBACK, which is wire-safe; blank faction name → slug_key(key).
        let blanks = json!({"editor": {"factions": [{"key": "blufor", "name": ""}], "slots": [{"id": "s1", "role": ""}]}});
        assert!(scan_editor_payload(&blanks).is_empty());
    }

    #[test]
    fn identical_bad_values_collapse_to_one_row_with_a_count() {
        let slots: Vec<Value> = (0..2000)
            .map(|i| json!({"id": format!("s{i}"), "role": "SL\tX"}))
            .collect();
        let d = scan_editor_payload(&json!({"editor": {"slots": slots}}));
        assert_eq!(
            d.len(),
            1,
            "a bulk paste must not produce 2000 lines: {d:?}"
        );
        assert!(d[0].contains("and 1999 more with the same value"), "{d:?}");
    }

    #[test]
    fn distinct_bad_values_are_capped_with_a_tail_line() {
        let slots: Vec<Value> = (0..MAX_REPORTED + 5)
            .map(|i| json!({"id": format!("s{i}"), "role": format!("SL\t{i}")}))
            .collect();
        let d = scan_editor_payload(&json!({"editor": {"slots": slots}}));
        assert_eq!(d.len(), MAX_REPORTED + 1);
        assert!(
            d[MAX_REPORTED].contains("5 further distinct value(s)"),
            "{d:?}"
        );
    }

    #[test]
    fn slot_id_is_scanned_because_flatten_copies_it_verbatim_into_uid() {
        let p = json!({"editor": {"slots": [{"id": "s\u{7f}1", "role": "SL"}]}});
        let d = scan_editor_payload(&p);
        assert_eq!(d.len(), 1, "{d:?}");
        assert!(d[0].contains("DEL (U+007F)"), "{d:?}");
        assert!(d[0].contains("slots[].uid"), "{d:?}");
    }

    #[test]
    fn missing_editor_block_is_not_an_error() {
        assert!(scan_editor_payload(&json!({})).is_empty());
        assert!(scan_editor_payload(&json!({"editor": 7})).is_empty());
    }
}
