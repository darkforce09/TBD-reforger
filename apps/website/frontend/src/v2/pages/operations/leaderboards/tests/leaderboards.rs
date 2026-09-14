//! The guards on the ladders: the wire row reader, the win-rate scaling, the initials fallback
//! and the avatar image sink.

use super::*;

/// The exact row `GET /api/v1/leaderboards` served off the dev stack, against a seeded totals
/// table. It pins [`parse_row`] to the handler's own field names.
const WIRE_ROW: &str = r#"{"avatar_url":"","command_win_rate":0.0,"command_wins":0,
    "deaths":0,"discord_id":"000000000000000001","kd_ratio":1.5,"kills":3,"longest_kill_m":412,
    "missions_played":1,"rank":1,"team_kills":2,"username":"Dev Operator",
    "vehicles_destroyed":0}"#;

fn wire() -> Value {
    serde_json::from_str(WIRE_ROW).expect("fixture parses")
}

#[test]
fn parse_row_reads_every_rendered_field() {
    let r = parse_row(&wire(), 0);
    assert_eq!(r.rank, 1);
    assert_eq!(r.discord_id, "000000000000000001");
    assert_eq!(r.username, "Dev Operator");
    assert_eq!(r.avatar_url, "");
    assert_eq!(r.kills, 3);
    assert_eq!(r.kd_ratio, 1.5);
    assert_eq!(r.team_kills, 2);
    assert_eq!(r.missions_played, 1);
    assert_eq!(r.longest_kill_m, 412);
}

#[test]
fn parse_row_prefers_the_server_rank_over_position() {
    // Page 2 of the board: the handler ranks `offset + i + 1`, so row 0 is #21, not #1.
    let mut v = wire();
    v["rank"] = Value::from(21);
    assert_eq!(parse_row(&v, 0).rank, 21);
}

#[test]
fn parse_row_falls_back_to_position_when_rank_is_missing_or_zero() {
    // Without the fallback every operator renders as "#0" / "00".
    let mut absent = wire();
    absent.as_object_mut().expect("object").remove("rank");
    assert_eq!(parse_row(&absent, 2).rank, 3);
    let mut zero = wire();
    zero["rank"] = Value::from(0);
    assert_eq!(parse_row(&zero, 2).rank, 3);
}

#[test]
fn parse_row_survives_a_row_missing_its_stats() {
    let r = parse_row(&Value::Null, 0);
    assert_eq!(r.rank, 1);
    assert_eq!(r.username, "");
    assert_eq!(r.kills, 0);
}

#[test]
fn command_win_rate_renders_the_wire_fraction_as_a_percentage() {
    // The wire sends a fraction: a rate of 0.81 is eighty-one percent. Storing it already
    // multiplied out is the mistake that reads "0%" for every real operator.
    assert_eq!(win_rate_pct(0.81), "81%");
    assert_eq!(win_rate_pct(0.0), "0%");
    assert_eq!(win_rate_pct(1.0), "100%");

    let mut v = wire();
    v["command_win_rate"] = Value::from(0.812);
    let r = parse_row(&v, 0);
    assert_eq!(stat_for(&r, "command_win").0, "81%");
}

#[test]
fn stat_for_leaves_kd_ratio_unscaled() {
    // kd_ratio is kills/deaths, not a proportion — scaling it would be the mirror mistake.
    let r = parse_row(&wire(), 0);
    assert_eq!(stat_for(&r, "kd").0, "1.50");
}

#[test]
fn initials_cover_the_empty_avatar_url_the_api_actually_sends() {
    assert_eq!(initials("Dev Operator"), "DO");
    assert_eq!(initials("Reaper"), "R");
    assert_eq!(initials("a b c"), "AB");
    assert_eq!(initials(""), "?");
}

// Shared adversarial corpus — same table both `is_http_url` implementations pin to.
// If a future edit reverts this cell to `!url.is_empty()`, every `false` row starts
// emitting an `<img src>` again and this names the payload.
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../shared/is_http_url_cases.rs"
));

#[test]
fn avatar_img_emits_src_only_for_http_urls() {
    let mut wrong = Vec::new();
    for (input, should_img) in IS_HTTP_URL_CASES {
        match (avatar_img_src(input), should_img) {
            (Some(_), false) => wrong.push(format!("  RENDERED AN IMG FOR {input:?}")),
            (None, true) => wrong.push(format!("  refused a legitimate avatar {input:?}")),
            _ => {}
        }
    }
    assert!(
        wrong.is_empty(),
        "the leaderboard avatar sink is wrong on {} of {} cases:\n{}",
        wrong.len(),
        IS_HTTP_URL_CASES.len(),
        wrong.join("\n")
    );
}
