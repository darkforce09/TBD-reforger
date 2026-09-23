// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/registry-items.schema.json — regenerate with: cargo xtask ci schema-codegen

///`Item`
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
    ///Per-item mod provenance: the addon ID this prefab comes from. Must match an `addons[].name` entry; vanilla-ness derives from `addons[].vanilla`, so there is no separate flag to drift.
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub addon: ::std::option::Option<::std::string::String>,
    ///SCR_EArsenalItemType flag name (e.g. RIFLE, NON_LETHAL_THROWABLE) when the item appears in a faction EntityCatalog SCR_ArsenalItem entry (Tier-B classification metadata). Absent when no catalog entry exists.
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub arsenal_type: ::std::option::Option<::std::string::String>,
    ///Inventory UI grid height in cells for container items. Derived from MaxCumulativeVolume with VOLUME_PER_CELL_CM3=50 and fixed width 4: cargo_grid_w=4; cells=ceil(max_volume_cm3/50); cargo_grid_h=max(3, ceil(cells/4)). Matches SCR_InventoryStorageBaseUI garment panel layout.
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub cargo_grid_h: ::std::option::Option<::std::num::NonZeroU64>,
    ///Inventory UI grid width in cells for container items. Derived from MaxCumulativeVolume with VOLUME_PER_CELL_CM3=50 and fixed width 4: cargo_grid_w=4; cells=ceil(max_volume_cm3/50); cargo_grid_h=max(3, ceil(cells/4)). Matches SCR_InventoryStorageBaseUI garment panel layout.
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub cargo_grid_w: ::std::option::Option<::std::num::NonZeroU64>,
    ///Slash-delimited browse path, e.g. NATO/Rifleman.
    pub category: ItemCategory,
    pub display_name: ItemDisplayName,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub icon_url: ::std::option::Option<::std::string::String>,
    ///v3 item classification. The phase 1 kinds are all still valid. `gear_uniform` carries no rows — it is split into `gear_jacket`, `gear_pants` and `gear_boots` — but is still accepted. `other` is the escape hatch, and its count is reported in the export verify logs. The taxonomy mapping lives at `.ai/artifacts/ace_arsenal_taxonomy_map.md`.
    pub kind: ItemKind,
    ///Container volume capacity (storage component MaxCumulativeVolume, cm³) for items that ARE containers. Absent when the prefab relies on the engine class default — never guessed. Feeds the later cargo-budget slice.
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub max_volume_cm3: ::std::option::Option<f64>,
    ///Container carry capacity (storage component m_fMaxWeight, kg) for items that ARE containers (vests/backpacks/jackets). Absent when the prefab relies on the engine class default — never guessed. Feeds the later cargo-budget slice.
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub max_weight_kg: ::std::option::Option<f64>,
    ///Enfusion ResourceName ({GUID}Prefabs/.../File.et) used by Resource.Load.
    pub resource_name: ItemResourceName,
    ///Set on a factory attachment or camo configuration of a base weapon — same family prefix, magwell, attachment-slot-type set and mesh, differing only in pre-mounted attachments or materials (for example 'Rifle AK74N 1P29' against 'Rifle AK74N'). Points at the immediate parent item, which must exist in the same envelope. Pickers hide variant rows the way they hide abstracts.
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
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for ItemCategory {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ItemCategory {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ItemCategory {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
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
            .map_err(|e: super::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///`ItemDisplayName`
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
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for ItemDisplayName {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ItemDisplayName {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ItemDisplayName {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
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
            .map_err(|e: super::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///v3 item classification. The phase 1 kinds are all still valid. `gear_uniform` carries no rows — it is split into `gear_jacket`, `gear_pants` and `gear_boots` — but is still accepted. `other` is the escape hatch, and its count is reported in the export verify logs. The taxonomy mapping lives at `.ai/artifacts/ace_arsenal_taxonomy_map.md`.
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
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
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
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ItemKind {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ItemKind {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
///Enfusion ResourceName ({GUID}Prefabs/.../File.et) used by Resource.Load.
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
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
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
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ItemResourceName {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ItemResourceName {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
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
            .map_err(|e: super::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///Set on a factory attachment or camo configuration of a base weapon — same family prefix, magwell, attachment-slot-type set and mesh, differing only in pre-mounted attachments or materials (for example 'Rifle AK74N 1P29' against 'Rifle AK74N'). Points at the immediate parent item, which must exist in the same envelope. Pickers hide variant rows the way they hide abstracts.
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
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
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
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ItemVariantOf {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ItemVariantOf {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
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
            .map_err(|e: super::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
