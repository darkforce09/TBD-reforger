//! The development compose project: the folder `cargo xtask db up`, `down`, `logs` and `seed` run
//! compose in, the file they name, and the line they echo.
//!
//! **Role:** resolves the folder and file of the local development stack from
//! [`crate::core::repository_layout::DEVELOPMENT_COMPOSE_FILE`], builds the compose argv, and
//! renders the shell line that argv stands for.
//!
//! **Position:** consumed by the compose lane of [`super`], which runs the argv in
//! [`ComposeProject::folder`], and by [`super::recipes::rendered_recipes`], which renders the
//! lane's lines through the same [`ComposeLine`] the runner echoes with.
//!
//! **Signals & state:** none; pure functions over a repository root, a working directory and the
//! optional folder override [`PROJECT_FOLDER_OVERRIDE`].
//!
//! **Invariants:** every compose call names its file with `-f` and runs in the folder that holds
//! it, so compose resolves the file's relative paths against that folder whichever folder xtask
//! started in, and the project name comes from the file's own `name:` (no `-p` is passed). The
//! echoed line is the shell form of the call: `cd <folder> && <runtime> compose -f <file> <args>`,
//! with a stdin file shown as the path that line's shell would open, and the runner opens exactly
//! that path resolved against the folder, so the line shown and the file read cannot disagree.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::core::repository_layout::DEVELOPMENT_COMPOSE_FILE;
use repository_layout::find_repository_root;

/// Names another folder holding a compose file under the development file's name. The self-test
/// points it at a throwaway project so a `db down` in a comparison never stops the shared
/// database. A relative value resolves against the working directory.
pub(crate) const PROJECT_FOLDER_OVERRIDE: &str = "TBD_MK_WEB";

/// The development compose file split into its folder and its file name.
pub(crate) fn compose_file_parts() -> (&'static str, &'static str) {
    DEVELOPMENT_COMPOSE_FILE
        .rsplit_once('/')
        .unwrap_or((".", DEVELOPMENT_COMPOSE_FILE))
}

/// The compose argv for `args`: the runtime's own argv, `compose`, `-f` and the file name, which
/// the folder of [`ComposeProject`] holds.
pub(crate) fn compose_argv(runtime: &[String], args: &[&str]) -> Vec<String> {
    let (_, file_name) = compose_file_parts();
    let mut argv = runtime.to_vec();
    argv.extend(["compose", "-f", file_name].map(str::to_string));
    argv.extend(args.iter().map(|argument| argument.to_string()));
    argv
}

/// How the echoed line names the project.
pub(crate) struct ComposeLine {
    /// The folder the line enters, as given: repository-relative for the checkout's own file.
    folder: String,
    /// The prefix that leads from that folder to the repository root: `../` once per level for
    /// the checkout's own folder, the root's absolute path for an overridden one.
    root_prefix: String,
}

impl ComposeLine {
    /// The line for the checkout's own development compose file.
    pub(crate) fn of_checkout() -> Self {
        let (folder, _) = compose_file_parts();
        Self {
            folder: folder.to_string(),
            root_prefix: "../".repeat(Path::new(folder).components().count()),
        }
    }

    /// The path the line shows for a stdin file named relative to the repository root.
    pub(crate) fn stdin_path(&self, from_root: &str) -> String {
        format!("{}{from_root}", self.root_prefix)
    }

    /// `cd <folder> && <runtime> compose -f <file> <args>[ < <stdin>]`, with `runtime_name` the
    /// runtime as a shell would name it (no bridge prefix).
    pub(crate) fn render(&self, runtime_name: &str, args: &[&str], stdin: Option<&str>) -> String {
        let (_, file_name) = compose_file_parts();
        let redirect = stdin
            .map(|from_root| format!(" < {}", self.stdin_path(from_root)))
            .unwrap_or_default();
        format!(
            "cd {} && {runtime_name} compose -f {file_name} {}{redirect}",
            self.folder,
            args.join(" ")
        )
    }
}

/// The compose project one command drives.
pub(crate) struct ComposeProject {
    /// The folder compose runs in, which holds the compose file.
    pub(crate) folder: PathBuf,
    /// How the echoed line names it.
    pub(crate) shown: ComposeLine,
}

impl ComposeProject {
    /// The checkout's own development compose project under `root`.
    pub(crate) fn in_checkout(root: &Path) -> Self {
        let (folder, _) = compose_file_parts();
        Self {
            folder: root.join(folder),
            shown: ComposeLine::of_checkout(),
        }
    }

    /// The project in the folder `value` names: absolute, or relative to `working_directory`.
    pub(crate) fn in_override_folder(value: &str, working_directory: &Path, root: &Path) -> Self {
        Self {
            folder: working_directory.join(value),
            shown: ComposeLine {
                folder: value.to_string(),
                root_prefix: format!("{}/", root.display()),
            },
        }
    }

    /// The project of this run: the [`PROJECT_FOLDER_OVERRIDE`] folder when it is set and not
    /// empty, else the checkout's own.
    pub(crate) fn resolve() -> Result<Self> {
        let root = find_repository_root()?;
        match std::env::var_os(PROJECT_FOLDER_OVERRIDE).filter(|value| !value.is_empty()) {
            Some(value) => Ok(Self::in_override_folder(
                &value.to_string_lossy(),
                &std::env::current_dir().context("read the working directory")?,
                &root,
            )),
            None => Ok(Self::in_checkout(&root)),
        }
    }

    /// The file the line's redirect opens: [`ComposeLine::stdin_path`] resolved against the folder.
    pub(crate) fn stdin_file(&self, from_root: &str) -> PathBuf {
        self.folder.join(self.shown.stdin_path(from_root))
    }
}

#[cfg(test)]
#[path = "tests/development_compose/tests.rs"]
mod tests;
