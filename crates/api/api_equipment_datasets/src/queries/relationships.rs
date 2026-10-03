//! Forward and reverse reference views preserve every native occurrence.
use super::super::Dataset;
use super::{ViewerQuery, page};
use crate::error::Result;
use serde_json::{Value, json};
use sqlx::{QueryBuilder, Row, Sqlite};

const FROM: &str = " FROM resource_references x JOIN resources r ON r.id=x.resource JOIN nodes n ON n.id=x.node LEFT JOIN resources t ON t.id=x.target";
fn filters(q: &mut QueryBuilder<Sqlite>, p: &ViewerQuery) -> Result<()> {
    q.push(if p.direction.as_deref() == Some("incoming") {
        " WHERE t.resource_id="
    } else {
        " WHERE r.resource_id="
    })
    .push_bind(p.resource()?);
    if p.view() != "all" {
        q.push(" AND n.view=").push_bind(p.view());
    }
    if let Some(kind) = &p.kind {
        q.push(" AND x.kind=").push_bind(kind);
    }
    if let Some(node) = &p.node_id {
        q.push(" AND n.node_id=").push_bind(node);
    }
    Ok(())
}
/// One page of the references from or to the queried resource, every native occurrence kept;
/// `kind=native_type_match` answers [`super::native_matches::list`] instead.
pub async fn list(d: &Dataset, p: &ViewerQuery) -> Result<Value> {
    if p.kind.as_deref() == Some("native_type_match") {
        return super::native_matches::list(d, p).await;
    }
    let offset = p.offset()?;
    let mut count = QueryBuilder::new(format!("SELECT COUNT(*){FROM}"));
    filters(&mut count, p)?;
    let total: i64 = count.build_query_scalar().fetch_one(&d.pool).await?;
    let mut q = QueryBuilder::new(format!(
        "SELECT r.resource_id,r.resource_name,n.node_id,n.view,x.property,x.target_resource_name,x.kind,x.method,t.resource_id target_resource_id{FROM}"
    ));
    filters(&mut q, p)?;
    q.push(" ORDER BY x.id LIMIT 100 OFFSET ")
        .push_bind(offset as i64);
    let rows = q.build().fetch_all(&d.pool).await?;
    let items=rows.iter().map(|r|json!({"resource_id":r.get::<String,_>("resource_id"),"resource_name":r.get::<String,_>("resource_name"),"node_id":r.get::<String,_>("node_id"),"property":r.get::<String,_>("property"),"target_resource_id":r.get::<Option<String>,_>("target_resource_id"),"target_resource_name":r.get::<String,_>("target_resource_name"),"kind":r.get::<String,_>("kind"),"method":r.get::<String,_>("method"),"view":r.get::<String,_>("view")})).collect();
    page(d.generation_id.as_str(), total, offset, items)
}
