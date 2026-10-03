//! Every host action's command: the host tool's argv and guards, the backup, the server update,
//! the relay's control, the outage drop-in; each names its purpose and none carries a secret.
use super::database_backup;
use super::game_server_update::{self, EXPERIMENTAL_SERVER_APP};
use super::host_fixture_commands::{
    self, CredentialExecutor, GuildScope, RotationStage, SpendStart, observe_discord_member,
    spend_discord_member_bucket,
};
use super::outage_dropin;
use super::relay_control::{self, DropTarget};
use crate::procedure_runner::runner_support::test_settings;
use crate::remote_observers::remote_command::{CommandPurpose, RemoteCommand};

fn everything(command: &RemoteCommand) -> String {
    format!(
        "{} {}",
        command.command_line,
        command.stdin.as_deref().unwrap_or_default()
    )
}

#[test]
fn staging_host_tool_runs_from_the_checkout_with_the_database_guard_and_apply() {
    let settings = test_settings();
    let provision = host_fixture_commands::provision_fleet(&settings).unwrap();
    assert_eq!(provision.purpose, CommandPurpose::Change);
    assert_eq!(
        provision.command_line,
        "cd '/home/deploy/tbd/repo' && './target/release/staging-fixtures' 'provision-fleet' \
         '--instances' '5' '--actor' '123456789012345678' '--ip' '192.0.2.10' '--secrets-root' \
         '/home/deploy/tbd/fleet' '--game-port-base' '2000' '--confirm-database' 'tbd_reforger' '--apply'"
    );
    let stage = host_fixture_commands::rotate_credential(
        &settings,
        1,
        CredentialExecutor::HostAgent,
        RotationStage::Stage,
    )
    .unwrap();
    assert!(stage.command_line.contains("'rotate-credential' '--instance' '1' '--executor' 'host_agent' '--secrets-root' '/home/deploy/tbd/fleet' '--stage' '--actor' '123456789012345678'"), "{}", stage.command_line);
    let promote = host_fixture_commands::rotate_credential(
        &settings,
        2,
        CredentialExecutor::ModRuntime,
        RotationStage::Promote,
    )
    .unwrap();
    assert!(promote.command_line.contains("'--executor' 'mod_runtime' '--secrets-root' '/home/deploy/tbd/fleet' '--promote' '--confirm-database'"), "{}", promote.command_line);
    assert!(
        host_fixture_commands::rotate_credential(
            &settings,
            6,
            CredentialExecutor::HostAgent,
            RotationStage::Promote
        )
        .is_err()
    );
    let seeding = host_fixture_commands::LoadSeeding {
        accounts: 1_100,
        id_base: "9100000000000000000",
        discord_role: "Player",
        account_file: "/home/deploy/tbd/load/accounts.json",
        mission_id: "00000000-0000-4000-8000-000000000001",
    };
    let seed: Vec<String> = host_fixture_commands::load_fixtures(&settings, Some(&seeding))
        .iter()
        .map(|c| c.command_line.clone())
        .collect();
    assert!(
        seed[0].contains("'seed-load-population'")
            && seed[1].contains("'seed-load-fixture-events'")
    );
    assert!(seed[0].contains(
        "'--accounts' '1100' '--role' 'Player' '--account-file' \
         '/home/deploy/tbd/load/accounts.json' '--id-base' '9100000000000000000'"
    ));
    assert!(seed[1].contains("'--mission' '00000000-0000-4000-8000-000000000001'"));
    let clean: Vec<String> = host_fixture_commands::load_fixtures(&settings, None)
        .iter()
        .map(|c| c.command_line.clone())
        .collect();
    assert!(
        clean[0].contains("'clean-load-fixture-events'")
            && clean[1].contains("'clean-load-population'")
    );
    let spend =
        host_fixture_commands::spend_discord_member_bucket(&settings, 1_800_000_000_300, 3, 50)
            .unwrap();
    assert!(spend.command_line.contains(
        "'--start-at-unix-ms' '1800000000300' '--hold-seconds' '3' '--max-requests' '50'"
    ));
    assert!(host_fixture_commands::spend_discord_member_bucket(&settings, 0, 3, 51).is_err());
    let aging = host_fixture_commands::age_membership_snapshot(&settings, "123456789012345678", 49);
    assert!(
        aging.command_line.contains(
            "'age-membership-snapshot' '--discord-id' '123456789012345678' '--hours' '49'"
        )
    );
    assert_eq!(
        observe_discord_member(&settings, GuildScope::Main).purpose,
        CommandPurpose::Read
    );
    let mut missing_operator = settings.clone();
    missing_operator.operator_discord_id = None;
    assert!(
        format!(
            "{:#}",
            host_fixture_commands::provision_fleet(&missing_operator).unwrap_err()
        )
        .contains("TBD_STAGING_OPERATOR_DISCORD_ID")
    );
}

/// The spend's one argv builder writes a start known now as a quoted time and a start the host
/// script computes as the expansion of that script's variable, in the same place, and refuses a
/// variable name that is not a shell identifier.
#[test]
fn staging_bucket_spend_start_is_a_time_or_a_host_script_variable() {
    let settings = test_settings();
    let scheduled =
        spend_discord_member_bucket(&settings, SpendStart::HostVariable("start_ms"), 3, 50)
            .unwrap();
    assert_eq!(scheduled.purpose, CommandPurpose::Change);
    assert!(
        scheduled.command_line.ends_with(
            "'spend-discord-member-bucket' '--discord-id' '123456789012345678' '--guild' 'main' \
             '--start-at-unix-ms' \"$start_ms\" '--hold-seconds' '3' '--max-requests' '50' \
             '--confirm-database' 'tbd_reforger' '--apply'"
        ),
        "{}",
        scheduled.command_line
    );
    let known = spend_discord_member_bucket(&settings, 1_800_000_000_300, 3, 50).unwrap();
    assert_eq!(
        known
            .command_line
            .replace("'1800000000300'", "\"$start_ms\""),
        scheduled.command_line
    );
    for name in ["", "1st", "start ms", "start_ms;reboot", "$(id)"] {
        assert!(
            spend_discord_member_bucket(&settings, SpendStart::HostVariable(name), 3, 50).is_err(),
            "{name}"
        );
    }
    assert!(spend_discord_member_bucket(&settings, SpendStart::UnixMillis(0), 3, 0).is_err());
}

