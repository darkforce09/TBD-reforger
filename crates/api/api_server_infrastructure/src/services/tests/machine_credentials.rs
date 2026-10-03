use super::*;

#[test]
fn labels_and_reasons_are_trimmed_and_bounded() {
    assert_eq!(
        validated_text("  Primary runtime ", "label", 128).unwrap(),
        "Primary runtime"
    );
    assert!(validated_text("   ", "label", 128).is_err());
    assert!(validated_text(&"x".repeat(129), "label", 128).is_err());
    assert!(validated_text(&"x".repeat(128), "label", 128).is_ok());
}
