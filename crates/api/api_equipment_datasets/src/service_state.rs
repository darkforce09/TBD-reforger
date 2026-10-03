//! Generation-pinned query state and bounded source-document residency.
use super::{
    INDEX_VERSION,
    source::manifest::{self, Manifest},
};
use crate::error::{Required, Result, ensure};
use api_identifiers::EquipmentGenerationId;
use serde_json::{Value, json};
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tokio::sync::{Mutex, Semaphore};

/// One imported generation opened for reading: its verified manifest, its sealed index and the
/// field definitions and native type tree its queries resolve against.
pub struct Dataset {
    /// The native type hierarchy: `native_types.json` of a gameplay dataset, the `type_hierarchy`
    /// of `generation.json` otherwise.
    pub native_types: Value,
    /// The gameplay field definitions, keyed by field name; empty for a diagnostic dataset.
    pub definitions: Arc<super::source::gameplay::FieldDefinitions>,
    /// The generation this dataset answers from.
    pub generation_id: EquipmentGenerationId,
    /// The generation's `export` folder, the root every manifest path resolves under.
    pub root: PathBuf,
    /// The read-only, immutable pool over the generation's sealed `catalog.sqlite`.
    pub pool: SqlitePool,
    /// The export manifest whose digest the index seal names.
    pub manifest: Manifest,
    /// The overview summary the index build stored.
    pub overview: Value,
}

/// Where the running or last import stands, as the status route reports it.
#[derive(Clone, Default)]
pub struct ImportProgress {
    /// The import stage (`starting`, `ready`, or the step in progress).
    pub stage: String,
    /// Units of the stage done.
    pub completed: u64,
    /// Units of the stage in all; `0` when the stage counts none.
    pub total: u64,
    /// The failure of the last import with its causes, or a note on the stage.
    pub message: Option<String>,
}

type CachedDocument = (String, Arc<Vec<u8>>);

#[cfg(test)]
type ImportTestHook = Box<dyn FnMut(&str) -> Result<()> + Send>;

/// One dataset kind's imports and reads: the data directory, the export source it imports from,
/// the current generation, up to four further loaded generations and a document cache of at most
/// 64 MiB.
pub struct EquipmentDataService {
    #[cfg(test)]
    /// A hook the import calls at each stage, so a test can fail or observe a stage.
    pub test_hook: std::sync::Mutex<Option<ImportTestHook>>,
    /// The local data directory the generations and indexes live in; empty when unconfigured.
    pub data_dir: PathBuf,
    /// The export source directory the imports read published generations from.
    pub source_dir: Option<PathBuf>,
    /// The generation `latest` names, once one is imported.
    pub current: RwLock<Option<Arc<Dataset>>>,
    /// Where the running or last import stands.
    pub progress_state: RwLock<ImportProgress>,
    datasets: Mutex<BTreeMap<EquipmentGenerationId, Arc<Dataset>>>,
    cache: Mutex<VecDeque<CachedDocument>>,
    /// The permits that bound concurrent reads to two.
    pub readers: Arc<Semaphore>,
}

impl EquipmentDataService {
    /// A service over `data_dir` importing from `source`, with no generation loaded.
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
    /// Runs the test hook for `stage`, if one is set.
    pub fn test_event(&self, stage: &str) -> Result<()> {
        if let Some(hook) = self.test_hook.lock().unwrap().as_mut() {
            hook(stage)?;
        }
        Ok(())
    }

    /// Records where the import stands.
    pub fn progress(&self, stage: &str, completed: u64, total: u64, message: Option<String>) {
        *self.progress_state.write().expect("progress lock") = ImportProgress {
            stage: stage.into(),
            completed,
            total,
            message,
        };
    }

    /// The current generation, or `None` before the first import.
    pub fn current_id(&self) -> Option<EquipmentGenerationId> {
        self.current
            .read()
            .expect("dataset lock")
            .as_ref()
            .map(|d| d.generation_id.clone())
    }

    /// The generation `id` names (`latest` is the current one), loaded and verified on first use;
    /// at most four generations besides the current one stay loaded.
    pub async fn dataset(&self, id: &EquipmentGenerationId) -> Result<Arc<Dataset>> {
        let id = if id == "latest" {
            self.current_id().required("no dataset imported")?
        } else {
            id.clone()
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

    /// Loads and verifies generation `id` and makes it current.
    pub async fn activate(&self, id: &EquipmentGenerationId) -> Result<()> {
        let dataset = Arc::new(load_dataset(&self.data_dir, id).await?);
        *self.current.write().expect("dataset lock") = Some(dataset);
        self.progress("ready", 0, 0, None);
        Ok(())
    }

    /// The bytes of the manifest file `relative` of `dataset`, checked against the size and digest
    /// its manifest records; documents of at most 64 MiB stay in the shared cache.
    pub async fn document(&self, dataset: &Dataset, relative: &str) -> Result<Arc<Vec<u8>>> {
        let expected = dataset
            .manifest
            .files
            .get(relative)
            .required("document is not in manifest")?
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

    /// The status answer: the current generation and its overview, the import progress and one page
    /// of up to 100 indexed generations, newest first, from `offset`.
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

/// Opens generation `id` under `root`: the index seal must name this generation, this index
/// version, the export manifest's digest and the index file's digest, or the load fails.
pub(crate) async fn load_dataset(root: &Path, id: &EquipmentGenerationId) -> Result<Dataset> {
    ensure!(
        !root.as_os_str().is_empty(),
        "equipment data directory is not configured"
    );
    let generation = manifest::safe_child(root, &format!("generations/{id}/export"))?;
    let index = manifest::safe_child(root, &format!("indexes/{id}/{INDEX_VERSION}"))?;
    let seal: Value = serde_json::from_slice(&manifest::read(&index, "manifest.json")?)?;
    ensure!(
        seal["generation_id"].as_str() == Some(id.as_str())
            && seal["index_version"] == INDEX_VERSION,
        "index identity mismatch"
    );
    let bytes = manifest::read(&generation, "manifest.json")?;
    ensure!(
        seal["manifest_sha256"].as_str() == Some(&manifest::digest(&bytes)),
        "export manifest changed"
    );
    let manifest: Manifest = serde_json::from_slice(&bytes)?;
    ensure!(manifest.generation_id == *id, "manifest identity mismatch");
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
        generation_id: id.clone(),
        root: generation,
        pool,
        manifest,
        overview: serde_json::from_str(&summary)?,
    })
}
