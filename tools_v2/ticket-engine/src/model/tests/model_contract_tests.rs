use super::*;

#[test]
fn frozen_unmappable_is_49() {
    assert_eq!(FROZEN_UNMAPPABLE.len(), 49);
    let mut s: Vec<_> = FROZEN_UNMAPPABLE.to_vec();
    s.sort();
    s.dedup();
    assert_eq!(s.len(), 49);
}

#[test]
fn ready_constructor_rejects_empty_goal() {
    let err = Status::live_ready(
        StatusName::Ready,
        1,
        "spec.md".into(),
        "   ".into(),
        vec!["a".into()],
    )
    .unwrap_err();
    assert!(err.contains("main_goal"));
}

/// T-920.1 — the shared title-debt instrument: id-as-title and >10-word titles
/// are debt; a real title within the cap is not; the id can never trip the
/// word-count arm (one token), so the two debt classes cannot overlap.
#[test]
fn title_debt_instrument() {
    assert!(title_is_debt("T-915.4", "T-915.4"), "id-as-title is debt");
    assert!(
        title_is_debt(
            "T-1",
            "one two three four five six seven eight nine ten eleven"
        ),
        "11 words is debt"
    );
    assert!(
        !title_is_debt("T-1", "one two three four five six seven eight nine ten"),
        "exactly 10 words is legal"
    );
    assert!(!title_is_debt("T-1", "Fix the marker save regression"));
    // Emptiness is deliberately NOT this instrument's business: the corpus-wide
    // idea-tier check rule reds an empty title (measured zero offenders), and the
    // ops post-image gate refuses writing one — the pin meters only the two
    // measured history-debt classes.
    assert!(!title_is_debt("T-1", ""));
}

/// T-917.2: the class triage is token-boundary conservative — "prefix"/"fixture"
/// must never classify as bug — and deterministic in its precedence order.
#[test]
fn classify_work_is_token_boundary_and_ordered() {
    assert_eq!(classify_work("Fix editor crash"), "bug");
    assert_eq!(classify_work("marker regression on save"), "bug");
    assert_eq!(classify_work("prefix and fixture handling"), "feature");
    assert_eq!(classify_work("Audit gate coverage"), "audit");
    assert_eq!(classify_work("README doc-only pass"), "docs");
    assert_eq!(classify_work("delete the Makefile"), "chore");
    assert_eq!(classify_work("port wave.sh to xtask"), "chore");
    assert_eq!(classify_work("Marker style widening"), "feature");
    // Precedence: a fix that mentions docs is a bug, not docs.
    assert_eq!(classify_work("fix README typo"), "bug");
    for c in [
        classify_work("a"),
        classify_work("fix"),
        classify_work("audit"),
        classify_work("docs"),
        classify_work("ci"),
    ] {
        assert!(CLASS_VALUES.contains(&c));
    }
}

#[test]
fn domain_as_str_is_snake_case() {
    for (d, s) in [
        (Domain::Website, "website"),
        (Domain::Mod, "mod"),
        (Domain::Schema, "schema"),
        (Domain::Engine, "engine"),
        (Domain::Repo, "repo"),
    ] {
        assert_eq!(d.as_str(), s);
    }
}

#[test]
fn status_name_roundtrip() {
    for s in [
        "idea",
        "queued",
        "ready",
        "running",
        "review",
        "shipped",
        "deferred",
        "cancelled",
    ] {
        assert_eq!(StatusName::parse(s).unwrap().as_str(), s);
    }
    assert!(StatusName::parse("nope").is_none());
}
