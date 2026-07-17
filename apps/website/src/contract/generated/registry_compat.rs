// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: packages/tbd-schema/schema/registry-compat.schema.json — regenerate with: make schema-codegen

/// Error types.
pub mod error {
    /// Error from a `TryFrom` or `FromStr` implementation.
    pub struct ConversionError(::std::borrow::Cow<'static, str>);
    impl ::std::error::Error for ConversionError {}
    impl ::std::fmt::Display for ConversionError {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
            ::std::fmt::Display::fmt(&self.0, f)
        }
    }
    impl ::std::fmt::Debug for ConversionError {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
            ::std::fmt::Debug::fmt(&self.0, f)
        }
    }
    impl From<&'static str> for ConversionError {
        fn from(value: &'static str) -> Self {
            Self(value.into())
        }
    }
    impl From<String> for ConversionError {
        fn from(value: String) -> Self {
            Self(value.into())
        }
    }
}
///`Addon`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "guid",
///    "name"
///  ],
///  "properties": {
///    "guid": {
///      "description": "Addon GUID from GameProject.GetLoadedAddons.",
///      "type": "string"
///    },
///    "name": {
///      "description": "Addon ID (GameProject.GetAddonID), e.g. ArmaReforger.",
///      "type": "string"
///    },
///    "title": {
///      "description": "Human title (GameProject.GetAddonTitle).",
///      "type": "string"
///    },
///    "vanilla": {
///      "description": "GameProject.IsVanillaAddon.",
///      "type": "boolean"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Addon {
    ///Addon GUID from GameProject.GetLoadedAddons.
    pub guid: ::std::string::String,
    ///Addon ID (GameProject.GetAddonID), e.g. ArmaReforger.
    pub name: ::std::string::String,
    ///Human title (GameProject.GetAddonTitle).
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub title: ::std::option::Option<::std::string::String>,
    ///GameProject.IsVanillaAddon.
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub vanilla: ::std::option::Option<bool>,
}
///Directed compatibility edge. from_node = the item that goes in/on (magazine, ammo, optic, attachment, gear); to_node = the host that accepts it (weapon, magazine, vehicle weapon, character). Per edge_type: mag_in_weapon mag->weapon; ammo_in_mag ammo->magazine; optic_on_weapon optic->weapon; attachment_on_weapon attachment->weapon; mag_in_vehicle_weapon mag->vehicle weapon prefab; ammo_in_vehicle_weapon ammo->vehicle weapon prefab; character_default_loadout gear item->character.
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "Directed compatibility edge. from_node = the item that goes in/on (magazine, ammo, optic, attachment, gear); to_node = the host that accepts it (weapon, magazine, vehicle weapon, character). Per edge_type: mag_in_weapon mag->weapon; ammo_in_mag ammo->magazine; optic_on_weapon optic->weapon; attachment_on_weapon attachment->weapon; mag_in_vehicle_weapon mag->vehicle weapon prefab; ammo_in_vehicle_weapon ammo->vehicle weapon prefab; character_default_loadout gear item->character.",
///  "type": "object",
///  "required": [
///    "edge_type",
///    "from_node",
///    "to_node"
///  ],
///  "properties": {
///    "edge_type": {
///      "type": "string",
///      "enum": [
///        "mag_in_weapon",
///        "ammo_in_mag",
///        "optic_on_weapon",
///        "attachment_on_weapon",
///        "mag_in_vehicle_weapon",
///        "ammo_in_vehicle_weapon",
///        "character_default_loadout",
///        "character_default_weapon"
///      ]
///    },
///    "evidence": {
///      "description": "Engine class or container var that proved the edge, e.g. MagazineWellStanag556 (well-class match), AttachmentOpticsRIS1913 (slot type match), MagazineTemplate (direct prefab ref), LoadoutSlotInfo (character slot).",
///      "type": "string"
///    },
///    "from_node": {
///      "$ref": "#/$defs/resourceName"
///    },
///    "to_node": {
///      "$ref": "#/$defs/resourceName"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
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
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "string",
///  "enum": [
///    "mag_in_weapon",
///    "ammo_in_mag",
///    "optic_on_weapon",
///    "attachment_on_weapon",
///    "mag_in_vehicle_weapon",
///    "ammo_in_vehicle_weapon",
///    "character_default_loadout",
///    "character_default_weapon"
///  ]
///}
/// ```
/// </details>
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
        }
    }
}
impl ::std::str::FromStr for EdgeEdgeType {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "mag_in_weapon" => Ok(Self::MagInWeapon),
            "ammo_in_mag" => Ok(Self::AmmoInMag),
            "optic_on_weapon" => Ok(Self::OpticOnWeapon),
            "attachment_on_weapon" => Ok(Self::AttachmentOnWeapon),
            "mag_in_vehicle_weapon" => Ok(Self::MagInVehicleWeapon),
            "ammo_in_vehicle_weapon" => Ok(Self::AmmoInVehicleWeapon),
            "character_default_loadout" => Ok(Self::CharacterDefaultLoadout),
            "character_default_weapon" => Ok(Self::CharacterDefaultWeapon),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for EdgeEdgeType {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for EdgeEdgeType {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for EdgeEdgeType {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
///Enfusion ResourceName ({GUID}Prefabs/.../File.et).
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "Enfusion ResourceName ({GUID}Prefabs/.../File.et).",
///  "type": "string",
///  "pattern": "^\\{[0-9A-F]{16}\\}[A-Za-z0-9/_.\\- ()']+$"
///}
/// ```
/// </details>
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct ResourceName(::std::string::String);
impl ::std::ops::Deref for ResourceName {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<ResourceName> for ::std::string::String {
    fn from(value: ResourceName) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for ResourceName {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
            ::std::sync::LazyLock::new(|| {
                ::regress::Regex::new("^\\{[0-9A-F]{16}\\}[A-Za-z0-9/_.\\- ()']+$").unwrap()
            });
        if PATTERN.find(value).is_none() {
            return Err(
                "doesn't match pattern \"^\\{[0-9A-F]{16}\\}[A-Za-z0-9/_.\\- ()']+$\"".into(),
            );
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for ResourceName {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ResourceName {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ResourceName {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for ResourceName {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///Engine-derived compatibility edge graph between registry items (T-150). Nodes are full Enfusion ResourceNames and must exist in the paired registry-items envelope; edges are read from prefab container data (magazine wells, attachment slot types, vehicle weapon slots, character loadout slots) — never hand-authored. Drives canEquip/canAttach (T-068.9 ingest, T-068.10 smart Forge).
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "$id": "https://schema.tbdevent.eu/registry-compat/v1.json",
///  "title": "TBD Registry Compat",
///  "description": "Engine-derived compatibility edge graph between registry items (T-150). Nodes are full Enfusion ResourceNames and must exist in the paired registry-items envelope; edges are read from prefab container data (magazine wells, attachment slot types, vehicle weapon slots, character loadout slots) — never hand-authored. Drives canEquip/canAttach (T-068.9 ingest, T-068.10 smart Forge).",
///  "type": "object",
///  "required": [
///    "edges",
///    "modpackId",
///    "registryCompatVersion"
///  ],
///  "properties": {
///    "addons": {
///      "description": "Workbench addons loaded during the export (the scan set).",
///      "type": "array",
///      "items": {
///        "$ref": "#/$defs/addon"
///      }
///    },
///    "edges": {
///      "type": "array",
///      "items": {
///        "$ref": "#/$defs/edge"
///      },
///      "minItems": 1
///    },
///    "generatedAt": {
///      "type": "string",
///      "format": "date-time"
///    },
///    "modpackId": {
///      "type": "string"
///    },
///    "registryCompatVersion": {
///      "type": "string"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct TbdRegistryCompat {
    ///Workbench addons loaded during the export (the scan set).
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub addons: ::std::vec::Vec<Addon>,
    pub edges: ::std::vec::Vec<Edge>,
    #[serde(
        rename = "generatedAt",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub generated_at: ::std::option::Option<::chrono::DateTime<::chrono::offset::Utc>>,
    #[serde(rename = "modpackId")]
    pub modpack_id: ::std::string::String,
    #[serde(rename = "registryCompatVersion")]
    pub registry_compat_version: ::std::string::String,
}
