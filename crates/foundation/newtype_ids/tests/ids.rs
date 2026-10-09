//! The three id macros expanded in a crate of their own, with and without attributes passed
//! through: each id serialises exactly as its inner value and refuses what its inner type
//! refuses, and a string id keys a map looked up by `&str`.

use std::collections::HashMap;

use uuid::Uuid;

newtype_ids::integer_id! {
    /// The row number of an event, declared with documentation and an extra derive.
    #[derive(Default)]
    pub struct EventNumber(i64);
}

newtype_ids::integer_id!(struct SlotIndex(u16));

newtype_ids::string_id! {
    /// A terrain's identifier, declared with documentation and an extra derive.
    #[derive(Default)]
    pub struct TerrainName;
}

newtype_ids::string_id!(struct Callsign);

newtype_ids::uuid_id! {
    /// A member's account, declared with documentation and an extra derive.
    #[derive(Default)]
    pub struct AccountKey;
}

newtype_ids::uuid_id!(struct UploadKey);

#[test]
fn an_integer_id_serialises_exactly_as_its_integer_within_its_bounds() {
    let event = EventNumber::new(-7);
    let id_json = serde_json::to_string(&event).unwrap();
    assert_eq!(id_json, "-7");
    let back: EventNumber = serde_json::from_str(&id_json).unwrap();
    assert_eq!(back, event);
    assert!(serde_json::from_str::<EventNumber>("\"7\"").is_err());
    assert!(serde_json::from_str::<SlotIndex>("65535").is_ok());
    assert!(serde_json::from_str::<SlotIndex>("65536").is_err());
    assert!(serde_json::from_str::<SlotIndex>("-1").is_err());
}

#[test]
fn a_string_id_serialises_exactly_as_its_string() {
    let everon = TerrainName::new("everon");
    let id_json = serde_json::to_string(&everon).unwrap();
    assert_eq!(id_json, "\"everon\"");
    let back: TerrainName = serde_json::from_str(&id_json).unwrap();
    assert_eq!(back, everon);
    let nested = serde_json::json!({ "terrainId": everon });
    assert_eq!(nested.to_string(), r#"{"terrainId":"everon"}"#);
    assert!(serde_json::from_str::<TerrainName>("42").is_err());
    assert!(serde_json::from_str::<Callsign>(r#"{"0":"T-12"}"#).is_err());
}

#[test]
fn a_map_keyed_by_a_string_id_is_looked_up_by_str() {
    let mut sizes: HashMap<TerrainName, u32> = HashMap::new();
    sizes.insert(TerrainName::new("everon"), 12_800);
    sizes.insert(TerrainName::new("arland"), 4_096);
    assert_eq!(sizes.get("everon"), Some(&12_800));
    assert_eq!(sizes.get("eden"), None);
}

#[test]
fn a_uuid_id_serialises_exactly_as_its_uuid() {
    let text = "67e55044-10b1-426f-9247-bb680e5fe0c8";
    let uuid = Uuid::parse_str(text).unwrap();
    let account = AccountKey::new(uuid);
    let id_json = serde_json::to_string(&account).unwrap();
    assert_eq!(id_json, format!("\"{text}\""));
    let back: AccountKey = serde_json::from_str(&id_json).unwrap();
    assert_eq!(back, account);
    assert!(serde_json::from_str::<AccountKey>("\"not-a-uuid\"").is_err());
    let upload = UploadKey::new(Uuid::from_u128(7));
    assert_eq!(
        serde_json::to_string(&upload).unwrap(),
        "\"00000000-0000-0000-0000-000000000007\""
    );
}
