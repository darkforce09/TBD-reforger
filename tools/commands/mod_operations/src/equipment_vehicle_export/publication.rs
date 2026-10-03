//! The locked, staged, hash-verified publication and the pointer swap.
//!
//! **Role:** publishes a validated generation: takes the publication lock, stages and verifies
//! every file, swaps the current pointer and writes the upload bundle.
//! **Position:** under [`crate::equipment_vehicle_export`]; `mod publish-equipment-vehicle-export`
//! calls it.
//! **Signals & state:** the publication lock file under the publication root for the run.
//! **Invariants:** a second publication never runs at the same time; the pointer moves only after
//! every file verifies; a failed run leaves the previous generation current.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::error::{Result, ResultExt, ensure};
use serde_json::json;

use super::{
    FileDigest, ValidationReport, files, unversioned_export_archive, upload_bundle, validate,
};

pub(super) fn publish(input: &Path) -> Result<()> {
    let input = input.canonicalize()?;
    let generations = input.parent().context("generation has no parent")?;
    ensure!(
        generations.file_name().is_some_and(|n| n == "generations"),
        "publish input must be inside equipment_vehicle_exports/generations"
    );
    let root = generations.parent().context("missing export root")?;
    let gameplay = root.file_name().is_some_and(|n| n == "gameplay")
        && root
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|n| n == "equipment_vehicle_exports");
    ensure!(
        root.file_name()
            .is_some_and(|n| n == "equipment_vehicle_exports")
            || gameplay,
        "unexpected export root"
    );
    let _lock = PublicationLock::acquire(root)?;
    if !gameplay {
        unversioned_export_archive::recover(root)?;
    }
    let (generation, _) = files::read(&input, "generation.json")?;
    ensure!(
        (generation["dataset_kind"] == "gameplay") == gameplay,
        "dataset kind does not match publication location"
    );
    let initial = validate(&input)?;
    ensure!(
        initial.valid,
        "export is not publishable:\n{}",
        initial.errors.join("\n")
    );
    ensure!(
        input.file_name().and_then(|v| v.to_str()) == Some(&initial.generation_id),
        "generation directory and ID disagree"
    );
    let published_root = files::child(root, "published")?;
    fs::create_dir_all(&published_root)?;
    let destination = files::child(&published_root, &initial.generation_id)?;
    let staging = files::child(root, &format!(".publish-{}", initial.generation_id))?;
    if staging.exists() {
        // This exact directory is private to the locked publisher and contains copies only.
        fs::remove_dir_all(&staging).context("remove interrupted publication staging")?;
    }
    fs::create_dir(&staging)?;
    let result = prepare_and_publish(root, &input, &staging, &destination, &initial);
    if staging.exists() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

