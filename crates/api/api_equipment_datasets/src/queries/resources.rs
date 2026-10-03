//! Catalog searches combine native identities, component classes and field definitions.
use super::super::Dataset;
use super::{ViewerQuery, page};
use crate::error::Result;
use serde_json::{Value, json};
use sqlx::{QueryBuilder, Row, Sqlite};

fn filters(query: &mut QueryBuilder<Sqlite>, params: &ViewerQuery) {
    query.push(" WHERE 1=1");
    if let Some(id) = &params.resource_id {
        query.push(" AND r.resource_id=").push_bind(id);
    }
    if let Some(domain) = params.domain.as_ref().filter(|s| !s.is_empty()) {
        query
            .push(" AND EXISTS(SELECT 1 FROM json_each(r.domains) WHERE value=")
            .push_bind(domain)
            .push(")");
    }
    if let Some(cap) = params.capability.as_ref().filter(|s| !s.is_empty()) {
        query
            .push(
                " AND EXISTS(SELECT 1 FROM capabilities c WHERE c.resource=r.id AND c.capability=",
            )
            .push_bind(cap)
            .push(")");
    }
    if let Some(q) = params.q.as_ref().filter(|s| !s.is_empty()) {
        let pattern = format!("%{}%", q.replace('%', "\\%").replace('_', "\\_"));
        query.push(" AND (r.label LIKE ").push_bind(pattern.clone()).push(" ESCAPE '\\' OR r.resource_name LIKE ").push_bind(pattern.clone()).push(" ESCAPE '\\' OR r.resource_id LIKE ").push_bind(pattern.clone()).push(" ESCAPE '\\' OR r.id IN (SELECT resource FROM nodes WHERE class_name LIKE ").push_bind(pattern.clone()).push(" ESCAPE '\\') OR r.id IN (SELECT DISTINCT n.resource FROM fields f JOIN occurrences o ON o.field=f.id JOIN nodes n ON n.id=o.node WHERE f.property LIKE ").push_bind(pattern).push(" ESCAPE '\\'))");
    }
}

/// One page of the catalog's resources matching the query's search text, domain, capability and
/// class filters.
pub async fn list(dataset: &Dataset, params: &ViewerQuery) -> Result<Value> {
    let offset = params.offset()?;
    let mut count = QueryBuilder::new("SELECT COUNT(*) FROM resources r");
    filters(&mut count, params);
    let total: i64 = count.build_query_scalar().fetch_one(&dataset.pool).await?;
    let mut q = QueryBuilder::new("SELECT r.* FROM resources r");
    filters(&mut q, params);
    q.push(" ORDER BY r.label COLLATE NOCASE,r.resource_id LIMIT 50 OFFSET ")
        .push_bind(offset as i64);
    let rows = q.build().fetch_all(&dataset.pool).await?;
    let mut items = Vec::new();
    for r in rows {
        items.push(json!({"resource_id":r.get::<String,_>("resource_id"),"resource_name":r.get::<String,_>("resource_name"),"label":r.get::<String,_>("label"),"domains":serde_json::from_str::<Value>(r.get::<&str,_>("domains"))?,"capabilities":serde_json::from_str::<Value>(r.get::<&str,_>("capabilities"))?,"source_addons":serde_json::from_str::<Value>(r.get::<&str,_>("source_addons"))?,"node_count":r.get::<i64,_>("node_count"),"fact_count":r.get::<i64,_>("fact_count")}));
    }
    page(dataset.generation_id.as_str(), total, offset, items)
}
