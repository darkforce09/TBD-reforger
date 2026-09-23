//! Source pins for the lifecycle handlers: the authorisation tier every mutator must keep, and
//! the two field guards (title, weather) whose absence is silent rather than loud.

const LIFECYCLE: &str = include_str!("../mission_lifecycle.rs");
const SUBMISSION: &str = include_str!("../mission_submission.rs");
const REVIEWS: &str = include_str!("../mission_reviews.rs");
const VERSIONS: &str = include_str!("../mission_versions.rs");
const ARMORY: &str = include_str!("../mission_armory.rs");

/// The production half of a handler file — everything before its sibling-test declaration.
fn production_half<'a>(source: &'a str, file: &str) -> &'a str {
    source
        .split("#[cfg(test)]")
        .next()
        .unwrap_or_else(|| panic!("{file} must have a #[cfg(test)] module"))
}

/// The `(…)` argument list of one handler, cut at the `) ->` return arrow rather than the first
/// `)` so the `State(state)` extractor does not end the window early.
fn signature<'a>(production: &'a str, name: &str, file: &str) -> &'a str {
    let marker = format!("pub async fn {name}(");
    let start = production
        .find(&marker)
        .unwrap_or_else(|| panic!("{name} must exist in {file}"));
    let after = &production[start..];
    let end = after
        .find(") ->")
        .unwrap_or_else(|| panic!("{name} must have a `) ->` return arrow"));
    &after[..=end]
}

/// PATCH must require `MissionMakerUser`, the same tier as create — demotion revokes edit, and
/// ownership-outlives-role is deliberately not the product rule.
///
/// RED: change `update_mission`'s extractor back to `user: AuthUser` — this pin fails.
#[test]
fn update_mission_requires_mission_maker_tier() {
    let production = production_half(LIFECYCLE, "mission_lifecycle.rs");
    let sig = signature(production, "update_mission", "mission_lifecycle.rs");
    assert!(
        sig.contains("maker: MissionMakerUser"),
        "update_mission must take MissionMakerUser (role still required to edit); got:\n{sig}"
    );
    assert!(
        !sig.contains("user: AuthUser"),
        "update_mission must not take bare AuthUser (that is the demotion-survives-edit bug); got:\n{sig}"
    );
}

/// DELETE / submit / review comments / create_version / set_armory / set_current_version must
/// require `MissionMakerUser`, the same tier as PATCH — demotion revokes every mutator, wherever
/// the handler lives.
///
/// RED: change any of the extractors back to `user: AuthUser` — this pin fails.
#[test]
fn mission_mutators_require_mission_maker_tier() {
    for (file, source, names) in [
        ("mission_lifecycle.rs", LIFECYCLE, &["delete_mission"][..]),
        ("mission_submission.rs", SUBMISSION, &["submit_mission"][..]),
        (
            "mission_reviews.rs",
            REVIEWS,
            &["add_mission_review_comment"][..],
        ),
        (
            "mission_versions.rs",
            VERSIONS,
            &["create_version", "set_current_version"][..],
        ),
        ("mission_armory.rs", ARMORY, &["set_armory"][..]),
    ] {
        let production = production_half(source, file);
        for name in names {
            let sig = signature(production, name, file);
            assert!(
                sig.contains("maker: MissionMakerUser"),
                "{name} must take MissionMakerUser (role still required after demotion); got:\n{sig}"
            );
            assert!(
                !sig.contains("user: AuthUser"),
                "{name} must not take bare AuthUser (demotion-survives-mutator bug); got:\n{sig}"
            );
        }
    }
}

/// Both title writers must route through `validated_mission_title` — a raw bind on PATCH lets a
/// blank title clobber a real one.
#[test]
fn create_and_patch_both_guard_mission_title() {
    let production = production_half(LIFECYCLE, "mission_lifecycle.rs");
    let create = production
        .split("pub async fn create_mission(")
        .nth(1)
        .and_then(|s| s.split("pub async fn update_mission(").next())
        .expect("create_mission body");
    assert!(
        create.contains("validated_mission_title(&input.title)"),
        "create_mission must validate title via validated_mission_title"
    );
    let update = production
        .split("pub async fn update_mission(")
        .nth(1)
        .and_then(|s| s.split("pub async fn").next())
        .expect("update_mission body");
    assert!(
        update.contains("validated_mission_title(t)"),
        "update_mission must validate title via validated_mission_title; got a bare bind"
    );
    assert!(
        !update.contains("qb.push(\", title = \").push_bind(t.clone())"),
        "update_mission must not bind a raw title (the clobber path)"
    );
}

