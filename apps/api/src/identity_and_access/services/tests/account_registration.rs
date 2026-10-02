//! The avatar URL guard of account registration: only an http(s) URL reaches `users.avatar_url`.

use super::{AccountProfile, stored_avatar_url};

fn profile(avatar_url: &str) -> AccountProfile<'_> {
    AccountProfile {
        discord_id: "741000000000000001",
        username: "Operator",
        discord_handle: "operator",
        avatar_url,
    }
}

#[test]
fn account_registration_keeps_an_http_avatar_url() {
    let url = "https://cdn.discordapp.com/avatars/741000000000000001/a_0123abcd.png";
    assert_eq!(stored_avatar_url(&profile(url)), url);
}

#[test]
fn account_registration_keeps_the_empty_no_avatar_value() {
    assert_eq!(stored_avatar_url(&profile("")), "");
}

#[test]
fn account_registration_blanks_every_url_that_is_not_http() {
    for rejected in [
        "javascript:alert(1)",
        "data:image/png;base64,AAAA",
        "//cdn.discordapp.com/avatars/1/2.png",
        "cdn.discordapp.com/avatars/1/2.png",
        "   ",
    ] {
        assert_eq!(stored_avatar_url(&profile(rejected)), "", "{rejected:?}");
    }
}
