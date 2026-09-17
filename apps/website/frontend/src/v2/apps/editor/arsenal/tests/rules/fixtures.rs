use super::*;

pub(super) fn edge(from: &str, to: &str, ty: &str) -> RegistryCompatEdge {
    RegistryCompatEdge {
        id: String::new(),
        modpack_id: String::new(),
        from_node: from.into(),
        to_node: to.into(),
        edge_type: ty.into(),
        evidence: String::new(),
        qty: 1,
        created_at: String::new(),
        updated_at: String::new(),
    }
}

pub(super) fn item(rn: &str, name: &str, kind: &str) -> RegistryItem {
    RegistryItem {
        id: String::new(),
        modpack_id: String::new(),
        resource_name: rn.into(),
        display_name: name.into(),
        category: String::new(),
        icon_url: None,
        kind: kind.into(),
        r#abstract: None,
        arsenal_type: None,
        weight_kg: None,
        volume_cm3: None,
        max_weight_kg: None,
        max_volume_cm3: None,
        cargo_grid_w: None,
        cargo_grid_h: None,
        addon: None,
        variant_of: None,
        sort_order: 0,
        created_at: String::new(),
        updated_at: String::new(),
    }
}

pub(super) fn picks(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}
