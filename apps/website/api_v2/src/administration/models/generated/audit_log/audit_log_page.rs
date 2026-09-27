// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/audit-log.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::AuditLogEntry;

///GET /api/v1/admin/audit-logs?severity=&q=&before=&limit= (administrator): audit lines, newest id first. severity (info, warn or crit) keeps one severity and a non-blank q the lines whose message contains it, ignoring case; before=<id> continues below that id; limit is served when it is 1 to 100 and is 20 otherwise. next_cursor is the last line's id when the page is full, the before of the next page, and null otherwise.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct AuditLogPage {
    pub data: ::std::vec::Vec<AuditLogEntry>,
    pub next_cursor: ::std::option::Option<::std::num::NonZeroU64>,
}
