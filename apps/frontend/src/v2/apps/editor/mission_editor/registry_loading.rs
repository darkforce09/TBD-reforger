//! Registry loading.
use super::*;

#[cfg(target_arch = "wasm32")]
const REGISTRY_COLD_PAGE: i64 = 500;

#[cfg(target_arch = "wasm32")]
const EDITOR_COMPAT_EDGE_TYPES: &str = "optic_on_weapon,mag_in_weapon,attachment_on_weapon";

#[cfg(target_arch = "wasm32")]
/// Fetches registry rows in bounded pages until the catalog is complete.
pub(super) async fn fetch_registry_pages(
    auth: crate::v2::core::auth::AuthStore,
) -> Result<Vec<crate::v2::core::api::dto::RegistryItem>, crate::v2::core::api::client::ApiErr> {
    use crate::v2::core::api::dto::{RegistryItem, RegistryResponse};

    let mut all: Vec<RegistryItem> = Vec::new();
    let mut offset: i64 = 0;
    loop {
        let path = format!("/registry?limit={REGISTRY_COLD_PAGE}&offset={offset}");
        let page: RegistryResponse = crate::v2::core::api::client::api_get(auth, &path).await?;
        let n = page.data.len() as i64;
        let total = page.total.unwrap_or(offset + n);
        all.extend(page.data);
        offset += n;
        if n == 0 || offset >= total {
            break;
        }
        if n > REGISTRY_COLD_PAGE {
            break;
        }
    }
    Ok(all)
}

#[cfg(target_arch = "wasm32")]
/// Fetches compatibility edges and cargo defaults for the arsenal.
pub(super) async fn fetch_compat_cold(
    auth: crate::v2::core::auth::AuthStore,
) -> Result<
    (
        crate::v2::apps::editor::arsenal::rules::CompatFeed,
        std::collections::HashMap<String, Vec<crate::v2::apps::editor::arsenal::rules::CargoRow>>,
    ),
    crate::v2::core::api::client::ApiErr,
> {
    use crate::v2::apps::editor::arsenal::rules::{
        CargoRow, CompatFeed, CompatGraph, CompatStatus,
    };
    use crate::v2::core::api::dto::{RegistryCargoDefaultsResponse, RegistryCompatResponse};
    use std::collections::HashMap;

    let edges_path = format!("/registry/compat?edge_type={EDITOR_COMPAT_EDGE_TYPES}");
    let edges: RegistryCompatResponse =
        crate::v2::core::api::client::api_get(auth, &edges_path).await?;
    let cargo_resp: RegistryCargoDefaultsResponse =
        crate::v2::core::api::client::api_get(auth, "/registry/compat?view=cargo_defaults").await?;

    let mut cargo: HashMap<String, Vec<CargoRow>> = HashMap::new();
    for (character, rows) in cargo_resp.data {
        cargo.insert(
            character,
            rows.into_iter()
                .map(|r| CargoRow {
                    container: r.container,
                    item: r.item,
                    qty: r.qty,
                })
                .collect(),
        );
    }

    let feed = CompatFeed {
        status: CompatStatus::Ready,
        graph: CompatGraph::from_edges(&edges.data),
    };
    Ok((feed, cargo))
}
