use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub(crate) enum SetupCmd {
    /// Prepare Arma Reforger dedicated-server profile files (T-861).
    #[command(name = "server-profile")]
    ServerProfile {
        /// Profile directory (default: $TBD_PROFILE or apps/mod/.local-test-profile)
        profile: Option<PathBuf>,
    },
    /// Symlink Steam Arma Reforger .gproj for Proton Workbench (T-875).
    #[command(name = "workbench")]
    Workbench,
    /// Flattened pak symlink farm for enfusion-mcp (T-876).
    #[command(name = "mcp-game-root")]
    McpGameRoot {
        /// Game install with addons/ (default: Steam Arma Reforger path)
        game: Option<PathBuf>,
        /// Output symlink farm (default: $HOME/.cache/enfusion-mcp-root)
        fake: Option<PathBuf>,
    },
    /// Local client addon staging symlink + Steam launch options (T-878).
    #[command(name = "client-addons")]
    ClientAddons,
}
