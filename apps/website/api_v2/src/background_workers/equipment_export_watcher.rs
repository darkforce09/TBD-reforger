//! Schedule verified publication imports independently of HTTP startup.
use crate::community_content::services::equipment_data_viewer::{
    EquipmentDatasets, importing::generation_import,
};
use std::{sync::Arc, time::Duration};

pub fn start(catalogs: Arc<EquipmentDatasets>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(error) = generation_import::initialize(&catalogs.diagnostic).await {
            catalogs
                .diagnostic
                .progress("error", 0, 0, Some(format!("{error:#}")));
        }
        let service = catalogs.gameplay.clone();
        if let Err(error) = generation_import::initialize(&service).await {
            service.progress("error", 0, 0, Some(format!("{error:#}")));
            tracing::error!(%error, "equipment dataset recovery failed");
        }
        let mut delay = 5;
        loop {
            match generation_import::poll(service.clone()).await {
                Ok(()) => delay = 5,
                Err(error) => {
                    service.progress("error", 0, 0, Some(format!("{error:#}")));
                    tracing::warn!(%error, retry_seconds=delay, "equipment export import failed");
                    delay = (delay * 2).min(300);
                }
            }
            tokio::time::sleep(Duration::from_secs(delay)).await;
        }
    })
}
