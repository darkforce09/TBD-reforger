use super::*;

/// **The CDN path-segment trust boundary.**
///
/// `avatar_url()` interpolates two strings from Discord's HTTP response into a URL path. Every
/// input below is what that response would have to contain for the resulting URL to point
/// somewhere other than `cdn.discordapp.com/avatars/<id>/<hash>.png` — stored public-tier and
/// rendered in an `<img src>`.
#[test]
fn a_hostile_avatar_hash_cannot_walk_the_url_off_the_cdn_path() {
    for (id, avatar, why) in [
        (
            "7",
            "../../evil",
            "parent-directory traversal out of /avatars/",
        ),
        ("7", "..", "bare parent directory"),
        ("7", "a/b", "an extra path segment"),
        ("7", "x?y=z", "everything after it becomes a query string"),
        ("7", "x#frag", "everything after it becomes a fragment"),
        ("7", "x%2f..%2fevil", "percent-encoded separator"),
        ("7", "x@evil.com", "re-points the authority once combined"),
        ("7", "x\\y", "backslash, which WHATWG folds to a slash"),
        ("7", "x y", "space"),
        ("7", "x.png", "a dot ends the segment early"),
        // The `id` half is interpolated too, and is guarded for the same reason.
        ("../../evil", "abc", "traversal through the id"),
        ("7/../..", "abc", "traversal through the id"),
        ("7?x=y", "abc", "query injection through the id"),
        ("", "abc", "empty id yields a doubled slash"),
    ] {
        let u = DiscordUser {
            id: id.into(),
            username: "n".into(),
            global_name: String::new(),
            discriminator: String::new(),
            avatar: avatar.to_string(),
        };
        assert_eq!(
            u.avatar_url(),
            "",
            "id={id:?} avatar={avatar:?} built a URL despite {why}"
        );
    }
}

/// The guard is deliberately a character class, not Discord's documented formats. Pinned so a
/// later "tighten this up" pass has to argue with a test rather than silently start blanking
/// real avatars the day Discord widens its hash alphabet.
#[test]
fn the_rule_is_a_character_class_not_a_format() {
    assert!(is_cdn_path_segment("abc123"));
    assert!(is_cdn_path_segment("a_b_C9"));
    assert!(is_cdn_path_segment("ZZZ"));
    // Not hex, not a snowflake, not 32 characters — and deliberately still accepted.
    assert!(is_cdn_path_segment("zzzz"));
    assert!(!is_cdn_path_segment(""));
    for bad in [
        "a.b", "a/b", "a\\b", "a?b", "a#b", "a%b", "a@b", "a:b", "a b", "a\tb",
    ] {
        assert!(!is_cdn_path_segment(bad), "accepted {bad:?}");
    }
}
