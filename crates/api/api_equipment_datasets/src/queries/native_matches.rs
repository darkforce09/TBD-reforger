//! Accessory compatibility by native type: the resources whose attachment or magazine well types
//! fit the queried resource, under the committed native matching policy.
use super::super::Dataset;
use super::{ViewerQuery, page};
use crate::error::{Required, Result, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{QueryBuilder, Row, Sqlite};

#[derive(Deserialize)]
struct Rule {
    id: String,
    accepting_class: String,
    providing_class: String,
    property: String,
}
#[derive(Deserialize)]
struct Policy {
    rules: Vec<Rule>,
}

fn inherits(types: &Value, class: &str, base: &str) -> bool {
    class == base
        || types["types"][class]["ancestor_types"]
            .as_array()
            .is_some_and(|a| a.iter().any(|v| v == base))
}

/// One page of the resources whose attachment or magazine well native types match the queried
/// resource's, under the native matching policy of
/// `contracts/rules/equipment-gameplay/native-matching.json`.
pub async fn list(dataset: &Dataset, query: &ViewerQuery) -> Result<Value> {
    let policy: Policy = serde_json::from_str(include_str!(
        "../../../../../contracts/rules/equipment-gameplay/native-matching.json"
    ))?;
    let source=sqlx::query("SELECT r.resource_id,r.resource_name,n.node_id,n.class_name,e.property,t.class_name native_type FROM resources r JOIN nodes n ON n.resource=r.id JOIN edges e ON e.node=n.id JOIN nodes t ON t.id=e.target WHERE r.resource_id=? AND n.view='effective' AND e.relationship='property' AND e.property IN ('AttachmentType','MagazineWell') ORDER BY n.ordinal,e.ordinal")
        .bind(query.resource()?).fetch_all(&dataset.pool).await?;
    ensure!(
        source.len() <= 512,
        "too many matching rules in one resource"
    );
    let mut offset = query.offset()?;
    let mut total = 0;
    let mut items = Vec::new();
    for row in source {
        let class: String = row.get("class_name");
        let property: String = row.get("property");
        let native_type: String = row.get("native_type");
        for rule in policy.rules.iter().filter(|r| r.property == property) {
            let accepts = inherits(&dataset.native_types, &class, &rule.accepting_class);
            let provides = inherits(&dataset.native_types, &class, &rule.providing_class);
            if !accepts && !provides {
                continue;
            }
            let target_base = if accepts {
                &rule.providing_class
            } else {
                &rule.accepting_class
            };
            let hierarchy = dataset.native_types["types"]
                .as_object()
                .required("native hierarchy")?;
            let owner_classes: Vec<_> = hierarchy
                .keys()
                .filter(|c| inherits(&dataset.native_types, c, target_base))
                .collect();
            let native_classes: Vec<_> = hierarchy
                .keys()
                .filter(|c| {
                    if accepts {
                        inherits(&dataset.native_types, c, &native_type)
                    } else {
                        inherits(&dataset.native_types, &native_type, c)
                    }
                })
                .collect();
            if owner_classes.is_empty() || native_classes.is_empty() {
                continue;
            }
            let from = " FROM resources r JOIN nodes n ON n.resource=r.id JOIN edges e ON e.node=n.id JOIN nodes t ON t.id=e.target";
            let filters = |q: &mut QueryBuilder<Sqlite>| {
                q.push(" WHERE n.view='effective' AND e.relationship='property' AND e.property=")
                    .push_bind(property.clone());
                q.push(" AND n.class_name IN (");
                let mut owner = q.separated(",");
                for c in &owner_classes {
                    owner.push_bind((*c).clone());
                }
                owner.push_unseparated(")");
                q.push(" AND t.class_name IN (");
                let mut native = q.separated(",");
                for c in &native_classes {
                    native.push_bind((*c).clone());
                }
                native.push_unseparated(")");
            };
            let mut count = QueryBuilder::new(format!("SELECT COUNT(*){from}"));
            filters(&mut count);
            let count: i64 = count.build_query_scalar().fetch_one(&dataset.pool).await?;
            total += count;
            if offset >= count as usize {
                offset -= count as usize;
                continue;
            }
            if items.len() >= 100 {
                continue;
            }
            let mut candidates = QueryBuilder::new(format!(
                "SELECT r.resource_id,r.resource_name,n.node_id,e.property,t.class_name native_type{from}"
            ));
            filters(&mut candidates);
            candidates
                .push(" ORDER BY r.id,n.ordinal,e.ordinal LIMIT ")
                .push_bind((100 - items.len()) as i64)
                .push(" OFFSET ")
                .push_bind(offset as i64);
            offset = 0;
            for candidate in candidates.build().fetch_all(&dataset.pool).await? {
                let evidence = json!({"rule":rule.id,"result":"native_type_match","installation_verified":false,"source_type":native_type,"target_type":candidate.get::<String,_>("native_type"),"source_role":if accepts{"accepts"}else{"provides"},"target_node_id":candidate.get::<String,_>("node_id"),"target_property":property,"limitations":["Occupied slots, exclusions, adapters and conditional installation restrictions require evaluating the complete installation configuration."]});
                items.push(json!({"resource_id":row.get::<String,_>("resource_id"),"resource_name":row.get::<String,_>("resource_name"),"node_id":row.get::<String,_>("node_id"),"property":property,"target_resource_id":candidate.get::<String,_>("resource_id"),"target_resource_name":candidate.get::<String,_>("resource_name"),"kind":"native_type_match","method":"exported native type hierarchy","view":"effective","evidence_json":evidence.to_string()}));
            }
        }
    }
    page(
        dataset.generation_id.as_str(),
        total,
        query.offset()?,
        items,
    )
}
