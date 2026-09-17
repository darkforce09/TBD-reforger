//! Classification.

use super::*;
use anyhow::Context;

/// Map one v1 `[scope]` Value onto the v2 shape. `Err` = an UNMAPPED shape — the
/// migrator collects these and refuses the whole run before writing.
pub(super) fn map_scope(id: &str, scope: &toml::Value, owns: &[String]) -> Result<MappedScope> {
    let table = scope
        .as_table()
        .with_context(|| format!("{id}: [scope] is not a table"))?;
    if table.len() != 1 {
        bail!(
            "{id}: expected exactly one [scope.*] table, got {}",
            table.len()
        );
    }
    let (domain_key, dv) = table.iter().next().expect("len checked");
    let deterministic = |domain, layer: &str, component: Option<&str>, surface: Vec<String>| {
        Ok(MappedScope {
            domain,
            layer: layer.to_string(),
            component: component.map(str::to_string),
            surface,
            owns_inferred: false,
        })
    };
    match domain_key.as_str() {
        "website" => {
            let wt = dv
                .as_table()
                .with_context(|| format!("{id}: [scope.website] is not a table"))?;
            if wt.len() != 1 {
                bail!("{id}: expected exactly one [scope.website.*] table");
            }
            let (kind, kv) = wt.iter().next().expect("len checked");
            let kt = kv
                .as_table()
                .with_context(|| format!("{id}: [scope.website.{kind}] is not a table"))?;
            match kind.as_str() {
                "editor" => {
                    if kt.contains_key("capability") {
                        bail!("{id}: editor capability is populated — unmapped (0 measured)");
                    }
                    let chrome = str_array(kt.get("chrome"));
                    if !chrome.is_empty() {
                        let mut surface = Vec::new();
                        for c in &chrome {
                            let mapped = CHROME_MAP
                                .iter()
                                .find(|(k, _)| k == c)
                                .map(|(_, s)| (*s).to_string())
                                .with_context(|| format!("{id}: unknown chrome value {c}"))?;
                            if !surface.contains(&mapped) {
                                surface.push(mapped);
                            }
                        }
                        return deterministic(
                            "website",
                            "frontend",
                            Some("mission_creator"),
                            surface,
                        );
                    }
                    let (component, surface) = infer_editor(owns);
                    Ok(MappedScope {
                        domain: "website",
                        layer: "frontend".into(),
                        component: Some(component),
                        surface,
                        owns_inferred: true,
                    })
                }
                "page" => {
                    let surface = kt
                        .get("route")
                        .and_then(|r| r.as_str())
                        .and_then(|route| {
                            let seg = route.trim_start_matches('/').split('/').next()?;
                            if ROUTE_SURFACES.contains(&seg) {
                                Some(vec![seg.to_string()])
                            } else if matches!(seg, "auth" | "login") {
                                Some(vec!["auth_pages".to_string()])
                            } else {
                                None
                            }
                        })
                        .unwrap_or_default();
                    deterministic("website", "frontend", Some("site_pages"), surface)
                }
                "shell" => deterministic("website", "frontend", Some("shell"), vec![]),
                "backend" => {
                    let layers = str_array(kt.get("layers"));
                    let first = layers
                        .first()
                        .with_context(|| format!("{id}: backend without layers"))?;
                    let component = BACKEND_COMPONENTS
                        .iter()
                        .find(|(k, _)| k == first)
                        .map(|(_, c)| *c)
                        .with_context(|| format!("{id}: unknown backend layer {first}"))?;
                    deterministic("website", "backend", Some(component), vec![])
                }
                "tests" => deterministic("website", "tests", None, vec![]),
                other => bail!("{id}: unmapped website scope kind {other}"),
            }
        }
        "mod" => {
            let layers = str_array(dv.as_table().and_then(|t| t.get("layers")));
            let first = layers
                .first()
                .with_context(|| format!("{id}: mod without layers"))?;
            if let Some((_, layer, component)) = MOD_LAYERS.iter().find(|(k, _, _)| k == first) {
                return deterministic("mod", layer, *component, vec![]);
            }
            if first != "feature" {
                bail!("{id}: unmapped mod layer {first}");
            }
            let (layer, component) = infer_mod(owns);
            Ok(MappedScope {
                domain: "mod",
                layer,
                component,
                surface: vec![],
                owns_inferred: true,
            })
        }
        "schema" | "engine" => {
            let layers = str_array(dv.as_table().and_then(|t| t.get("layers")));
            let first = layers
                .first()
                .with_context(|| format!("{id}: {domain_key} without layers"))?;
            let domain = if domain_key == "schema" {
                "schema"
            } else {
                "engine"
            };
            deterministic(domain, first, None, vec![])
        }
        "repo" => {
            let layers = str_array(dv.as_table().and_then(|t| t.get("layers")));
            let first = layers
                .first()
                .with_context(|| format!("{id}: repo without layers"))?;
            if first == "xtask" {
                return Ok(MappedScope {
                    domain: "repo",
                    layer: "xtask".into(),
                    component: infer_xtask(owns),
                    surface: vec![],
                    owns_inferred: true,
                });
            }
            deterministic("repo", first, None, vec![])
        }
        other => bail!("{id}: unmapped scope domain {other}"),
    }
}
