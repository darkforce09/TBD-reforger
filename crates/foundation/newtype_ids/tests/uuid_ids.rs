//! `uuid_id!` expanded in a crate of its own: transparent serde, ordering, `Display`, `FromStr`,
//! the conversions, `Copy`, and the declaration with and without attributes passed through.

use std::collections::HashSet;

use uuid::Uuid;

newtype_ids::uuid_id! {
    /// A member's account, declared with documentation and an extra derive.
    #[derive(Default)]
    pub struct AccountKey;
}

newtype_ids::uuid_id!(struct UploadKey);

const TEXT: &str = "67e55044-10b1-426f-9247-bb680e5fe0c8";

#[test]
fn serialises_exactly_as_its_uuid() {
    let uuid = Uuid::parse_str(TEXT).unwrap();
    let account = AccountKey::new(uuid);
    let id_json = serde_json::to_string(&account).unwrap();
    assert_eq!(id_json, serde_json::to_string(&uuid).unwrap());
    assert_eq!(id_json, format!("\"{TEXT}\""));
    let back: AccountKey = serde_json::from_str(&id_json).unwrap();
    assert_eq!(back, account);
    assert!(serde_json::from_str::<AccountKey>("\"not-a-uuid\"").is_err());
}

#[test]
fn orders_as_its_uuid() {
    let low = AccountKey::new(Uuid::from_u128(1));
    let high = AccountKey::new(Uuid::from_u128(2));
    assert!(low < high);
    assert!(AccountKey::default() < low);
    assert_eq!(AccountKey::default().into_inner(), Uuid::nil());
}

#[test]
fn displays_and_parses_as_its_uuid() {
    let account: AccountKey = TEXT.parse().unwrap();
    assert_eq!(account.to_string(), TEXT);
    let upper: AccountKey = TEXT.to_uppercase().parse().unwrap();
    assert_eq!(upper, account, "parsing accepts what `Uuid` accepts");
    assert_eq!(
        "67e55044".parse::<AccountKey>().unwrap_err(),
        "67e55044".parse::<Uuid>().unwrap_err()
    );
}

#[test]
fn converts_both_ways_and_copies() {
    let uuid = Uuid::parse_str(TEXT).unwrap();
    let account = AccountKey::from(uuid);
    let copy = account;
    assert_eq!(account, copy);
    assert_eq!(Uuid::from(account), uuid);
    assert_eq!(account.as_uuid(), &uuid);
    assert_eq!(account.into_inner(), uuid);
}

#[test]
fn declares_without_attributes() {
    let upload = UploadKey::new(Uuid::from_u128(7));
    let mut seen = HashSet::new();
    assert!(seen.insert(upload));
    assert!(!seen.insert(UploadKey::new(Uuid::from_u128(7))));
    assert_eq!(
        serde_json::to_string(&upload).unwrap(),
        "\"00000000-0000-0000-0000-000000000007\""
    );
}
