use super::*;

/// The SemVer 2.0 accept set. A live DB census of 133 rows was all `X.Y.Z` before enforce.
///
/// RED: replace `valid_semver` with `!s.is_empty()` — padded / partial versions pass and the
/// unique-index hole reopens.
#[test]
fn semver_accepts_real_versions_only() {
    for ok in [
        "0.1.0",
        "0.2.0",
        "1.2.3",
        "1.0.0-alpha",
        "1.0.0-alpha.1",
        "1.0.0-0.3.7",
        "1.0.0+20130313144700",
        "1.0.0-beta+exp.sha.5114f85",
    ] {
        assert!(valid_semver(ok), "expected accept {ok:?}");
    }
    for bad in [
        "", "   ", " 0.1.0 ", "0.1.0 ", " 0.1.0", "1", "1.2", "banana", "01.2.3", "1.02.3",
        "1.2.03", "v1.2.3", "1.2.3.4", "1.2.3-", "1.2.3+",
    ] {
        assert!(!valid_semver(bad), "expected reject {bad:?}");
    }
}
