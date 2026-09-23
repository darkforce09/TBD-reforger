// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/mission-review.schema.json — regenerate with: cargo xtask ci schema-codegen

///`WeatherType`
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
pub enum WeatherType {
    #[serde(rename = "clear")]
    Clear,
    #[serde(rename = "overcast")]
    Overcast,
    #[serde(rename = "heavy_rain")]
    HeavyRain,
    #[serde(rename = "dense_fog")]
    DenseFog,
}
impl ::std::fmt::Display for WeatherType {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Clear => f.write_str("clear"),
            Self::Overcast => f.write_str("overcast"),
            Self::HeavyRain => f.write_str("heavy_rain"),
            Self::DenseFog => f.write_str("dense_fog"),
        }
    }
}
impl ::std::str::FromStr for WeatherType {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "clear" => Ok(Self::Clear),
            "overcast" => Ok(Self::Overcast),
            "heavy_rain" => Ok(Self::HeavyRain),
            "dense_fog" => Ok(Self::DenseFog),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for WeatherType {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for WeatherType {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for WeatherType {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
