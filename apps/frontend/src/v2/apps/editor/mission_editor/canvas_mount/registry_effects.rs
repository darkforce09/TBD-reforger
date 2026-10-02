//! Loads registry and compatibility data for the active editor session.

use super::*;
use leptos::task::spawn_local;

/// Loads registry and compatibility data from the session cache or API.
pub(super) fn install(
    auth: crate::v2::core::auth::AuthStore,
    registry_fetch_gen: RwSignal<u64>,
    registry_items: RwSignal<Option<Vec<crate::v2::core::api::dto::RegistryItem>>>,
    registry_failed: RwSignal<bool>,
    vehicle_catalog: RwSignal<crate::v2::apps::editor::arsenal::asset_catalog::CatalogState>,
    catalog: RwSignal<crate::v2::apps::editor::arsenal::asset_catalog::CatalogState>,
    compat: RwSignal<crate::v2::apps::editor::arsenal::rules::CompatFeed>,
) {
    {
        use crate::v2::apps::editor::arsenal::asset_catalog::{
            build_vehicle_catalog_tree, CatalogState,
        };
        Effect::new(move |_| {
            let gen = registry_fetch_gen.get();
            if gen == 0 {
                if !registry_session::must_fetch_registry() {
                    if let Some(items) = registry_session::cached_registry() {
                        registry_items.set(Some(items.clone()));
                        registry_failed.set(false);
                        vehicle_catalog
                            .set(CatalogState::Ready(build_vehicle_catalog_tree(&items)));
                    }
                    return;
                }
            } else {
                registry_failed.set(false);
                catalog.set(CatalogState::Loading);
                vehicle_catalog.set(CatalogState::Loading);
            }
            spawn_local({
                async move {
                    match fetch_registry_pages(auth).await {
                        Ok(items) => {
                            registry_session::store_registry(items.clone());
                            registry_items.set(Some(items.clone()));
                            registry_failed.set(false);
                            vehicle_catalog
                                .set(CatalogState::Ready(build_vehicle_catalog_tree(&items)));
                        }
                        Err(_) => {
                            mark_registry_fetch_failed(catalog, vehicle_catalog, registry_failed);
                        }
                    }
                }
            });
        });
    }

    {
        use crate::v2::apps::editor::arsenal::rules::{CompatFeed, CompatGraph, CompatStatus};
        if registry_session::must_fetch_compat() {
            spawn_local({
                async move {
                    match fetch_compat_cold(auth).await {
                        Ok((feed, cargo)) => {
                            registry_session::store_compat(feed.clone(), cargo.clone());
                            engine_ops::set_cargo_defaults(cargo);
                            compat.set(feed);
                        }
                        Err(_) => {
                            compat.set(CompatFeed {
                                status: CompatStatus::Unavailable,
                                graph: CompatGraph::default(),
                            });
                        }
                    }
                }
            });
        } else if let Some((feed, cargo)) = registry_session::cached_compat() {
            engine_ops::set_cargo_defaults(cargo);
            compat.set(feed);
        }
    }
}
