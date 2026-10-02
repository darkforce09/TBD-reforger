//! The schema-module partition: schema quotes leave the documentation, derived types follow
//! their definition, impls follow their type, and unknown items are refused.
use super::*;

fn documentation(attributes: &[syn::Attribute]) -> Vec<String> {
    attributes.iter().filter_map(documentation_text).collect()
}

#[test]
fn schema_quote_is_stripped_and_the_description_kept_line_by_line() {
    let item: syn::ItemStruct = syn::parse_quote! {
        #[doc = "The artifact under review."]
        #[doc = ""]
        #[doc = " <details><summary>JSON schema</summary>"]
        #[doc = ""]
        #[doc = " ```json"]
        #[doc = "{ \"type\": \"object\" }"]
        #[doc = " ```"]
        #[doc = " </details>"]
        #[derive(Debug)]
        pub struct Review;
    };
    let mut attributes = item.attrs;
    strip_schema_document(&mut attributes).unwrap();
    assert_eq!(documentation(&attributes), ["The artifact under review."]);
    assert_eq!(attributes.len(), 2, "the derive stays");
}

#[test]
fn schema_quote_inside_one_attribute_is_cut_at_its_opening() {
    let item: syn::ItemStruct = syn::parse_quote! {
        #[doc = "A digest.\n\n<details><summary>JSON schema</summary>\n{}\n</details>"]
        pub struct Digest;
    };
    let mut attributes = item.attrs;
    strip_schema_document(&mut attributes).unwrap();
    assert_eq!(documentation(&attributes), ["A digest."]);
}

#[test]
fn an_unterminated_schema_quote_is_refused() {
    let item: syn::ItemStruct = syn::parse_quote! {
        #[doc = " <details><summary>JSON schema</summary>"]
        #[doc = "{}"]
        pub struct Broken;
    };
    let mut attributes = item.attrs;
    assert!(strip_schema_document(&mut attributes).is_err());
}

#[test]
fn derived_types_belong_to_the_longest_definition_they_extend() {
    let definitions = ["Mission".to_string(), "MissionRow".to_string()];
    assert_eq!(
        owning_definition("MissionRowStatus", &definitions),
        "MissionRow"
    );
    assert_eq!(owning_definition("MissionRow", &definitions), "MissionRow");
    assert_eq!(owning_definition("MissionTitle", &definitions), "Mission");
    assert_eq!(owning_definition("Missionary", &definitions), "Missionary");
    assert_eq!(owning_definition("GameMode", &definitions), "GameMode");
}

#[test]
fn definition_names_follow_typify_pascal_case_and_include_the_titled_root() {
    let schema = serde_json::json!({
        "title": "fleet command contract",
        "definitions": { "FleetAction": {}, "claim-request": {} }
    });
    assert_eq!(
        definition_names(&schema),
        ["ClaimRequest", "FleetAction", "FleetCommandContract"]
    );
}

#[test]
fn impls_follow_their_type_and_foreign_impls_follow_the_preceding_type() {
    let file: syn::File = syn::parse_quote! {
        pub mod error { pub struct ConversionError; }
        pub struct Row { pub status: RowStatus }
        pub struct RowStatus(String);
        impl ::std::convert::From<RowStatus> for ::std::string::String {
            fn from(value: RowStatus) -> Self { value.0 }
        }
        impl Row { pub fn empty() -> bool { true } }
        pub enum GameMode { Coop }
    };
    let output = partition(file, &["Row".to_string(), "GameMode".to_string()]).unwrap();
    assert_eq!(output.support.len(), 1);
    assert_eq!(output.support[0].name, "error");
    let groups: Vec<(&str, Vec<(&str, usize)>)> = output
        .groups
        .iter()
        .map(|group| {
            (
                group.name.as_str(),
                group
                    .blocks
                    .iter()
                    .map(|block| (block.name.as_str(), block.items.len()))
                    .collect(),
            )
        })
        .collect();
    assert_eq!(
        groups,
        [
            ("Row", vec![("Row", 2), ("RowStatus", 2)]),
            ("GameMode", vec![("GameMode", 1)]),
        ]
    );
}

#[test]
fn unexpected_top_level_items_are_refused() {
    let file: syn::File = syn::parse_quote! {
        pub struct Row;
        pub const LIMIT: usize = 1;
    };
    let error = partition(file, &[]).err().expect("a const is refused");
    assert!(error.to_string().contains("const"), "{error}");
}