fn prepare_and_publish(
    root: &Path,
    input: &Path,
    staging: &Path,
    destination: &Path,
    validated: &ValidationReport,
) -> Result<()> {
    let id = &validated.generation_id;
    let mut expected = validated.files.clone();
    for (relative, expected_digest) in &expected {
        let source = files::child(input, relative)?;
        let target = files::child(staging, relative)?;
        let bytes = fs::read(&source)?;
        ensure!(
            &files::digest(&bytes) == expected_digest,
            "source changed during publication: {relative}"
        );
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        durable_write(&target, &bytes)?;
    }
    // Exact hashes transfer every schema/graph check to the copied bytes without
    // parsing millions of unchanged facts a second time.
    verify_contents(staging, &expected, false)?;
    let gameplay = root.file_name().is_some_and(|n| n == "gameplay");
    if gameplay
        && !expected.contains_key("publication_receipt.json")
        && (!destination.exists() || destination.join("publication_receipt.json").exists())
    {
        let bytes = if destination.exists() {
            let (existing, _) = files::read(destination, "manifest.json")?;
            let bytes = fs::read(files::child(destination, "publication_receipt.json")?)?;
            let digest: FileDigest =
                serde_json::from_value(existing["files"]["publication_receipt.json"].clone())?;
            ensure!(files::digest(&bytes) == digest, "published receipt changed");
            super::gameplay_receipt::validate(
                staging,
                &serde_json::from_slice(&bytes)?,
                validated,
            )?;
            bytes
        } else {
            serde_json::to_vec(&super::gameplay_receipt::build(root, staging, validated)?)?
        };
        durable_write(&staging.join("publication_receipt.json"), &bytes)?;
        expected.insert("publication_receipt.json".into(), files::digest(&bytes));
    }
    let version = if gameplay { 1 } else { 2 };
    let mut manifest = json!({"schema_version": version, "generation_id": id, "resource_count": validated.resource_count, "files": expected});
    if gameplay {
        manifest["dataset_kind"] = json!("gameplay");
    }
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
    durable_write(&staging.join("manifest.json"), &manifest_bytes)?;
    sync_directories(staging)?;
    if destination.exists() {
        let (existing, _) = files::read(destination, "manifest.json")?;
        ensure!(
            existing == manifest,
            "generation ID already published with different content"
        );
        verify_contents(destination, &expected, true)?;
    } else {
        fs::rename(staging, destination)?;
        File::open(destination.parent().context("missing publication parent")?)?.sync_all()?;
    }
    if gameplay {
        upload_bundle::write(root, destination, id, &expected)?;
    }
    let mut pointer = json!({"schema_version": version, "generation_id": id, "directory": format!("published/{id}"), "manifest_sha256": files::digest(&manifest_bytes).sha256});
    if gameplay {
        pointer["dataset_kind"] = json!("gameplay");
    }
    let current = files::child(root, "current.json")?;
    let temp = files::child(root, ".current.json.tmp")?;
    if temp.exists() {
        fs::remove_file(&temp)?;
    }
    durable_write(&temp, &serde_json::to_vec_pretty(&pointer)?)?;
    let backup = files::child(root, ".previous-current.json")?;
    if backup.exists() {
        fs::remove_file(&backup)?;
    }
    if current.exists() {
        durable_write(&backup, &fs::read(&current)?)?;
    }
    if !current.exists() && !gameplay {
        unversioned_export_archive::archive(root, id)?;
    }
    if let Err(error) = fs::rename(&temp, &current) {
        if !gameplay {
            unversioned_export_archive::recover(root)?;
        }
        return Err(error.into());
    }
    if let Err(error) = File::open(root).and_then(|directory| directory.sync_all()) {
        if backup.exists() {
            fs::rename(&backup, &current)?;
        } else {
            fs::remove_file(&current)?;
        }
        if !gameplay {
            unversioned_export_archive::recover(root)?;
        }
        return Err(error.into());
    }
    if let Err(error) = if gameplay {
        Ok(())
    } else {
        unversioned_export_archive::recover(root)
    } {
        eprintln!("Publication succeeded; archive journal cleanup will retry: {error:#}");
    }
    if backup.exists() {
        let _ = fs::remove_file(&backup);
    }
    println!(
        "Published {id}: {} resources; current pointer {}",
        validated.resource_count,
        current.display()
    );
    Ok(())
}

fn verify_contents(
    root: &Path,
    expected: &BTreeMap<String, FileDigest>,
    has_manifest: bool,
) -> Result<()> {
    for (relative, digest) in expected {
        let file = files::child(root, relative)?;
        ensure!(
            &files::digest(&fs::read(file)?) == digest,
            "copied or published file changed: {relative}"
        );
    }
    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = entry?;
        ensure!(
            !entry.file_type().is_symlink(),
            "symlink in copied generation"
        );
        if entry.file_type().is_file() {
            let relative = entry
                .path()
                .strip_prefix(root)?
                .to_string_lossy()
                .replace('\\', "/");
            ensure!(
                expected.contains_key(&relative) || has_manifest && relative == "manifest.json",
                "unlisted copied generation file: {relative}"
            );
        }
    }
    Ok(())
}

fn durable_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn sync_directories(root: &Path) -> Result<()> {
    for entry in walkdir::WalkDir::new(root).contents_first(true) {
        let entry = entry?;
        if entry.file_type().is_dir() {
            File::open(entry.path())?.sync_all()?;
        }
    }
    Ok(())
}

struct PublicationLock {
    file: File,
}
impl PublicationLock {
    fn acquire(root: &Path) -> Result<Self> {
        // OS ownership is released on crash, so retries cannot inherit a stale lock file.
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(root.join(".publication.lock"))?;
        file.try_lock()
            .context("another export publication is running")?;
        Ok(Self { file })
    }
}
impl Drop for PublicationLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}
