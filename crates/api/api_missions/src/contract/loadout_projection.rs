//! The loadout-export document model. **Hand-maintained**, and deliberately absent from
//! `cargo xtask ci schema-codegen`: the generator's target list covers only the four schemas whose
//! typify output is faithful.
//!
//! Source of truth: `contracts/definitions/loadout-export.schema.json`.
//!
//! Generated output is provably lossy for this schema — it merges the versioned root `oneOf` into
//! a single struct and emits empty `Wear {}` / `Equipment {}` because `patternProperties` is
//! dropped. The faithful model below is guarded instead by value-level round-trip tests against
//! BOTH committed sample fixtures.

use api_identifiers::LoadoutModpackId;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A slot value: Enfusion ResourceName, or null when the slot is empty.
pub type SlotValue = Option<String>;

/// The loadout-export document — versioned root `oneOf`, tagged by `loadoutVersion`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "loadoutVersion")]
pub enum LoadoutExport {
    /// Envelope version 1 (`"loadoutVersion": "1"`): the flat four-slot gear block.
    #[serde(rename = "1")]
    V1(LoadoutV1),
    /// Envelope version 2 (`"loadoutVersion": "2"`): wear map, slot-indexed weapons, gear block.
    #[serde(rename = "2")]
    V2(LoadoutV2),
}

/// v1 — the flat four-slot gear block (the mod's Phase-1 reader).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LoadoutV1 {
    /// Modpack the loadout's ResourceNames resolve against (wire key `modpackId`).
    #[serde(rename = "modpackId")]
    pub modpack_id: LoadoutModpackId,
    /// The fixed v1 gear slots.
    pub gear: Gear,
}

/// v2 — Reforger-shaped wear map + slot-indexed weapons (+ skeleton equipment/cargo) plus the
/// derived legacy `gear` block v1 readers keep consuming.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LoadoutV2 {
    /// Modpack the loadout's ResourceNames resolve against (wire key `modpackId`).
    #[serde(rename = "modpackId")]
    pub modpack_id: LoadoutModpackId,
    /// Wear areas keyed by engine LoadoutSlotInfo name — pattern-open (mod-added areas legal).
    pub wear: BTreeMap<String, SlotValue>,
    /// Slot-indexed weapons (vanilla: 0-1 primary, 2 secondary, 3 grenade, 4 throwable).
    pub weapons: Vec<Weapon>,
    /// Equipment micro-slots (skeleton in v2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub equipment: Option<BTreeMap<String, SlotValue>>,
    /// Container cargo (volume/weight budget model; skeleton in v2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo: Option<Vec<CargoEntry>>,
    /// Derived legacy block (jacket→uniform, armoredVest||vest→vest, headCover→helmet,
    /// `weapons[0]`→primary/optic/magazine).
    pub gear: Gear,
}

/// The v1/legacy gear block. The four base keys are REQUIRED (nullable); optic/magazine are
/// optional-and-nullable (absent ≠ null — preserved via the double-Option idiom).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Gear {
    /// Primary weapon ResourceName, or null when empty.
    pub primary: SlotValue,
    /// Uniform (jacket) ResourceName, or null when empty.
    pub uniform: SlotValue,
    /// Vest ResourceName (armoured vest preferred), or null when empty.
    pub vest: SlotValue,
    /// Helmet (head cover) ResourceName, or null when empty.
    pub helmet: SlotValue,
    /// Primary weapon optic: `None` when the key is absent, `Some(None)` when explicitly null.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "double_option"
    )]
    pub optic: Option<SlotValue>,
    /// Primary weapon magazine: `None` when the key is absent, `Some(None)` when explicitly null.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "double_option"
    )]
    pub magazine: Option<SlotValue>,
}

/// One engine weapon slot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Weapon {
    /// Engine weapon slot index, zero-based (wire key `slotIndex`).
    #[serde(rename = "slotIndex")]
    pub slot_index: i64,
    /// Engine weapon slot type, e.g. `primary` or `grenade` (wire key `slotType`).
    #[serde(rename = "slotType")]
    pub slot_type: String,
    /// Weapon ResourceName equipped in the slot.
    pub weapon: String,
    /// Optic ResourceName: `None` when the key is absent, `Some(None)` when explicitly null.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "double_option"
    )]
    pub optic: Option<SlotValue>,
    /// Magazine ResourceName: `None` when the key is absent, `Some(None)` when explicitly null.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "double_option"
    )]
    pub magazine: Option<SlotValue>,
    /// Further attachment ResourceNames mounted on the weapon, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<String>>,
}

/// One cargo row (container/item/qty).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CargoEntry {
    /// Worn container the item goes into: `vest`, `pants`, `jacket` or `backpack`.
    pub container: String,
    /// ResourceName of the cargo item.
    pub item: String,
    /// Number of items, at least one.
    pub qty: i64,
}

/// Serde double-Option: outer None = key absent, Some(None) = explicit null.
mod double_option {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub(super) fn serialize<T, S>(v: &Option<Option<T>>, s: S) -> Result<S::Ok, S::Error>
    where
        T: Serialize,
        S: Serializer,
    {
        match v {
            Some(inner) => inner.serialize(s),
            None => s.serialize_none(),
        }
    }

    pub(super) fn deserialize<'de, T, D>(d: D) -> Result<Option<Option<T>>, D::Error>
    where
        T: Deserialize<'de>,
        D: Deserializer<'de>,
    {
        Ok(Some(Option::<T>::deserialize(d)?))
    }
}

#[cfg(test)]
#[path = "tests/loadout_projection.rs"]
mod tests;
