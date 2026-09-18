use super::*;

#[test]
fn extracts_markers() {
    let c = extract(
        "claim is `X` @idx crf#UpdateSlotPlayerID here\n@idx api#SCR_BaseGameMode",
        "d.md",
    );
    assert_eq!(c.len(), 2);
    assert_eq!(c[0].lane, "crf");
    assert_eq!(c[0].symbol, "UpdateSlotPlayerID");
    assert_eq!(c[0].line, 1);
    assert_eq!(c[1].lane, "api");
    assert_eq!(c[1].symbol, "SCR_BaseGameMode");
}

#[test]
fn ignores_prose_without_markers() {
    assert!(extract("no citations here at all", "d.md").is_empty());
}
