//! Unit tests for the Discord read target: the guild flags, the id checks and the loopback-only
//! API base.

use super::{ApiBase, DiscordTargetOptions, GuildChoice, is_discord_id, loopback_api_base};
use crate::argument_list::ArgumentList;
use crate::tool_failure::ToolFailure;

fn take(tokens: &[&str]) -> Result<DiscordTargetOptions, ToolFailure> {
    let tokens: Vec<String> = tokens.iter().map(|token| (*token).to_owned()).collect();
    let mut arguments = ArgumentList::parse(&tokens)?;
    let options = DiscordTargetOptions::take(&mut arguments)?;
    arguments.finish()?;
    Ok(options)
}

fn refusal<T: std::fmt::Debug>(result: Result<T, ToolFailure>) -> String {
    match result {
        Err(ToolFailure::Refused(reason)) => reason,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn staging_fixtures_discord_target_reads_the_main_guild_on_discord_by_default() {
    let options = take(&["--discord-id", "741000000000000001", "--guild", "main"]).expect("parses");
    assert_eq!(options.guild, GuildChoice::Main);
    assert_eq!(options.member, "741000000000000001");
    assert_eq!(options.api_base, ApiBase::Discord);
}

#[test]
fn staging_fixtures_discord_target_partner_guild_takes_its_id() {
    let options = take(&[
        "--discord-id",
        "741000000000000001",
        "--guild",
        "partner",
        "--partner-guild-id",
        "1400000000000000002",
    ])
    .expect("parses");
    assert_eq!(
        options.guild,
        GuildChoice::Partner("1400000000000000002".to_owned())
    );
}

#[test]
fn staging_fixtures_discord_target_refuses_mismatched_guild_flags() {
    let member = ["--discord-id", "741000000000000001"];
    let partner_without_id = [&member[..], &["--guild", "partner"]].concat();
    assert!(refusal(take(&partner_without_id)).contains("--partner-guild-id"));
    let main_with_id = [&member[..], &["--guild", "main", "--partner-guild-id", "1"]].concat();
    assert!(refusal(take(&main_with_id)).contains("--guild partner only"));
    let other = [&member[..], &["--guild", "all"]].concat();
    assert!(refusal(take(&other)).contains("main or partner"));
}

#[test]
fn staging_fixtures_discord_target_ids_are_decimal_discord_ids() {
    assert!(is_discord_id("741000000000000001"));
    assert!(is_discord_id("18446744073709551615"));
    for rejected in [
        "",
        "123456789012345678901",
        "12/../34",
        "-1",
        "12 34",
        "abc",
    ] {
        assert!(!is_discord_id(rejected), "{rejected:?}");
    }
    assert!(refusal(take(&["--discord-id", "../1", "--guild", "main"])).contains("decimal"));
}

#[test]
fn staging_fixtures_discord_target_accepts_only_a_plain_loopback_base() {
    assert_eq!(
        loopback_api_base("http://127.0.0.1:18080/api/v10/").expect("loopback"),
        "http://127.0.0.1:18080/api/v10"
    );
    assert_eq!(
        loopback_api_base("http://[::1]:18080").expect("loopback"),
        "http://[::1]:18080"
    );
    for rejected in [
        "https://discord.com/api/v10",
        "http://localhost:18080",
        "http://192.168.0.129:3080",
        "http://user:secret@127.0.0.1:18080",
        "http://127.0.0.1:18080/?next=https://example.com",
        "ftp://127.0.0.1/",
        "127.0.0.1:18080",
    ] {
        let reason = refusal(loopback_api_base(rejected));
        assert!(reason.contains("loopback"), "{rejected}: {reason}");
        assert!(
            !reason.contains("secret"),
            "the refusal quotes no URL: {reason}"
        );
    }
}
