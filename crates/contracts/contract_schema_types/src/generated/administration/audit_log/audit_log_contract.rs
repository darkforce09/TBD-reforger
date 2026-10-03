// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts/definitions/audit-log.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::AuditLogPage;

///The administrator's audit console: the history list and the live stream that replays from a cursor. The root is one history page.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(transparent)]
pub struct AuditLogContract(pub AuditLogPage);
impl ::std::ops::Deref for AuditLogContract {
    type Target = AuditLogPage;
    fn deref(&self) -> &AuditLogPage {
        &self.0
    }
}
impl ::std::convert::From<AuditLogContract> for AuditLogPage {
    fn from(value: AuditLogContract) -> Self {
        value.0
    }
}
impl ::std::convert::From<AuditLogPage> for AuditLogContract {
    fn from(value: AuditLogPage) -> Self {
        Self(value)
    }
}
