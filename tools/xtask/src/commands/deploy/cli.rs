use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub(crate) enum DeployCmd {
    /// Rsync + remote build/restart for the TBD website.
    #[command(name = "website", disable_help_flag = true)]
    Website {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Shared database backup/restore plumbing for the deploy drivers.
    #[command(subcommand)]
    Db(crate::commands::deploy::database_operations::DeployDbCmd),
    /// Staging deploy driver for the dedicated game server
    #[command(name = "staging", disable_help_flag = true)]
    Staging {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}
