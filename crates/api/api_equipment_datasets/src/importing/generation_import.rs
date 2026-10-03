//! Validate copied bytes, build a complete index and switch generations atomically.
use super::super::{
    EquipmentDataService, INDEX_VERSION, index,
    source::manifest::{self, Manifest, PublishedPointer},
};
use super::publication;
use crate::error::{Error, Required, Result, ensure};
use api_identifiers::EquipmentGenerationId;
use serde_json::Value;
use std::fs::{self, File, OpenOptions};
use std::path::Path;
use std::sync::Arc;

/// Activates the generation the data directory's `current.json` names, or records that the
/// service is unconfigured or still waiting for a first import.
pub async fn initialize(service: &EquipmentDataService) -> Result<()> {
    if service.data_dir.as_os_str().is_empty() {
        service.progress(
            "unconfigured",
            0,
            0,
            Some("Equipment data directory is not configured".into()),
        );
        return Ok(());
    }
    let path = service.data_dir.join("current.json");
    if path.exists() {
        let pointer: Value =
            serde_json::from_slice(&manifest::read(&service.data_dir, "current.json")?)?;
        service
            .activate(&EquipmentGenerationId::from(
                pointer["generation_id"]
                    .as_str()
                    .required("invalid viewer pointer")?,
            ))
            .await?;
    } else {
        service.progress(
            "waiting",
            0,
            0,
            Some("Waiting for a published equipment export".into()),
        );
    }
    Ok(())
}

/// Imports the generation the export source's `current.json` publishes when it is not current
/// yet: under the data directory's import lock it copies and verifies every file, builds and
/// seals the index, then activates the generation; the current one stays served until then.
pub async fn poll(service: Arc<EquipmentDataService>) -> Result<()> {
    let Some(source) = &service.source_dir else {
        return Ok(());
    };
    ensure!(
        !service.data_dir.as_os_str().is_empty(),
        "equipment data directory is not configured"
    );
    let source = source
        .canonicalize()
        .map_err(Error::ExportSourceUnavailable)?;
    if !source.join("current.json").exists() {
        service.progress(
            "waiting",
            0,
            0,
            Some("Waiting for a successfully published equipment export".into()),
        );
        return Ok(());
    }
    let pointer: PublishedPointer =
        serde_json::from_slice(&manifest::read(&source, "current.json")?)?;
    ensure!(
        manifest::supported(&pointer.dataset_kind, pointer.schema_version),
        "unsupported export version"
    );
    ensure!(
        pointer.directory == format!("published/{}", pointer.generation_id),
        "pointer is not a published generation"
    );
    if service.current_id().as_ref() == Some(&pointer.generation_id) {
        let current = service
            .current
            .read()
            .expect("current lock")
            .clone()
            .expect("current dataset");
        let bytes = manifest::read(&current.root, "manifest.json")?;
        ensure!(
            manifest::digest(&bytes) == pointer.manifest_sha256,
            "published generation identity changed"
        );
        service.progress("ready", 0, 0, None);
        return Ok(());
    }
    fs::create_dir_all(&service.data_dir)?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(service.data_dir.join(".import.lock"))?;
    lock.try_lock().map_err(Error::ImportActive)?;
    let published = manifest::safe_child(&source, &pointer.directory)?;
    let manifest_bytes = manifest::read(&published, "manifest.json")?;
    ensure!(
        manifest::digest(&manifest_bytes) == pointer.manifest_sha256,
        "published manifest hash mismatch"
    );
    let manifest: Manifest = serde_json::from_slice(&manifest_bytes)?;
    ensure!(
        manifest::supported(&manifest.dataset_kind, manifest.schema_version)
            && manifest.dataset_kind == pointer.dataset_kind
            && manifest.generation_id == pointer.generation_id,
        "published manifest identity mismatch"
    );
    let staging = service.data_dir.join(".staging");
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir(&staging)?;
    let staged_export = staging.join("export");
    let staged_index = staging.join("index");
    fs::create_dir(&staged_export)?;
    fs::create_dir(&staged_index)?;
    let destination = manifest::safe_child(
        &service.data_dir,
        &format!("generations/{}/export", pointer.generation_id),
    )?;
    let index_destination = manifest::safe_child(
        &service.data_dir,
        &format!("indexes/{}/{INDEX_VERSION}", pointer.generation_id),
    )?;
    let copy_manifest = manifest.clone();
    let copy_source = published.clone();
    let copy_target = staged_export.clone();
    let progress = service.clone();
    tokio::task::spawn_blocking(move || {
        copy(
            &copy_source,
            &copy_target,
            &copy_manifest,
            &manifest_bytes,
            &progress,
        )
    })
    .await??;
    let summary = index::writer::build(
        &staged_export,
        &staged_index.join("catalog.sqlite"),
        &manifest,
        &service,
    )
    .await?;
    let database = staged_index.join("catalog.sqlite");
    let index_hash =
        tokio::task::spawn_blocking(move || publication::file_digest(&database)).await??;
    let seal = serde_json::json!({"generation_id":pointer.generation_id,"index_version":INDEX_VERSION,"manifest_sha256":pointer.manifest_sha256,"index_sha256":index_hash,"overview":summary});
    publication::write(
        &staged_index.join("manifest.json"),
        &serde_json::to_vec_pretty(&seal)?,
    )?;
    publication::sync_tree(&staging)?;
    if destination.exists() {
        let existing = destination.clone();
        let expected = manifest.clone();
        tokio::task::spawn_blocking(move || verify_copy(&existing, &expected)).await??;
        fs::remove_dir_all(&staged_export)?;
    } else {
        publication::finish_directory(&staged_export, &destination)?;
    }
    if index_destination.exists() {
        // A completed identical generation can remain from a crash before pointer activation.
        let existing: Value =
            serde_json::from_slice(&manifest::read(&index_destination, "manifest.json")?)?;
        ensure!(
            existing["manifest_sha256"] == seal["manifest_sha256"],
            "existing index belongs to different source bytes"
        );
        fs::remove_dir_all(&staged_index)?;
    } else {
        publication::finish_directory(&staged_index, &index_destination)?;
    }
    publication::sync_tree(&service.data_dir.join("generations"))?;
    publication::sync_tree(&service.data_dir.join("indexes"))?;
    let candidate =
        super::super::service_state::load_dataset(&service.data_dir, &pointer.generation_id)
            .await?;
    #[cfg(test)]
    service.test_event("activation")?;
    publication::activate(
        &service.data_dir,
        &pointer.generation_id,
        &pointer.manifest_sha256,
    )?;
    *service.current.write().expect("current lock") = Some(Arc::new(candidate));
    service.progress(
        "ready",
        manifest.resource_count,
        manifest.resource_count,
        None,
    );
    let _ = fs::remove_dir(&staging);
    drop(lock);
    Ok(())
}

