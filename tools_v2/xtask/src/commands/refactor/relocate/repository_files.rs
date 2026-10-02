//! The tracked files of the checkout, which of them are text, and the crate each sits in.
//!
//! **Role:** lists the index with `git ls-files -z`, asks `git check-attr` which files carry a
//! non-text attribute, and reads a file's content as [`FileContent`]: editable text, or a binary
//! or Git LFS pointer that is moved but never edited. [`PathSet`] answers whether a path names a
//! tracked file or folder and which crate folder holds a path.
//!
//! **Position:** the snapshot every relocation mode starts from; the reference passes resolve
//! literals against its [`PathSet`], and the plan builder reads every file through it.
//!
//! **Signals & state:** none held beyond the snapshot, which is read-only once loaded.
//!
//! **Invariants:** a listing or attribute query that fails is a [`NotRun`], never an empty
//! snapshot; a file is text only when no attribute marks it `binary`, `-text` or `filter=lfs`, it
//! holds no NUL byte in its first 8000 bytes (git's own test), it is valid UTF-8 and it is not a
//! Git LFS pointer; a folder exists in a [`PathSet`] exactly when a file sits below it.

use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use verification_core::NotRun;
use verification_core::proc::Run;

use super::path_mapping::parent_folder;

/// The manifest file that makes a folder a crate.
pub(crate) const CRATE_MANIFEST: &str = "Cargo.toml";

/// The pathspec of every crate manifest at any depth.
const UNTRACKED_CRATE_MANIFESTS: &str = ":(glob)**/Cargo.toml";

/// The first line of every Git LFS pointer file.
const LARGE_FILE_POINTER_HEADER: &[u8] = b"version https://git-lfs.github.com/spec/";

/// How many leading bytes git inspects for a NUL when it guesses that a file is binary.
const BINARY_PROBE_LENGTH: usize = 8000;

/// How many paths one `git check-attr` call names on its command line: small enough for any
/// argument-length limit; the paths go on the command line rather than stdin, so the child's
/// output pipe is drained while it runs.
const ATTRIBUTE_QUERY_CHUNK: usize = 400;

/// A git query that takes longer than this is reported as a timeout.
const GIT_DEADLINE: Duration = Duration::from_secs(300);

/// Variables that would point git at another repository or index than the checkout's own.
const GIT_LOCATION_VARIABLES: [&str; 3] = ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE"];

/// A set of repository paths: files, and every folder that holds one.
#[derive(Clone, Debug, Default)]
pub(crate) struct PathSet {
    files: BTreeSet<String>,
    folders: BTreeSet<String>,
}

impl PathSet {
    /// The set of `files` and their folders.
    pub(crate) fn from_files<I: IntoIterator<Item = String>>(files: I) -> PathSet {
        let files: BTreeSet<String> = files.into_iter().collect();
        let mut folders = BTreeSet::new();
        for file in &files {
            let mut folder = parent_folder(file);
            while !folder.is_empty() && folders.insert(folder.to_string()) {
                folder = parent_folder(folder);
            }
        }
        PathSet { files, folders }
    }

    /// Every file, in path order.
    pub(crate) fn files(&self) -> impl Iterator<Item = &String> {
        self.files.iter()
    }

    /// Whether `path` names a file of the set.
    pub(crate) fn is_file(&self, path: &str) -> bool {
        self.files.contains(path)
    }

    /// Whether `path` names a file or a folder of the set; the root `""` always exists.
    pub(crate) fn contains(&self, path: &str) -> bool {
        path.is_empty() || self.files.contains(path) || self.folders.contains(path)
    }

    /// Every folder that holds a crate manifest, the repository root as `""`.
    pub(crate) fn crate_folders(&self) -> Vec<String> {
        self.files
            .iter()
            .filter(|file| file.rsplit('/').next() == Some(CRATE_MANIFEST))
            .map(|file| parent_folder(file).to_string())
            .collect()
    }

    /// The nearest folder at or above `folder` that holds a crate manifest; the repository root
    /// when none does.
    pub(crate) fn crate_folder_of(&self, folder: &str) -> String {
        let mut current = folder;
        loop {
            let manifest = if current.is_empty() {
                CRATE_MANIFEST.to_string()
            } else {
                format!("{current}/{CRATE_MANIFEST}")
            };
            if self.files.contains(&manifest) || current.is_empty() {
                return current.to_string();
            }
            current = parent_folder(current);
        }
    }
}

/// What a tracked file holds, as the rewrite passes see it.
#[derive(Debug)]
pub(crate) enum FileContent {
    /// Editable UTF-8 text.
    Text(String),
    /// A binary file, a symbolic link or a file git marks non-text: moved, never edited.
    NotText,
    /// A Git LFS pointer: moved, never edited.
    LargeFilePointer,
    /// Tracked but deleted from the working tree: nothing to read or rewrite.
    Missing,
}

