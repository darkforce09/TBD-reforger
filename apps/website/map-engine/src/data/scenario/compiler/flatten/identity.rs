//! Role: identity.
//! Position: `mission/compiler/flatten` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

/// Stable rule id: the squad leader designation (`editor.squads[].leaderSlotId`) is dropped.
pub const DIAG_DROP_SQUAD_LEADER: &str = "COMPILE-DROP-SQUAD-LEADER";

/// Stable rule id: a slot's authored `tag` is dropped.
pub const DIAG_DROP_SLOT_TAG: &str = "COMPILE-DROP-SLOT-TAG";

/// Canonical diag drop slot callsign value.
pub const DIAG_DROP_SLOT_CALLSIGN: &str = "COMPILE-DROP-SLOT-CALLSIGN";

/// Stable rule id: a slot's authored `rank` is dropped.
pub const DIAG_DROP_SLOT_RANK: &str = "COMPILE-DROP-SLOT-RANK";

/// Stable rule id: a slot's authored `stance` is dropped.
pub const DIAG_DROP_SLOT_STANCE: &str = "COMPILE-DROP-SLOT-STANCE";

/// Canonical diag drop slot unit name value.
pub const DIAG_DROP_SLOT_UNIT_NAME: &str = "COMPILE-DROP-SLOT-UNIT-NAME";

/// Stable rule id: the authored vehicle ROSTER (top-level `vehicles[]`, seats + crew) is dropped.
pub const DIAG_DROP_VEHICLE_ROSTER: &str = "COMPILE-DROP-VEHICLE-ROSTER";

/// **One id for four situations, deliberately, exactly as [`DIAG_DROP_VEHICLE_ROSTER`] is one id for six drop reasons.** They are one rule to a consumer — "your win rule is not the rule this mission will run" — and the MESSAGE is what says which:.
pub const DIAG_WIN_CONDITIONS: &str = "COMPILE-WIN-CONDITIONS";

/// Every diagnostic rule id this compile can emit, in emission order. The single source of truth a consumer (the panel legend, a smoke harness, the `/compiled` response header) enumerates rather than re-listing.
pub const COMPILE_DIAGNOSTIC_RULE_IDS: [&str; 8] = [
    DIAG_DROP_SQUAD_LEADER,
    DIAG_DROP_SLOT_TAG,
    DIAG_DROP_SLOT_CALLSIGN,
    DIAG_DROP_SLOT_RANK,
    DIAG_DROP_SLOT_STANCE,
    DIAG_DROP_SLOT_UNIT_NAME,
    DIAG_DROP_VEHICLE_ROSTER,
    DIAG_WIN_CONDITIONS,
];

/// Canonical slot identity drops value.
pub(super) const SLOT_IDENTITY_DROPS: [(&str, &str); 5] = [
    ("tag", DIAG_DROP_SLOT_TAG),
    ("callsign", DIAG_DROP_SLOT_CALLSIGN),
    ("rank", DIAG_DROP_SLOT_RANK),
    ("stance", DIAG_DROP_SLOT_STANCE),
    ("unitName", DIAG_DROP_SLOT_UNIT_NAME),
];

/// `mission.schema.json#/$defs/slot/properties/rank/enum`, in the schema's own order.
pub(super) const SLOT_RANKS: [&str; 7] = [
    "private",
    "corporal",
    "sergeant",
    "lieutenant",
    "captain",
    "major",
    "colonel",
];

/// `mission.schema.json#/$defs/slot/properties/stance/enum` — the initial spawn pose. Same schema-read coupling as [`SLOT_RANKS`]; `doc/store.rs` authors exactly these three spellings.
pub(super) const SLOT_STANCES: [&str; 3] = ["stand", "crouch", "prone"];

/// `mission.schema.json#/$defs/vehicle/properties/seats/items/properties/role/enum`, in the schema's own order — which is also the order the seats of one vehicle are emitted in.
pub(super) const VEHICLE_SEAT_ROLES: [&str; 7] = [
    "driver",
    "commander",
    "gunner",
    "cargo",
    "pilot",
    "copilot",
    "turret",
];

/// Project one authored crew seat id onto `(role, index)`, or `None` when the wire has no station that means it.
pub(super) fn parse_seat_id(seat_id: &str) -> Option<(&'static str, Option<u32>)> {
    let lower = seat_id.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return None;
    }
    if let Some(role) = VEHICLE_SEAT_ROLES.iter().find(|r| **r == lower) {
        return Some((*role, None));
    }

    let at = lower.find(|c: char| c.is_ascii_digit())?;
    let (head, ordinal) = lower.split_at(at);
    let role = VEHICLE_SEAT_ROLES.iter().find(|r| **r == head)?;
    let n: u32 = ordinal.parse().ok()?;

    n.checked_sub(1).map(|index| (*role, Some(index)))
}
