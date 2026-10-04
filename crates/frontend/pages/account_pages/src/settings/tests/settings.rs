//! The guard on the settings page avatar sink.

use frontend_ui::DEFAULT_AVATAR;
use http_url_guard::cases::IS_HTTP_URL_CASES;

#[test]
fn profile_avatar_src_only_keeps_http_urls() {
    let mut wrong = Vec::new();
    for (input, ok) in IS_HTTP_URL_CASES {
        let got = frontend_ui::safe_avatar_url(input);
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
        "settings avatar sink wrong on {} of {} cases:\n{}",
        wrong.len(),
        IS_HTTP_URL_CASES.len(),
        wrong.join("\n")
    );
    assert_eq!(frontend_ui::safe_avatar_url(""), DEFAULT_AVATAR);
}
