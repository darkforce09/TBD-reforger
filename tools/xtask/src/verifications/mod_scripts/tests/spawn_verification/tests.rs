use super::DEFAULT_PATTERN;

#[test]
fn default_pattern_matches_bash() {
    assert_eq!(
        DEFAULT_PATTERN,
        r"\[TBD\]\[Slots\]|\[TBD\]\[Loadout\]|\[TBD\]\[Spawn\]|assigned slot|bound player"
    );
}
