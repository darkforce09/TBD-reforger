//! Role: classify.
//! Position: `overlay/symbology/roles` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Canonical side blufor rgba value.
pub const SIDE_BLUFOR_RGBA: [u8; 4] = [173, 198, 255, 255];

/// Canonical side opfor rgba value.
pub const SIDE_OPFOR_RGBA: [u8; 4] = [248, 113, 113, 255];

/// Canonical side indfor rgba value.
pub const SIDE_INDFOR_RGBA: [u8; 4] = [34, 197, 94, 255];

/// Map faction `key` → unselected ring RGBA. Unknown / empty → BLUFOR (C-L4).
#[must_use]
pub fn side_rgba(key: &str) -> [u8; 4] {
    match key {
        "BLUFOR" => SIDE_BLUFOR_RGBA,
        "OPFOR" => SIDE_OPFOR_RGBA,
        "INDFOR" => SIDE_INDFOR_RGBA,
        _ => SIDE_BLUFOR_RGBA,
    }
}

/// Side tints rgba bytes.
#[must_use]
pub fn side_tints_rgba_bytes(side_keys: &[String]) -> Vec<u8> {
    let mut out = Vec::with_capacity(side_keys.len() * 4);
    for k in side_keys {
        out.extend_from_slice(&side_rgba(k));
    }
    out
}

/// Role class of one slot — the drawable distinction a milsim author reads a map by.
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitRoleClass {
    /// Plain filled disc — the DEFAULT for every unrecognised role or kit.
    Rifleman = 0,

    /// Chevron knocked out of the disc. Squad / team / platoon leaders, commanders, officers.
    Leader = 1,

    /// Plus cross knocked out of the disc. Medic / corpsman / CLS.
    Medic = 2,

    /// Solid down-triangle knocked out of the disc. AT gunner / launcher / assistant.
    AntiTank = 3,

    /// Twin horizontal bars knocked out of the disc. Automatic rifleman / MG / SAW gunner.
    MachineGun = 4,
}

/// Number of [`UnitRoleClass`] variants (also the count of unselected unit cells in the atlas).
pub const UNIT_ROLE_CLASS_COUNT: usize = 5;

/// Top-down vehicle silhouette class. Discriminant is the offset within the vehicle cell block.
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VehicleKind {
    /// Short 4-wheel hull with a chamfered nose — M1025 / M998 Humvee, jeeps, UAZ/BRDM-sized light wheeled. The DEFAULT for an unrecognised alias (the commonest thing on a mission map).
    WheeledLight = 0,

    /// Long hull with a separate cab block and six wheels — M923A1, Ural, cargo/fuel/supply trucks.
    Truck = 1,

    /// Trapezoid hull with continuous TRACK RAILS — M113, BTR/BMP-class carriers, IFVs, tanks.
    Apc = 2,
}

/// Number of [`VehicleKind`] variants (also the count of vehicle cells in the atlas).
pub const VEHICLE_KIND_COUNT: usize = 3;

/// Normalise role.
#[must_use]
pub(crate) fn normalise_role(raw: &str) -> String {
    let t = raw
        .trim()
        .to_ascii_lowercase()
        .replace(['-', ' ', '/'], "_");
    for p in ["kit:", "veh:", "vehicle:", "preset:", "comp:"] {
        if let Some(rest) = t.strip_prefix(p) {
            return rest.to_string();
        }
    }
    t
}

/// **THE role → glyph table.** One place, unit-tested against every seeded kit; nothing else in the tree may re-derive this mapping.
#[must_use]
pub fn unit_role_class(role_or_kit: &str) -> UnitRoleClass {
    let n = normalise_role(role_or_kit);
    let has = |w: &str| n.split('_').any(|t| t == w);

    if has("sl")
        || has("tl")
        || has("pl")
        || has("co")
        || has("xo")
        || has("leader")
        || has("lead")
        || has("commander")
        || has("officer")
        || has("squadleader")
        || n.contains("squad_leader")
        || n.contains("team_leader")
        || n.contains("platoon_leader")
    {
        return UnitRoleClass::Leader;
    }

    if has("medic")
        || has("medical")
        || has("corpsman")
        || has("cls")
        || has("doc")
        || has("aid")
        || has("casevac")
        || has("medevac")
    {
        return UnitRoleClass::Medic;
    }

    if has("at")
        || has("atgm")
        || has("antitank")
        || has("rpg")
        || has("law")
        || has("panzerfaust")
        || has("launcher")
        || n.contains("anti_tank")
        || n.contains("at_gunner")
        || n.contains("at_rifleman")
        || n.contains("at_assistant")
    {
        return UnitRoleClass::AntiTank;
    }

    if has("ar")
        || has("mg")
        || has("lmg")
        || has("hmg")
        || has("gpmg")
        || has("saw")
        || has("gunner")
        || has("machinegunner")
        || has("autorifleman")
        || n.contains("machine_gun")
        || n.contains("automatic_rifleman")
    {
        return UnitRoleClass::MachineGun;
    }
    UnitRoleClass::Rifleman
}

/// **THE vehicle alias → silhouette table.** Same contract as [`unit_role_class`]: one place, unit-tested, unknown → [`VehicleKind::WheeledLight`].
#[must_use]
pub fn vehicle_kind_for_alias(alias: &str) -> VehicleKind {
    let n = normalise_role(alias);

    if n.contains("m113")
        || n.contains("apc")
        || n.contains("btr")
        || n.contains("bmp")
        || n.contains("bradley")
        || n.contains("tracked")
        || n.contains("tank")
        || n.contains("ifv")
    {
        return VehicleKind::Apc;
    }
    if n.contains("m923")
        || n.contains("truck")
        || n.contains("ural")
        || n.contains("cargo")
        || n.contains("supply")
        || n.contains("fuel")
        || n.contains("m35")
    {
        return VehicleKind::Truck;
    }
    VehicleKind::WheeledLight
}