#[test]
fn staging_backup_is_verified_before_it_takes_its_name() {
    let settings = test_settings();
    let backup = database_backup::backup(&settings, "pre-setup").unwrap();
    let script = backup.stdin.as_deref().unwrap();
    assert_eq!(backup.purpose, CommandPurpose::Change);
    assert!(
        script.contains("dir='/home/deploy/tbd/backups'/\"$(date -u +%Y-%m-%d)\""),
        "{script}"
    );
    assert!(
        script.contains(
            "docker exec 'tbd_staging_db' pg_dump -U tbd -d tbd_reforger -Fc > \"$file.partial\""
        ),
        "{script}"
    );
    let verify = script.find("pg_restore --list").unwrap();
    assert!(
        verify < script.find("mv \"$file.partial\" \"$file\"").unwrap(),
        "verified before it is named"
    );
    assert!(script.contains("chmod 600 \"$file\"") && script.contains("umask 077"));
    for label in ["", "Pre Setup", "a;b", &"x".repeat(65)] {
        assert!(
            database_backup::backup(&settings, label).is_err(),
            "{label:?}"
        );
    }
    assert_eq!(
        database_backup::backup_line(
            "noise\nbackup: /home/deploy/tbd/backups/2026-09-29/x.dump (1024 bytes)\n"
        ),
        Some("/home/deploy/tbd/backups/2026-09-29/x.dump (1024 bytes)")
    );
}

#[test]
fn staging_game_server_update_validates_app_1890870_only_with_the_fleet_stopped() {
    let settings = test_settings();
    let update = game_server_update::update(&settings);
    let script = update.stdin.as_deref().unwrap();
    assert_eq!(EXPERIMENTAL_SERVER_APP, 1_890_870);
    assert!(
        script.contains("list-units 'tbd-reforger@*' --state=active"),
        "{script}"
    );
    let refusal = script.find("stop them first").unwrap();
    let steamcmd = script.find("steamcmd +force_install_dir '/home/deploy/steam/arma-reforger-server' +login anonymous +app_update 1890870 validate +quit").unwrap();
    assert!(refusal < steamcmd, "{script}");
    assert!(script.contains("appmanifest_1890870.acf"));
    assert_eq!(
        game_server_update::installed_build(&settings).purpose,
        CommandPurpose::Read
    );
    assert_eq!(
        game_server_update::build_id("Success!\nbuildid: 20231234\n"),
        Some("20231234")
    );
    assert_eq!(game_server_update::build_id("buildid: \n"), None);
}

#[test]
fn staging_relay_control_and_outage_dropin_commands() {
    let arm = relay_control::arm(5, DropTarget::ClaimResponse);
    assert_eq!(arm.purpose, CommandPurpose::Change);
    assert_eq!(
        arm.command_line,
        "\"$HOME/.local/bin/acknowledgement-dropping-relay\" control --control-socket \
         \"${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/acknowledgement-dropping-relay-5/control.sock\" \
         arm drop-next-claim-response"
    );
    assert!(
        relay_control::arm(5, DropTarget::ResultResponse)
            .command_line
            .ends_with("arm drop-next-result-response")
    );
    assert!(relay_control::disarm(5).command_line.ends_with(" disarm"));
    assert_eq!(relay_control::status(5).purpose, CommandPurpose::Read);
    assert!(
        relay_control::is_disarmed("{\"arming\":\"disarmed\",\"drop_count\":1,\"last_drop\":null}")
            .unwrap()
    );
    assert!(
        !relay_control::is_disarmed("{\"arming\":\"drop-next-claim-response\",\"last_drop\":null}")
            .unwrap()
    );
    assert!(!relay_control::is_disarmed("{\"arming\":\"drop-next-result-response\"}").unwrap());
    assert!(relay_control::is_disarmed("{\"armed\":null}").is_err());
    assert!(relay_control::is_disarmed("not json").is_err());
    let settings = test_settings();
    let install = outage_dropin::install(&settings);
    let script = install.stdin.as_deref().unwrap();
    assert!(
        script.contains("Environment=HTTPS_PROXY=http://127.0.0.1:9"),
        "{script}"
    );
    assert!(script.contains("'/home/deploy/.config/systemd/user/tbd-website-api.service.d/staging-discord-outage.conf'"), "{script}");
    assert!(script.contains("systemctl --user restart 'tbd-website-api.service'"));
    assert!(outage_dropin::remove(&settings).stdin.as_deref().unwrap().contains("rm -f '/home/deploy/.config/systemd/user/tbd-website-api.service.d/staging-discord-outage.conf'"));
    assert_eq!(
        outage_dropin::state(&settings).purpose,
        CommandPurpose::Read
    );
    for command in [
        install,
        outage_dropin::remove(&settings),
        arm,
        database_backup::backup(&settings, "x").unwrap(),
    ] {
        assert!(!everything(&command).contains("ssh-password-canary"));
    }
}
