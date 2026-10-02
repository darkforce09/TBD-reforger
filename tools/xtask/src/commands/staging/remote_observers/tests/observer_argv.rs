//! Every observer's command: the read-only psql guard, the quoting that keeps a value one word,
//! the ssh argv that carries no secret, the metrics bearer piped on the host, and the parsers.
use super::console_log_reader;
use super::database_reader::{
    self, CommittedQuery, READ_ONLY_SESSION_OPTIONS, RESTING_STATE_COUNTS, SCHEMA_IDENTITY,
    SESSION_GUARD,
};
use super::discord_member_reader::{self, GuildScope};
use super::host_shell::HostShell;
use super::metrics_reader;
use super::remote_command::{CommandPurpose, RemoteCommand, shell_quote, shell_words};
use super::unit_journal_reader;
use super::unit_state_reader;
use crate::commands::staging::procedure_runner::runner_support::test_settings;

const PARAMETERISED: CommittedQuery = CommittedQuery {
    name: "command_row",
    sql: "SELECT state FROM fleet_commands WHERE id = :'command_id'",
    parameters: &["command_id"],
};

#[test]
fn staging_database_reads_run_read_only_with_committed_selects_and_bound_values() {
    let command = database_reader::select(
        "tbd_staging_db",
        &PARAMETERISED,
        &[("command_id", "x'; DROP TABLE users; --".to_string())],
    )
    .unwrap();
    assert_eq!(command.purpose, CommandPurpose::Read);
    assert!(command.command_line.starts_with(&format!(
        "'docker' 'exec' '-i' '-e' '{READ_ONLY_SESSION_OPTIONS}' 'tbd_staging_db' 'psql' '-X' '-A' '-t'"
    )), "{}", command.command_line);
    assert!(command.command_line.contains("'-v' 'ON_ERROR_STOP=1'"));
    assert!(command.command_line.contains("'-d' 'tbd_reforger'"));
    // The value is one quoted word of the command line and never part of the statement.
    assert!(
        command
            .command_line
            .ends_with(r"'-v' 'command_id=x'\''; DROP TABLE users; --'"),
        "{}",
        command.command_line
    );
    assert_eq!(
        command.stdin.as_deref(),
        Some("SELECT state FROM fleet_commands WHERE id = :'command_id';\n")
    );
    assert_eq!(
        READ_ONLY_SESSION_OPTIONS,
        "PGOPTIONS=-c default_transaction_read_only=on"
    );
    for query in [RESTING_STATE_COUNTS, SESSION_GUARD, SCHEMA_IDENTITY] {
        database_reader::validate(&query).unwrap();
    }
}

#[test]
fn staging_database_reader_refuses_writes_second_statements_and_loose_bindings() {
    let refuse =
        |sql: &'static str, parameters: &'static [&'static str], bindings: &[(&str, String)]| {
            let query = CommittedQuery {
                name: "probe",
                sql,
                parameters,
            };
            format!(
                "{:#}",
                database_reader::select("db", &query, bindings).unwrap_err()
            )
        };
    assert!(refuse("DELETE FROM users", &[], &[]).contains("not a SELECT"));
    assert!(refuse("UPDATE users SET role = 'admin'", &[], &[]).contains("not a SELECT"));
    assert!(refuse("SELECT 1; DELETE FROM users", &[], &[]).contains("`;`"));
    assert!(refuse("SELECT :'id'", &["id"], &[]).contains("not exactly its parameters"));
    assert!(refuse("SELECT 1", &[], &[("id", "1".into())]).contains("not exactly its parameters"));
    assert!(refuse("SELECT 1", &["id"], &[("id", "1".into())]).contains("does not use"));
    assert!(
        refuse("SELECT :'id'", &["id"], &[("id", "1\n\\! rm -rf /".into())])
            .contains("control character")
    );
    assert_eq!(
        database_reader::rows("a|1\n\nb|2\n"),
        vec![vec!["a", "1"], vec!["b", "2"]]
    );
}

#[test]
fn staging_shell_quoting_keeps_every_value_one_word() {
    assert_eq!(shell_quote("plain"), "'plain'");
    assert_eq!(shell_quote("it's; rm -rf ~"), r"'it'\''s; rm -rf ~'");
    assert_eq!(shell_words(&["a b", "$HOME"]), "'a b' '$HOME'");
}

