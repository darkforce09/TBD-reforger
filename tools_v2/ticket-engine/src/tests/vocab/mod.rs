use super::*;

use crate::Domain;

const MINI: &str =
    "[repo.docs]\n\n[website.frontend]\nmission_creator = [\"map_canvas\", \"toolbelt\"]\n";

fn scope(domain: Domain, layer: &str, component: Option<&str>, surface: &[&str]) -> ScopeV2 {
    ScopeV2 {
        domain,
        layer: layer.into(),
        component: component.map(str::to_string),
        surface: surface.iter().map(|s| (*s).to_string()).collect(),
    }
}

mod vocabulary_resolution_tests;
