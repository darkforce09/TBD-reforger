use super::*;

#[test]
fn slice_id_normalisation_matches_the_parameter_expansion() {
    assert_eq!(strip_through_first_t_dash("T-742"), "742");
    assert_eq!(strip_through_first_t_dash("t-742"), "742");
    // `${tid#*[Tt]-}` is the SHORTEST match, so a stray earlier `t-` wins.
    assert_eq!(strip_through_first_t_dash("xt-9T-742"), "9T-742");
}

#[test]
fn foreign_slice_tokens_are_recognised_so_the_banner_can_be_withheld() {
    assert_eq!(adhoc_token("tbd-target-T-739"), Some("T-739".into()));
    assert_eq!(
        adhoc_token("tbd-target-t739"),
        None,
        "the sed needs the dash"
    );
    assert_eq!(adhoc_token("tbd-target-wave138-verify"), None);
}
