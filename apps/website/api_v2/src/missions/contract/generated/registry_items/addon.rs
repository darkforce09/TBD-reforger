// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/registry-items.schema.json — regenerate with: cargo xtask ci schema-codegen

///`Addon`
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
