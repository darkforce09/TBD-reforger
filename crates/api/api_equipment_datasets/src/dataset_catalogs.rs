use super::EquipmentDataService;
use crate::error::{Error, Result};
use std::{path::PathBuf, sync::Arc};

/// The two equipment dataset kinds the API serves: the gameplay catalog, imported from the export
/// source, and the diagnostic dataset, read where it lies in the data directory.
pub struct EquipmentDatasets {
    /// The gameplay catalog, under `<data_dir>/gameplay`.
    pub gameplay: Arc<EquipmentDataService>,
    /// The diagnostic dataset, at the data directory's root; it imports nothing.
    pub diagnostic: Arc<EquipmentDataService>,
}

impl EquipmentDatasets {
    /// Both services over `data_dir`; the gameplay catalog imports from `<source>/gameplay`. An empty
    /// `data_dir` leaves both unconfigured.
    pub fn new(data_dir: impl Into<PathBuf>, source: Option<PathBuf>) -> Self {
        let root = data_dir.into();
        let gameplay_root = if root.as_os_str().is_empty() {
            root.clone()
        } else {
            root.join("gameplay")
        };
        Self {
            gameplay: Arc::new(EquipmentDataService::new(
                gameplay_root,
                source.as_ref().map(|p| p.join("gameplay")),
            )),
            diagnostic: Arc::new(EquipmentDataService::new(root, None)),
        }
    }

    /// The service a request's `dataset` parameter names: `gameplay` (the default) or `diagnostic`.
    pub fn select(&self, kind: Option<&str>) -> Result<Arc<EquipmentDataService>> {
        match kind.unwrap_or("gameplay") {
            "gameplay" => Ok(self.gameplay.clone()),
            "diagnostic" => Ok(self.diagnostic.clone()),
            _ => Err(Error::check_failed("unsupported dataset kind")),
        }
    }
}
