// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/wiki-page.schema.json — regenerate with: cargo xtask ci schema-codegen

///One inline run, tagged by type.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum WikiInline {
    ///Plain text.
    #[serde(rename = "text")]
    Text { text: ::std::string::String },
    ///Strong emphasis.
    #[serde(rename = "strong")]
    Strong {
        children: ::std::vec::Vec<WikiInline>,
    },
    ///Emphasis.
    #[serde(rename = "emphasis")]
    Emphasis {
        children: ::std::vec::Vec<WikiInline>,
    },
    ///Struck-through text.
    #[serde(rename = "strikethrough")]
    Strikethrough {
        children: ::std::vec::Vec<WikiInline>,
    },
    ///Inline code.
    #[serde(rename = "code")]
    Code { text: ::std::string::String },
    ///A link to a safe target: https, http, mailto, a site-relative /path or a #fragment. external is true for absolute http, https and mailto targets.
    #[serde(rename = "link")]
    Link {
        children: ::std::vec::Vec<WikiInline>,
        external: bool,
        href: WikiInlineHref,
    },
    ///An image from an https URL or a site-relative /path; title is absent when the markup gives none.
    #[serde(rename = "image")]
    Image {
        alt: ::std::string::String,
        src: WikiInlineSrc,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        title: ::std::option::Option<WikiInlineTitle>,
    },
    #[serde(rename = "line_break")]
    LineBreak,
}
///`WikiInlineHref`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct WikiInlineHref(::std::string::String);
impl ::std::ops::Deref for WikiInlineHref {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<WikiInlineHref> for ::std::string::String {
    fn from(value: WikiInlineHref) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for WikiInlineHref {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        static PATTERN: ::std::sync::LazyLock<::regress::Regex> = ::std::sync::LazyLock::new(
            || {
                ::regress::Regex::new(
                    "^(https?://[^\\s\\\\/\\x00-\\x1F\\x7F-\\x9F][^\\s\\\\\\x00-\\x1F\\x7F-\\x9F]*|mailto:[^\\s\\\\\\x00-\\x1F\\x7F-\\x9F]+|#[^\\s\\\\\\x00-\\x1F\\x7F-\\x9F]*|/([^\\s\\\\/\\x00-\\x1F\\x7F-\\x9F][^\\s\\\\\\x00-\\x1F\\x7F-\\x9F]*)?)$",
                )
                .unwrap()
            },
        );
        if PATTERN.find(value).is_none() {
            return Err(
                "doesn't match pattern \"^(https?://[^\\s\\\\/\\x00-\\x1F\\x7F-\\x9F][^\\s\\\\\\x00-\\x1F\\x7F-\\x9F]*|mailto:[^\\s\\\\\\x00-\\x1F\\x7F-\\x9F]+|#[^\\s\\\\\\x00-\\x1F\\x7F-\\x9F]*|/([^\\s\\\\/\\x00-\\x1F\\x7F-\\x9F][^\\s\\\\\\x00-\\x1F\\x7F-\\x9F]*)?)$\""
                    .into(),
            );
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for WikiInlineHref {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for WikiInlineHref {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for WikiInlineHref {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for WikiInlineHref {
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
///`WikiInlineSrc`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct WikiInlineSrc(::std::string::String);
impl ::std::ops::Deref for WikiInlineSrc {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<WikiInlineSrc> for ::std::string::String {
    fn from(value: WikiInlineSrc) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for WikiInlineSrc {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        static PATTERN: ::std::sync::LazyLock<::regress::Regex> = ::std::sync::LazyLock::new(
            || {
                ::regress::Regex::new(
                    "^(https://[^\\s\\\\/\\x00-\\x1F\\x7F-\\x9F][^\\s\\\\\\x00-\\x1F\\x7F-\\x9F]*|/[^\\s\\\\/\\x00-\\x1F\\x7F-\\x9F][^\\s\\\\\\x00-\\x1F\\x7F-\\x9F]*)$",
                )
                .unwrap()
            },
        );
        if PATTERN.find(value).is_none() {
            return Err(
                "doesn't match pattern \"^(https://[^\\s\\\\/\\x00-\\x1F\\x7F-\\x9F][^\\s\\\\\\x00-\\x1F\\x7F-\\x9F]*|/[^\\s\\\\/\\x00-\\x1F\\x7F-\\x9F][^\\s\\\\\\x00-\\x1F\\x7F-\\x9F]*)$\""
                    .into(),
            );
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for WikiInlineSrc {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for WikiInlineSrc {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for WikiInlineSrc {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for WikiInlineSrc {
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
///`WikiInlineTitle`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct WikiInlineTitle(::std::string::String);
impl ::std::ops::Deref for WikiInlineTitle {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<WikiInlineTitle> for ::std::string::String {
    fn from(value: WikiInlineTitle) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for WikiInlineTitle {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for WikiInlineTitle {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for WikiInlineTitle {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for WikiInlineTitle {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for WikiInlineTitle {
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
