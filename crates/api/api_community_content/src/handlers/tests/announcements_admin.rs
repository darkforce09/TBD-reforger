use super::*;

#[test]
fn snippet_from_caps_explicit_and_derives_from_body() {
    let long = "x".repeat(250);
    let capped = snippet_from(&long, "ignored");
    assert_eq!(capped.chars().count(), 200);
    assert!(capped.ends_with('…'));

    let derived = snippet_from("", "a < b & c");
    assert_eq!(derived, "a < b & c");
    assert!(!derived.contains("&lt;"));
}
