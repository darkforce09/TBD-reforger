use super::*;
use clap::CommandFactory;

/// [`LANE_COMMANDS`] is a second copy of the clap enum's names, and it exists only because
/// `schema_gates`' gate 7 needs the NAMES without a parse. Diff the two so the copy cannot
/// rot — a new `DbCmd` variant that never reaches the list would make a correct spec citation
/// read as a typo.
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
