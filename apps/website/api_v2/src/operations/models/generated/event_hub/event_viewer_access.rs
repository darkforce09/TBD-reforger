// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/event-hub.schema.json — regenerate with: cargo xtask ci schema-codegen

///`EventViewerAccess`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EventViewerAccess {
    ///Discord verification is pending or stale for a guild this event's policies rely on.
    pub membership_verification_pending: bool,
    ///The pool a new place comes from first.
    pub quota_class: EventViewerAccessQuotaClass,
    pub visibility: EventViewerAccessVisibility,
}
///The pool a new place comes from first.
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
pub enum EventViewerAccessQuotaClass {
    #[serde(rename = "member")]
    Member,
    #[serde(rename = "guest")]
    Guest,
}
impl ::std::fmt::Display for EventViewerAccessQuotaClass {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Member => f.write_str("member"),
            Self::Guest => f.write_str("guest"),
        }
    }
}
impl ::std::str::FromStr for EventViewerAccessQuotaClass {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "member" => Ok(Self::Member),
            "guest" => Ok(Self::Guest),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for EventViewerAccessQuotaClass {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for EventViewerAccessQuotaClass {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for EventViewerAccessQuotaClass {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
///`EventViewerAccessVisibility`
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
pub enum EventViewerAccessVisibility {
    #[serde(rename = "full")]
    Full,
    #[serde(rename = "partial")]
    Partial,
}
impl ::std::fmt::Display for EventViewerAccessVisibility {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Full => f.write_str("full"),
            Self::Partial => f.write_str("partial"),
        }
    }
}
impl ::std::str::FromStr for EventViewerAccessVisibility {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "full" => Ok(Self::Full),
            "partial" => Ok(Self::Partial),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for EventViewerAccessVisibility {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for EventViewerAccessVisibility {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for EventViewerAccessVisibility {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
