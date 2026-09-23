// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/event-access-administration.schema.json — regenerate with: cargo xtask ci schema-codegen

///Exactly one of created_by and system_origin is present.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct AuthorshipProvenance {
    pub created_at: ::chrono::DateTime<::chrono::offset::Utc>,
    pub created_by: ::std::option::Option<::std::string::String>,
    pub system_origin: ::std::option::Option<::std::string::String>,
}
