use super::*;

#[test]
fn user_derived_fields() {
    let modern = DiscordUser {
        id: "1".into(),
        username: "dave".into(),
        global_name: "Dave".into(),
        discriminator: "0".into(),
        avatar: "abc".into(),
    };
    assert_eq!(modern.display_name(), "Dave");
    assert_eq!(modern.handle(), "dave"); // discriminator "0" → username only
    assert_eq!(
        modern.avatar_url(),
        "https://cdn.discordapp.com/avatars/1/abc.png"
    );

    let legacy = DiscordUser {
        id: "2".into(),
        username: "bob".into(),
        global_name: String::new(),
        discriminator: "1234".into(),
        avatar: String::new(),
    };
    assert_eq!(legacy.display_name(), "bob");
    assert_eq!(legacy.handle(), "bob#1234");
    assert_eq!(legacy.avatar_url(), "");
}

/// **Whitespace-only `global_name` must not win field selection.**
///
/// A `global_name.is_empty()` test lets `"   "` win, and oauth then stores it verbatim into
/// `users.username`. The guarantee is the *selection test* (`trim().is_empty()` → fall
/// through to `username`), not trimming a meaningful winner. Tab/newline/NBSP also fall
/// through because `str::trim` is Unicode White_Space.
#[test]
fn whitespace_only_global_name_falls_through_to_username() {
    let mk = |global_name: &str| DiscordUser {
        id: "7".into(),
        username: "sam".into(),
        global_name: global_name.into(),
        discriminator: "0".into(),
        avatar: String::new(),
    };
    assert_eq!(mk("").display_name(), "sam");
    assert_eq!(mk("   ").display_name(), "sam");
    assert_eq!(mk("\t\n").display_name(), "sam");
    assert_eq!(mk("\u{00A0}").display_name(), "sam"); // NBSP
    // Meaningful name still wins — returned exactly as Discord sent it (no display trim).
    assert_eq!(mk("Dave").display_name(), "Dave");
    assert_eq!(mk("  Dave  ").display_name(), "  Dave  ");
}

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
            id: id.to_string(),
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

/// The other half, and the half that keeps the guard alive: a guard that blanks every real
/// avatar gets reverted by whoever ships next.
#[test]
fn real_discord_avatars_still_build_a_cdn_url() {
    let mk = |id: &str, avatar: &str| DiscordUser {
        id: id.to_string(),
        username: "n".into(),
        global_name: String::new(),
        discriminator: String::new(),
        avatar: avatar.to_string(),
    };
    // A real snowflake and a real 32-hex avatar hash.
    assert_eq!(
        mk("80351110224678912", "8342729096ea3675442027381ff50dfe").avatar_url(),
        "https://cdn.discordapp.com/avatars/80351110224678912/8342729096ea3675442027381ff50dfe.png"
    );
    // Animated avatars carry the `a_` prefix — the underscore is why the class is not
    // `is_ascii_alphanumeric` alone.
    assert_eq!(
        mk("80351110224678912", "a_8342729096ea3675442027381ff50dfe").avatar_url(),
        "https://cdn.discordapp.com/avatars/80351110224678912/a_8342729096ea3675442027381ff50dfe.png"
    );
    // No custom avatar stays the empty string it always was.
    assert_eq!(mk("80351110224678912", "").avatar_url(), "");
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
