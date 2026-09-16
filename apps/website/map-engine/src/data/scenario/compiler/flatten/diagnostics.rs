//! Role: diagnostics.
//! Position: `mission/compiler/flatten` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{
    DIAG_DROP_SQUAD_LEADER, DIAG_DROP_VEHICLE_ROSTER, DIAG_WIN_CONDITIONS, Finding, Primitive,
    Severity, SlotIn, SquadIn, VehicleIn, is_wire_unsafe, or_fallback,
};

/// A `wireSafeString` identity value (`callsign` / `unitName` / `tag` / `leaderSlotId`), or `None` when the compile must drop it whole.
pub(super) fn emit_wire_safe_identity(v: &serde_json::Value) -> Option<String> {
    let s = v.as_str()?;
    if s.trim().is_empty() || s.bytes().any(is_wire_unsafe) {
        return None;
    }
    Some(s.to_string())
}

/// An enum-typed identity value (`rank` / `stance`) resolved to the SCHEMA'S OWN spelling, or `None` when the compile must drop it whole.
pub(super) fn emit_enum_identity(v: &serde_json::Value, allowed: &[&str]) -> Option<String> {
    let raw = v.as_str()?.trim();
    allowed
        .iter()
        .find(|token| token.eq_ignore_ascii_case(raw))
        .map(|token| (*token).to_string())
}

/// Why an authored identity value could not be carried — the clause the diagnostic message reads out, so the finding tells the author what to change instead of what the platform has not built.
pub(super) fn identity_drop_reason(key: &str, value: &serde_json::Value) -> &'static str {
    if !value.is_string() {
        return "the authored value is not a string";
    }
    match key {
        "rank" => {
            "it is not one of the ranks `$defs/slot.rank` declares \
                   (private, corporal, sergeant, lieutenant, captain, major, colonel)"
        }
        "stance" => {
            "it is not one of the poses `$defs/slot.stance` declares \
                     (stand, crouch, prone)"
        }
        _ => {
            "it carries a control character, which `wireSafeString` forbids — a tab or newline \
              here shifts every column of the mod's tab-separated roster wire and makes a seat \
              unselectable"
        }
    }
}

/// Accumulates the compile's [`Finding`]s during the one document walk.
#[derive(Debug, Default)]
pub(super) struct DiagnosticAcc {
    /// Findings.
    pub(super) findings: Vec<Finding>,
}

impl DiagnosticAcc {
    /// Push using the supplied domain data.
    pub(super) fn push(
        &mut self,
        rule_id: &'static str,
        severity: Severity,
        message: String,
        subject: String,
        subject_id: &str,
    ) {
        self.findings.push(Finding {
            rule_id,
            severity,
            primitive: Primitive::PerObjectInvariant,
            message,
            subject,

            subject_id: (!subject_id.is_empty()).then(|| subject_id.to_string()),
        });
    }

    /// The squad leader designation, dropped. `Warning`, not `Info`: nothing on the wire says who leads, so the game server picks one — a behavioural difference, not a cosmetic one.
    pub(super) fn squad_leader_dropped(
        &mut self,
        squad_index: usize,
        sq: &SquadIn,
        leader: &str,
        reason: &str,
    ) {
        self.push(
            DIAG_DROP_SQUAD_LEADER,
            Severity::Warning,
            format!(
                "Squad {} designates slot {leader} as its leader and the compile drops it: \
                 {reason}. `$defs/group.leaderSlotId` cannot carry it, so the game server chooses \
                 a leader for you.",
                display_squad(sq)
            ),
            format!("/editor/squads/{squad_index}/leaderSlotId"),
            &sq.id,
        );
    }

    /// One of the five per-seat identity keys, dropped. `Info`: the seat still spawns in the right place with the right kit; what is lost is how it is LABELLED.
    pub(super) fn slot_identity_dropped(
        &mut self,
        rule_id: &'static str,
        key: &str,
        slot_index: usize,
        sl: &SlotIn,
        value: &serde_json::Value,
        reason: &str,
    ) {
        self.push(
            rule_id,
            Severity::Info,
            format!(
                "Slot {} authors {key} {} and the compile drops it: {reason}. \
                 `mission.schema.json` closes `$defs/slot`, so an unrepresentable {key} would be \
                 a 500 at `/compiled` rather than a seat label — the game server never sees this \
                 seat's {key}.",
                display_slot(sl),
                render_authored(value)
            ),
            format!("/editor/slots/{slot_index}/{key}"),
            &sl.id,
        );
    }

