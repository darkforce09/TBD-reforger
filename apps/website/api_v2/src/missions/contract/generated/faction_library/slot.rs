// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/faction-library.schema.json — regenerate with: cargo xtask ci schema-codegen

///`Slot`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(transparent)]
pub struct Slot(pub ::std::option::Option<::std::string::String>);
impl ::std::ops::Deref for Slot {
    type Target = ::std::option::Option<::std::string::String>;
    fn deref(&self) -> &::std::option::Option<::std::string::String> {
        &self.0
    }
}
impl ::std::convert::From<Slot> for ::std::option::Option<::std::string::String> {
    fn from(value: Slot) -> Self {
        value.0
    }
}
impl ::std::convert::From<::std::option::Option<::std::string::String>> for Slot {
    fn from(value: ::std::option::Option<::std::string::String>) -> Self {
        Self(value)
    }
}
