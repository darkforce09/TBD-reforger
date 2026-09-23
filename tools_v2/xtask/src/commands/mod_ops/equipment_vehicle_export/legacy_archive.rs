use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use serde_json::json;

use super::{files, validation};

const JOURNAL: &str = ".legacy-archive.json";

// The durable journal lets the next publisher restore old directories after interruption.
pub(super) fn archive(root: &Path, id: &str) -> Result<()> {
    let export_root = root.parent().context("missing export destination")?;
    let mut entries = Vec::new();
    for name in ["equipment", "vehicles"] {
        let source = files::child(export_root, name)?;
        if !source.exists() {
            continue;
        }
        ensure!(
            fs::symlink_metadata(&source)?.is_dir(),
            "legacy {name} is not a regular directory"
        );
        let destination = format!("legacy/{id}/{name}");
        ensure!(
            !files::child(root, &destination)?.exists(),
            "legacy archive already exists: {destination}"
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
            fs::create_dir_all(destination.parent().context("missing archive parent")?)?;
            // Persist every new directory name before moving the only legacy copy.
            for parent in destination.parent().unwrap().ancestors() {
                File::open(parent)?.sync_all()?;
                if parent == root {
                    break;
                }
            }
            fs::rename(source, &destination)?;
            File::open(destination.parent().unwrap())?.sync_all()?;
            File::open(export_root)?.sync_all()?;
        }
        Ok(())
    })();
    if result.is_err() {
        recover(root)?;
    }
    result
}

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
                ["equipment", "vehicles"].contains(&name),
                "invalid legacy archive source"
            );
            let relative = validation::text(entry, "destination");
            ensure!(
                relative == format!("legacy/{id}/{name}"),
                "invalid legacy archive destination"
            );
            let source = files::child(export_root, name)?;
            let destination = files::child(root, relative)?;
            if destination.exists() {
                ensure!(
                    !source.exists(),
                    "cannot recover archive: both old and archived {name} exist"
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
