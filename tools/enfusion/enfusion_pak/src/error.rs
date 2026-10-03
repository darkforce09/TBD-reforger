//! Why a pak archive, a loose folder or a virtual path could not be read.
//!
//! **Role:** the crate's error type and its `Result` alias.
//! **Position:** returned by every fallible reader of the crate ([`crate::PakIndex`],
//! [`crate::PakSet`], [`crate::PakVfs`], [`crate::DirSource`], [`crate::LayeredSource`] and the
//! [`crate::AssetSource`] trait); the blueprint compiler, the world export and map raster pipelines
//! and the `enf` command line convert it with `?` and print it with its cause.
//! **Signals & state:** none; plain data.
//! **Invariants:** a variant with a cause prints that cause after `: ` and exposes no
//! [`std::error::Error::source`], so a caller printing an `anyhow` chain with `{:#}` shows each
//! cause exactly once; `Error::headline` is the message without its cause.

use std::array::TryFromSliceError;
use std::io;
use std::num::TryFromIntError;
use std::path::PathBuf;

/// Why a pak archive, a loose folder or a virtual path could not be read.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A file could not be opened or read.
    #[error("{}: {cause}", path.display())]
    File {
        /// The file asked for.
        path: PathBuf,
        /// The operating system's reason.
        cause: io::Error,
    },
    /// The folder of archives could not be listed.
    #[error("{}: {cause}", directory.display())]
    ReadDirectory {
        /// The folder asked for.
        directory: PathBuf,
        /// The operating system's reason.
        cause: io::Error,
    },
    /// An archive's chunks or directory could not be parsed.
    #[error("{}: {cause}", archive.display())]
    Archive {
        /// The archive file.
        archive: PathBuf,
        /// What the parse found.
        cause: Box<Error>,
    },
    /// An entry's stored bytes could not be read from its archive.
    #[error("{}: truncated read of {entry}: {cause}", archive.display())]
    TruncatedRead {
        /// The archive file.
        archive: PathBuf,
        /// The entry's path inside the archive.
        entry: String,
        /// What the read found.
        cause: Box<Error>,
    },
    /// A read of `len` bytes at `offset` runs past the end of the archive.
    #[error("EOF at {offset}+{len}")]
    EndOfFile {
        /// The absolute file offset of the read.
        offset: u64,
        /// The bytes asked for.
        len: u64,
    },
    /// A read length does not fit this machine's address space.
    #[error("pak read length exceeds address space: {cause}")]
    ReadLengthOverflow {
        /// The conversion's reason.
        cause: TryFromIntError,
    },
    /// The file does not start with the `FORM` magic and the `PAC1` type.
    #[error("not a FORM/PAC1 pak (magic mismatch)")]
    MagicMismatch,
    /// A chunk's declared size runs past the largest file offset.
    #[error("pak chunk size overflows")]
    ChunkSizeOverflow,
    /// The archive has no `DATA` chunk.
    #[error("pak has no DATA chunk")]
    MissingDataChunk,
    /// The archive has no `FILE` chunk.
    #[error("pak has no FILE chunk")]
    MissingFileChunk,
    /// The `FILE` directory tree does not start with a directory record.
    #[error("pak FILE chunk root entry is not a directory")]
    RootNotDirectory,
    /// Under the blueprint policy, an entry's stored bytes lie outside the `DATA` chunk.
    #[error(
        "pak entry {entry} at {offset} (+{stored_len}) lies outside the DATA chunk {data_start}..{data_end}"
    )]
    EntryOutsideData {
        /// The entry's path inside the archive.
        entry: String,
        /// The entry's absolute file offset.
        offset: u64,
        /// The entry's stored length in bytes.
        stored_len: u32,
        /// The first byte of the `DATA` chunk's payload.
        data_start: u64,
        /// The byte after the `DATA` chunk.
        data_end: u64,
    },
    /// The `FILE` directory tree ends inside a record.
    #[error("pak FILE tree truncated at {cursor} (+{len})")]
    DirectoryTruncated {
        /// The tree offset of the record being read.
        cursor: usize,
        /// The bytes the record still needed.
        len: usize,
    },
    /// A fixed-width record field had another width.
    #[error(transparent)]
    FieldWidth(#[from] TryFromSliceError),
    /// Under the blueprint policy, an entry failed to inflate as zlib.
    #[error("{entry}: zlib inflate failed: {cause}")]
    ZlibInflate {
        /// The entry's path inside the archive.
        entry: String,
        /// The decoder's reason.
        cause: io::Error,
    },
    /// Under the blueprint policy, an entry inflated to another length than its directory record.
    #[error("{entry}: inflated {inflated} bytes, directory says {declared}")]
    InflatedLength {
        /// The entry's path inside the archive.
        entry: String,
        /// The bytes the decoder produced.
        inflated: usize,
        /// The decompressed length the directory records.
        declared: u32,
    },
    /// Under the world policy, an entry inflated neither as zlib nor as raw deflate.
    #[error(
        "inflate failed as zlib and raw deflate: {entry} (clen={stored} dlen={declared} head={head})"
    )]
    InflateFailed {
        /// The entry's path inside the archive.
        entry: String,
        /// The stored bytes handed to the decoders.
        stored: usize,
        /// The decompressed length the directory records.
        declared: u32,
        /// The first twelve stored bytes in hex, space-separated.
        head: String,
    },
    /// A loose folder holds no file at the path, exactly or case-insensitively.
    #[error("{path}: not under {}", root.display())]
    NotUnderRoot {
        /// The virtual path asked for.
        path: String,
        /// The loose folder.
        root: PathBuf,
    },
    /// No layer of a layered source holds the path.
    #[error("{path}: not found in any asset source")]
    NotInAnySource {
        /// The virtual path asked for.
        path: String,
    },
    /// The folder holds no `.pak` file.
    #[error("no .pak files under {}", directory.display())]
    NoPakFiles {
        /// The folder searched.
        directory: PathBuf,
    },
    /// No archive of a pak set holds the path.
    #[error("{path}: not in any pak")]
    NotInAnyPak {
        /// The virtual path asked for.
        path: String,
    },
    /// The game folder has no `addons/` folder.
    #[error("no addons/ under {}", game.display())]
    NoAddons {
        /// The game folder.
        game: PathBuf,
    },
    /// The default game folder could not be opened as a virtual file system.
    #[error("No pak VFS ({reason}) — set ENFUSION_GAME_PATH")]
    NoPakVfs {
        /// The headline of the failed open.
        reason: String,
    },
    /// The world tooling's virtual file system holds no file at the path.
    #[error("File not found in pak: {path}")]
    FileNotInPak {
        /// The virtual path asked for.
        path: String,
    },
    /// Seeking or reading an archive failed.
    #[error(transparent)]
    Io(#[from] io::Error),
}

impl Error {
    /// The message without its cause: the file, folder or step that failed, which the world
    /// tooling's diagnostics name when they skip an archive or refuse the default game folder.
    #[must_use]
    pub(crate) fn headline(&self) -> String {
        match self {
            Self::File { path, .. } => path.display().to_string(),
            Self::ReadDirectory { directory, .. } => directory.display().to_string(),
            Self::Archive { archive, .. } => archive.display().to_string(),
            Self::TruncatedRead { archive, entry, .. } => {
                format!("{}: truncated read of {entry}", archive.display())
            }
            Self::ReadLengthOverflow { .. } => "pak read length exceeds address space".to_string(),
            Self::ZlibInflate { entry, .. } => format!("{entry}: zlib inflate failed"),
            other => other.to_string(),
        }
    }
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
