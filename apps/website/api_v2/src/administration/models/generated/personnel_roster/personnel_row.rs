// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/personnel-roster.schema.json — regenerate with: cargo xtask ci schema-codegen

///One member as the roster lists them. username, discord_handle and arma_character are empty strings when unset, and arma_id is null until the member links an Arma identity. role is the platform permission level, lowest first. warnings counts the member's disciplinary warnings; total_deployments is the member's recorded deployment tally.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct PersonnelRow {
    pub arma_character: ::std::string::String,
    pub arma_id: ::std::option::Option<::std::string::String>,
    pub discord_handle: ::std::string::String,
    pub discord_id: PersonnelRowDiscordId,
    pub is_banned: bool,
    pub role: PersonnelRowRole,
    pub total_deployments: u64,
    pub username: ::std::string::String,
    pub warnings: u64,
}
///`PersonnelRowDiscordId`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct PersonnelRowDiscordId(::std::string::String);
impl ::std::ops::Deref for PersonnelRowDiscordId {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<PersonnelRowDiscordId> for ::std::string::String {
    fn from(value: PersonnelRowDiscordId) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for PersonnelRowDiscordId {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for PersonnelRowDiscordId {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for PersonnelRowDiscordId {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for PersonnelRowDiscordId {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for PersonnelRowDiscordId {
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
///`PersonnelRowRole`
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
pub enum PersonnelRowRole {
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
impl ::std::fmt::Display for PersonnelRowRole {
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
impl ::std::str::FromStr for PersonnelRowRole {
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
impl ::std::convert::TryFrom<&str> for PersonnelRowRole {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for PersonnelRowRole {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for PersonnelRowRole {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
