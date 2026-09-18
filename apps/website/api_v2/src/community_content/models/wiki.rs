//! Doctrine knowledgebase models: the markdown SOP page and the vehicle IFF table row.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::wire_format::go_time;

/// SOP / manual document (markdown body).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WikiPage {
    pub id: Uuid,
    pub slug: String,
    pub category: String,
    pub title: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub icon: String,
    pub body_md: String,
    pub nav_order: i64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub updated_by: Option<String>,
    #[serde(with = "go_time")]
    pub updated_at: DateTime<Utc>,
}

/// Structured IFF table row on the Vehicle Database wiki page.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct VehicleDatabase {
    pub id: Uuid,
    pub name: String,
    pub faction: String,
    pub armor_type: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub amphibious: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub primary_threat: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub profile_image_url: String,
}
