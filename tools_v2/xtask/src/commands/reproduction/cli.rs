use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
#[allow(clippy::enum_variant_names)] // mission-id / mission-version-body / mission-upload
pub(crate) enum ReproCmd {
    /// stdin JSON → print .id
    #[command(name = "mission-id")]
    MissionId,
    /// Write padded mission-version POST body
    #[command(name = "mission-version-body")]
    MissionVersionBody {
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        mb: u64,
        #[arg(long)]
        semver: String,
    },
    /// Orchestrate mission-version upload repro (ex mission-version-upload-repro.sh)
    #[command(name = "mission-upload")]
    MissionUpload,
}
