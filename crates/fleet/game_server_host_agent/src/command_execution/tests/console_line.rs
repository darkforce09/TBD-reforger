use serde_json::json;

use super::*;

fn invalid_line() -> CommandRefusal {
    CommandRefusal::InvalidArgument {
        action: CONSOLE_COMMAND,
        key: LINE_KEY,
        expected: LINE_EXPECTATION,
    }
}

#[test]
fn a_console_line_is_kept_verbatim() {
    let longest = "x".repeat(CONSOLE_LINE_MAX_BYTES);
    let longest_in_two_byte_characters = "é".repeat(CONSOLE_LINE_MAX_BYTES / 2);
    for raw in [
        "#players",
        "#restart",
        " #players ",
        "#kick 3 spamming @everyone",
        longest.as_str(),
        longest_in_two_byte_characters.as_str(),
    ] {
        assert_eq!(
            ConsoleLine::parse(raw).map(|line| line.as_str().to_owned()),
            Some(raw.to_owned()),
            "{raw:?}"
        );
    }
}

#[test]
fn a_console_line_holds_no_line_or_paragraph_separator() {
    for raw in [
        "#players\u{2028}#shutdown",
        "#players\u{2029}",
        "\u{2028}#players",
    ] {
        assert_eq!(ConsoleLine::parse(raw), None, "{raw:?}");
    }
}

#[test]
fn a_console_line_is_one_line_of_at_most_256_bytes_without_control_characters() {
    let too_long = "x".repeat(CONSOLE_LINE_MAX_BYTES + 1);
    // 86 characters, 258 bytes.
    let too_long_in_three_byte_characters = "€".repeat(86);
    for raw in [
        "",
        "#players\n#shutdown",
        "#players\r",
        "#players\u{85}#shutdown",
        "#players\t1",
        "#players\u{0}",
        "#players\u{7f}",
        too_long.as_str(),
        too_long_in_three_byte_characters.as_str(),
    ] {
        assert_eq!(ConsoleLine::parse(raw), None, "{raw:?}");
    }
}

#[test]
fn a_blank_line_and_custom_rcon_commands_are_refused() {
    for raw in [
        " ",
        "   ",
        "\u{2003}",
        "@logout",
        "@",
        "  @logout",
        "\u{2003}@logout",
    ] {
        assert_eq!(ConsoleLine::parse(raw), None, "{raw:?}");
    }
}

#[test]
fn the_arguments_hold_exactly_one_valid_line() {
    assert_eq!(
        ConsoleLine::from_arguments(&json!({"line": "#players"}))
            .unwrap()
            .as_str(),
        "#players"
    );
    for arguments in [
        json!({}),
        json!({"line": 7}),
        json!({"line": null}),
        json!({"line": "@logout"}),
        json!({"line": "#players\n#shutdown"}),
    ] {
        assert_eq!(
            ConsoleLine::from_arguments(&arguments),
            Err(invalid_line()),
            "{arguments}"
        );
    }
    assert_eq!(
        ConsoleLine::from_arguments(&json!({"line": "#players", "repeat": 2})),
        Err(CommandRefusal::UnexpectedArgument {
            action: CONSOLE_COMMAND,
            key: "repeat".to_owned(),
        })
    );
    assert_eq!(
        ConsoleLine::from_arguments(&json!(["#players"])),
        Err(CommandRefusal::ArgumentsNotAnObject {
            action: CONSOLE_COMMAND,
        })
    );
}

#[test]
fn the_refusal_states_the_line_rules() {
    assert_eq!(
        invalid_line().to_string(),
        format!(
            "console_command needs line to be one line of 1 to {CONSOLE_LINE_MAX_BYTES} bytes, \
             not blank, without control characters or a leading @"
        )
    );
}
