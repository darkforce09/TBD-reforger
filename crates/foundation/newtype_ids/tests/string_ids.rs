//! `string_id!` expanded in a crate of its own: transparent serde, `Borrow<str>` lookups,
//! ordering, `Display`, `FromStr`, the conversions, and the declaration with and without
//! attributes passed through.

use std::collections::{BTreeSet, HashMap};

newtype_ids::string_id! {
    /// A terrain's identifier, declared with documentation and an extra derive.
    #[derive(Default)]
    pub struct TerrainName;
}

newtype_ids::string_id!(struct Callsign);

#[test]
fn serialises_exactly_as_its_string() {
    let everon = TerrainName::new("everon");
    let id_json = serde_json::to_string(&everon).unwrap();
    assert_eq!(id_json, serde_json::to_string("everon").unwrap());
    assert_eq!(id_json, "\"everon\"");
    let back: TerrainName = serde_json::from_str(&id_json).unwrap();
    assert_eq!(back, everon);
    let nested = serde_json::json!({ "terrainId": everon });
    assert_eq!(nested.to_string(), r#"{"terrainId":"everon"}"#);
}

#[test]
fn refuses_a_non_string_value() {
    assert!(serde_json::from_str::<TerrainName>("42").is_err());
    assert!(serde_json::from_str::<TerrainName>(r#"{"0":"everon"}"#).is_err());
}

#[test]
fn a_map_keyed_by_the_id_is_looked_up_by_str() {
    let mut sizes: HashMap<TerrainName, u32> = HashMap::new();
    sizes.insert(TerrainName::new("everon"), 12_800);
    sizes.insert(TerrainName::new("arland"), 4_096);
    assert_eq!(sizes.get("everon"), Some(&12_800));
    assert_eq!(sizes.get("arland"), Some(&4_096));
    assert_eq!(sizes.get("eden"), None);
    let set: BTreeSet<TerrainName> = sizes.into_keys().collect();
    assert!(set.contains("arland"));
}

#[test]
fn orders_as_its_string() {
    let mut names = [
        TerrainName::new("everon"),
        TerrainName::new("arland"),
        TerrainName::new("Everon"),
    ];
    names.sort();
    let spelled: Vec<&str> = names.iter().map(TerrainName::as_str).collect();
    assert_eq!(spelled, ["Everon", "arland", "everon"]);
    assert!(TerrainName::new("a") < TerrainName::new("b"));
}

#[test]
fn displays_and_parses_as_its_string() {
    let everon: TerrainName = "everon".parse().unwrap();
    assert_eq!(everon.to_string(), "everon");
    assert_eq!(format!("{everon}"), "everon");
    let empty: TerrainName = "".parse().unwrap();
    assert_eq!(empty, TerrainName::default());
}

#[test]
fn converts_and_compares_with_strings() {
    let from_owned = TerrainName::from(String::from("everon"));
    let from_slice = TerrainName::from("everon");
    assert_eq!(from_owned, from_slice);
    assert_eq!(from_owned, *"everon");
    assert_eq!(from_owned, "everon");
    assert_ne!(from_owned, "arland");
    assert_eq!(from_slice.as_ref(), "everon");
    assert_eq!(from_slice.as_str(), "everon");
    let owned: String = from_slice.into();
    assert_eq!(owned, "everon");
    assert_eq!(from_owned.into_inner(), "everon");
}

#[test]
fn declares_without_attributes() {
    let lead = Callsign::new("Hammer 1-1");
    let copy = lead.clone();
    assert_eq!(lead, copy);
    assert_eq!(format!("{lead:?}"), r#"Callsign("Hammer 1-1")"#);
    assert_eq!(serde_json::to_string(&lead).unwrap(), r#""Hammer 1-1""#);
    let back: Callsign = serde_json::from_str(r#""Hammer 1-1""#).unwrap();
    assert_eq!(back, lead);
}
