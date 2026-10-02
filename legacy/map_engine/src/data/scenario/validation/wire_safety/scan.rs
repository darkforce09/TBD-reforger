//! Role: scan.
//! Position: `mission/validation/wire_safety` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{HashMap, Value};

/// Canonical max reported value.
pub const MAX_REPORTED: usize = 20;

/// True for the characters `wireSafeString` forbids: the C0 control block plus DEL.
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

/// Render an offending value safely for a log line or an error body: control characters become visible escapes (a raw TAB echoed into the message would be as invisible there as it was in the editor), and a long value is elided.
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

/// Where an authored string lands in the compiled document — the half of a finding that tells the author why a field they thought was free text is not.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Dest {
    /// Domain representation of display name.
    DisplayName,
    /// Domain representation of group callsign.
    GroupCallsign,
    /// Domain representation of role.
    Role,
    /// Domain representation of uid.
    Uid,
}

impl Dest {
    /// Compiled field using the supplied domain data.
    pub(super) const fn compiled_field(self) -> &'static str {
        match self {
            Dest::DisplayName => "factions[].displayName",
            Dest::GroupCallsign => "slots[].groupCallsign (and slots[].id)",
            Dest::Role => "slots[].role (and slots[].id)",
            Dest::Uid => "slots[].uid",
        }
    }
}

/// Domain representation of findings.
#[derive(Default)]
pub(super) struct Findings {
    /// (destination, offending value) → index into `rows`. Deduping on the VALUE is what keeps a bulk paste of one bad role from producing 10k identical lines.
    pub(super) seen: HashMap<(Dest, String), usize>,
    /// Rows.
    pub(super) rows: Vec<Row>,

    /// Distinct values dropped after [`MAX_REPORTED`].
    pub(super) dropped: usize,
}

/// Domain representation of row.
pub(super) struct Row {
    /// Location.
    pub(super) location: String,
    /// Value.
    pub(super) value: String,
    /// Byte.
    pub(super) byte: u8,
    /// Dest.
    pub(super) dest: Dest,
    /// Occurrences.
    pub(super) occurrences: usize,
}

