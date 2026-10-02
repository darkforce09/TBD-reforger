//! The command line: the unit's `serve` line parses as written, `control` takes its three
//! requests, and `serve` refuses a non-loopback address before anything binds.

use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use clap::Parser;

use super::super::drop_policy::DropTarget;
use super::super::relay_settings::RelaySettings;
use super::super::stub_upstream::TemporaryFolder;
use super::{ControlRequest, RelayCommand, RelayCommandLine, run};
use crate::repository_paths::find_repo_root;

/// The unit the staging deploy installs, relative to the checkout root.
const RELAY_UNIT: &str = "tools/xtask/deploy/systemd/acknowledgement-dropping-relay@.service";

fn parse(arguments: &[&str]) -> Result<RelayCommand, clap::Error> {
    let mut argv = vec!["acknowledgement-dropping-relay"];
    argv.extend_from_slice(arguments);
    RelayCommandLine::try_parse_from(argv).map(|command_line| command_line.command)
}

#[test]
fn the_units_exec_start_line_parses_as_a_serve_command() {
    let unit = fs::read_to_string(find_repo_root().unwrap().join(RELAY_UNIT)).expect("unit file");
    let exec_start = unit
        .lines()
        .find_map(|line| line.strip_prefix("ExecStart="))
        .expect("an ExecStart line");
    let expanded = exec_start
        .replace("${RELAY_LISTEN}", "127.0.0.1:18085")
        .replace("${RELAY_UPSTREAM}", "http://127.0.0.1:8080")
        .replace("%h", "/home/staging")
        .replace("%t", "/run/user/1000")
        .replace("%i", "5");
    let argv: Vec<&str> = expanded.split_whitespace().collect();
    assert!(
        argv[0].ends_with("/acknowledgement-dropping-relay"),
        "{}",
        argv[0]
    );
    let command = RelayCommandLine::try_parse_from(argv).expect("the unit's line parses");
    let RelayCommand::Serve {
        listen,
        upstream,
        control_socket,
    } = command.command
    else {
        panic!("the unit runs serve");
    };
    assert_eq!(
        control_socket,
        PathBuf::from("/run/user/1000/acknowledgement-dropping-relay-5/control.sock")
    );
    let settings = RelaySettings::from_flags(&listen, &upstream, control_socket).unwrap();
    assert_eq!(settings.listen.to_string(), "127.0.0.1:18085");
    assert_eq!(settings.upstream.as_str(), "http://127.0.0.1:8080");
}

#[test]
fn control_takes_arm_disarm_and_status() {
    let socket = "/run/user/1000/acknowledgement-dropping-relay-5/control.sock";
    for (words, expected) in [
        (
            vec!["arm", "drop-next-claim-response"],
            ControlRequest::Arm {
                target: DropTarget::DropNextClaimResponse,
            },
        ),
        (
            vec!["arm", "drop-next-result-response"],
            ControlRequest::Arm {
                target: DropTarget::DropNextResultResponse,
            },
        ),
        (vec!["disarm"], ControlRequest::Disarm),
        (vec!["status"], ControlRequest::Status),
    ] {
        let mut arguments = vec!["control", "--control-socket", socket];
        arguments.extend(words);
        let Ok(RelayCommand::Control {
            control_socket,
            request,
        }) = parse(&arguments)
        else {
            panic!("{arguments:?} parses as control");
        };
        assert_eq!((control_socket, request), (PathBuf::from(socket), expected));
    }
    for refused in [
        vec![
            "control",
            "--control-socket",
            socket,
            "arm",
            "drop-everything",
        ],
        vec!["control", "--control-socket", socket],
        vec!["control", "status"],
        vec!["serve", "--listen", "127.0.0.1:18085"],
    ] {
        let error = parse(&refused).expect_err("a usage error");
        assert_eq!(error.exit_code(), 2, "{refused:?}");
    }
}

/// How long a refused `serve` may take to return; one that is still running has started serving.
const REFUSAL_DEADLINE: Duration = Duration::from_secs(5);

#[test]
fn serve_refuses_a_non_loopback_address_before_anything_binds() {
    let folder = TemporaryFolder::new("serve");
    let socket = folder.socket_path();
    for (listen, upstream) in [
        ("0.0.0.0:0", "http://127.0.0.1:8080"),
        ("203.0.113.7:0", "http://127.0.0.1:8080"),
        ("127.0.0.1:0", "http://203.0.113.7:8080"),
        ("127.0.0.1:0", "https://127.0.0.1:8080"),
    ] {
        let command = parse(&[
            "serve",
            "--listen",
            listen,
            "--upstream",
            upstream,
            "--control-socket",
            socket.to_str().unwrap(),
        ])
        .expect("the flags parse");
        // A `serve` that accepts the addresses runs until a signal, so it runs on a thread of its
        // own and a missed deadline fails the case instead of hanging the suite.
        let (sender, outcome) = mpsc::channel();
        thread::spawn(move || sender.send(run(command).map_err(|error| format!("{error:#}"))));
        let refused = outcome
            .recv_timeout(REFUSAL_DEADLINE)
            .unwrap_or_else(|_| panic!("{listen} {upstream}: serve started instead of refusing"))
            .expect_err("a non-loopback address is refused");
        assert!(refused.contains("loopback"), "{refused}");
        assert!(!socket.exists(), "{listen} {upstream}: nothing was bound");
    }
}
