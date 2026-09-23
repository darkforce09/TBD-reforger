// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/machine-credential.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::ExecutorKind;

///One issued credential: provenance, use and revocation, never the secret.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct MachineCredential {
    pub created_at: ::chrono::DateTime<::chrono::offset::Utc>,
    pub created_by: ::std::string::String,
    pub executor_kind: ExecutorKind,
    pub id: ::uuid::Uuid,
    pub label: MachineCredentialLabel,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub last_used_at: ::std::option::Option<::chrono::DateTime<::chrono::offset::Utc>>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub revoke_reason: ::std::option::Option<MachineCredentialRevokeReason>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub revoked_at: ::std::option::Option<::chrono::DateTime<::chrono::offset::Utc>>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub revoked_by: ::std::option::Option<::std::string::String>,
    pub server_id: ::uuid::Uuid,
}
///`MachineCredentialLabel`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct MachineCredentialLabel(::std::string::String);
impl ::std::ops::Deref for MachineCredentialLabel {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<MachineCredentialLabel> for ::std::string::String {
    fn from(value: MachineCredentialLabel) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for MachineCredentialLabel {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() > 128usize {
            return Err("longer than 128 characters".into());
        }
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for MachineCredentialLabel {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for MachineCredentialLabel {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for MachineCredentialLabel {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for MachineCredentialLabel {
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
///`MachineCredentialRevokeReason`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct MachineCredentialRevokeReason(::std::string::String);
impl ::std::ops::Deref for MachineCredentialRevokeReason {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<MachineCredentialRevokeReason> for ::std::string::String {
    fn from(value: MachineCredentialRevokeReason) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for MachineCredentialRevokeReason {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() > 512usize {
            return Err("longer than 512 characters".into());
        }
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for MachineCredentialRevokeReason {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for MachineCredentialRevokeReason {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for MachineCredentialRevokeReason {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for MachineCredentialRevokeReason {
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
