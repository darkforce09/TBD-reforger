//! Unit tests for the account file: the exact document the load engine decodes, and its target.

use serde_json::Value;

use super::{AccountFileTarget, SeededAccount, render_account_file};

fn account(discord_id: &str, refresh_token: &str) -> SeededAccount {
    SeededAccount {
        discord_id: discord_id.to_owned(),
        refresh_token: refresh_token.to_owned(),
    }
}

#[test]
fn staging_fixtures_account_file_lists_accounts_in_order_with_two_keys_each() {
    let accounts = [
        account("9100000000000000000", "first-token"),
        account("9100000000000000001", "second-token"),
    ];
    let document: Value =
        serde_json::from_str(&render_account_file(&accounts).expect("renders")).expect("JSON");
    let object = document.as_object().expect("an object");
    assert_eq!(object.keys().collect::<Vec<_>>(), ["accounts"]);
    let entries = document["accounts"].as_array().expect("a list");
    assert_eq!(entries.len(), 2);
    for (entry, expected) in entries.iter().zip(&accounts) {
        let keys: Vec<&String> = entry.as_object().expect("an object").keys().collect();
        assert_eq!(keys, ["discord_id", "refresh_token"]);
        assert_eq!(entry["discord_id"], expected.discord_id.as_str());
        assert_eq!(entry["refresh_token"], expected.refresh_token.as_str());
    }
}

#[test]
fn staging_fixtures_account_file_directory_is_the_parent_or_the_working_directory() {
    let nested = AccountFileTarget::new("/srv/load/accounts.json".to_owned()).expect("a file");
    assert_eq!(nested.path().to_str(), Some("/srv/load/accounts.json"));
    assert_eq!(nested.directory.to_str(), Some("/srv/load"));
    let bare = AccountFileTarget::new("accounts.json".to_owned()).expect("a file");
    assert_eq!(bare.directory.to_str(), Some("."));
}

#[test]
fn staging_fixtures_account_file_refuses_a_path_without_a_file_name() {
    assert!(AccountFileTarget::new("/srv/load/..".to_owned()).is_err());
    assert!(AccountFileTarget::new("/".to_owned()).is_err());
}
