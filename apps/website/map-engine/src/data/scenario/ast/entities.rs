//! Role: entities.
//! Position: `mission/ast` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use serde::Serialize;

/// Editor rows carry `id` / `resourceName` / nested `position` / `cargo`; this struct is the SCHEMA shape only (`alias`/`x`/`z` required). Extra editor keys are dropped here so `additionalProperties: false` on `$defs/entity` cannot 500 `/compiled`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModEntity {
    /// Alias.
    pub alias: String,

    /// `$defs/entity.uid` — the editor's own id for the authored object this row came from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid: Option<String>,
    /// X.
    pub x: f64,
    /// Z.
    pub z: f64,
    /// Heading deg.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading_deg: Option<f64>,
    /// Faction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faction: Option<String>,

    /// Inventory.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub inventory: Vec<ModEntityInventory>,
}

/// One `$defs/entityInventory` row — `{item, qty}` only (no `container` key).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ModEntityInventory {
    /// Item.
    pub item: String,
    /// Qty.
    pub qty: i64,
}

/// Domain representation of mod vehicle.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModVehicle {
    /// `$defs/vehicle.alias` — the `veh:` registry alias, resolved from the authored `resourceName` through the same kit-aliases table [`derive_vehicles_as_entities`] uses. Schema-required; a vehicle with no alias drops WHOLE rather than borrowing another's.
    pub alias: String,

    /// `$defs/vehicle.uid` — the editor's own vehicle id, carried verbatim. Optional: a vehicle nothing references needs no uid, and an unauthored (blank) id is not identity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid: Option<String>,
    /// X.
    pub x: f64,
    /// Z.
    pub z: f64,

    /// Unconditional, exactly like [`ModEntity::heading_deg`] on the sibling row: `position. rotation` is always readable and defaults to 0, so there is no "unauthored heading" state to distinguish. The key is optional in the schema; emitting it always keeps the two projections of one authored vehicle from disagreeing about which way it faces.
    pub heading_deg: f64,

    /// `$defs/factionKey`, derived exactly as the entity row derives it (shared [`vehicle_faction_key`]) so the two rows cannot claim different sides.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faction: Option<String>,

    /// The crew plan. Empty omits the key — an uncrewed vehicle is a legal roster row, and the omission is what keeps its wire shape identical to a vehicle that was never crewed.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub seats: Vec<ModVehicleSeat>,

    /// `$defs/vehicle.inventory` — the same cargo the `entities[]` twin carries, from the same [`vehicle_inventory`] derivation so the two projections cannot disagree about what is in the vehicle.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub inventory: Vec<ModEntityInventory>,
}

/// Domain representation of mod vehicle seat.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModVehicleSeat {
    /// References `slots[].uid` — the DURABLE identity, never the derived `slots[].id`, which shifts under role renames and reorders (the same rule `leaderSlotId` follows). Always emitted: every seat here came from a crew entry, so it always names an occupant.
    pub slot_id: String,

    /// A token of [`VEHICLE_SEAT_ROLES`], the schema's own closed enum.
    pub role: String,

    /// Disambiguates several stations of the same role (`cargo` 0, 1, 2…). Absent for a station the author named without an ordinal (`driver`); see [`parse_seat_id`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
}

/// One flattened `slots[]` entry.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModSlot {
    /// Id.
    pub id: String,

    /// Uid.
    pub uid: String,
    /// Faction.
    pub faction: String,
    /// Group callsign.
    pub group_callsign: String,
    /// Role.
    pub role: String,
    /// Kit.
    pub kit: String,
    /// X.
    pub x: f64,
    /// Z.
    pub z: f64,
    /// Y.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
    /// Heading deg.
    pub heading_deg: f64,

    /// Loadout.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loadout: Option<ModSlotLoadout>,

    /// Callsign.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callsign: Option<String>,

    /// `$defs/slot.rank` — one of [`SLOT_RANKS`], the schema's own ladder, in the schema's own spelling. Case-folded from the author's (`"Sergeant"` → `"sergeant"`); anything off the ladder drops.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rank: Option<String>,

    /// `$defs/slot.stance` — one of [`SLOT_STANCES`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stance: Option<String>,

    /// `$defs/slot.unitName`. `rename_all = "camelCase"` above spells this `unitName` on the wire — the mod binds this block by field NAME through `JsonLoadContext`, which ignores keys it does not recognise, so the casing is the contract (the `freqMHz` trap on [`ModNet`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_name: Option<String>,

    /// Tag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

