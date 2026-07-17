// HAND-MAINTAINED since T-165.3 (do edit — but keep the round-trip tests green).
// Source of truth: packages/tbd-schema/schema/loadout-export.schema.json.
// History: this file was quicktype-generated until T-165.3; that output was provably lossy —
// it merged the versioned root `oneOf` into one struct and emitted empty `Wear {}` /
// `Equipment {}` (patternProperties dropped). The faithful model below is guarded by
// value-level round-trip tests against BOTH committed sample fixtures.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A slot value: Enfusion ResourceName, or null when the slot is empty.
pub type SlotValue = Option<String>;

/// The loadout-export document — versioned root `oneOf`, tagged by `loadoutVersion`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "loadoutVersion")]
pub enum LoadoutExport {
    #[serde(rename = "1")]
    V1(LoadoutV1),
    #[serde(rename = "2")]
    V2(LoadoutV2),
}

/// v1 — the flat four-slot gear block (the mod's Phase-1 reader).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LoadoutV1 {
    #[serde(rename = "modpackId")]
    pub modpack_id: String,
    pub gear: Gear,
}

/// v2 — Reforger-shaped wear map + slot-indexed weapons (+ skeleton equipment/cargo) plus the
/// derived legacy `gear` block v1 readers keep consuming.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LoadoutV2 {
    #[serde(rename = "modpackId")]
    pub modpack_id: String,
    /// Wear areas keyed by engine LoadoutSlotInfo name — pattern-open (mod-added areas legal).
    pub wear: BTreeMap<String, SlotValue>,
    pub weapons: Vec<Weapon>,
    /// Equipment micro-slots (skeleton in v2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub equipment: Option<BTreeMap<String, SlotValue>>,
    /// Container cargo (volume/weight budget model; skeleton in v2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo: Option<Vec<CargoEntry>>,
    /// Derived legacy block (jacket→uniform, armoredVest||vest→vest, headCover→helmet,
    /// weapons[0]→primary/optic/magazine).
    pub gear: Gear,
}

/// The v1/legacy gear block. The four base keys are REQUIRED (nullable); optic/magazine are
/// optional-and-nullable (absent ≠ null — preserved via the double-Option idiom).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Gear {
    pub primary: SlotValue,
    pub uniform: SlotValue,
    pub vest: SlotValue,
    pub helmet: SlotValue,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "double_option"
    )]
    pub optic: Option<SlotValue>,
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
    #[serde(rename = "slotIndex")]
    pub slot_index: i64,
    #[serde(rename = "slotType")]
    pub slot_type: String,
    pub weapon: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "double_option"
    )]
    pub optic: Option<SlotValue>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "double_option"
    )]
    pub magazine: Option<SlotValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<String>>,
}

/// One cargo row (container/item/qty).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CargoEntry {
    pub container: String,
    pub item: String,
    pub qty: i64,
}

/// Serde double-Option: outer None = key absent, Some(None) = explicit null.
mod double_option {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<T, S>(v: &Option<Option<T>>, s: S) -> Result<S::Ok, S::Error>
    where
        T: Serialize,
        S: Serializer,
    {
        match v {
            Some(inner) => inner.serialize(s),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, T, D>(d: D) -> Result<Option<Option<T>>, D::Error>
    where
        T: Deserialize<'de>,
        D: Deserializer<'de>,
    {
        Ok(Some(Option::<T>::deserialize(d)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const V1: &str =
        include_str!("../../../../../packages/tbd-schema/registry/loadout-export.sample.json");
    const V2: &str =
        include_str!("../../../../../packages/tbd-schema/registry/loadout-export.v2.sample.json");

    /// Value-level round-trip: parse → serialize → parse; the two JSON values must be EQUAL
    /// (key order irrelevant; null-vs-absent must be preserved — the double-Option contract).
    fn round_trips(fixture: &str) {
        let parsed: LoadoutExport = serde_json::from_str(fixture).expect("deserialize");
        let re = serde_json::to_string(&parsed).expect("serialize");
        let a: serde_json::Value = serde_json::from_str(fixture).unwrap();
        let b: serde_json::Value = serde_json::from_str(&re).unwrap();
        assert_eq!(a, b, "value round-trip drift");
    }

    #[test]
    fn v1_sample_round_trips() {
        round_trips(V1);
        let LoadoutExport::V1(doc) = serde_json::from_str(V1).unwrap() else {
            panic!("v1 fixture parsed as wrong version");
        };
        assert!(doc.gear.primary.is_some() && doc.gear.helmet.is_none());
    }

    #[test]
    fn v2_sample_round_trips() {
        round_trips(V2);
        let LoadoutExport::V2(doc) = serde_json::from_str(V2).unwrap() else {
            panic!("v2 fixture parsed as wrong version");
        };
        assert_eq!(doc.wear.len(), 8);
        assert!(doc.weapons.iter().any(|w| w.slot_index == 0));
        // The fixture's second weapon omits optic entirely — absent, not null.
        let grenade = doc.weapons.iter().find(|w| w.slot_index == 3).unwrap();
        assert!(grenade.optic.is_none() && grenade.attachments.is_none());
    }
}
