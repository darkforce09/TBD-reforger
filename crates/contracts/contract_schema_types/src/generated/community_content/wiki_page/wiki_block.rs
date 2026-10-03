// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts/definitions/wiki-page.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::{WikiCalloutKind, WikiInline, WikiListItem, WikiTableAlignment};

///One block of a page, tagged by type. A table cell is a list of inlines, and a list item, a callout and a quote hold blocks.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum WikiBlock {
    ///A heading of level 1 to 6. anchor is the slugified heading text, made unique within the page with a -2, -3, ... suffix.
    #[serde(rename = "heading")]
    Heading {
        anchor: ::std::string::String,
        inlines: ::std::vec::Vec<WikiInline>,
        level: ::std::num::NonZeroU64,
    },
    ///A paragraph.
    #[serde(rename = "paragraph")]
    Paragraph {
        inlines: ::std::vec::Vec<WikiInline>,
    },
    ///A bulleted or numbered list; a numbered list carries the number of its first item as start.
    #[serde(rename = "list")]
    List {
        items: ::std::vec::Vec<WikiListItem>,
        ordered: bool,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        start: ::std::option::Option<u64>,
    },
    ///A table: one alignment per column, the header cells, and the body rows of cells.
    #[serde(rename = "table")]
    Table {
        alignments: ::std::vec::Vec<WikiTableAlignment>,
        header: ::std::vec::Vec<::std::vec::Vec<WikiInline>>,
        rows: ::std::vec::Vec<::std::vec::Vec<::std::vec::Vec<WikiInline>>>,
    },
    ///A callout: a GitHub blockquote alert (> [!NOTE], [!TIP], [!IMPORTANT], [!WARNING] or [!CAUTION]) or a legacy > [!CRITICAL], [!CAUTION], [!WARNING], [!TIP], [!NOTE] or [!INFO] marker.
    #[serde(rename = "callout")]
    Callout {
        blocks: ::std::vec::Vec<WikiBlock>,
        kind: WikiCalloutKind,
    },
    ///A block quote.
    #[serde(rename = "quote")]
    Quote { blocks: ::std::vec::Vec<WikiBlock> },
    ///A code block; language is the fence's info string, absent when it has none.
    #[serde(rename = "code")]
    Code {
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        language: ::std::option::Option<WikiBlockLanguage>,
        text: ::std::string::String,
    },
    #[serde(rename = "rule")]
    Rule,
}
///`WikiBlockLanguage`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct WikiBlockLanguage(::std::string::String);
impl ::std::ops::Deref for WikiBlockLanguage {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<WikiBlockLanguage> for ::std::string::String {
    fn from(value: WikiBlockLanguage) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for WikiBlockLanguage {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for WikiBlockLanguage {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for WikiBlockLanguage {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for WikiBlockLanguage {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for WikiBlockLanguage {
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
