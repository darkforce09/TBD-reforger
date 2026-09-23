// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/mission-editor-payload.schema.json — regenerate with: cargo xtask ci schema-codegen

///One authored faction row. `key` here is deliberately not `mission.schema.json#/$defs/factionKey`. That `$def` is `^[a-z][a-z0-9_]*$`, it is correct, and it belongs to the other layer: it describes the compiled game-server document, where the compiler slugs the key on the way through and keys `orbat`, `slots[].faction` and `briefings` identically. This schema describes the author's raw graph, whose faction-key vocabulary is uppercase by construction — the four canonical sides are BLUFOR, OPFOR, INDFOR and CIV, the editor sets `key = side`, and `faction-library.schema.json` pins the same four as an enum — so the lowercase pattern would reject every faction the live Mission Creator can produce. The vocabulary is not closed at this layer either, since the API accepts keys such as `USA` that appear in no enum, so an enum would be wrong here too. The ORBAT slot table's own `faction` column is a third thing that no schema describes, correctly, because it is a database column rather than a wire document: the ORBAT derivation copies this field verbatim into the squad template, which is why that table holds uppercase keys — this layer's vocabulary arriving unchanged, as designed. What is constrained here is `required: [key]` plus `minLength: 1`, and nothing else. An absent or empty key is not a vocabulary question: the squad template's `faction` field is `#[serde(default)]`, so an empty key lands an empty `orbat_slots.faction`, matches no armory group, and renders an Event Hub dossier card with zero items. Rejecting it here kills that at the write boundary, before it can reach the table. This rejects; it does not transform. Both sides of that join store bytes verbatim, so a one-sided trim here would break the case where an ORBAT value and an armory value agree with identical padding. Whitespace padding is therefore deliberately not constrained — that rule belongs in Rust at one site, because ECMA-262's `\s` is a strict superset of Rust's `char::is_whitespace` (U+FEFF), and one rule expressed in two languages whose definitions differ is the disagreement this design avoids. Also not covered: the explicit top-level `orbat[]`, the other input to the ORBAT template parser, which wins when present and is absent from every live payload. `additionalProperties` is deliberately left open: the row is the wire for authored per-faction briefing prose (`briefing.{situation,mission,execution,markers}`) and the document hydrate path reloads every non-`id` field verbatim, so closing it would make the graph lossy on reload and break the next slice that adds a field before this schema hears about it.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
pub struct EditorFaction {
    ///The author's faction key, stored and read VERBATIM. Required and non-empty; deliberately NOT matched against `mission.schema.json#/$defs/factionKey` — see this row's note for the layer argument and the measurements.
    pub key: EditorFactionKey,
}
///The author's faction key, stored and read VERBATIM. Required and non-empty; deliberately NOT matched against `mission.schema.json#/$defs/factionKey` — see this row's note for the layer argument and the measurements.
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct EditorFactionKey(::std::string::String);
impl ::std::ops::Deref for EditorFactionKey {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<EditorFactionKey> for ::std::string::String {
    fn from(value: EditorFactionKey) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for EditorFactionKey {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for EditorFactionKey {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for EditorFactionKey {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for EditorFactionKey {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for EditorFactionKey {
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