fn copy(
    source: &Path,
    target: &Path,
    manifest: &Manifest,
    manifest_bytes: &[u8],
    service: &EquipmentDataService,
) -> Result<()> {
    let mut expected: std::collections::BTreeSet<_> = manifest.files.keys().cloned().collect();
    expected.insert("manifest.json".into());
    ensure!(
        manifest::inventory(source)? == expected,
        "published bundle contains unexpected or missing files"
    );
    for (ordinal, (relative, digest)) in manifest.files.iter().enumerate() {
        let bytes = manifest::read(source, relative)?;
        ensure!(
            bytes.len() as u64 == digest.bytes && manifest::digest(&bytes) == digest.sha256,
            "published document hash mismatch: {relative}"
        );
        let destination = manifest::safe_child(target, relative)?;
        fs::create_dir_all(destination.parent().expect("document parent"))?;
        publication::write(&destination, &bytes)?;
        #[cfg(test)]
        service.test_event("copying")?;
        service.progress(
            "copying",
            ordinal as u64 + 1,
            manifest.files.len() as u64,
            None,
        );
    }
    publication::write(&target.join("manifest.json"), manifest_bytes)?;
    verify_copy(target, manifest)?;
    File::open(target)?.sync_all()?;
    Ok(())
}

/// Checks that the files under `root` are exactly those `manifest` lists, plus `manifest.json`,
/// each with the size and digest it records.
pub fn verify_copy(root: &Path, manifest: &Manifest) -> Result<()> {
    for (relative, expected) in &manifest.files {
        let path = manifest::safe_child(root, relative)?;
        ensure!(
            fs::metadata(&path)?.len() == expected.bytes
                && publication::file_digest(&path)? == expected.sha256,
            "copied document hash mismatch: {relative}"
        );
    }
    let mut expected: std::collections::BTreeSet<_> = manifest.files.keys().cloned().collect();
    expected.insert("manifest.json".into());
    ensure!(
        manifest::inventory(root)? == expected,
        "copied bundle file inventory differs"
    );
    Ok(())
}
