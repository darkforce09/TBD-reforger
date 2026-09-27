//! Generation-pinned query state and bounded source-document residency.
use super::{
    INDEX_VERSION,
    source::manifest::{self, Manifest},
};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tokio::sync::{Mutex, Semaphore};

pub struct Dataset {
    pub native_types: Value,
    pub definitions: Arc<super::source::gameplay::FieldDefinitions>,
    pub generation_id: String,
    pub root: PathBuf,
    pub pool: SqlitePool,
    pub manifest: Manifest,
    pub overview: Value,
}

#[derive(Clone, Default)]
pub struct ImportProgress {
    pub stage: String,
    pub completed: u64,
    pub total: u64,
    pub message: Option<String>,
}

type CachedDocument = (String, Arc<Vec<u8>>);

#[cfg(test)]
type ImportTestHook = Box<dyn FnMut(&str) -> Result<()> + Send>;

pub struct EquipmentDataService {
    #[cfg(test)]
    pub test_hook: std::sync::Mutex<Option<ImportTestHook>>,
    pub data_dir: PathBuf,
    pub source_dir: Option<PathBuf>,
    pub current: RwLock<Option<Arc<Dataset>>>,
    pub progress_state: RwLock<ImportProgress>,
    datasets: Mutex<BTreeMap<String, Arc<Dataset>>>,
    cache: Mutex<VecDeque<CachedDocument>>,
    pub readers: Arc<Semaphore>,
}

impl EquipmentDataService {
    pub fn new(data_dir: impl Into<PathBuf>, source: Option<PathBuf>) -> Self {
        Self {
            #[cfg(test)]
            test_hook: std::sync::Mutex::new(None),
            data_dir: data_dir.into(),
            source_dir: source,
            current: RwLock::new(None),
            progress_state: RwLock::new(ImportProgress {
                stage: "starting".into(),
                ..Default::default()
            }),
            datasets: Mutex::new(BTreeMap::new()),
            cache: Mutex::new(VecDeque::new()),
            readers: Arc::new(Semaphore::new(2)),
        }
    }

    #[cfg(test)]
    pub fn test_event(&self, stage: &str) -> Result<()> {
        if let Some(hook) = self.test_hook.lock().unwrap().as_mut() {
            hook(stage)?;
        }
        Ok(())
    }

    pub fn progress(&self, stage: &str, completed: u64, total: u64, message: Option<String>) {
        *self.progress_state.write().expect("progress lock") = ImportProgress {
            stage: stage.into(),
            completed,
            total,
            message,
        };
    }

    pub fn current_id(&self) -> Option<String> {
        self.current
            .read()
            .expect("dataset lock")
            .as_ref()
            .map(|d| d.generation_id.clone())
    }

    pub async fn dataset(&self, id: &str) -> Result<Arc<Dataset>> {
        let id = if id == "latest" {
            self.current_id().context("no dataset imported")?
        } else {
            id.to_owned()
        };
        if let Some(current) = self
            .current
            .read()
            .expect("dataset lock")
            .as_ref()
            .filter(|d| d.generation_id == id)
        {
            return Ok(current.clone());
        }
        let mut datasets = self.datasets.lock().await;
        if let Some(dataset) = datasets.get(&id) {
            return Ok(dataset.clone());
        }
        let loaded = Arc::new(load_dataset(&self.data_dir, &id).await?);
        if datasets.len() >= 4 {
            datasets.pop_first();
        }
        datasets.insert(id, loaded.clone());
        Ok(loaded)
    }

    pub async fn activate(&self, id: &str) -> Result<()> {
        let dataset = Arc::new(load_dataset(&self.data_dir, id).await?);
        *self.current.write().expect("dataset lock") = Some(dataset);
        self.progress("ready", 0, 0, None);
        Ok(())
    }

