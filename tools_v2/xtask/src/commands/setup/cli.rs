use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub(crate) enum SetupCmd {
    /// Prepare Arma Reforger dedicated-server profile files.
    #[command(name = "server-profile")]
    ServerProfile {
        /// Profile directory (default: $TBD_PROFILE or apps/mod/.local-test-profile)
        profile: Option<PathBuf>,
    },
    /// Symlink Steam Arma Reforger .gproj for Proton Workbench.
    #[command(name = "workbench")]
    Workbench,
    /// Flattened pak symlink farm for enfusion-mcp.
    #[command(name = "mcp-game-root")]
    McpGameRoot {
        /// Game install with addons/ (default: Steam Arma Reforger path)
        game: Option<PathBuf>,
        /// Output symlink farm (default: $HOME/.cache/enfusion-mcp-root)
        fake: Option<PathBuf>,
    },
    /// Local client addon staging symlink + Steam launch options.
    #[command(name = "client-addons")]
    ClientAddons,
}
