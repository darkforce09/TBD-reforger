//! The active-link rule and the frame classifier, plus the avatar sink the top bar renders through.

use super::{classify_frame, is_active, FrameKind};
use crate::v2::core::ui::DEFAULT_AVATAR;

#[test]
fn is_active_dashboard_exact() {
    assert!(is_active("/", "/"));
    assert!(!is_active("/", "/missions"));
}

#[test]
fn is_active_prefix_and_exact() {
    assert!(is_active("/missions", "/missions"));
    assert!(is_active("/missions", "/missions/abc"));
    assert!(!is_active("/missions", "/missions-archive"));
    assert!(!is_active("/events", "/missions"));
}

#[test]
fn classify_frame_kinds() {
    assert!(matches!(classify_frame("/login"), FrameKind::Bare));
    assert!(matches!(classify_frame("/auth/callback"), FrameKind::Bare));
    assert!(matches!(
        classify_frame("/missions/abc/edit"),
        FrameKind::Chromeless
    ));
    assert!(matches!(classify_frame("/"), FrameKind::Chrome));
    assert!(matches!(classify_frame("/missions"), FrameKind::Chrome));
}

include!("../../../../../../shared/is_http_url_cases.rs");

#[test]
fn topnav_avatar_src_only_keeps_http_urls() {
    let mut wrong = Vec::new();
    for (input, ok) in IS_HTTP_URL_CASES {
        let got = crate::v2::core::utils::safe_avatar_url(input);
        if *ok {
            if got != *input {
                wrong.push(format!("  dropped a legitimate avatar {input:?}"));
            }
        } else if got != DEFAULT_AVATAR {
            wrong.push(format!("  kept a non-http avatar {input:?} (got {got:?})"));
        }
    }
    assert!(
        wrong.is_empty(),
        "layout avatar sink wrong on {} of {} cases:\n{}",
        wrong.len(),
        IS_HTTP_URL_CASES.len(),
        wrong.join("\n")
    );
    // Empty input falls back to the placeholder.
    assert_eq!(crate::v2::core::utils::safe_avatar_url(""), DEFAULT_AVATAR);
}
