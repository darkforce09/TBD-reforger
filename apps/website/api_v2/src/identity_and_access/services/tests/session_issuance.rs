use super::arma_id_is_linked;

/// `Option::is_some()` alone returns true for every case below. This pin fails if the
/// helper is rewritten back to `is_some()`, which would report whitespace-only rows as
/// linked while every trimmed resolve path finds nothing.
#[test]
fn whitespace_only_arma_id_is_not_linked() {
    for raw in ["", " ", "   ", "\t", "\n", " \t\n "] {
        let v = Some(raw.to_string());
        assert!(v.is_some(), "precondition: Option is Some");
        assert!(
            !arma_id_is_linked(&v),
            "whitespace-only {raw:?} must not count as linked (is_some()-only would)"
        );
    }
    assert!(!arma_id_is_linked(&None));
}

#[test]
fn nonempty_arma_id_is_linked_even_if_padded() {
    assert!(arma_id_is_linked(&Some("76561198000000999".into())));
    // Padding around a real id still has content after trim — linked.
    assert!(arma_id_is_linked(&Some("  76561198000000999  ".into())));
}
