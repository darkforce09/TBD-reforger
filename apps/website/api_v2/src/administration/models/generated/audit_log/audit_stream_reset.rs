// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/audit-log.schema.json — regenerate with: cargo xtask ci schema-codegen

///`event: reset` on the audit stream, whose id is the tail: the requested history cannot be replayed. cursor_ahead: the cursor is beyond the newest publication. history_unavailable: the cursor is below the retained floor, at open or when a later wake finds the floor above the stream's cursor. The stream continues after resume_after, the tail, and the client reloads history from the list route.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct AuditStreamReset {
    pub reason: AuditStreamResetReason,
    pub resume_after: u64,
    pub retained_after: u64,
}
///`AuditStreamResetReason`
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
pub enum AuditStreamResetReason {
    #[serde(rename = "cursor_ahead")]
    CursorAhead,
    #[serde(rename = "history_unavailable")]
    HistoryUnavailable,
}
impl ::std::fmt::Display for AuditStreamResetReason {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::CursorAhead => f.write_str("cursor_ahead"),
            Self::HistoryUnavailable => f.write_str("history_unavailable"),
        }
    }
}
impl ::std::str::FromStr for AuditStreamResetReason {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "cursor_ahead" => Ok(Self::CursorAhead),
            "history_unavailable" => Ok(Self::HistoryUnavailable),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for AuditStreamResetReason {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for AuditStreamResetReason {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for AuditStreamResetReason {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
