//! Why an oracle command, an index build or a vanilla page mirror stopped.
//!
//! **Role:** the crate's error type and its `Result` alias.
//! **Position:** returned by every fallible function of the crate; [`crate::run_command_line`]
//! prints it after `enf: ` with its cause chain and exits 2, and xtask's `fetch` command converts
//! it with `?` and prints it with its cause.
//! **Signals & state:** none; plain data.
//! **Invariants:** a variant with a cause prints that cause after `: ` and exposes no
//! [`std::error::Error::source`], so a printed chain names each cause exactly once; the errors of
//! the crates it calls pass through unchanged (`transparent`), with their own sources.

use std::io;
use std::path::PathBuf;

use verification_core::NotRun;

/// Why an oracle command, an index build or a vanilla page mirror stopped.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A pak archive of the game could not be read.
    #[error(transparent)]
    Pak(#[from] enfusion_pak::Error),
    /// No checkout root was found from the working directory.
    #[error(transparent)]
    RepositoryRoot(#[from] repository_layout::Error),
    /// A filesystem read or write failed.
    #[error(transparent)]
    Io(#[from] io::Error),
    /// A write would replace a committed index or manifest with an empty one.
    #[error("refusing empty write ({context}): {detail}")]
    RefusedEmptyWrite {
        /// The output being written.
        context: String,
        /// Why the result counts as empty.
        detail: String,
    },
    /// An index, a document or an output folder could not be read.
    #[error("reading {}: {cause}", path.display())]
    Read {
        /// The file or folder asked for.
        path: PathBuf,
        /// The operating system's reason.
        cause: io::Error,
    },
    /// The cached pages a parser reads are absent; the named fetch command fills them.
    #[error("reading {} — run `{fetch_command}` first: {cause}", path.display())]
    PageCacheMissing {
        /// The cache file or folder asked for.
        path: PathBuf,
        /// The `cargo xtask fetch` command that fills the cache.
        fetch_command: &'static str,
        /// The operating system's reason.
        cause: io::Error,
    },
    /// A pak archive could not be opened for carving.
    #[error("opening {}: {cause}", path.display())]
    OpenArchive {
        /// The archive file.
        path: PathBuf,
        /// The operating system's reason.
        cause: io::Error,
    },
    /// The install's `addons/data` and `addons/core` folders hold no `.pak` file.
    #[error("no .pak files under {} — is this an Arma Reforger install?", addons.display())]
    NoArchives {
        /// The install's `addons/` folder.
        addons: PathBuf,
    },
    /// A previous carve is still in the output folder.
    #[error("{} already holds a previous carve; clear it first", carved_root.display())]
    PreviousCarve {
        /// The folder holding the carved blobs.
        carved_root: PathBuf,
    },
    /// The capability verdict table could not be read.
    #[error("reading verdict table {}: {cause}", path.display())]
    ReadVerdictTable {
        /// The verdict table.
        path: PathBuf,
        /// The operating system's reason.
        cause: io::Error,
    },
    /// A verdict table row has fewer than three columns.
    #[error("{}:{line}: need prefix\\tcapability\\tverdict[\\tnote]", path.display())]
    VerdictRowShape {
        /// The verdict table.
        path: PathBuf,
        /// The one-based line of the row.
        line: usize,
    },
    /// A verdict table row names a verdict outside the legal list.
    #[error("{}:{line}: unknown verdict '{verdict}' (expected one of {expected})", path.display())]
    UnknownVerdict {
        /// The verdict table.
        path: PathBuf,
        /// The one-based line of the row.
        line: usize,
        /// The verdict the row names.
        verdict: String,
        /// The legal verdicts, comma-separated.
        expected: String,
    },
    /// The working directory could not be read.
    #[error("reading the working directory: {cause}")]
    WorkingDirectory {
        /// The operating system's reason.
        cause: io::Error,
    },
    /// The checkout has no references folder, so no lane output may be written.
    #[error(
        "{} is missing; a reference lane is written only inside it (its README.md is tracked, so run from a checkout of this repository)",
        references.display()
    )]
    ReferencesMissing {
        /// The references folder expected.
        references: PathBuf,
    },
    /// An output folder spelled with `..`.
    #[error("refusing output folder {}: `..` is not accepted", resolved.display())]
    OutputClimbs {
        /// The output folder, resolved against the working directory.
        resolved: PathBuf,
    },
    /// An output folder outside the references folder, or the references folder itself.
    #[error(
        "refusing output folder {}: reference lanes are written only inside {}",
        resolved.display(),
        references.display()
    )]
    OutputOutsideReferences {
        /// The output folder, resolved against the working directory.
        resolved: PathBuf,
        /// The references folder.
        references: PathBuf,
    },
    /// The output folder holds a previous output and `--replace` was not given.
    #[error(
        "{} already holds a previous output; pass --replace to remove it and write a fresh one",
        folder.display()
    )]
    PreviousOutput {
        /// The output folder.
        folder: PathBuf,
    },
    /// A previous output could not be removed.
    #[error("removing {}: {cause}", folder.display())]
    RemoveOutput {
        /// The output folder.
        folder: PathBuf,
        /// The operating system's reason.
        cause: io::Error,
    },
    /// `enf index` was given a lane other than `crf` or `vanilla`.
    #[error("unknown lane '{lane}' (expected crf|vanilla)")]
    UnknownLane {
        /// The lane given.
        lane: String,
    },
    /// The source tree `enf index` was pointed at does not exist.
    #[error(
        "source root {} does not exist.\n(the reference lanes in {} are gitignored — fill them as its README.md describes before reindexing)",
        root.display(),
        repository_layout::REFERENCES_DIR
    )]
    SourceRootMissing {
        /// The source tree given.
        root: PathBuf,
    },
    /// The folder `enf carve` was given has no `addons/` folder.
    #[error("{} has no addons/ — expected an Arma Reforger install", game.display())]
    NotAGameInstall {
        /// The folder given.
        game: PathBuf,
    },
    /// The checkout has no references folder, so the vanilla page cache may not be created.
    #[error(
        "{} is missing; the vanilla lane is written only inside it (its README.md is tracked, so run from a checkout of this repository)",
        references.display()
    )]
    VanillaLaneMissing {
        /// The references folder expected.
        references: PathBuf,
    },
    /// The page cache folder could not be created.
    #[error("mkdir -p {}: {cause}", cache.display())]
    CreateCache {
        /// The cache folder.
        cache: PathBuf,
        /// The operating system's reason.
        cause: io::Error,
    },
    /// A class list or the page map could not be opened.
    #[error("open {}: {cause}", path.display())]
    Open {
        /// The file asked for.
        path: PathBuf,
        /// The operating system's reason.
        cause: io::Error,
    },
    /// No `curl` is on `PATH` for a download that must succeed.
    #[error("curl: command not found")]
    CurlNotFound,
    /// `curl` could not be run, or died on a signal.
    #[error("curl: {cause:?}")]
    CurlNotRun {
        /// The runner's reason.
        cause: NotRun,
    },
    /// `curl` exited non-zero on a download that must succeed.
    #[error("curl exited {status} fetching {url}")]
    CurlFailed {
        /// The exit code.
        status: i32,
        /// The URL asked for.
        url: String,
    },
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
