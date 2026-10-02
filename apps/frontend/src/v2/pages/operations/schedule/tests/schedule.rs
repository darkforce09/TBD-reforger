//! The schedule detail column must keep routing briefings through the shared hub body.

/// Schedule detail must keep going through `event_hub_view` (trim-aware briefing), and must
/// not revive the earlier non-trim briefing emptiness guard.
#[test]
fn schedule_briefing_empty_check_stays_trim_aligned() {
    let src = crate::v2::core::test_support::pins::event_schedule_source();
    assert!(
        src.contains("event_hub_view(ev, on_change)"),
        "schedule detail must render via event_hub_view so operation/mission briefing \
         emptiness stays trim-aware (same rule as event_hub / mission_overview)"
    );
    // concat! so this test body does not match itself.
    let old_then = concat!("(!briefing.", "is_empty())");
    assert!(
        !src.contains(old_then),
        "pre-T-353 non-trim briefing guard must not return on the schedule page — \
         whitespace-only briefings would show an empty Briefing section again"
    );
    let old_bare = concat!("!briefing.", "is_empty()");
    assert!(
        !src.contains(old_bare),
        "any bare briefing.is_empty without trim must not return on the schedule page"
    );
    let old_filter = concat!(".filter(|b| !b.", "is_empty())");
    assert!(
        !src.contains(old_filter),
        "is_empty-only briefing filter must not return on the schedule page"
    );
    let old_arm = concat!("Some(b) if !b.", "is_empty()");
    assert!(
        !src.contains(old_arm),
        "match-arm without trim must not return on the schedule page"
    );
}