impl Findings {
    /// Check one authored string. `location` is only formatted when there is something to report, so the clean path — every slot of a healthy mission — allocates nothing.
    pub(super) fn check(
        &mut self,
        value: Option<&Value>,
        dest: Dest,
        location: impl FnOnce() -> String,
    ) {
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

    /// Into details using the supplied domain data.
    pub(super) fn into_details(self) -> Vec<String> {
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

/// Scan an already-parsed editor payload for authored strings that would violate `wireSafeString` once compiled. Empty result = nothing to report.
#[must_use]
pub fn scan_editor_payload(payload: &Value) -> Vec<String> {
    let Some(editor) = payload.get("editor") else {
        return Vec::new();
    };
    let mut acc = Findings::default();

    if let Some(factions) = editor.get("factions").and_then(Value::as_array) {
        for (i, f) in factions.iter().enumerate() {
            if non_empty(f.get("name")) {
                acc.check(f.get("name"), Dest::DisplayName, || {
                    format!("/editor/factions/{i}/name")
                });
            }
        }
    }

    if let Some(squads) = editor.get("squads").and_then(Value::as_array) {
        for (i, sq) in squads.iter().enumerate() {
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
        for (i, sl) in slots.iter().enumerate() {
            if non_empty(sl.get("role")) {
                acc.check(sl.get("role"), Dest::Role, || {
                    format!("/editor/slots/{i}/role")
                });
            }

            acc.check(sl.get("id"), Dest::Uid, || format!("/editor/slots/{i}/id"));
        }
    }

    acc.into_details()
}

/// Non empty using the supplied domain data.
pub(super) fn non_empty(v: Option<&Value>) -> bool {
    v.and_then(Value::as_str).is_some_and(|s| !s.is_empty())
}

/// Wear/container keys that carry cargo on `SlotLoadoutV2` — byte-identical to `arsenal::rules::CARGO_CONTAINERS`.
pub(super) const CARGO_CONTAINERS: &[&str] = &["vest", "pants", "jacket", "backpack"];

/// Why an over-capacity fault is a refusal and not a prediction. Copied in substance from `arsenal::rules::CARGO_CAPACITY_CAVEAT` so Save and Arsenal export do not disagree about what the number means.
pub const CARGO_CAPACITY_CAVEAT: &str = "Capacity is a build-time catalogue figure the game never reads back, so treat it as an estimate, not a guarantee. The failure it points at is real: at spawn, cargo the character cannot hold is silently moved to another container or dropped — the rest of that row goes with it — and nothing reports it.";

/// Phys attrs for one `registry_items` row — the only registry surface this crate will accept.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CargoPhys {
    /// Display name.
    pub display_name: String,
    /// Weight kg.
    pub weight_kg: Option<f64>,
    /// Volume cm3.
    pub volume_cm3: Option<f64>,
    /// Max weight kg.
    pub max_weight_kg: Option<f64>,
    /// Max volume cm3.
    pub max_volume_cm3: Option<f64>,
}

/// `resource_name →` phys attrs. Empty map ⇒ [`scan_cargo_capacity`] reports nothing (never invent).
pub type CargoPhysCatalog = HashMap<String, CargoPhys>;

/// Scan authored `editor.slots[].loadout` cargo against garment capacities in `catalog`.
#[must_use]
pub fn scan_cargo_capacity(payload: &Value, catalog: &CargoPhysCatalog) -> Vec<String> {
    if catalog.is_empty() {
        return Vec::new();
    }
    let Some(slots) = payload
        .get("editor")
        .and_then(|e| e.get("slots"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for (i, sl) in slots.iter().enumerate() {
        if out.len() >= MAX_REPORTED {
            out.push(
                "/editor: further slot(s) carry over-capacity cargo — fix the ones above and save \
                 again to see the rest"
                    .to_string(),
            );
            break;
        }
        let Some(lo) = sl.get("loadout").filter(|v| v.is_object()) else {
            continue;
        };
        let wear = lo.get("wear");
        let rows = lo.get("cargo").and_then(Value::as_array);
        for container in CARGO_CONTAINERS {
            if out.len() >= MAX_REPORTED {
                break;
            }
            let Some((row_key, garment_rn)) = cargo_garment(wear, container) else {
                continue;
            };
            let garment = catalog.get(garment_rn);
            let mut weight = 0.0_f64;
            let mut volume = 0.0_f64;
            if let Some(rows) = rows {
                for r in rows {
                    let Some(c) = r.get("container").and_then(Value::as_str) else {
                        continue;
                    };
                    if c != *container {
                        continue;
                    }
                    let Some(item) = r.get("item").and_then(Value::as_str) else {
                        continue;
                    };
                    let qty = r
                        .get("qty")
                        .and_then(Value::as_i64)
                        .filter(|q| *q >= 1)
                        .unwrap_or(0) as f64;
                    if qty == 0.0 {
                        continue;
                    }
                    if let Some(it) = catalog.get(item) {
                        weight += it.weight_kg.unwrap_or(0.0) * qty;
                        volume += it.volume_cm3.unwrap_or(0.0) * qty;
                    }
                }
            }
            let max_weight = garment.and_then(|g| g.max_weight_kg);
            let max_volume = garment.and_then(|g| g.max_volume_cm3);
            let over_w = max_weight.is_some_and(|m| weight > m);
            let over_v = max_volume.is_some_and(|m| volume > m);
            if !over_w && !over_v {
                continue;
            }
            let mut dims: Vec<String> = Vec::new();
            if let Some(m) = max_weight.filter(|m| weight > *m) {
                dims.push(format!("{weight:.1} / {m} kg"));
            }
            if let Some(m) = max_volume.filter(|m| volume > *m) {
                dims.push(format!("{volume:.0} / {m} cm³"));
            }
            let garment_label = garment
                .map(|g| g.display_name.as_str())
                .filter(|s| !s.is_empty())
                .unwrap_or(garment_rn);
            out.push(format!(
                "/editor/slots/{i}/loadout/wear/{row_key}: {container} cargo is over the \
                 catalogued capacity of {garment_label} — {}. {CARGO_CAPACITY_CAVEAT}",
                dims.join(" · "),
            ));
        }
    }
    out
}

/// Worn garment backing a cargo container key. `vest` accepts `armoredVest` — same spike lock as `arsenal::rules::cargo_garment`. Returns the **wear row key** the author must change.
pub(super) fn cargo_garment<'a>(
    wear: Option<&'a Value>,
    container: &str,
) -> Option<(&'static str, &'a str)> {
    let wear = wear?;
    let live = |k: &'static str| {
        wear.get(k)
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
            .map(|v| (k, v))
    };
    match container {
        "vest" => live("vest").or_else(|| live("armoredVest")),
        "pants" => live("pants"),
        "jacket" => live("jacket"),
        "backpack" => live("backpack"),
        _ => None,
    }
}
