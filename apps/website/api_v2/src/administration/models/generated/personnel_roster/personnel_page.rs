// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/personnel-roster.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::{PersonnelRosterContract, PersonnelRow};

///GET /api/v1/admin/users?q=&page=&per_page= (administrator): one page of members in one stable order, lower(username) then discord_id, so every member appears on exactly one page. page defaults to 1 and per_page to 20; a per_page above 100 is served as 100; a page or per_page below 1, or one that is not a number, answers 400. A non-blank q keeps the members whose username, Discord handle, Arma character or Arma id contains it, ignoring case. total counts every matching member; a page past the end has no items and the real total.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct PersonnelPage {
    pub items: ::std::vec::Vec<PersonnelRow>,
    pub page: ::std::num::NonZeroU64,
    pub per_page: ::std::num::NonZeroU64,
    pub total: u64,
}
impl ::std::convert::From<PersonnelRosterContract> for PersonnelPage {
    fn from(value: PersonnelRosterContract) -> Self {
        value.0
    }
}
