//! Journalled archive of the unversioned export folders at first publication.
//!
//! **Role:** moves the unversioned `equipment/` and `vehicles/` export folders that sit beside
//! the publication root into `unversioned_exports/<generation id>/` when the first generation
//! publishes, and restores them after an interrupted publication.
//!
//! **Position:** called by [`super::publication`] under its publication lock: [`recover`] runs
//! before validation and after a failed pointer swap, [`archive`] runs once the first generation
//! is sealed and before `current.json` first names it.
//! Only this module reads or writes the archive journal.
//!
//! **Signals & state:** none in memory; the durable state is the journal file
//! `.unversioned-export-archive.json` in the publication root, present only between an archive
//! start and the commit (or the recovery) that follows it.
//!
//! **Invariants:** the journal is written and synced before the first folder moves, and every
//! new directory name is synced before a rename, so an interruption always leaves either the
//! original folder or a journalled archived copy; an uncommitted generation's archive is moved
//! back in reverse order, and a committed one keeps its archive.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::error::{Result, ResultExt, ensure};
use serde_json::json;

use super::{files, validation};

/// The archive journal inside the publication root.
const JOURNAL: &str = ".unversioned-export-archive.json";

/// The folder under the publication root that receives each generation's archived folders.
const ARCHIVE_FOLDER: &str = "unversioned_exports";

/// The unversioned export folders beside the publication root.
const UNVERSIONED_FOLDERS: [&str; 2] = ["equipment", "vehicles"];

fn archive_destination(id: &str, name: &str) -> String {
    format!("{ARCHIVE_FOLDER}/{id}/{name}")
}

/// Moves every unversioned export folder beside `root` into the archive of generation `id`,
/// journalling the moves first; on failure, restores what already moved.
pub(super) fn archive(root: &Path, id: &str) -> Result<()> {
    let export_root = root.parent().context("missing export destination")?;
    let mut entries = Vec::new();
    for name in UNVERSIONED_FOLDERS {
        let source = files::child(export_root, name)?;
        if !source.exists() {
            continue;
        }
        ensure!(
            fs::symlink_metadata(&source)?.is_dir(),
            "unversioned export {name} is not a regular directory"
        );
        let destination = archive_destination(id, name);
        ensure!(
            !files::child(root, &destination)?.exists(),
            "unversioned export archive already exists: {destination}"
        );
        entries.push(json!({"source":name,"destination":destination}));
    }
    let journal = json!({"generation_id":id,"entries":entries});
    let path = files::child(root, JOURNAL)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    file.write_all(&serde_json::to_vec(&journal)?)?;
    file.sync_all()?;
    File::open(root)?.sync_all()?;
    let result = (|| -> Result<()> {
        for entry in &entries {
            let source = files::child(export_root, validation::text(entry, "source"))?;
            let destination = files::child(root, validation::text(entry, "destination"))?;
            let destination_parent = destination.parent().context("missing archive parent")?;
            fs::create_dir_all(destination_parent)?;
            // Persist every new directory name before moving the only copy of the folder.
            for parent in destination_parent.ancestors() {
                File::open(parent)?.sync_all()?;
                if parent == root {
                    break;
                }
            }
            fs::rename(source, &destination)?;
            File::open(destination_parent)?.sync_all()?;
            File::open(export_root)?.sync_all()?;
        }
        Ok(())
    })();
    if result.is_err() {
        recover(root)?;
    }
    result
}

/// Settles a journal left in `root`: an uncommitted generation's archived folders move back
/// beside `root`, a committed generation keeps its archive, and the journal is removed.
pub(super) fn recover(root: &Path) -> Result<()> {
    if !root.join(JOURNAL).exists() {
        return Ok(());
    }
    let (journal, _) = files::read(root, JOURNAL)?;
    let id = validation::text(&journal, "generation_id");
    ensure!(
        !id.is_empty()
            && id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-'),
        "invalid archive journal generation"
    );
    let current = root.join("current.json");
    let committed = current.exists() && files::read(root, "current.json")?.0["generation_id"] == id;
    if !committed {
        let export_root = root.parent().context("missing export root")?;
        for entry in validation::array(&journal["entries"]).iter().rev() {
            let name = validation::text(entry, "source");
            ensure!(
                UNVERSIONED_FOLDERS.contains(&name),
                "invalid unversioned export archive source"
            );
            let relative = validation::text(entry, "destination");
            ensure!(
                relative == archive_destination(id, name),
                "invalid unversioned export archive destination"
            );
            let source = files::child(export_root, name)?;
            let destination = files::child(root, relative)?;
            if destination.exists() {
                ensure!(
                    !source.exists(),
                    "cannot recover archive: both the unversioned and the archived {name} exist"
                );
                fs::rename(&destination, source)?;
                File::open(destination.parent().context("missing archive parent")?)?.sync_all()?;
            }
        }
        File::open(export_root)?.sync_all()?;
    }
    fs::remove_file(files::child(root, JOURNAL)?)?;
    File::open(root)?.sync_all()?;
    Ok(())
}