    /// **It drops the ROW, never a field of it.** Emitting a vehicle whose crew plan is one seat short would ship a roster that reads differently from the author's editor with nobody told — the trimming [`emit_wire_safe_identity`] refuses for a name, with a soldier left on the ground instead of a label.
    pub(super) fn vehicle_roster_dropped(
        &mut self,
        vehicle_index: usize,
        v: &VehicleIn,
        reason: &str,
    ) {
        self.push(
            DIAG_DROP_VEHICLE_ROSTER,
            Severity::Warning,
            format!(
                "Vehicle {} is on the authored roster and the compile drops the whole row: \
                 {reason}. `mission.schema.json` closes `$defs/vehicle`, so a row the wire cannot \
                 carry would be a 500 at `/compiled` rather than a vehicle — the game server never \
                 sees this vehicle's crew plan.",
                display_vehicle(v)
            ),
            format!("/vehicles/{vehicle_index}"),
            &v.id,
        );
    }

    /// `subject_id` is the offending REFERENCE (a slot uid, a zone id) when the finding is about one, so the panel can select what the author must fix; `"winConditions"` otherwise, because the block itself is the subject and a finding with no owner is one the panel cannot route.
    pub(super) fn win_conditions(&mut self, message: String, subject_id: &str) {
        self.push(
            DIAG_WIN_CONDITIONS,
            Severity::Warning,
            message,
            "/winConditions".to_string(),
            subject_id,
        );
    }
}

/// `Alpha (sq1)` / `sq1` — a squad named the way an author recognises it, never a bare index.
pub(super) fn display_squad(sq: &SquadIn) -> String {
    let label = or_fallback(&sq.callsign, &sq.name);
    if label.is_empty() {
        format!("`{}`", sq.id)
    } else {
        format!("{label} (`{}`)", sq.id)
    }
}

/// `RFL (s1)` / `s1` — a slot named by its role, which is what the ORBAT tree shows.
pub(super) fn display_slot(sl: &SlotIn) -> String {
    if sl.role.is_empty() {
        format!("`{}`", sl.id)
    } else {
        format!("{} (`{}`)", sl.role, sl.id)
    }
}

/// `M151A2_M2HB.et (v1)` — the tail of the ResourceName, which is the readable half.
pub(super) fn display_vehicle(v: &VehicleIn) -> String {
    let tail = v
        .resource_name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .trim();
    if tail.is_empty() {
        format!("`{}`", v.id)
    } else {
        format!("{tail} (`{}`)", v.id)
    }
}

/// Render an authored value for a message without letting a hostile payload write the sentence.
pub(super) fn render_authored(v: &serde_json::Value) -> String {
    const MAX: usize = 60;
    match v {
        serde_json::Value::String(s) => {
            let clean: String = s.chars().filter(|c| !c.is_control()).take(MAX).collect();
            if s.chars().count() > MAX {
                format!("\"{clean}…\"")
            } else {
                format!("\"{clean}\"")
            }
        }
        serde_json::Value::Number(n) => format!("`{n}`"),
        serde_json::Value::Bool(b) => format!("`{b}`"),
        serde_json::Value::Array(_) => "an array".to_string(),
        serde_json::Value::Object(_) => "an object".to_string(),
        serde_json::Value::Null => "null".to_string(),
    }
}

/// [`render_authored`] for a value already known to be a string — the same quoting, control-byte stripping and truncation, so a hostile `resourceName`, vehicle id or seat key cannot write the diagnostic's sentence either.
pub(super) fn render_authored_str(s: &str) -> String {
    render_authored(&serde_json::Value::String(s.to_string()))
}

/// Is this raw authored value something the author actually SET?.
pub(super) fn is_authored(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Null => false,
        serde_json::Value::String(s) => !s.trim().is_empty(),
        _ => true,
    }
}
