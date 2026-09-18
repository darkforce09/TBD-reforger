use super::*;

#[test]
fn defaults_match_live_pipeline_constants() {
    let p = Params::default();
    assert_eq!(p.max_pair_m, 0.7);
    assert_eq!(p.band_low_m, 0.45);
    assert_eq!(p.wall_max_thickness_m, 0.6);
    assert_eq!(p.slab_spacing_m, 1.8);
}

#[test]
fn partial_override_keeps_other_defaults() {
    let p: Params = serde_json::from_str(r#"{"max_drift_m": 0.12}"#).unwrap();
    assert_eq!(p.max_drift_m, 0.12);
    assert_eq!(p.max_pair_m, 0.7);
}

#[test]
fn unknown_key_rejected() {
    assert!(serde_json::from_str::<Params>(r#"{"max_drfit_m": 0.12}"#).is_err());
}
