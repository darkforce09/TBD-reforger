// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/mission-editor-payload.schema.json — regenerate with: cargo xtask ci schema-codegen

///One authored slot row.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EditorSlot {
    #[serde(
        rename = "assetId",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub asset_id: ::std::option::Option<::std::string::String>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub callsign: ::std::option::Option<::std::string::String>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub description: ::std::option::Option<::std::string::String>,
    #[serde(
        rename = "editorHidden",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub editor_hidden: ::std::option::Option<bool>,
    pub id: ::std::string::String,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub index: ::std::option::Option<i64>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub loadout: ::std::option::Option<EditorSlotLoadout>,
    #[serde(
        rename = "loadoutId",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub loadout_id: ::std::option::Option<::std::string::String>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub position: ::std::option::Option<EditorSlotPosition>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub rank: ::std::option::Option<::std::string::String>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub role: ::std::option::Option<::std::string::String>,
    #[serde(
        rename = "squadId",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub squad_id: ::std::option::Option<::std::string::String>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub stance: ::std::option::Option<::std::string::String>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub tag: ::std::option::Option<::std::string::String>,
    #[serde(
        rename = "unitName",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub unit_name: ::std::option::Option<::std::string::String>,
}
///`EditorSlotLoadout`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(untagged)]
pub enum EditorSlotLoadout {
    Null,
    String(::std::string::String),
    Object(::serde_json::Map<::std::string::String, ::serde_json::Value>),
}
impl ::std::convert::From<::serde_json::Map<::std::string::String, ::serde_json::Value>>
    for EditorSlotLoadout
{
    fn from(value: ::serde_json::Map<::std::string::String, ::serde_json::Value>) -> Self {
        Self::Object(value)
    }
}
///`EditorSlotPosition`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EditorSlotPosition {
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub rotation: ::std::option::Option<f64>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub x: ::std::option::Option<f64>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub y: ::std::option::Option<f64>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub z: ::std::option::Option<f64>,
}
impl ::std::default::Default for EditorSlotPosition {
    fn default() -> Self {
        Self {
            rotation: Default::default(),
            x: Default::default(),
            y: Default::default(),
            z: Default::default(),
        }
    }
}