#[test]
fn staging_host_shell_sends_one_command_line_and_no_secret_in_any_argv() {
    let settings = test_settings();
    let shell = HostShell::new(&settings);
    let commands: Vec<RemoteCommand> = vec![
        database_reader::select(
            &settings.database_container,
            &database_reader::SESSION_GUARD,
            &[],
        )
        .unwrap(),
        unit_state_reader::show(&settings.game_server_units()),
        unit_journal_reader::since("tbd-website-api.service", 1_800_000_000),
        console_log_reader::newest(&settings.fleet_root(), 3),
        metrics_reader::exposition(&settings.api_env_file(), &settings.api_origin),
        discord_member_reader::member(&settings, GuildScope::Partner),
    ];
    for command in &commands {
        assert_eq!(
            command.purpose,
            CommandPurpose::Read,
            "{}",
            command.observer
        );
        let argv = shell.argv(command);
        assert_eq!(
            &argv[..3],
            ["sshpass", "-e", "ssh"],
            "the password travels in SSHPASS"
        );
        assert_eq!(argv[argv.len() - 2], "deploy@192.0.2.10");
        assert_eq!(
            argv.last(),
            Some(&command.command_line),
            "one remote argument"
        );
        let everything = format!("{argv:?}{:?}", command.stdin);
        assert!(!everything.contains("ssh-password-canary"), "{everything}");
    }
    assert!(!format!("{settings:?}").contains("ssh-password-canary"));
}

#[test]
fn staging_metrics_bearer_is_read_on_the_host_and_piped_to_curl() {
    let settings = test_settings();
    let command = metrics_reader::exposition(&settings.api_env_file(), &settings.api_origin);
    let script = command.stdin.as_deref().unwrap();
    assert_eq!(command.command_line, "bash -s");
    assert!(
        script.contains("s/^OBSERVABILITY_TOKEN=//p' '/home/deploy/tbd/repo/apps/api/.env'"),
        "{script}"
    );
    assert!(
        script.contains("| curl -fsS --max-time 20 -H @- 'http://127.0.0.1:8080/metrics'"),
        "{script}"
    );
    assert!(
        !script.contains("Authorization: Bearer $token\""),
        "the token never lands in an argument"
    );
    let exposition = "# HELP x\ntbd_build_info{version=\"0.9.1\"} 1\n\
        tbd_http_requests_total{route=\"/api/v1/me\",status=\"200\"} 42\n";
    assert_eq!(
        metrics_reader::build_version(exposition).as_deref(),
        Some("0.9.1")
    );
    assert_eq!(
        metrics_reader::sample(exposition, "tbd_http_requests_total", &[("status", "200")]),
        Some(42.0)
    );
    assert_eq!(
        metrics_reader::sample(exposition, "tbd_http_requests_total", &[("status", "500")]),
        None
    );
}

#[test]
fn staging_unit_journal_and_console_readers_build_reads_and_parse_answers() {
    let states = unit_state_reader::parse(
        "Id=tbd-reforger@1.service\nActiveState=active\nSubState=running\nMainPID=4242\n\
         ExecMainStartTimestampMonotonic=99\nMemoryCurrent=18446744073709551615\nCPUUsageNSec=5000000000\n\n\
         Id=tbd-reforger@2.service\nActiveState=inactive\nSubState=dead\nMainPID=0\nMemoryCurrent=[not set]\n",
    );
    let first = &states["tbd-reforger@1.service"];
    assert_eq!(
        (
            first.main_pid,
            first.memory_current_bytes,
            first.cpu_usage_nanoseconds
        ),
        (Some(4242), None, Some(5_000_000_000))
    );
    let second = &states["tbd-reforger@2.service"];
    assert_eq!(
        (second.active_state.as_str(), second.main_pid),
        ("inactive", None)
    );
    let journal = unit_journal_reader::since("fleet_host_agent@1.service", 1_800_000_000);
    assert_eq!(
        journal.command_line,
        "'journalctl' '--user' '--unit=fleet_host_agent@1.service' '--since=@1800000000' '--no-pager' '--quiet' '--output=short-unix'"
    );
    let lines = unit_journal_reader::parse(
        "1800000001.250000 host fleet_host_agent[9]: claimed\nnot a line\n",
    );
    assert_eq!((lines.len(), lines[0].unix_ms), (1, 1_800_000_001_250));
    let console = console_log_reader::newest("/home/deploy/tbd/fleet", 2);
    assert!(
        console
            .stdin
            .as_deref()
            .unwrap()
            .contains("'/home/deploy/tbd/fleet/instance-2/profile/logs'/logs_*")
    );
    let log =
        console_log_reader::parse("log: /x/logs_1/console.log\nline one\nline two\n").unwrap();
    assert_eq!(
        (log.path.as_str(), log.text.as_str()),
        ("/x/logs_1/console.log", "line one\nline two\n")
    );
    assert_eq!(
        console_log_reader::parse("instance 2 has no console.log"),
        None
    );
    let read = discord_member_reader::member_read(
        "noise\ndiscord-member-read {\"guild_id\":\"1\",\"answered_at_unix_ms\":5,\"outcome\":\"member\",\"roles\":[\"7\"]}\n",
    )
    .unwrap();
    assert_eq!(
        (read.holds_role("7"), read.answered_at_unix_ms),
        (Some(true), 5)
    );
    assert_eq!(
        discord_member_reader::member_read("role_present=true\n"),
        None
    );
}
