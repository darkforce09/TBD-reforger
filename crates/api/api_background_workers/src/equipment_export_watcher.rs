//! Schedule verified publication imports independently of HTTP startup.
//!
//! **Role:** restores both equipment datasets at boot, then imports each new gameplay publication.
//! **Position:** armed by [`crate::worker_set::spawn_all`] with the state's
//! `api_equipment_datasets::EquipmentDatasets`; the imports are
//! `api_equipment_datasets::importing::generation_import`.
//! **Signals & state:** one Tokio task owning the datasets `Arc`; the import progress lives in the
//! datasets.
//! **Invariants:** the poll runs every 5 seconds, doubling after each failed import up to 300
//! seconds and back to 5 after a success; a failure is recorded as the dataset's progress as well
//! as logged.

use api_equipment_datasets::{EquipmentDatasets, importing::generation_import};
use api_foundation::error_handling::error_causes::message_with_causes;
use std::{sync::Arc, time::Duration};

/// Spawn the watcher: restore the diagnostic and gameplay datasets from their stored
/// generations, then poll the gameplay export source for a new publication, every 5 seconds after
/// a success and with a doubling delay of at most 300 seconds after a failure.
pub fn start(catalogs: Arc<EquipmentDatasets>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(error) = generation_import::initialize(&catalogs.diagnostic).await {
            catalogs
                .diagnostic
                .progress("error", 0, 0, Some(message_with_causes(&error)));
        }
        let service = catalogs.gameplay.clone();
        if let Err(error) = generation_import::initialize(&service).await {
            service.progress("error", 0, 0, Some(message_with_causes(&error)));
            tracing::error!(%error, "equipment dataset recovery failed");
        }
        let mut delay = 5;
        loop {
            match generation_import::poll(service.clone()).await {
                Ok(()) => delay = 5,
                Err(error) => {
                    service.progress("error", 0, 0, Some(message_with_causes(&error)));
                    tracing::warn!(%error, retry_seconds=delay, "equipment export import failed");
                    delay = (delay * 2).min(300);
                }
            }
            tokio::time::sleep(Duration::from_secs(delay)).await;
        }
    })
}
