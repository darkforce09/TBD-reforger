// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/current-profile.schema.json — regenerate with: cargo xtask ci schema-codegen

///`UserAccount`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct UserAccount {
    pub arma_character: ::std::string::String,
    pub arma_id: ::std::option::Option<::std::string::String>,
    pub attendance_rate: f64,
    pub avatar_url: ::std::string::String,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub ban_reason: ::std::option::Option<::std::string::String>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub banned_at: ::std::option::Option<::std::string::String>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub banned_by: ::std::option::Option<::std::string::String>,
    pub created_at: ::std::string::String,
    pub discord_handle: ::std::string::String,
    pub discord_id: ::std::string::String,
    pub is_banned: bool,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub last_login_at: ::std::option::Option<::std::string::String>,
    pub role: UserAccountRole,
    pub total_deployments: i64,
    pub updated_at: ::std::string::String,
    pub username: ::std::string::String,
}
///`UserAccountRole`
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
pub enum UserAccountRole {
    #[serde(rename = "guest")]
    Guest,
    #[serde(rename = "enlisted")]
    Enlisted,
    #[serde(rename = "leader")]
    Leader,
    #[serde(rename = "mission_maker")]
    MissionMaker,
    #[serde(rename = "admin")]
    Admin,
}
impl ::std::fmt::Display for UserAccountRole {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Guest => f.write_str("guest"),
            Self::Enlisted => f.write_str("enlisted"),
            Self::Leader => f.write_str("leader"),
            Self::MissionMaker => f.write_str("mission_maker"),
            Self::Admin => f.write_str("admin"),
        }
    }
}
impl ::std::str::FromStr for UserAccountRole {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "guest" => Ok(Self::Guest),
            "enlisted" => Ok(Self::Enlisted),
            "leader" => Ok(Self::Leader),
            "mission_maker" => Ok(Self::MissionMaker),
            "admin" => Ok(Self::Admin),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for UserAccountRole {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for UserAccountRole {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for UserAccountRole {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
