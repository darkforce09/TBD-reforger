//! Build a complete index in staging before making it queryable.
use super::super::{
    EquipmentDataService,
    source::{
        document::Snapshot,
        gameplay,
        manifest::{self, Manifest, ResourceEntry},
        resource_labels,
    },
};
use super::resource_writer;
use crate::error::{Required, Result, ensure};
use serde_json::{Value, json};
use sqlx::{Connection, QueryBuilder, Sqlite, SqliteConnection, sqlite::SqliteConnectOptions};
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

pub(super) type FieldKey = (String, String, String);
pub(super) struct BuildState {
    pub next_node: i64,
    pub fields: HashMap<FieldKey, i64>,
    pub aliases: BTreeSet<(i64, String, String)>,
    pub resources: HashMap<String, i64>,
    pub fact_count: u64,
}

/// Builds the SQLite navigation index of the complete generation at `root` into `database`,
/// recording progress on `service`, and returns the overview summary it stored.
pub async fn build(
    root: &Path,
    database: &Path,
    manifest: &Manifest,
    service: &EquipmentDataService,
) -> Result<Value> {
    let bytes = manifest::read(root, "generation.json")?;
    let generation: Value = serde_json::from_slice(&bytes)?;
    let compact = manifest.dataset_kind == "gameplay";
    ensure!(
        generation["schema_version"] == manifest.schema_version
            && generation["scope"] == "complete"
            && generation["status"] == "completed",
        "only complete supported exports can be imported"
    );
    ensure!(
        generation["generation_id"].as_str() == Some(manifest.generation_id.as_str()),
        "generation identity mismatch"
    );
    ensure!(
        generation["errors"].as_array().is_some_and(Vec::is_empty),
        "generation contains extraction errors"
    );
    let entries: Vec<ResourceEntry> = gameplay::entries(root, &generation, compact)?;
    let definitions = std::sync::Arc::new(gameplay::definitions(root, compact)?);
    let selection: Option<Value> = if compact {
        let report: Value =
            serde_json::from_slice(&manifest::read(root, "selection_report.json")?)?;
        ensure!(
            report["policy_version"] == 1
                && report["unreviewed"].as_array().is_some_and(Vec::is_empty),
            "gameplay policy contains unreviewed fields"
        );
        Some(report)
    } else {
        None
    };
    ensure!(
        entries.len() as u64 == manifest.resource_count,
        "resource count mismatch"
    );
    let mut expected: BTreeSet<String> = ["generation.json".to_owned()].into();
    if compact {
        expected.extend(
            [
                "resource_index.json",
                "field_definitions.json",
                "native_types.json",
                "selection_report.json",
            ]
            .map(str::to_owned),
        );
    }
    let receipt = compact && manifest.files.contains_key("publication_receipt.json");
    if receipt {
        expected.insert("publication_receipt.json".into());
    }
    for entry in &entries {
        expected.insert(entry.record_file.clone());
        expected.insert(entry.source_file.clone());
    }
    ensure!(
        expected == manifest.files.keys().cloned().collect(),
        "manifest and resource inventory differ"
    );
    ensure!(
        expected.len()
            == if compact {
                entries.len() + 5 + usize::from(receipt)
            } else {
                entries.len() * 2 + 1
            },
        "reused resource document paths"
    );
    let mut db = SqliteConnection::connect_with(
        &SqliteConnectOptions::new()
            .filename(database)
            .create_if_missing(true),
    )
    .await?;
    sqlx::query("PRAGMA foreign_keys=OFF")
        .execute(&mut db)
        .await?;
    sqlx::raw_sql(include_str!("schema.sql"))
        .execute(&mut db)
        .await?;
    let resources: HashMap<_, _> = entries
        .iter()
        .enumerate()
        .map(|(i, r)| (r.resource_name.clone(), i as i64 + 1))
        .collect();
    ensure!(resources.len() == entries.len(), "duplicate resource names");
    let mut state = BuildState {
        next_node: 1,
        fields: HashMap::new(),
        aliases: BTreeSet::new(),
        resources,
        fact_count: 0,
    };
    let mut counts = HashMap::<String, u64>::new();
    let mut cap_counts = std::collections::BTreeMap::<String, u64>::new();
    for (ordinal, entry) in entries.iter().enumerate() {
        let root = root.to_path_buf();
        let source_file = entry.source_file.clone();
        let record_file = entry.record_file.clone();
        let definitions = definitions.clone();
        let (source, record) = tokio::task::spawn_blocking(move || -> Result<(Snapshot, Value)> {
            let bytes = manifest::read(&root, &source_file)?;
            let source = gameplay::snapshot(&bytes, &definitions)?;
            let record = if compact {
                gameplay::record(&bytes, &source)?
            } else {
                serde_json::from_slice(&manifest::read(&root, &record_file)?)?
            };
            Ok((source, record))
        })
        .await??;
        ensure!(
            source.schema_version == manifest.schema_version
                && source.resource_id == entry.resource_id
                && record["resource_id"].as_str() == Some(entry.resource_id.as_str()),
            "resource identity mismatch"
        );
        let caps = record["capabilities"]
            .as_object()
            .required("missing capabilities")?;
        let capabilities: Vec<_> = caps.keys().cloned().collect();
        for cap in &capabilities {
            *cap_counts.entry(cap.clone()).or_default() += 1;
        }
        for domain in &entry.domains {
            *counts.entry(domain.clone()).or_default() += 1;
        }
        let id = ordinal as i64 + 1;
        let fact_count: usize = source.nodes.iter().map(|n| n.properties.len()).sum();
        let mut tx = db.begin().await?;
        sqlx::query("INSERT INTO resources VALUES (?,?,?,?,?,?,?,?,?,?,?)")
            .bind(id)
            .bind(&entry.resource_id)
            .bind(&entry.resource_name)
            .bind(resource_labels::resource_label(
                &record,
                &source,
                entry.domains.iter().any(|d| d == "vehicle"),
            ))
            .bind(serde_json::to_string(&entry.domains)?)
            .bind(serde_json::to_string(&capabilities)?)
            .bind(record["source_addons"].to_string())
            .bind(&entry.record_file)
            .bind(&entry.source_file)
            .bind(source.nodes.len() as i64)
            .bind(fact_count as i64)
            .execute(&mut *tx)
            .await?;
        resource_writer::write(&mut tx, id, &source, &record, &mut state).await?;
        tx.commit().await?;
        #[cfg(test)]
        service.test_event("indexing")?;
        service.progress("indexing", ordinal as u64 + 1, entries.len() as u64, None);
    }
    let fields: Vec<_> = state.fields.iter().collect();
    for chunk in fields.chunks(400) {
        let mut query =
            QueryBuilder::<Sqlite>::new("INSERT INTO fields (id,class_name,property,native_type) ");
        query.push_values(chunk, |mut row, (key, id)| {
            row.push_bind(**id)
                .push_bind(&key.0)
                .push_bind(&key.1)
                .push_bind(&key.2);
        });
        query.build().execute(&mut db).await?;
    }
    let aliases: Vec<_> = state.aliases.iter().collect();
    for chunk in aliases.chunks(500) {
        let mut query = QueryBuilder::<Sqlite>::new("INSERT INTO aliases ");
        query.push_values(chunk, |mut row, a| {
            row.push_bind(a.0).push_bind(&a.1).push_bind(&a.2);
        });
        query.build().execute(&mut db).await?;
    }
    service.progress(
        "finalizing",
        entries.len() as u64,
        entries.len() as u64,
        None,
    );
    sqlx::raw_sql("CREATE INDEX occurrence_field ON occurrences(field,node); CREATE INDEX node_resource_view ON nodes(resource,view,id); CREATE INDEX reference_target ON resource_references(target,id); CREATE INDEX reference_source ON resource_references(resource,id); CREATE INDEX edge_source ON edges(node,ordinal); CREATE INDEX capability_name ON capabilities(capability,resource); CREATE INDEX field_name ON fields(property,class_name);
        UPDATE fields SET effective_count=(SELECT COUNT(*) FROM occurrences o JOIN nodes n ON n.id=o.node WHERE o.field=fields.id AND n.view='effective'), ancestor_count=(SELECT COUNT(*) FROM occurrences o JOIN nodes n ON n.id=o.node WHERE o.field=fields.id AND n.view='ancestor'), resource_count=(SELECT COUNT(DISTINCT n.resource) FROM occurrences o JOIN nodes n ON n.id=o.node WHERE o.field=fields.id);").execute(&mut db).await?;
    ensure!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM occurrences")
            .fetch_one(&mut db)
            .await? as u64
            == state.fact_count,
        "fact coverage mismatch"
    );
    ensure!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM resource_references WHERE kind='gameplay' AND target IS NULL"
        )
        .fetch_one(&mut db)
        .await?
            == 0,
        "gameplay relationship not closed"
    );
    let summary = json!({"resources":entries.len(),"nodes":state.next_node-1,"facts":state.fact_count,"fields":state.fields.len(),"equipment":counts.get("equipment").unwrap_or(&0),"vehicles":counts.get("vehicle").unwrap_or(&0),"dependencies":counts.get("dependency").unwrap_or(&0),"bytes":manifest.files.values().map(|f|f.bytes).sum::<u64>(),"capabilities":cap_counts});
    if let Some(selection) = selection {
        ensure!(
            selection["resource_count"] == summary["resources"]
                && selection["node_count"] == summary["nodes"]
                && selection["fact_count"] == summary["facts"],
            "gameplay coverage counts differ from imported records"
        );
    }
    sqlx::query("INSERT INTO summary VALUES ('overview',?)")
        .bind(summary.to_string())
        .execute(&mut db)
        .await?;
    ensure!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&mut db)
            .await?
            .is_empty(),
        "index contains dangling keys"
    );
    let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_one(&mut db)
        .await?;
    ensure!(integrity == "ok", "index integrity check failed");
    db.close().await?;
    Ok(summary)
}
