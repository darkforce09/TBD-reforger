// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/registry-compat.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::ResourceName;

///Directed compatibility edge. from_node = the item that goes in/on (magazine, ammo, optic, attachment, gear, default-cargo item); to_node = the host that accepts it (weapon, magazine, vehicle weapon, character). Per edge_type: mag_in_weapon mag->weapon; ammo_in_mag ammo->magazine; optic_on_weapon optic->weapon; attachment_on_weapon attachment->weapon; mag_in_vehicle_weapon mag->vehicle weapon prefab; ammo_in_vehicle_weapon ammo->vehicle weapon prefab; character_default_loadout gear item->character; character_default_cargo item->character (evidence TargetStorage=<path> from SCR_InventoryStorageManagerComponent InitialInventoryItems).
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    pub edge_type: EdgeEdgeType,
    ///Engine class or container var that proved the edge, e.g. MagazineWellStanag556 (well-class match), AttachmentOpticsRIS1913 (slot type match), MagazineTemplate (direct prefab ref), LoadoutSlotInfo (character slot).
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub evidence: ::std::option::Option<::std::string::String>,
    pub from_node: ResourceName,
    pub to_node: ResourceName,
}
///`EdgeEdgeType`
#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub enum EdgeEdgeType {
    #[serde(rename = "mag_in_weapon")]
    MagInWeapon,
    #[serde(rename = "ammo_in_mag")]
    AmmoInMag,
    #[serde(rename = "optic_on_weapon")]
    OpticOnWeapon,
    #[serde(rename = "attachment_on_weapon")]
    AttachmentOnWeapon,
    #[serde(rename = "mag_in_vehicle_weapon")]
    MagInVehicleWeapon,
    #[serde(rename = "ammo_in_vehicle_weapon")]
    AmmoInVehicleWeapon,
    #[serde(rename = "character_default_loadout")]
    CharacterDefaultLoadout,
    #[serde(rename = "character_default_weapon")]
    CharacterDefaultWeapon,
    #[serde(rename = "character_default_cargo")]
    CharacterDefaultCargo,
}
impl ::std::fmt::Display for EdgeEdgeType {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::MagInWeapon => f.write_str("mag_in_weapon"),
            Self::AmmoInMag => f.write_str("ammo_in_mag"),
            Self::OpticOnWeapon => f.write_str("optic_on_weapon"),
            Self::AttachmentOnWeapon => f.write_str("attachment_on_weapon"),
            Self::MagInVehicleWeapon => f.write_str("mag_in_vehicle_weapon"),
            Self::AmmoInVehicleWeapon => f.write_str("ammo_in_vehicle_weapon"),
            Self::CharacterDefaultLoadout => f.write_str("character_default_loadout"),
            Self::CharacterDefaultWeapon => f.write_str("character_default_weapon"),
            Self::CharacterDefaultCargo => f.write_str("character_default_cargo"),
        }
    }
}
impl ::std::str::FromStr for EdgeEdgeType {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "mag_in_weapon" => Ok(Self::MagInWeapon),
            "ammo_in_mag" => Ok(Self::AmmoInMag),
            "optic_on_weapon" => Ok(Self::OpticOnWeapon),
            "attachment_on_weapon" => Ok(Self::AttachmentOnWeapon),
            "mag_in_vehicle_weapon" => Ok(Self::MagInVehicleWeapon),
            "ammo_in_vehicle_weapon" => Ok(Self::AmmoInVehicleWeapon),
            "character_default_loadout" => Ok(Self::CharacterDefaultLoadout),
            "character_default_weapon" => Ok(Self::CharacterDefaultWeapon),
            "character_default_cargo" => Ok(Self::CharacterDefaultCargo),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for EdgeEdgeType {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for EdgeEdgeType {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for EdgeEdgeType {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