    pub async fn document(&self, dataset: &Dataset, relative: &str) -> Result<Arc<Vec<u8>>> {
        let expected = dataset
            .manifest
            .files
            .get(relative)
            .context("document is not in manifest")?
            .clone();
        let key = format!("{}:{}:{}", dataset.generation_id, relative, expected.sha256);
        {
            let mut cache = self.cache.lock().await;
            if let Some(position) = cache.iter().position(|(k, _)| k == &key) {
                let entry = cache.remove(position).expect("cached document");
                let result = entry.1.clone();
                cache.push_back(entry);
                return Ok(result);
            }
        }
        let root = dataset.root.clone();
        let relative = relative.to_owned();
        let bytes = tokio::task::spawn_blocking(move || -> Result<Vec<u8>> {
            let bytes = manifest::read(&root, &relative)?;
            ensure!(
                bytes.len() as u64 == expected.bytes && manifest::digest(&bytes) == expected.sha256,
                "stored document does not match manifest"
            );
            Ok(bytes)
        })
        .await??;
        let bytes = Arc::new(bytes);
        let mut cache = self.cache.lock().await;
        if bytes.len() <= 64 * 1024 * 1024 {
            while cache.iter().map(|(_, b)| b.len()).sum::<usize>() + bytes.len() > 64 * 1024 * 1024
            {
                cache.pop_front();
            }
            cache.push_back((key, bytes.clone()));
        }
        Ok(bytes)
    }

    pub async fn status(&self, offset: usize) -> Result<Value> {
        let progress = self.progress_state.read().expect("progress lock").clone();
        let current = self.current.read().expect("dataset lock").clone();
        let dir = self.data_dir.join("indexes");
        let mut generations = Vec::new();
        if !self.data_dir.as_os_str().is_empty()
            && let Ok(entries) = std::fs::read_dir(dir)
        {
            for entry in entries.flatten() {
                if entry.file_type()?.is_dir()
                    && entry
                        .path()
                        .join(INDEX_VERSION)
                        .join("manifest.json")
                        .is_file()
                {
                    generations.push(entry.file_name().to_string_lossy().into_owned());
                }
            }
        }
        generations.sort();
        generations.reverse();
        let next = (offset + 100 < generations.len()).then(|| (offset + 100).to_string());
        Ok(
            json!({"generation_id":current.as_ref().map(|d|&d.generation_id),"stage":progress.stage,"completed":progress.completed,"total":progress.total,"message":progress.message,"overview":current.as_ref().map(|d|d.overview.clone()),"generations":generations.into_iter().skip(offset).take(100).collect::<Vec<_>>(),"next_cursor":next}),
        )
    }
}

pub async fn load_dataset(root: &Path, id: &str) -> Result<Dataset> {
    ensure!(
        !root.as_os_str().is_empty(),
        "equipment data directory is not configured"
    );
    let generation = manifest::safe_child(root, &format!("generations/{id}/export"))?;
    let index = manifest::safe_child(root, &format!("indexes/{id}/{INDEX_VERSION}"))?;
    let seal: Value = serde_json::from_slice(&manifest::read(&index, "manifest.json")?)?;
    ensure!(
        seal["generation_id"].as_str() == Some(id) && seal["index_version"] == INDEX_VERSION,
        "index identity mismatch"
    );
    let bytes = manifest::read(&generation, "manifest.json")?;
    ensure!(
        seal["manifest_sha256"].as_str() == Some(&manifest::digest(&bytes)),
        "export manifest changed"
    );
    let manifest: Manifest = serde_json::from_slice(&bytes)?;
    ensure!(manifest.generation_id == id, "manifest identity mismatch");
    let definitions = Arc::new(super::source::gameplay::definitions(
        &generation,
        manifest.dataset_kind == "gameplay",
    )?);
    let native_types = if manifest.dataset_kind == "gameplay" {
        serde_json::from_slice(&manifest::read(&generation, "native_types.json")?)?
    } else {
        let metadata: Value =
            serde_json::from_slice(&manifest::read(&generation, "generation.json")?)?;
        metadata["type_hierarchy"].clone()
    };
    let path = manifest::safe_child(&index, "catalog.sqlite")?;
    let hash_path = path.clone();
    let index_digest =
        tokio::task::spawn_blocking(move || super::importing::publication::file_digest(&hash_path))
            .await??;
    ensure!(
        seal["index_sha256"].as_str() == Some(&index_digest),
        "index digest mismatch"
    );
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(path)
                .read_only(true)
                .immutable(true),
        )
        .await?;
    let summary: String = sqlx::query_scalar("SELECT value FROM summary WHERE key='overview'")
        .fetch_one(&pool)
        .await?;
    Ok(Dataset {
        native_types,
        definitions,
        generation_id: id.into(),
        root: generation,
        pool,
        manifest,
        overview: serde_json::from_str(&summary)?,
    })
}
