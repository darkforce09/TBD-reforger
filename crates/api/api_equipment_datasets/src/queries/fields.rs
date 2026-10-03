//! Dataset-wide field definitions and exact source occurrences.
use super::super::Dataset;
use super::{ViewerQuery, page};
use crate::error::Result;
use serde_json::{Value, json};
use sqlx::{QueryBuilder, Row, Sqlite};

fn filters(q: &mut QueryBuilder<Sqlite>, p: &ViewerQuery) {
    q.push(" WHERE 1=1");
    if let Some(class) = &p.class_name {
        q.push(" AND f.class_name=").push_bind(class);
    }
    if let Some(cap) = p.capability.as_ref().filter(|s| !s.is_empty()) {
        q.push(" AND EXISTS (SELECT 1 FROM aliases a WHERE a.field=f.id AND a.capability=")
            .push_bind(cap)
            .push(")");
    }
    if let Some(text) = p.q.as_ref().filter(|s| !s.is_empty()) {
        q.push(" AND (f.property LIKE ")
            .push_bind(format!("%{text}%"))
            .push(" OR f.class_name LIKE ")
            .push_bind(format!("%{text}%"))
            .push(")");
    }
}

/// One page of the dataset's field definitions with their aliases and units, or, when the query
/// names a field, one page of that field's exact source occurrences.
pub async fn list(d: &Dataset, p: &ViewerQuery) -> Result<Value> {
    if p.field_id.is_some() {
        return occurrences(d, p).await;
    }
    let offset = p.offset()?;
    let mut count = QueryBuilder::new("SELECT COUNT(*) FROM fields f");
    filters(&mut count, p);
    let total: i64 = count.build_query_scalar().fetch_one(&d.pool).await?;
    let mut q = QueryBuilder::new("SELECT f.* FROM fields f");
    filters(&mut q, p);
    q.push(" ORDER BY f.class_name,f.property,f.native_type LIMIT 100 OFFSET ")
        .push_bind(offset as i64);
    let rows = q.build().fetch_all(&d.pool).await?;
    let mut items = Vec::new();
    for r in rows {
        let id: i64 = r.get("id");
        let aliases: Vec<(String, String)> = sqlx::query_as(
            "SELECT capability,alias FROM aliases WHERE field=? ORDER BY capability,alias",
        )
        .bind(id)
        .fetch_all(&d.pool)
        .await?;
        let units:Vec<String>=sqlx::query_scalar("SELECT DISTINCT unit FROM occurrences WHERE field=? AND unit IS NOT NULL ORDER BY unit").bind(id).fetch_all(&d.pool).await?;
        let caps: std::collections::BTreeSet<_> = aliases.iter().map(|a| a.0.clone()).collect();
        items.push(json!({"field_id":id,"class_name":r.get::<String,_>("class_name"),"property":r.get::<String,_>("property"),"native_type":r.get::<String,_>("native_type"),"effective_count":r.get::<i64,_>("effective_count"),"ancestor_count":r.get::<i64,_>("ancestor_count"),"resource_count":r.get::<i64,_>("resource_count"),"aliases":aliases.iter().map(|a|format!("{}.{}",a.0,a.1)).collect::<Vec<_>>(),"capabilities":caps,"units":units}));
    }
    let mut result = page(d.generation_id.as_str(), total, offset, items)?;
    result["occurrences"] = json!([]);
    Ok(result)
}

async fn occurrences(d: &Dataset, p: &ViewerQuery) -> Result<Value> {
    let offset = p.offset()?;
    let field = p.field_id.expect("field id");
    let view = p.view();
    let total:i64=sqlx::query_scalar("SELECT COUNT(*) FROM occurrences o JOIN nodes n ON n.id=o.node WHERE o.field=? AND (?='all' OR n.view=?)").bind(field).bind(view).bind(view).fetch_one(&d.pool).await?;
    let rows=sqlx::query("SELECT r.resource_id,r.resource_name,r.label,n.node_id,n.view,f.property,o.status,o.origin FROM occurrences o JOIN nodes n ON n.id=o.node JOIN resources r ON r.id=n.resource JOIN fields f ON f.id=o.field WHERE o.field=? AND (?='all' OR n.view=?) ORDER BY n.id LIMIT 100 OFFSET ?").bind(field).bind(view).bind(view).bind(offset as i64).fetch_all(&d.pool).await?;
    let items=rows.iter().map(|r|json!({"resource_id":r.get::<String,_>("resource_id"),"resource_name":r.get::<String,_>("resource_name"),"label":r.get::<String,_>("label"),"node_id":r.get::<String,_>("node_id"),"property":r.get::<String,_>("property"),"view":r.get::<String,_>("view"),"origin":r.get::<String,_>("origin"),"status":r.get::<String,_>("status")})).collect();
    let mut result = page(d.generation_id.as_str(), total, offset, items)?;
    result["occurrences"] = result["items"].take();
    result["items"] = json!([]);
    Ok(result)
}
