use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub(crate) enum DeployCmd {
    /// Rsync + remote build/restart for the TBD website (T-858).
    #[command(name = "website", disable_help_flag = true)]
    Website {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Shared DB backup/restore plumbing (T-884 port of scripts/deploy/lib/db-common.sh).
    #[command(subcommand)]
    Db(crate::commands::deploy::database_operations::DeployDbCmd),
    /// T-853: staging deploy driver (port of scripts/mod/deploy-staging.sh)
    #[command(name = "staging", disable_help_flag = true)]
    Staging {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}