/// The tracked files of one checkout.
#[derive(Debug)]
pub(crate) struct RepositorySnapshot {
    root: PathBuf,
    paths: PathSet,
    marked_not_text: HashSet<String>,
    untracked_crate_manifests: Vec<String>,
}

impl RepositorySnapshot {
    /// List the index of the checkout at `root` and the attributes of every file in it.
    pub(crate) fn load(root: &Path) -> Result<RepositorySnapshot, NotRun> {
        let listing = git(root, &["ls-files", "-z"])?;
        let files: Vec<String> = listing
            .split('\0')
            .filter(|path| !path.is_empty())
            .map(str::to_string)
            .collect();
        let mut attributes = String::new();
        for chunk in files.chunks(ATTRIBUTE_QUERY_CHUNK) {
            let mut arguments = vec!["check-attr", "-z", "binary", "text", "filter", "--"];
            arguments.extend(chunk.iter().map(String::as_str));
            attributes.push_str(&git(root, &arguments)?);
        }
        let untracked = git(
            root,
            &[
                "ls-files",
                "-z",
                "--others",
                "--exclude-standard",
                "--",
                UNTRACKED_CRATE_MANIFESTS,
            ],
        )?;
        Ok(RepositorySnapshot {
            root: root.to_path_buf(),
            marked_not_text: not_text_paths(&attributes),
            paths: PathSet::from_files(files),
            untracked_crate_manifests: untracked
                .split('\0')
                .filter(|path| !path.is_empty())
                .map(str::to_string)
                .collect(),
        })
    }

    /// The checkout root.
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    /// The tracked files and folders.
    pub(crate) fn paths(&self) -> &PathSet {
        &self.paths
    }

    /// The crate manifests on disk that git does not track yet nor ignore: crates being born,
    /// whose folders already anchor `CARGO_MANIFEST_DIR` after the moves.
    pub(crate) fn untracked_crate_manifests(&self) -> impl Iterator<Item = &String> {
        self.untracked_crate_manifests.iter()
    }

    /// Read a tracked file's content.
    pub(crate) fn read(&self, path: &str) -> Result<FileContent, NotRun> {
        let full = self.root.join(path);
        let metadata = match std::fs::symlink_metadata(&full) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(FileContent::Missing);
            }
            Err(source) => {
                return Err(NotRun::Unreadable {
                    path: full.clone(),
                    source,
                });
            }
        };
        if !metadata.is_file() {
            return Ok(FileContent::NotText);
        }
        let bytes = std::fs::read(&full).map_err(|source| NotRun::Unreadable {
            path: full.clone(),
            source,
        })?;
        Ok(classify_content(bytes, self.marked_not_text.contains(path)))
    }
}

/// Classify file bytes: an LFS pointer first, then git's attribute, NUL and UTF-8 tests.
pub(crate) fn classify_content(bytes: Vec<u8>, marked_not_text: bool) -> FileContent {
    if bytes.starts_with(LARGE_FILE_POINTER_HEADER) {
        return FileContent::LargeFilePointer;
    }
    if marked_not_text || bytes[..bytes.len().min(BINARY_PROBE_LENGTH)].contains(&0) {
        return FileContent::NotText;
    }
    match String::from_utf8(bytes) {
        Ok(text) => FileContent::Text(text),
        Err(_) => FileContent::NotText,
    }
}

/// The paths `git check-attr -z` output marks `binary`, `-text` or `filter=lfs`.
fn not_text_paths(output: &str) -> HashSet<String> {
    let fields: Vec<&str> = output.split('\0').collect();
    fields
        .chunks(3)
        .filter(|triple| triple.len() == 3)
        .filter(|triple| {
            matches!(
                (triple[1], triple[2]),
                ("binary", "set") | ("text", "unset") | ("filter", "lfs")
            )
        })
        .map(|triple| triple[0].to_string())
        .collect()
}

/// Run git at `root` with `arguments`, never against the repository or index the environment
/// names. Nothing is fed on stdin: a large stdin written before the output drains start can fill
/// the child's output pipe and stall both sides.
pub(crate) fn git(root: &Path, arguments: &[&str]) -> Result<String, NotRun> {
    let run = GIT_LOCATION_VARIABLES.iter().fold(
        Run::new("git")
            .args(arguments)
            .cwd(root)
            .timeout(GIT_DEADLINE),
        |run, variable| run.env_remove(*variable),
    );
    let output = run.output()?;
    if output.code != 0 {
        return Err(NotRun::ToolError {
            tool: format!("git {}", arguments.join(" ")),
            status: output.code,
            stderr: output.stderr,
        });
    }
    Ok(output.stdout)
}
