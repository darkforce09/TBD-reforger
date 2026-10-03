//! The gameplay catalog's field selection report: why each native field was kept or left out.
use super::super::{Dataset, EquipmentDataService};
use super::{ViewerQuery, page, values};
use crate::error::{Required, Result, ensure};
use serde_json::{Value, json, value::RawValue};
use std::{collections::BTreeMap, sync::Arc};

/// One page of the gameplay catalog's field selection decisions, filtered by disposition and
/// search text, with the decision counts and the publication receipt's figures.
pub async fn list(
    service: Arc<EquipmentDataService>,
    dataset: Arc<Dataset>,
    query: ViewerQuery,
) -> Result<Value> {
    ensure!(
        dataset.manifest.dataset_kind == "gameplay",
        "selection policy applies to gameplay catalogs"
    );
    let _permit = service.readers.clone().acquire_owned().await?;
    let bytes = service.document(&dataset, "selection_report.json").await?;
    let receipt = if dataset
        .manifest
        .files
        .contains_key("publication_receipt.json")
    {
        Some(
            service
                .document(&dataset, "publication_receipt.json")
                .await?,
        )
    } else {
        None
    };
    tokio::task::spawn_blocking(move || {
        let report: Value = serde_json::from_slice(&bytes)?;
        let decisions = report["decisions"].as_array().required("field selection report")?;
        let mut counts = BTreeMap::<String,usize>::new();
        for decision in decisions {*counts.entry(decision["disposition"].as_str().required("disposition")?.into()).or_default() += 1;}
        let needle = query.q.as_deref().unwrap_or("").to_lowercase();
        let selected: Vec<_> = decisions.iter().filter(|d|query.kind.as_deref().is_none_or(|kind|kind==d["disposition"].as_str().unwrap_or("")))
            .filter(|d|needle.is_empty() || d.to_string().to_lowercase().contains(&needle)).collect();
        let offset=query.offset()?;
        let items = selected.iter().skip(offset).take(100).map(|decision| {
            let key=format!("{}.{}",decision["class_name"].as_str().unwrap_or(""),decision["property"].as_str().unwrap_or(""));
            let mut item=values::entry(&key,&RawValue::from_string(decision.to_string())?)?;
            item["class_name"]=decision["class_name"].clone();
            item["native_type"]=decision["native_type"].clone();
            Ok(item)
        }).collect::<Result<Vec<_>>>()?;
        let mut result=page(dataset.generation_id.as_str(),selected.len() as i64,offset,items)?;
        result["resource_id"]=json!("");result["node_id"]=Value::Null;
        let receipt: Value = receipt.as_deref().map(|bytes|serde_json::from_slice(bytes)).transpose()?.unwrap_or(Value::Null);
        result["node_metadata_json"]=json!(json!({"policy_version":report["policy_version"],"policy_sha256":report["policy_sha256"],"counts":counts,"retained_facts":report["fact_count"],"retained_containers":report["node_count"],"unreviewed":report["unreviewed"],"excluded_dependency_count":receipt["excluded_diagnostic_dependencies"].as_array().map(Vec::len),"fact_bytes_by_section":receipt["retained_fact_json_bytes_by_section"],"structure_metadata_and_indexes_bytes":receipt["structure_metadata_and_indexes_bytes"],"has_publication_receipt":!receipt.is_null()}).to_string());
        Ok(result)
    }).await?
}
