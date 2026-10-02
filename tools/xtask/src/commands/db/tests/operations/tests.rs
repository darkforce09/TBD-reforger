use super::*;
use clap::CommandFactory;

/// [`LANE_COMMANDS`] is a second copy of the clap enum's names, and it exists only because
/// `cargo xtask help` prints the NAMES as the database lane's index without a parse. Diff the
/// two so the copy cannot rot — a new `DbCmd` variant that never reaches the list would be
/// missing from that index.
#[test]
fn lane_commands_match_the_clap_enum() {
    #[derive(Parser)]
    struct Argv {
        #[command(subcommand)]
        _cmd: DbCmd,
    }
    let mut from_clap: Vec<String> = Argv::command()
        .get_subcommands()
        .map(|c| c.get_name().to_string())
        .collect();
    let mut declared: Vec<String> = LANE_COMMANDS.iter().map(|s| (*s).to_string()).collect();
    from_clap.sort();
    declared.sort();
    assert_eq!(declared, from_clap, "LANE_COMMANDS drifted from DbCmd");
}

/// `psql` reading a file on stdin carries on past a failed statement and exits 0 unless
/// `ON_ERROR_STOP` is set; without the flag `db seed` reports success over a seed that did not
/// apply (for example before the API's boot migrations have created the tables).
#[test]
fn seed_psql_arguments_stop_at_the_first_failed_statement() {
    let arguments = seed_psql_arguments();
    assert!(
        arguments
            .windows(2)
            .any(|pair| pair == ["-v", "ON_ERROR_STOP=1"]),
        "the seed psql argv lacks `-v ON_ERROR_STOP=1`: {arguments:?}"
    );
    assert_eq!(&arguments[..4], ["exec", "-T", "db", "psql"]);
}

/// `cargo xtask db --help` and every `cargo xtask db <command> --help` describe the lane's own
/// commands: each command has a description, and no page names a `make` target or a Makefile,
/// since the lane has none beside it.
#[test]
fn database_lane_help_describes_every_command_without_make_targets() {
    #[derive(Parser)]
    #[command(name = "db")]
    struct Argv {
        #[command(subcommand)]
        _cmd: DbCmd,
    }
    let mut lane = Argv::command();
    let mut pages = vec![("db".to_string(), lane.render_long_help().to_string())];
    for command in lane.get_subcommands_mut() {
        assert!(
            command.get_about().is_some(),
            "`db {}` has no description",
            command.get_name()
        );
        pages.push((
            command.get_name().to_string(),
            command.render_long_help().to_string(),
        ));
    }
    let offences: Vec<String> = pages
        .iter()
        .flat_map(|(name, page)| {
            page.lines()
                .filter(|line| {
                    line.contains("Makefile")
                        || line.split_whitespace().any(|word| {
                            word.trim_matches(|c: char| !c.is_ascii_alphanumeric()) == "make"
                        })
                })
                .map(move |line| format!("db {name}: {}", line.trim()))
        })
        .collect();
    assert!(
        offences.is_empty(),
        "help names retired make targets:\n{}",
        offences.join("\n")
    );
}