/// Domain representation of mod slot loadout.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModSlotLoadout {
    /// Gear.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gear: Option<ModSlotGear>,
    /// Cargo.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cargo: Vec<ModSlotCargo>,
}

/// Domain representation of mod slot gear.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModSlotGear {
    /// Primary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary: Option<String>,
    /// Optic.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optic: Option<String>,
    /// Magazine.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub magazine: Option<String>,

    /// Attachments.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<String>,

    /// Launcher.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launcher: Option<String>,
    /// Handgun.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handgun: Option<String>,
    /// Throwable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub throwable: Option<String>,
    /// Uniform.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uniform: Option<String>,
    /// Vest.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vest: Option<String>,
    /// Helmet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub helmet: Option<String>,
    /// Pants.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pants: Option<String>,
    /// Boots.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boots: Option<String>,
    /// Handwear.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handwear: Option<String>,
    /// Backpack.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backpack: Option<String>,
}

impl ModSlotGear {
    /// Is empty using the supplied domain data.
    pub(in crate::data::scenario) fn is_empty(&self) -> bool {
        self.primary.is_none()
            && self.optic.is_none()
            && self.magazine.is_none()
            && self.attachments.is_empty()
            && self.launcher.is_none()
            && self.handgun.is_none()
            && self.throwable.is_none()
            && self.uniform.is_none()
            && self.vest.is_none()
            && self.helmet.is_none()
            && self.pants.is_none()
            && self.boots.is_none()
            && self.handwear.is_none()
            && self.backpack.is_none()
    }
}

/// One container cargo row (`{container, item, qty}` — loadout-export v2), copied verbatim from the editor cargo.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModSlotCargo {
    /// Container.
    pub container: String,
    /// Item.
    pub item: String,
    /// Qty.
    pub qty: i64,
}

/// Domain representation of mod orbat role.
#[derive(Debug, Serialize)]
pub struct ModOrbatRole {
    /// Slot.
    pub slot: String,
    /// Kit.
    pub kit: String,
    /// Count.
    pub count: i64,
}

/// Domain representation of mod orbat group.
#[derive(Debug, Serialize)]
pub struct ModOrbatGroup {
    /// Callsign.
    pub callsign: String,
    /// Kind.
    #[serde(rename = "type")]
    pub kind: String,
    /// Roles.
    pub roles: Vec<ModOrbatRole>,

    /// Squad leader identity, authored once at `/editor/squads/*/leaderSlotId` and emitted on the group so per-seat copies cannot disagree.
    #[serde(rename = "leaderSlotId", skip_serializing_if = "Option::is_none")]
    pub leader_slot_id: Option<String>,
}

/// Domain representation of mod orbat faction.
#[derive(Debug, Serialize)]
pub struct ModOrbatFaction {
    /// Groups.
    pub groups: Vec<ModOrbatGroup>,
}

/// One `radioPlan.nets[]` entry (`mission.schema.json#/$defs/net`) — see [`derive_radio_plan`] for where the values come from and what they do not claim.
#[derive(Debug, Serialize)]
pub struct ModNet {
    /// `^net:[a-z0-9_]+$`. Unique within the document — the mod treats it as the stable channel key and the VOIP bridge keys voice channels on it (`packages/tbd-schema/bridge/bridge-contract.md` §radioPlan → voice net mapping).
    pub id: String,

    /// Display name. Capped at [`MOD_MAX_LABEL_CHARS`] here so the mod never has to.
    pub label: String,

    /// NOT `#[serde(rename_all = "camelCase")]`: serde would camel-case `freq_mhz` to `freqMhz`, and the mod binds this block by field NAME through `JsonLoadContext`, which ignores keys it does not recognise. The whole radio plan would arrive with every frequency at 0 — which `TBD_RadioPlan.Fault` then rejects as out-of-band, so the failure mode is a silently empty plan, not a parse error.
    #[serde(rename = "freqMHz")]
    pub freq_mhz: f64,

    /// Always set on the derived plan. Authored nets may omit it (unscoped / shared); empty skips the key so a document with no faction still validates.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub faction: String,

    /// `"long"` on command nets, ABSENT on squad nets. Never `"short"` — see [`derive_radio_plan`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<String>,
}
