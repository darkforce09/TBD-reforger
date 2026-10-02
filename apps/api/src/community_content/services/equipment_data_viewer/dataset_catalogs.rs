use super::EquipmentDataService;
use anyhow::{Result, bail};
use std::{path::PathBuf, sync::Arc};

pub struct EquipmentDatasets {
    pub gameplay: Arc<EquipmentDataService>,
    pub diagnostic: Arc<EquipmentDataService>,
}

impl EquipmentDatasets {
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

    pub fn select(&self, kind: Option<&str>) -> Result<Arc<EquipmentDataService>> {
        match kind.unwrap_or("gameplay") {
            "gameplay" => Ok(self.gameplay.clone()),
            "diagnostic" => Ok(self.diagnostic.clone()),
            _ => bail!("unsupported dataset kind"),
        }
    }
}
