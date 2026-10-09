use super::*;

/// The slug, game-build and Enfusion GUID checks accept exactly what the contract patterns
/// accept.
#[test]
fn game_ballistics_identifier_patterns_match_the_contract() {
    for good in ["vanilla_mortars", "m252-81mm", "a1_b2-c3", &"a".repeat(64)] {
        assert!(is_slug(good), "{good} is a slug");
    }
    for bad in [
        "",
        "Vanilla",
        "_lead",
        "trail-",
        "double__joint",
        "dot.ted",
        &"a".repeat(65),
    ] {
        assert!(!is_slug(bad), "{bad:?} is not a slug");
    }
    for good in ["1.8", "1.8.0.13"] {
        assert!(is_game_build(good), "{good} is a game build");
    }
    for bad in ["", "1", "1.8.0.13.2", "1..8", "v1.8"] {
        assert!(!is_game_build(bad), "{bad:?} is not a game build");
    }
    assert!(is_enfusion_guid("6A6F008DC5395616"));
    for bad in ["6a6f008dc5395616", "6A6F008DC539561", "6A6F008DC539561G"] {
        assert!(!is_enfusion_guid(bad), "{bad:?} is not a GUID");
    }
}