/// The clear arm must not include `""`, and both writers must route their weather through the
/// predicate. Source pin so a match-arm typo cannot reintroduce the silent rewrite without
/// failing. The predicate lives in `missions::validation::mission_fields`; the handlers that call
/// it live here, so this pin reads both files.
///
/// RED: change the clear arm back to `"" | "clear"` — this pin fails.
#[test]
fn valid_weather_clear_arm_excludes_empty_string() {
    const FIELDS: &str = include_str!("../../validation/mission_fields.rs");
    let production = production_half(LIFECYCLE, "mission_lifecycle.rs");
    let helper = FIELDS
        .split("fn valid_weather(s: &str)")
        .nth(1)
        .and_then(|s| s.split("fn valid_time_of_day(").next())
        .expect("valid_weather body");
    assert!(
        !helper.contains("\"\" | \"clear\""),
        "valid_weather must not map \"\" to Clear (a silent rewrite); got:\n{helper}"
    );
    assert!(
        helper.contains("\"clear\" => Some(WeatherType::Clear)"),
        "valid_weather must still accept the clear enum string"
    );
    // CREATE validates non-empty weather via valid_weather; empty is defaulted separately.
    assert!(
        production.contains("valid_weather(&input.weather)"),
        "create_mission must validate non-empty weather via valid_weather"
    );
    let update = production
        .split("pub async fn update_mission(")
        .nth(1)
        .and_then(|s| s.split("pub async fn").next())
        .expect("update_mission body");
    assert!(
        update.contains("valid_weather(w)"),
        "update_mission must validate weather via valid_weather"
    );
}

/// CREATE maps omitted/empty weather to `WeatherType::Clear` at the handler site.
///
/// `CreateMissionInput.weather` is `#[serde(default)]`, so a missing JSON field is `""`, and
/// `valid_weather("")` is `None` — without this default, a POST that never mentions weather 400s.
///
/// RED: delete the `input.weather.is_empty()` arm — this pin fails.
#[test]
fn create_mission_empty_weather_defaults_to_clear() {
    let production = production_half(LIFECYCLE, "mission_lifecycle.rs");
    let create = production
        .split("pub async fn create_mission(")
        .nth(1)
        .and_then(|s| s.split("pub async fn").next())
        .expect("create_mission body");
    assert!(
        create.contains("input.weather.is_empty()"),
        "create_mission must treat empty/omitted weather as the Clear default; got:\n{create}"
    );
    assert!(
        create.contains("WeatherType::Clear"),
        "create_mission empty-weather arm must bind WeatherType::Clear"
    );
    // Must still refuse explicit garbage (not only default empty).
    assert!(
        create.contains("invalid weather"),
        "create_mission must still 400 on explicit invalid weather strings"
    );
}

/// PATCH blank weather stays rejected — no empty→Clear on update, because a PATCH naming the key
/// is asking to SET it.
///
/// RED: add an `if w.is_empty()` default arm in update_mission — this pin fails.
#[test]
fn update_mission_has_no_empty_weather_clear_default() {
    let production = production_half(LIFECYCLE, "mission_lifecycle.rs");
    let update = production
        .split("pub async fn update_mission(")
        .nth(1)
        .and_then(|s| s.split("pub async fn").next())
        .expect("update_mission body");
    assert!(
        update.contains("valid_weather(w)"),
        "update_mission must still validate via valid_weather"
    );
    assert!(
        !update.contains("WeatherType::Clear"),
        "update_mission must not default blank weather to Clear (the PATCH contract); got:\n{update}"
    );
    assert!(
        !update.contains("w.is_empty()"),
        "update_mission must not special-case empty weather as Clear"
    );
}
