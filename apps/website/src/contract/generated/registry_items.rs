// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: packages/tbd-schema/schema/registry-items.schema.json — regenerate with: make schema-codegen

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
///`Item`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "category",
///    "display_name",
///    "kind",
///    "resource_name"
///  ],
///  "properties": {
///    "abstract": {
///      "description": "True for non-placeable template prefabs (filename *_base.et / display '* Base'). Kept in the catalog (bases carry classification signals for descendants) but hidden from loadout pickers.",
///      "type": "boolean"
///    },
///    "addon": {
///      "description": "Per-item mod provenance: the addon ID this prefab was scanned from. Must match an addons[].name entry (strict check in validate.mjs); vanilla-ness derives from addons[].vanilla — no separate flag to drift.",
///      "type": "string"
///    },
///    "arsenal_type": {
///      "description": "SCR_EArsenalItemType flag name (e.g. RIFLE, NON_LETHAL_THROWABLE) when the item appears in a faction EntityCatalog SCR_ArsenalItem entry (Tier-B classification metadata). Absent when no catalog entry exists.",
///      "type": "string"
///    },
///    "category": {
///      "description": "Slash-delimited browse path, e.g. NATO/Rifleman.",
///      "type": "string",
///      "minLength": 1
///    },
///    "display_name": {
///      "type": "string",
///      "minLength": 1
///    },
///    "icon_url": {
///      "type": "string"
///    },
///    "kind": {
///      "description": "v3 (T-068.10.2) classification. Phase 1 kinds remain valid; gear_uniform is retired (0 rows — split into gear_jacket/gear_pants/gear_boots) but still accepted; 'other' is the escape hatch and its count must be reported in export verify logs. Taxonomy: .ai/artifacts/ace_arsenal_taxonomy_map.md.",
///      "type": "string",
///      "enum": [
///        "character",
///        "gear_primary",
///        "gear_handgun",
///        "gear_launcher",
///        "gear_throwable",
///        "gear_explosive",
///        "gear_uniform",
///        "gear_jacket",
///        "gear_pants",
///        "gear_boots",
///        "gear_vest",
///        "gear_armored_vest",
///        "gear_helmet",
///        "gear_backpack",
///        "gear_glasses",
///        "gear_gloves",
///        "gear_binoculars",
///        "gear_item",
///        "magazine",
///        "ammo",
///        "optic",
///        "attachment",
///        "vehicle",
///        "vehicle_weapon",
///        "crate",
///        "other"
///      ]
///    },
///    "max_volume_cm3": {
///      "description": "Container volume capacity (storage component MaxCumulativeVolume, cm³) for items that ARE containers. Absent when the prefab relies on the engine class default — never guessed. Feeds the later cargo-budget slice.",
///      "type": "number",
///      "minimum": 0.0
///    },
///    "max_weight_kg": {
///      "description": "Container carry capacity (storage component m_fMaxWeight, kg) for items that ARE containers (vests/backpacks/jackets). Absent when the prefab relies on the engine class default — never guessed. Feeds the later cargo-budget slice.",
///      "type": "number",
///      "minimum": 0.0
///    },
///    "resource_name": {
///      "description": "Enfusion ResourceName ({GUID}Prefabs/.../File.et) used by Resource.Load.",
///      "type": "string",
///      "pattern": "^\\{[0-9A-F]{16}\\}[A-Za-z0-9/_.\\- ()']+$"
///    },
///    "variant_of": {
///      "description": "T-068.10.5: set on factory attachment/camo CONFIGURATIONS of a base weapon (same family prefix, magwell, attachment-slot-type set and mesh — only pre-mounted attachments/materials differ, e.g. 'Rifle AK74N 1P29' → 'Rifle AK74N'). Points at the immediate parent item (must exist in the envelope — strict check in validate.mjs). Pickers hide variant rows like abstracts; the census artifact t068_10_5_weapon_families.md carries the per-weapon evidence.",
///      "type": "string",
///      "pattern": "^\\{[0-9A-F]{16}\\}[A-Za-z0-9/_.\\- ()']+$"
///    },
///    "volume_cm3": {
///      "description": "ItemPhysicalAttributes.ItemVolume in cubic centimetres (API-documented unit), read from the prefab ancestry chain. Absent when the value is an engine class default not serialized in the prefab — never guessed.",
///      "type": "number",
///      "minimum": 0.0
///    },
///    "weight_kg": {
///      "description": "ItemPhysicalAttributes.Weight in kilograms (API-documented unit), read from the prefab ancestry chain. Absent when the value is an engine class default not serialized in the prefab — never guessed.",
///      "type": "number",
///      "minimum": 0.0
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Item {
    ///True for non-placeable template prefabs (filename *_base.et / display '* Base'). Kept in the catalog (bases carry classification signals for descendants) but hidden from loadout pickers.
    #[serde(
        rename = "abstract",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub abstract_: ::std::option::Option<bool>,
    ///Per-item mod provenance: the addon ID this prefab was scanned from. Must match an addons[].name entry (strict check in validate.mjs); vanilla-ness derives from addons[].vanilla — no separate flag to drift.
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub addon: ::std::option::Option<::std::string::String>,
    ///SCR_EArsenalItemType flag name (e.g. RIFLE, NON_LETHAL_THROWABLE) when the item appears in a faction EntityCatalog SCR_ArsenalItem entry (Tier-B classification metadata). Absent when no catalog entry exists.
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub arsenal_type: ::std::option::Option<::std::string::String>,
    ///Slash-delimited browse path, e.g. NATO/Rifleman.
    pub category: ItemCategory,
    pub display_name: ItemDisplayName,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub icon_url: ::std::option::Option<::std::string::String>,
    ///v3 (T-068.10.2) classification. Phase 1 kinds remain valid; gear_uniform is retired (0 rows — split into gear_jacket/gear_pants/gear_boots) but still accepted; 'other' is the escape hatch and its count must be reported in export verify logs. Taxonomy: .ai/artifacts/ace_arsenal_taxonomy_map.md.
    pub kind: ItemKind,
    ///Container volume capacity (storage component MaxCumulativeVolume, cm³) for items that ARE containers. Absent when the prefab relies on the engine class default — never guessed. Feeds the later cargo-budget slice.
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub max_volume_cm3: ::std::option::Option<f64>,
    ///Container carry capacity (storage component m_fMaxWeight, kg) for items that ARE containers (vests/backpacks/jackets). Absent when the prefab relies on the engine class default — never guessed. Feeds the later cargo-budget slice.
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub max_weight_kg: ::std::option::Option<f64>,
    ///Enfusion ResourceName ({GUID}Prefabs/.../File.et) used by Resource.Load.
    pub resource_name: ItemResourceName,
    ///T-068.10.5: set on factory attachment/camo CONFIGURATIONS of a base weapon (same family prefix, magwell, attachment-slot-type set and mesh — only pre-mounted attachments/materials differ, e.g. 'Rifle AK74N 1P29' → 'Rifle AK74N'). Points at the immediate parent item (must exist in the envelope — strict check in validate.mjs). Pickers hide variant rows like abstracts; the census artifact t068_10_5_weapon_families.md carries the per-weapon evidence.
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub variant_of: ::std::option::Option<ItemVariantOf>,
    ///ItemPhysicalAttributes.ItemVolume in cubic centimetres (API-documented unit), read from the prefab ancestry chain. Absent when the value is an engine class default not serialized in the prefab — never guessed.
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub volume_cm3: ::std::option::Option<f64>,
    ///ItemPhysicalAttributes.Weight in kilograms (API-documented unit), read from the prefab ancestry chain. Absent when the value is an engine class default not serialized in the prefab — never guessed.
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub weight_kg: ::std::option::Option<f64>,
}
///Slash-delimited browse path, e.g. NATO/Rifleman.
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "Slash-delimited browse path, e.g. NATO/Rifleman.",
///  "type": "string",
///  "minLength": 1
///}
/// ```
/// </details>
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct ItemCategory(::std::string::String);
impl ::std::ops::Deref for ItemCategory {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<ItemCategory> for ::std::string::String {
    fn from(value: ItemCategory) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for ItemCategory {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for ItemCategory {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ItemCategory {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ItemCategory {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for ItemCategory {
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
///`ItemDisplayName`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "string",
///  "minLength": 1
///}
/// ```
/// </details>
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct ItemDisplayName(::std::string::String);
impl ::std::ops::Deref for ItemDisplayName {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<ItemDisplayName> for ::std::string::String {
    fn from(value: ItemDisplayName) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for ItemDisplayName {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for ItemDisplayName {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ItemDisplayName {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ItemDisplayName {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for ItemDisplayName {
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
///v3 (T-068.10.2) classification. Phase 1 kinds remain valid; gear_uniform is retired (0 rows — split into gear_jacket/gear_pants/gear_boots) but still accepted; 'other' is the escape hatch and its count must be reported in export verify logs. Taxonomy: .ai/artifacts/ace_arsenal_taxonomy_map.md.
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "v3 (T-068.10.2) classification. Phase 1 kinds remain valid; gear_uniform is retired (0 rows — split into gear_jacket/gear_pants/gear_boots) but still accepted; 'other' is the escape hatch and its count must be reported in export verify logs. Taxonomy: .ai/artifacts/ace_arsenal_taxonomy_map.md.",
///  "type": "string",
///  "enum": [
///    "character",
///    "gear_primary",
///    "gear_handgun",
///    "gear_launcher",
///    "gear_throwable",
///    "gear_explosive",
///    "gear_uniform",
///    "gear_jacket",
///    "gear_pants",
///    "gear_boots",
///    "gear_vest",
///    "gear_armored_vest",
///    "gear_helmet",
///    "gear_backpack",
///    "gear_glasses",
///    "gear_gloves",
///    "gear_binoculars",
///    "gear_item",
///    "magazine",
///    "ammo",
///    "optic",
///    "attachment",
///    "vehicle",
///    "vehicle_weapon",
///    "crate",
///    "other"
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
pub enum ItemKind {
    #[serde(rename = "character")]
    Character,
    #[serde(rename = "gear_primary")]
    GearPrimary,
    #[serde(rename = "gear_handgun")]
    GearHandgun,
    #[serde(rename = "gear_launcher")]
    GearLauncher,
    #[serde(rename = "gear_throwable")]
    GearThrowable,
    #[serde(rename = "gear_explosive")]
    GearExplosive,
    #[serde(rename = "gear_uniform")]
    GearUniform,
    #[serde(rename = "gear_jacket")]
    GearJacket,
    #[serde(rename = "gear_pants")]
    GearPants,
    #[serde(rename = "gear_boots")]
    GearBoots,
    #[serde(rename = "gear_vest")]
    GearVest,
    #[serde(rename = "gear_armored_vest")]
    GearArmoredVest,
    #[serde(rename = "gear_helmet")]
    GearHelmet,
    #[serde(rename = "gear_backpack")]
    GearBackpack,
    #[serde(rename = "gear_glasses")]
    GearGlasses,
    #[serde(rename = "gear_gloves")]
    GearGloves,
    #[serde(rename = "gear_binoculars")]
    GearBinoculars,
    #[serde(rename = "gear_item")]
    GearItem,
    #[serde(rename = "magazine")]
    Magazine,
    #[serde(rename = "ammo")]
    Ammo,
    #[serde(rename = "optic")]
    Optic,
    #[serde(rename = "attachment")]
    Attachment,
    #[serde(rename = "vehicle")]
    Vehicle,
    #[serde(rename = "vehicle_weapon")]
    VehicleWeapon,
    #[serde(rename = "crate")]
    Crate,
    #[serde(rename = "other")]
    Other,
}
impl ::std::fmt::Display for ItemKind {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Character => f.write_str("character"),
            Self::GearPrimary => f.write_str("gear_primary"),
            Self::GearHandgun => f.write_str("gear_handgun"),
            Self::GearLauncher => f.write_str("gear_launcher"),
            Self::GearThrowable => f.write_str("gear_throwable"),
            Self::GearExplosive => f.write_str("gear_explosive"),
            Self::GearUniform => f.write_str("gear_uniform"),
            Self::GearJacket => f.write_str("gear_jacket"),
            Self::GearPants => f.write_str("gear_pants"),
            Self::GearBoots => f.write_str("gear_boots"),
            Self::GearVest => f.write_str("gear_vest"),
            Self::GearArmoredVest => f.write_str("gear_armored_vest"),
            Self::GearHelmet => f.write_str("gear_helmet"),
            Self::GearBackpack => f.write_str("gear_backpack"),
            Self::GearGlasses => f.write_str("gear_glasses"),
            Self::GearGloves => f.write_str("gear_gloves"),
            Self::GearBinoculars => f.write_str("gear_binoculars"),
            Self::GearItem => f.write_str("gear_item"),
            Self::Magazine => f.write_str("magazine"),
            Self::Ammo => f.write_str("ammo"),
            Self::Optic => f.write_str("optic"),
            Self::Attachment => f.write_str("attachment"),
            Self::Vehicle => f.write_str("vehicle"),
            Self::VehicleWeapon => f.write_str("vehicle_weapon"),
            Self::Crate => f.write_str("crate"),
            Self::Other => f.write_str("other"),
        }
    }
}
impl ::std::str::FromStr for ItemKind {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "character" => Ok(Self::Character),
            "gear_primary" => Ok(Self::GearPrimary),
            "gear_handgun" => Ok(Self::GearHandgun),
            "gear_launcher" => Ok(Self::GearLauncher),
            "gear_throwable" => Ok(Self::GearThrowable),
            "gear_explosive" => Ok(Self::GearExplosive),
            "gear_uniform" => Ok(Self::GearUniform),
            "gear_jacket" => Ok(Self::GearJacket),
            "gear_pants" => Ok(Self::GearPants),
            "gear_boots" => Ok(Self::GearBoots),
            "gear_vest" => Ok(Self::GearVest),
            "gear_armored_vest" => Ok(Self::GearArmoredVest),
            "gear_helmet" => Ok(Self::GearHelmet),
            "gear_backpack" => Ok(Self::GearBackpack),
            "gear_glasses" => Ok(Self::GearGlasses),
            "gear_gloves" => Ok(Self::GearGloves),
            "gear_binoculars" => Ok(Self::GearBinoculars),
            "gear_item" => Ok(Self::GearItem),
            "magazine" => Ok(Self::Magazine),
            "ammo" => Ok(Self::Ammo),
            "optic" => Ok(Self::Optic),
            "attachment" => Ok(Self::Attachment),
            "vehicle" => Ok(Self::Vehicle),
            "vehicle_weapon" => Ok(Self::VehicleWeapon),
            "crate" => Ok(Self::Crate),
            "other" => Ok(Self::Other),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for ItemKind {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ItemKind {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ItemKind {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
///Enfusion ResourceName ({GUID}Prefabs/.../File.et) used by Resource.Load.
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "Enfusion ResourceName ({GUID}Prefabs/.../File.et) used by Resource.Load.",
///  "type": "string",
///  "pattern": "^\\{[0-9A-F]{16}\\}[A-Za-z0-9/_.\\- ()']+$"
///}
/// ```
/// </details>
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct ItemResourceName(::std::string::String);
impl ::std::ops::Deref for ItemResourceName {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<ItemResourceName> for ::std::string::String {
    fn from(value: ItemResourceName) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for ItemResourceName {
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
impl ::std::convert::TryFrom<&str> for ItemResourceName {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ItemResourceName {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ItemResourceName {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for ItemResourceName {
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
///T-068.10.5: set on factory attachment/camo CONFIGURATIONS of a base weapon (same family prefix, magwell, attachment-slot-type set and mesh — only pre-mounted attachments/materials differ, e.g. 'Rifle AK74N 1P29' → 'Rifle AK74N'). Points at the immediate parent item (must exist in the envelope — strict check in validate.mjs). Pickers hide variant rows like abstracts; the census artifact t068_10_5_weapon_families.md carries the per-weapon evidence.
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "T-068.10.5: set on factory attachment/camo CONFIGURATIONS of a base weapon (same family prefix, magwell, attachment-slot-type set and mesh — only pre-mounted attachments/materials differ, e.g. 'Rifle AK74N 1P29' → 'Rifle AK74N'). Points at the immediate parent item (must exist in the envelope — strict check in validate.mjs). Pickers hide variant rows like abstracts; the census artifact t068_10_5_weapon_families.md carries the per-weapon evidence.",
///  "type": "string",
///  "pattern": "^\\{[0-9A-F]{16}\\}[A-Za-z0-9/_.\\- ()']+$"
///}
/// ```
/// </details>
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct ItemVariantOf(::std::string::String);
impl ::std::ops::Deref for ItemVariantOf {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<ItemVariantOf> for ::std::string::String {
    fn from(value: ItemVariantOf) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for ItemVariantOf {
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
impl ::std::convert::TryFrom<&str> for ItemVariantOf {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ItemVariantOf {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ItemVariantOf {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for ItemVariantOf {
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
///Flat catalog of placeable/equipable engine items exported from the TBD-Content Workbench. Items are identified by their full Enfusion ResourceName (resource_name). This is a separate layer from the alias spawn registry (registry.schema.json): the alias registry maps mission aliases to GUIDs for spawn, this catalog drives the web Virtual Arsenal (browse, seed/import, loadout build). v2 (T-150): kind vocabulary expanded for the universal mod-agnostic scanner; optional addons[] records the Workbench scan set.
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "$id": "https://schema.tbdevent.eu/registry-items/v1.json",
///  "title": "TBD Registry Items",
///  "description": "Flat catalog of placeable/equipable engine items exported from the TBD-Content Workbench. Items are identified by their full Enfusion ResourceName (resource_name). This is a separate layer from the alias spawn registry (registry.schema.json): the alias registry maps mission aliases to GUIDs for spawn, this catalog drives the web Virtual Arsenal (browse, seed/import, loadout build). v2 (T-150): kind vocabulary expanded for the universal mod-agnostic scanner; optional addons[] records the Workbench scan set.",
///  "type": "object",
///  "required": [
///    "items",
///    "modpackId",
///    "registryItemsVersion"
///  ],
///  "properties": {
///    "addons": {
///      "description": "Workbench addons loaded during the export (the scan set). Optional for v1 envelopes; the universal exporter (T-150) always writes it.",
///      "type": "array",
///      "items": {
///        "$ref": "#/$defs/addon"
///      }
///    },
///    "generatedAt": {
///      "type": "string",
///      "format": "date-time"
///    },
///    "items": {
///      "type": "array",
///      "items": {
///        "$ref": "#/$defs/item"
///      },
///      "minItems": 1
///    },
///    "modpackId": {
///      "type": "string"
///    },
///    "registryItemsVersion": {
///      "type": "string"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct TbdRegistryItems {
    ///Workbench addons loaded during the export (the scan set). Optional for v1 envelopes; the universal exporter (T-150) always writes it.
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub addons: ::std::vec::Vec<Addon>,
    #[serde(
        rename = "generatedAt",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub generated_at: ::std::option::Option<::chrono::DateTime<::chrono::offset::Utc>>,
    pub items: ::std::vec::Vec<Item>,
    #[serde(rename = "modpackId")]
    pub modpack_id: ::std::string::String,
    #[serde(rename = "registryItemsVersion")]
    pub registry_items_version: ::std::string::String,
}
