use super::*;

#[test]
fn positive_identification_spares_everything_it_cannot_parse() {
    // The three dirs a looser rule would have eaten, measured at MAIN_ROOT.
    assert_eq!(slice_token("target-T-454"), Some("T-454".into()));
    assert_eq!(slice_token("target-T-582-api"), Some("T-582".into()));
    assert_eq!(slice_token("target-t742"), Some("t742".into()));
    assert_eq!(slice_token("target-dev-api"), None);
    assert_eq!(slice_token("target-ci"), None);
    // A GATE dir that CONTAINS a ticket id — anchoring at the first component is what makes
    // this unparseable even without the explicit target-gate-* exclusion.
    assert_eq!(slice_token("target-gate-schema-T422"), None);
    // A dotted id with an `-api` suffix is not `(-.*)?$` after the digits.
    assert_eq!(slice_token("target-T-068.13-api"), None);
}

#[test]
fn adhoc_pattern_is_uppercase_t_with_a_dash_only() {
    assert_eq!(adhoc_token("tbd-target-T-742"), Some("T-742".into()));
    assert_eq!(adhoc_token("tbd-target-T-742-x"), Some("T-742".into()));
    assert_eq!(adhoc_token("tbd-target-t742"), None);
    assert_eq!(adhoc_token("tbd-target-wave138-verify"), None);
    // The shared cache must never parse as a slice dir.
    assert_eq!(adhoc_token("tbd-target"), None);
}

#[test]
fn key_matches_the_tr_pipeline() {
    assert_eq!(key_of("T-702"), "t702");
    assert_eq!(key_of("v2-target-T-1"), "v2targett1");
}
