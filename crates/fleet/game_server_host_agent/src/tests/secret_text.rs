use super::*;

#[test]
fn debug_formatting_never_shows_the_secret() {
    let secret = SecretText::new("tbdm_do-not-print");
    let printed = format!("{secret:?} {:?}", Some(secret.clone()));
    assert!(!printed.contains("do-not-print"), "{printed}");
    assert!(printed.contains("<redacted>"));
}

#[test]
fn the_secret_is_available_to_its_own_protocol() {
    assert_eq!(SecretText::new("range-master").expose(), "range-master");
}
