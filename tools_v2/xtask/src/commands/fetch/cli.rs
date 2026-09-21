use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub(crate) enum FetchCmd {
    /// Mirror vanilla Enfusion SOURCE pages from arexplorer.
    /// `--help` is a filename target (MISS), matching the former bash script — not clap usage.
    #[command(name = "vanilla-source", disable_help_flag = true)]
    VanillaSource {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Mirror BI Script API Doxygen HTML.
    #[command(name = "vanilla-api", disable_help_flag = true)]
    VanillaApi {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}
