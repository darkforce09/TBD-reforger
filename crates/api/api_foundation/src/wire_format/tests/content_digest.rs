use serde_json::json;

use super::{canonical_json, canonical_sha256};
use ::content_digest::sha256_hex;

#[test]
fn canonical_json_sorts_keys_at_every_depth_and_keeps_array_order() {
    let value = json!({"b": 1, "a": {"z": [3, {"y": 1, "x": 2}], "c": null}});
    assert_eq!(
        canonical_json(&value),
        r#"{"a":{"c":null,"z":[3,{"x":2,"y":1}]},"b":1}"#
    );
}

#[test]
fn key_order_does_not_change_the_digest() {
    let first: serde_json::Value = serde_json::from_str(r#"{"kind":"k","sequence":1}"#).unwrap();
    let second: serde_json::Value = serde_json::from_str(r#"{"sequence":1,"kind":"k"}"#).unwrap();
    assert_eq!(canonical_sha256(&first), canonical_sha256(&second));
}

#[test]
fn sha256_hex_is_lowercase_hex_of_the_bytes() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}
