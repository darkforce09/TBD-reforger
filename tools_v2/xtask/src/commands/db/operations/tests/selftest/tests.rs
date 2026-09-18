use super::*;

/// Arm 1 is only worth anything if the baseline and the renderer are separate texts. If a
/// future edit "fixes" the baseline by generating it, this test is the tripwire.
#[test]
fn baseline_covers_every_rendered_target() {
    let rendered = rendered_recipes();
    assert_eq!(rendered.len(), BASELINE.len());
    for (t, _) in &rendered {
        assert!(
            BASELINE.iter().any(|(b, _)| b == t),
            "no frozen baseline for {t}"
        );
    }
}

#[test]
fn frozen_baseline_matches_the_port() {
    match arm_frozen_baseline() {
        Verdict::Held => {}
        other => panic!("arm 1 must hold on a clean tree: {other:?}"),
    }
}
