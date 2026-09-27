// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/personnel-roster.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::PersonnelPage;

///The administrator's personnel roster. The root is one roster page.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(transparent)]
pub struct PersonnelRosterContract(pub PersonnelPage);
impl ::std::ops::Deref for PersonnelRosterContract {
    type Target = PersonnelPage;
    fn deref(&self) -> &PersonnelPage {
        &self.0
    }
}
impl ::std::convert::From<PersonnelPage> for PersonnelRosterContract {
    fn from(value: PersonnelPage) -> Self {
        Self(value)
    }
}
